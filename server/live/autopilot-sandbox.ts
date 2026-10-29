// Shared containment for every adapter. Agents cannot see the host home, network, PID space, Herdr socket,
// Shepherdr store, or gateway policy. Descendant processes remain in the same namespaces.
import { Transform } from 'node:stream'
import { createHash } from 'node:crypto'
import { createServer, type Server } from 'node:http'
import { connect, isIP, type Socket } from 'node:net'
import { lookup } from 'node:dns/promises'
import { appendFileSync, copyFileSync, existsSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { homedir, tmpdir } from 'node:os'
import { HOME } from '../store.ts'
import { ROOT } from '../root.ts'
import { BOX, q } from './box.ts'
import { reserveTraffic, pilotCommand, pilotProblem, type PilotRun } from './autopilot.ts'
import type { Spec } from './agents.ts'

export interface SandboxLaunch { run?: string; gateway?: string; wrapper: string; env: Record<string, string>; logsHome: string; captures?: string; close: () => Promise<void> }
export function publicAddress(ip: string) {
  if (isIP(ip) === 6) return !/^(::|fe[89ab]|f[cd]|2001:db8)/i.test(ip) && !ip.includes('.')
  if (isIP(ip) !== 4) return false
  const [a, b] = ip.split('.').map(Number)
  return !(a === 0 || a === 10 || a === 127 || a === 169 && b === 254 || a === 172 && b >= 16 && b <= 31 || a === 192 && (b === 168 || b === 0) || a === 100 && b >= 64 && b <= 127 || a >= 224)
}
export function sandboxArgs(o: { cwd: string; repo: string; home: string; state: string; control: string; binary: string; kind: string; bun: string; writable: boolean; git?: string[] }) {
  const a = ['--die-with-parent', '--new-session', '--unshare-all', '--cap-drop', 'ALL', '--clearenv']
  for (const p of ['/usr', '/bin', '/lib', '/lib64']) if (existsSync(p)) a.push('--ro-bind', p, p)
  a.push('--dir', '/etc')
  for (const p of ['/etc/ssl', '/etc/pki', '/etc/ca-certificates', '/etc/nsswitch.conf', '/etc/passwd', '/etc/group', '/etc/hosts', '/etc/resolv.conf']) if (existsSync(p)) a.push('--ro-bind', p, p)
  a.push('--proc', '/proc', '--dev', '/dev', '--tmpfs', '/tmp', '--dir', '/run', '--dir', '/run/shepherdr')
  a.push('--bind', o.state, o.home, '--bind', o.state, o.state, '--ro-bind', o.control, o.control)
  if (o.repo !== o.cwd) a.push('--ro-bind', o.repo, o.repo)
  a.push(o.writable ? '--bind' : '--ro-bind', o.cwd, o.cwd)
  // Git metadata is host-owned. A worker saves a commit with the checked checkpoint broker operation.
  for (const p of o.git ?? []) a.push('--ro-bind', p, p)
  a.push('--ro-bind', o.control, '/run/shepherdr', '--ro-bind', o.bun, '/run/shepherdr/bun')
  // Native Claude/Codex binaries, or pi's package (its script needs its relative modules).
  if (o.kind === 'pi') { const modules = o.binary.includes('/node_modules/') ? o.binary.slice(0, o.binary.indexOf('/node_modules/') + 14) : dirname(dirname(o.binary)); a.push('--ro-bind', modules, modules) }
  a.push('--ro-bind', o.binary, `/run/shepherdr/${o.kind}`)
  for (const [k, v] of Object.entries({ HOME: o.home, USER: 'agent', TERM: 'xterm-256color', PATH: '/run/shepherdr/bin:/usr/local/bin:/usr/bin:/bin', SHEPHERDR_CHAT: '', NODE_USE_ENV_PROXY: '1', SHEPHERDR_COMMAND: '/run/shepherdr/bin/shepherdr', CODEX_HOME: join(o.home, '.codex'), PI_CODING_AGENT_DIR: join(o.home, '.pi/agent'), CLAUDE_CONFIG_DIR: join(o.home, '.claude') })) a.push('--setenv', k, v)
  a.push('--chdir', o.cwd, '/run/shepherdr/bun', '/run/shepherdr/bootstrap.ts', o.kind === 'pi' ? o.binary : `/run/shepherdr/${o.kind}`)
  return a
}
export function capturedTranscript(root: string, kind: string, file: string) {
  const uuid = file.match(/([0-9a-f]{8}-[0-9a-f-]{27})\.jsonl$/i)?.[1]
  const name = createHash('sha256').update(file).digest('hex').slice(0, 24) + (uuid ? `-${uuid}` : '') + '.jsonl'
  return join(root, kind === 'claude' ? '.claude/projects' : kind === 'codex' ? '.codex/sessions' : '.pi/agent/sessions', file.includes('/subagents/') ? 'subagents' : '', name)
}
function privateDestination(root: string, file: string) {
  const parts = file.slice(root.length + 1).split('/')
  if (!file.startsWith(root + '/') || parts.includes('..')) throw new Error('Invalid private runtime path.')
  let current = root
  for (const part of parts) {
    current = join(current, part)
    try { if (lstatSync(current).isSymbolicLink()) throw new Error('Private runtime settings contain a symlink. Start a fresh Autopilot run.') }
    catch (e) { if ((e as NodeJS.ErrnoException).code !== 'ENOENT') throw e }
  }
}

export function privateRuntimePath(home: string, file: string) {
  try { const root = realpathSync(home), actual = realpathSync(file); return actual.startsWith(root + '/') ? actual : undefined }
  catch { return undefined }
}

export function killSandbox(control: string) {
  try {
    const pid = Number(readFileSync(join(control, 'pid'), 'utf8'))
    const argv = readFileSync(`/proc/${pid}/cmdline`, 'utf8').split('\0')
    if (pid > 1 && argv.includes(control) && argv.some((v) => /(?:^|\/)(?:bwrap|timeout)$/.test(v))) process.kill(pid, 'SIGKILL')
  } catch { /* already gone; never kill a reused PID */ }
}
function noHardlinks(root: string) {
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    if (entry.name === '.git') continue
    const p = join(root, entry.name)
    if (entry.isDirectory()) noHardlinks(p)
    else if (entry.isFile() && lstatSync(p).nlink > 1) throw new Error('The workspace contains shared hardlinks. Copy those files before using isolated Autopilot.')
  }
}

export async function sandboxAvailable() {
  if (BOX || process.platform !== 'linux') throw new Error('Isolated Autopilot currently needs a local Linux runtime. No unrestricted fallback was started.')
  const bwrap = Bun.which('bwrap')
  if (!bwrap) throw new Error('Install bubblewrap for isolated Autopilot. No unrestricted fallback was started.')
  const p = Bun.spawn([bwrap, '--unshare-all', '--ro-bind', '/', '/', '--', '/bin/true'], { stdout: 'ignore', stderr: 'pipe' })
  if (await p.exited) throw new Error('This machine cannot create the required Linux namespaces. Autopilot was not started.')
  return bwrap
}

/** Keep provider configuration, not hooks, MCP servers, plugins or another machine's command executors. */
export function providerConfig(kind: string, text: string) {
  if (kind === 'codex') {
    const input = Bun.TOML.parse(text) as Record<string, unknown>
    const out: Record<string, unknown> = {}
    for (const k of ['model_provider', 'model', 'model_reasoning_effort', 'forced_login_method', 'openai_base_url']) if (input[k] !== undefined) out[k] = input[k]
    const providers = input.model_providers as Record<string, unknown> | undefined
    const selected = typeof input.model_provider === 'string' ? input.model_provider : 'openai'
    if (providers?.[selected]) out.model_providers = { [selected]: providers[selected] }
    const toml = (value: unknown): string => Array.isArray(value) ? `[${value.map(toml).join(', ')}]` : value && typeof value === 'object' ? `{ ${Object.entries(value).filter(([, v]) => v !== null && v !== undefined).map(([k,v]) => `${JSON.stringify(k)} = ${toml(v)}`).join(', ')} }` : JSON.stringify(value)
    return Object.entries(out).map(([k,v]) => `${k} = ${toml(v)}`).join('\n') + '\n[features]\napps = false\nplugins = false\n'
  }
  const input = JSON.parse(text) as Record<string, unknown>
  if (kind === 'claude') {
    const env = Object.fromEntries(Object.entries(input.env ?? {}).filter(([k]) => /^ANTHROPIC_/.test(k)))
    return JSON.stringify({ env })
  }
  return JSON.stringify(Object.fromEntries(Object.entries(input).filter(([k]) => ['model', 'provider', 'thinkingLevel'].includes(k))))
}

export async function prepareSandbox(spec: Spec, run: PilotRun, repo: string, cli: (args: string[], ctx: { chat: string; cwd: string; stdin?: string }) => Promise<{ out: string; code: number; secrets?: Record<string, string> }>, reviewerKind?: string, fixture?: { providerHome: string; executable: (kind: string) => string | null }): Promise<SandboxLaunch> {
  const problem = pilotProblem(spec.kind) ?? (reviewerKind ? pilotProblem(reviewerKind) : undefined)
  if (problem) throw new Error(problem)
  const bwrap = await sandboxAvailable()
  if (spec.key.includes('/')) noHardlinks(spec.cwd)
  const executable = fixture ? fixture.executable(spec.kind) : Bun.which(spec.kind)
  if (!executable) throw new Error(`${spec.kind} is not installed.`)
  const binary = realpathSync(executable)
  const root = join(HOME, 'autopilot-runtime', run.id, spec.key.replace(/[^\w.-]/g, '_'))
  const captures = join(root, 'captures')
  const state = join(root, 'home'), control = join(root, 'control'), bin = join(root, 'launcher')
  rmSync(control, { recursive: true, force: true }); rmSync(bin, { recursive: true, force: true })
  for (const p of [state, captures, control, bin, join(control, 'bin')]) mkdirSync(p, { recursive: true, mode: 0o700 })
  for (const file of ['bun', spec.kind]) writeFileSync(join(control, file), '')
  // Only the chosen providers' own configuration/login files. Never mount the real home or arbitrary accounts.
  const kinds = [...new Set([spec.kind, reviewerKind].filter((k): k is string => !!k))]
  for (const kind of kinds) {
    const configs = kind === 'codex' ? ['.codex/auth.json', '.codex/config.toml'] : kind === 'claude' ? ['.claude/.credentials.json', '.claude/settings.json'] : ['.pi/agent/auth.json', '.pi/agent/models.json', '.pi/agent/settings.json']
    for (const name of configs) {
      const src = join(fixture?.providerHome ?? homedir(), name), dst = join(state, name)
      privateDestination(state, dst)
      if (!existsSync(src) || existsSync(dst)) continue
      mkdirSync(dirname(dst), { recursive: true, mode: 0o700 })
      if (name.endsWith('config.toml') || name.endsWith('settings.json')) writeFileSync(dst, providerConfig(kind, readFileSync(src, 'utf8')), { mode: 0o600 })
      else copyFileSync(src, dst)
    }
  }
  // Scripts are copied once to the read-only control mount; worktree edits cannot change trusted broker code.
  copyFileSync(join(ROOT, 'helper/autopilot/bootstrap.ts'), join(control, 'bootstrap.ts'))
  const result = await Bun.build({ entrypoints: [join(ROOT, 'server/cli.ts')], target: 'bun' })
  if (!result.success) throw new Error('Could not prepare the isolated command helper.')
  writeFileSync(join(control, 'cli.js'), await result.outputs[0].text(), { mode: 0o400 })
  writeFileSync(join(control, 'bin/shepherdr'), '#!/bin/sh\nexec /run/shepherdr/bun /run/shepherdr/cli.js "$@"\n', { mode: 0o500 })
  const sockets = new Set<Socket>()
  const received = new Map<string, number>()
  const server: Server = createServer(async (req, res) => {
    try {
      if (req.method !== 'POST' || !['/api/cli', '/api/transcript'].includes(req.url ?? '')) throw new Error('Only the authenticated Shepherdr command is available.')
      let body = ''
      for await (const chunk of req) { body += chunk; if (body.length > 1_000_000) throw new Error('Command is too large.') }
      if (req.url === '/api/transcript') {
        if (!['starting', 'on', 'workers'].includes(run.state)) throw new Error('Autopilot stopped.')
        const capture = JSON.parse(body) as { kind: string; file: string; data: string; offset: number }
        if (!['claude', 'codex', 'pi'].includes(capture.kind) || typeof capture.file !== 'string' || capture.file.length > 2048 || typeof capture.data !== 'string' || !Number.isSafeInteger(capture.offset) || capture.offset < 0) throw new Error('Invalid transcript.')
        const data = Buffer.from(capture.data, 'base64')
        if (data.length > 256 * 1024) throw new Error('Transcript chunk is too large.')
        const file = capturedTranscript(captures, capture.kind, capture.file)
        const offset = received.get(file) ?? (existsSync(file) ? statSync(file).size : 0)
        if (capture.offset > offset) throw new Error('Transcript has a gap; retry its earlier chunk.')
        const fresh = data.subarray(Math.max(0, offset - capture.offset))
        if (fresh.length) { mkdirSync(dirname(file), { recursive: true, mode: 0o700 }); appendFileSync(file, fresh, { mode: 0o600 }) }
        received.set(file, offset + fresh.length)
        res.end('{}'); return
      }
      const input = JSON.parse(body) as { args: string[]; stdin?: string }
      if (!Array.isArray(input.args) || input.args.some((x) => typeof x !== 'string')) throw new Error('Invalid command.')
      pilotCommand(run, spec.key, input.args)
      // The runtime, never a caller-supplied chat/cwd/token, selects the authenticated remote command identity.
      const answer = await cli(input.args, { chat: spec.key, cwd: spec.cwd, stdin: input.stdin })
      res.setHeader('content-type', 'application/json'); res.end(JSON.stringify(answer))
    } catch (e) { res.statusCode = 400; res.setHeader('content-type', 'application/json'); res.end(JSON.stringify({ code: 1, out: `${(e as Error).message}\n` })) }
  })
  server.on('connection', (s) => { sockets.add(s); s.on('close', () => sockets.delete(s)) })
  server.on('connect', async (req, client, head) => {
    try {
      const [host, port] = (req.url ?? '').toLowerCase().split(':')
      if (!['starting', 'on', 'workers'].includes(run.state) || port !== '443' || !run.config.hosts.includes(host)) throw new Error('Host is outside the permission profile.')
      const addresses = await lookup(host, { all: true })
      if (!addresses.length || addresses.some((a) => !publicAddress(a.address))) throw new Error('Local/private network access is blocked.')
      const upstream = connect({ host: addresses[0].address, port: 443 })
      sockets.add(upstream); upstream.on('close', () => sockets.delete(upstream))
      upstream.on('connect', () => { client.write('HTTP/1.1 200 Connection Established\r\n\r\n'); if (head.length) { try { reserveTraffic(run, head.length); upstream.write(head) } catch { upstream.destroy(); client.destroy(); return } }
        const meter = () => new Transform({ transform(chunk, _encoding, done) { try { reserveTraffic(run, chunk.length); done(null, chunk) } catch (e) { done(e as Error) } } })
        const outgoing = meter(), incoming = meter()
        outgoing.on('error', () => { upstream.destroy(); client.destroy() }); incoming.on('error', () => { upstream.destroy(); client.destroy() })
        client.pipe(outgoing).pipe(upstream); upstream.pipe(incoming).pipe(client) })
      upstream.on('error', () => client.destroy()); client.on('error', () => upstream.destroy()); client.on('close', () => upstream.destroy())
    } catch { client.end('HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\n') }
  })
  const gatewayDir = mkdtempSync(join(tmpdir(), 'shep-pilot-gateway-'))
  const socket = join(gatewayDir, 'gateway.sock')
  writeFileSync(join(control, 'gateway.sock'), '')
  rmSync(socket, { force: true })
  await new Promise<void>((resolve, reject) => { server.once('error', reject); server.listen(socket, resolve) })
  try {
  const git: string[] = []
  for (const cwd of [...new Set([repo, spec.cwd])]) {
    const dot = join(cwd, '.git')
    if (!existsSync(dot)) continue
    git.push(dot)
    try {
      const text = readFileSync(dot, 'utf8')
      if (text.startsWith('gitdir: ')) {
        const dir = resolve(cwd, text.slice(8).trim()); git.push(dir)
        const common = join(dir, 'commondir')
        if (existsSync(common)) git.push(resolve(dir, readFileSync(common, 'utf8').trim()))
      }
    } catch { /* ordinary directory */ }
  }
  const args = sandboxArgs({ cwd: realpathSync(spec.cwd), repo: realpathSync(repo), home: homedir(), state, control, binary, bun: realpathSync(process.execPath), kind: spec.kind, writable: spec.key.includes('/'), git: [...new Set(git)].sort((a, b) => a.length - b.length) })
  const start = args.indexOf('--chdir')
  args.splice(start, 0, '--setenv', 'SHEPHERDR_CHAT', spec.key, '--ro-bind', socket, '/run/shepherdr/gateway.sock')
  for (const kind of kinds) {
    const executable = fixture ? fixture.executable(kind) : Bun.which(kind)
    if (!executable) throw new Error(`${kind} reviewer is not installed.`)
    const real = realpathSync(executable)
    const target = join(control, 'bin', kind)
    if (kind === 'pi') {
      const modules = real.includes('/node_modules/') ? real.slice(0, real.indexOf('/node_modules/') + 14) : dirname(dirname(real))
      args.splice(start, 0, '--ro-bind', modules, modules)
      writeFileSync(target, `#!/bin/sh\nexec ${q(real)} "$@"\n`, { mode: 0o500 })
    } else {
      writeFileSync(target, '')
      args.splice(start, 0, '--ro-bind', real, `/run/shepherdr/bin/${kind}`)
    }
  }
  const helper = join(homedir(), '.shepherdr/helper')
  const skills = join(ROOT, 'skill')
  args.splice(start, 0, '--ro-bind', join(control, 'bin/shepherdr'), spec.env?.SHEPHERDR_COMMAND ?? join(ROOT, 'bin/shepherdr'), '--ro-bind', skills, join(helper, 'skill'))
  // Herdr starts the canonical executable through this pane's PATH. There is no fallback to a host executable.
  const wrapper = join(bin, spec.kind)
  const deadline = run.config.runtime ? run.started + run.config.runtime - Date.now() : undefined
  if (deadline !== undefined && deadline <= 0) { server.close(); throw new Error('Autopilot runtime limit reached.') }
  const command = deadline === undefined ? [bwrap, ...args] : ['/usr/bin/timeout', '--foreground', '--signal=KILL', `${Math.max(0.001, deadline / 1000)}s`, bwrap, ...args]
  // Keep the named adapter in the terminal's foreground process group: Herdr identifies
  // sh /…/claude (or codex/pi). The CLI itself stays in the new session for TIOCSTI safety.
  // Explicit stdin inheritance matters: a background shell command otherwise gets /dev/null.
  writeFileSync(wrapper, `#!/bin/sh\n${command.map(q).join(' ')} "$@" <&0 &\nchild=$!\nprintf '%s' "$child" > ${q(join(control, 'pid'))}\ntrap 'kill -KILL "$child" 2>/dev/null' HUP INT TERM\nwait "$child"\nexit $?\n`, { mode: 0o500 })
  return { run: run.id, gateway: socket, wrapper, env: { PATH: `${bin}:/usr/local/bin:/usr/bin:/bin` }, logsHome: state, captures, close: async () => { killSandbox(control); for (const s of sockets) s.destroy(); await new Promise<void>((resolve) => server.close(() => { rmSync(gatewayDir, { recursive: true, force: true }); resolve() })) } }
  } catch (e) { for (const socket of sockets) socket.destroy(); server.close(); rmSync(gatewayDir, { recursive: true, force: true }); throw e }
}

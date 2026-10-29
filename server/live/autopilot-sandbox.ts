// Shared containment for every adapter. Agents cannot see the host home, network, PID space, Herdr socket,
// Shepherdr store, or gateway policy. Descendant processes remain in the same namespaces.
import { createServer, type Server } from 'node:http'
import { connect, isIP, type Socket } from 'node:net'
import { lookup } from 'node:dns/promises'
import { copyFileSync, existsSync, mkdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { homedir } from 'node:os'
import { HOME } from '../store.ts'
import { ROOT } from '../root.ts'
import { BOX, q } from './box.ts'
import { pilotCommand, pilotProblem, type PilotRun } from './autopilot.ts'
import type { Spec } from './agents.ts'

export interface SandboxLaunch { wrapper: string; env: Record<string, string>; logsHome: string; close: () => Promise<void> }
export function publicAddress(ip: string) {
  if (isIP(ip) === 6) return !/^(::|fe[89ab]|f[cd]|2001:db8)/i.test(ip) && !ip.includes('.')
  if (isIP(ip) !== 4) return false
  const [a, b] = ip.split('.').map(Number)
  return !(a === 0 || a === 10 || a === 127 || a === 169 && b === 254 || a === 172 && b >= 16 && b <= 31 || a === 192 && (b === 168 || b === 0) || a === 100 && b >= 64 && b <= 127 || a >= 224)
}
export function sandboxArgs(o: { cwd: string; repo: string; home: string; state: string; control: string; binary: string; kind: string; bun: string; writable: boolean; git?: string[] }) {
  const a = ['--die-with-parent', '--new-session', '--unshare-all', '--cap-drop', 'ALL', '--clearenv']
  for (const p of ['/usr', '/bin', '/lib', '/lib64', '/etc']) if (existsSync(p)) a.push('--ro-bind', p, p)
  a.push('--proc', '/proc', '--dev', '/dev', '--tmpfs', '/tmp', '--dir', '/run', '--dir', '/run/shepherdr')
  a.push('--bind', o.state, o.home, '--bind', o.state, o.state, '--ro-bind', o.control, o.control)
  if (o.repo !== o.cwd) a.push('--ro-bind', o.repo, o.repo)
  a.push(o.writable ? '--bind' : '--ro-bind', o.cwd, o.cwd)
  // Git metadata is host-owned. A worker saves a commit with the checked checkpoint broker operation.
  for (const p of o.git ?? []) a.push('--ro-bind', p, p)
  a.push('--ro-bind', o.control, '/run/shepherdr', '--ro-bind', o.bun, '/run/shepherdr/bun')
  // Native Claude/Codex binaries, or pi's package (its script needs its relative modules).
  if (o.kind === 'pi') a.push('--ro-bind', dirname(dirname(o.binary)), dirname(dirname(o.binary)))
  a.push('--ro-bind', o.binary, `/run/shepherdr/${o.kind}`)
  for (const [k, v] of Object.entries({ HOME: o.home, USER: 'agent', TERM: 'xterm-256color', PATH: '/run/shepherdr/bin:/usr/local/bin:/usr/bin:/bin', SHEPHERDR_CHAT: '', SHEPHERDR_COMMAND: '/run/shepherdr/bin/shepherdr', CODEX_HOME: join(o.home, '.codex'), PI_CODING_AGENT_DIR: join(o.home, '.pi/agent'), CLAUDE_CONFIG_DIR: join(o.home, '.claude') })) a.push('--setenv', k, v)
  a.push('--chdir', o.cwd, '/run/shepherdr/bun', '/run/shepherdr/bootstrap.ts', `/run/shepherdr/${o.kind}`)
  return a
}
export async function sandboxAvailable() {
  if (BOX || process.platform !== 'linux') throw new Error('Isolated Autopilot currently needs a local Linux runtime. No unrestricted fallback was started.')
  const bwrap = Bun.which('bwrap')
  if (!bwrap) throw new Error('Install bubblewrap for isolated Autopilot. No unrestricted fallback was started.')
  const p = Bun.spawn([bwrap, '--unshare-all', '--ro-bind', '/', '/', '--', '/bin/true'], { stdout: 'ignore', stderr: 'pipe' })
  if (await p.exited) throw new Error('This machine cannot create the required Linux namespaces. Autopilot was not started.')
  return bwrap
}

export async function prepareSandbox(spec: Spec, run: PilotRun, repo: string, cli: (args: string[], ctx: { chat: string; cwd: string; stdin?: string }) => Promise<{ out: string; code: number; secrets?: Record<string, string> }>): Promise<SandboxLaunch> {
  const problem = pilotProblem(spec.kind)
  if (problem) throw new Error(problem)
  const bwrap = await sandboxAvailable()
  const executable = Bun.which(spec.kind)
  if (!executable) throw new Error(`${spec.kind} is not installed.`)
  const binary = realpathSync(executable)
  const root = join(HOME, 'autopilot-runtime', run.id, spec.key.replace(/[^\w.-]/g, '_'))
  const state = join(root, 'home'), control = join(root, 'control'), bin = join(root, 'launcher')
  for (const p of [state, control, bin, join(control, 'bin')]) mkdirSync(p, { recursive: true, mode: 0o700 })
  // Only the chosen providers' own configuration/login files. Never mount the real home or arbitrary accounts.
  const configs = spec.kind === 'codex' ? ['.codex/auth.json', '.codex/config.toml'] : spec.kind === 'claude' ? ['.claude/.credentials.json', '.claude/settings.json', '.claude.json'] : ['.pi/agent/auth.json', '.pi/agent/models.json', '.pi/agent/settings.json']
  for (const name of configs) {
    const src = join(homedir(), name), dst = join(state, name)
    if (!existsSync(src) || existsSync(dst)) continue
    mkdirSync(dirname(dst), { recursive: true, mode: 0o700 }); copyFileSync(src, dst)
  }
  // Scripts are copied once to the read-only control mount; worktree edits cannot change trusted broker code.
  copyFileSync(join(ROOT, 'helper/autopilot/bootstrap.ts'), join(control, 'bootstrap.ts'))
  const result = await Bun.build({ entrypoints: [join(ROOT, 'server/cli.ts')], target: 'bun' })
  if (!result.success) throw new Error('Could not prepare the isolated command helper.')
  writeFileSync(join(control, 'cli.js'), await result.outputs[0].text(), { mode: 0o400 })
  writeFileSync(join(control, 'bin/shepherdr'), '#!/bin/sh\nexec /run/shepherdr/bun /run/shepherdr/cli.js "$@"\n', { mode: 0o500 })
  const sockets = new Set<Socket>()
  const server: Server = createServer(async (req, res) => {
    try {
      if (req.method !== 'POST' || req.url !== '/api/cli') throw new Error('Only the authenticated Shepherdr command is available.')
      let body = ''
      for await (const chunk of req) { body += chunk; if (body.length > 1_000_000) throw new Error('Command is too large.') }
      const input = JSON.parse(body) as { args: string[]; stdin?: string }
      if (!Array.isArray(input.args) || input.args.some((x) => typeof x !== 'string')) throw new Error('Invalid command.')
      pilotCommand(run, spec.key, input.args)
      // The runtime, never a caller-supplied chat/cwd/token, selects the authenticated remote command identity.
      const answer = await cli(input.args, { chat: spec.key, cwd: spec.cwd, stdin: input.stdin })
      res.setHeader('content-type', 'application/json'); res.end(JSON.stringify(answer))
    } catch (e) { res.setHeader('content-type', 'application/json'); res.end(JSON.stringify({ code: 1, out: `${(e as Error).message}\n` })) }
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
      upstream.on('connect', () => { client.write('HTTP/1.1 200 Connection Established\r\n\r\n'); if (head.length) upstream.write(head); client.pipe(upstream).pipe(client) })
      upstream.on('error', () => client.destroy()); client.on('error', () => upstream.destroy()); client.on('close', () => upstream.destroy())
    } catch { client.end('HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\n') }
  })
  const socket = join(control, 'gateway.sock')
  rmSync(socket, { force: true })
  await new Promise<void>((resolve, reject) => { server.once('error', reject); server.listen(socket, resolve) })
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
  const helper = join(homedir(), '.shepherdr/helper')
  const skills = join(ROOT, 'skill')
  args.splice(start, 0, '--ro-bind', join(control, 'bin/shepherdr'), spec.env?.SHEPHERDR_COMMAND ?? join(ROOT, 'bin/shepherdr'), '--ro-bind', skills, join(helper, 'skill'))
  // Herdr starts the canonical executable through this pane's PATH. There is no fallback to a host executable.
  const wrapper = join(bin, spec.kind)
  writeFileSync(wrapper, `#!/bin/sh\nexec ${[bwrap, ...args].map(q).join(' ')} "$@"\n`, { mode: 0o500 })
  return { wrapper, env: { PATH: `${bin}:/usr/local/bin:/usr/bin:/bin` }, logsHome: state, close: async () => { for (const s of sockets) s.destroy(); await new Promise<void>((resolve) => server.close(() => resolve())) } }
}

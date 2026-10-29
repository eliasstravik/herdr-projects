// Putting Shepherdr's small helper on another machine, and checking that machine (shepherdr doctor). Runs on the main
// machine. A machine saved in herdr's Remote needs only herdr, git, perl and the agent CLIs: Shepherdr copies the rest
// over SSH into ~/.shepherdr/helper there, and again whenever Shepherdr's own copy of it changed (an update):
//   bin/shepherdr        the shepherdr command for agents there: it talks to Shepherdr through an SSH tunnel
//   bin/agent-browser    the pinned agent-browser for that machine's system (bin/agent-browser-bin), for the task's
//                        shared browser, marking when an agent last used it (helper/agent-browser)
//   hook.pl              records a Claude Code hook event (readers/hooks.ts)
//   mirror.pl            sends the agents' log files to the main machine as they grow (mirror.ts)
//   health.pl            how loaded the machine is (health.ts)
//   relay.pl             shares a task's browser stream on the tailnet straight from there (browser.ts)
//   skill/               Shepherdr's skills, which agents there read
import { createHash } from 'node:crypto'
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { join, relative } from 'node:path'
import { ROOT } from '../root.ts'
import { helperDir, q, sshArgs, type BoxInfo } from './box.ts'

/** a machine as herdr saved it (herdr machine list --json) */
export interface Saved {
  id: string
  label: string
  target: string
  session: string
  enabled: boolean
}

export async function herdrJson<T>(args: string[], fallback: T): Promise<T> {
  try {
    const p = Bun.spawn(['herdr', ...args], { stdout: 'pipe', stderr: 'pipe' })
    const t = setTimeout(() => p.kill(), 20_000)
    const out = await new Response(p.stdout).text()
    await p.exited
    clearTimeout(t)
    return JSON.parse(out) as T
  } catch {
    return fallback
  }
}

/** every machine saved in herdr's Remote that is turned on */
export const savedMachines = async () => (await herdrJson<Saved[]>(['machine', 'list', '--json'], [])).filter((m) => m.enabled !== false && m.label)

/** whether herdr reaches each: "reachable", or what is wrong */
export const machineStatus = async () => Object.fromEntries((await herdrJson<{ label: string; status: string; error: string | null }[]>(['machine', 'status', '--json'], [])).map((m) => [m.label, m]))

/** a command on a machine over the shared SSH connection, before a worker for it runs */
export async function onMachine(target: string, script: string, o: { stdin?: Uint8Array | string; timeoutMs?: number } = {}) {
  const p = Bun.spawn(sshArgs(target, script), { stdin: o.stdin === undefined ? 'ignore' : Buffer.from(o.stdin), stdout: 'pipe', stderr: 'pipe' })
  const t = setTimeout(() => p.kill(), o.timeoutMs ?? 60_000)
  const [out, err, code] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text(), p.exited])
  clearTimeout(t)
  return { code, out, err: err.trim() }
}

/** what a machine has: where things are, and which tools it lacks */
export interface Probe {
  home: string
  os: string
  arch: string
  path: string
  herdr: string
  tools: Record<string, boolean>
  helper: string
  /** the agent-browser there, by its version file: it is sent again only when it changed (it is large) */
  browser: string
  /** an old full install of Shepherdr there, from before (its config named a main machine) */
  oldInstall: boolean
}

const PROBE = `
echo "home=$HOME"
echo "os=$(uname -s)"
echo "arch=$(uname -m)"
lp=$("\${SHELL:-/bin/sh}" -lc 'echo "@@$PATH@@"' 2>/dev/null </dev/null | sed -n 's/.*@@\\(.*\\)@@.*/\\1/p' | tail -n 1)
[ -n "$lp" ] || lp="$PATH"
lp="$HOME/.local/bin:$HOME/.bun/bin:/opt/homebrew/bin:/usr/local/bin:$lp"
echo "path=$lp"
PATH="$lp"
for t in herdr git perl gh claude codex pi tar; do if command -v $t >/dev/null 2>&1; then echo "tool.$t=$(command -v $t)"; fi; done
perl -MJSON::PP -e1 2>/dev/null && echo "tool.jsonpp=1"
cat "$HOME/.shepherdr/helper/version" 2>/dev/null | sed 's/^/helper=/'
cat "$HOME/.shepherdr/helper/browser-version" 2>/dev/null | sed 's/^/browser=/'
grep -q '"main"' "$HOME/.shepherdr/config.json" 2>/dev/null && echo "old=1"
true
`

export async function probe(target: string): Promise<Probe> {
  const r = await onMachine(target, PROBE, { timeoutMs: 30_000 })
  if (r.code !== 0) throw new Error(r.err.split('\n').pop() || `ssh ${target} failed`)
  const kv = Object.fromEntries(
    r.out
      .split('\n')
      .map((l) => l.match(/^([\w.]+)=(.*)$/))
      .filter((m): m is RegExpMatchArray => !!m)
      .map((m) => [m[1], m[2]]),
  )
  const tools = Object.fromEntries(Object.entries(kv).filter(([k]) => k.startsWith('tool.')).map(([k, v]) => [k.slice(5), !!v]))
  return { home: kv.home, os: kv.os, arch: kv.arch, path: kv.path, herdr: kv['tool.herdr'] ?? 'herdr', tools, helper: kv.helper ?? '', browser: kv.browser ?? '', oldInstall: kv.old === '1' }
}

/** herdr's own record of where herdr is on that machine, when it has one */
export function herdrPath(id: string, fallback: string) {
  try {
    const f = join(process.env.XDG_STATE_HOME ?? join(process.env.HOME ?? '', '.local', 'state'), 'herdr', 'client', 'ssh-metadata', `${id}.json`)
    return (JSON.parse(readFileSync(f, 'utf8')) as { metadata?: { executable?: string } }).metadata?.executable || fallback
  } catch {
    return fallback
  }
}

/* ---------- the helper's files ---------- */

/** the agent-browser binary for a machine's system, as bin/agent-browser picks it */
function browserBinary(os: string, arch: string) {
  const o = os.toLowerCase() === 'darwin' ? 'darwin' : 'linux'
  const a = /^(x86_64|amd64)$/.test(arch) ? 'x64' : /^(aarch64|arm64)$/.test(arch) ? 'arm64' : arch
  const f = join(ROOT, 'node_modules', 'agent-browser', 'bin', `agent-browser-${o}-${a}`)
  return existsSync(f) ? f : null
}

/** every file of the helper: where it goes in the helper's folder, and where it comes from here */
export function helperFiles(os: string, arch: string): [string, string][] {
  const files: [string, string][] = [
    ['bin/shepherdr', join(ROOT, 'helper', 'shepherdr')],
    ['route.pl', join(ROOT, 'helper', 'route.pl')],
    ['hook.pl', join(ROOT, 'helper', 'hook.pl')],
    ['mirror.pl', join(ROOT, 'helper', 'mirror.pl')],
    ['health.pl', join(ROOT, 'helper', 'health.pl')],
    ['relay.pl', join(ROOT, 'helper', 'relay.pl')],
  ]
  const ab = browserBinary(os, arch)
  if (ab) files.push(['bin/agent-browser', join(ROOT, 'helper', 'agent-browser')], ['bin/agent-browser-bin', ab])
  const walk = (dir: string) => {
    for (const n of readdirSync(dir)) {
      const f = join(dir, n)
      if (statSync(f).isDirectory()) walk(f)
      else files.push([join('skill', relative(join(ROOT, 'skill'), f)), f])
    }
  }
  walk(join(ROOT, 'skill'))
  return files
}

/** what this Shepherdr's helper is: its files' contents, so any change means an update */
const versions = new Map<string, string>()
export function helperVersion(os: string, arch: string) {
  const k = `${os}/${arch}`
  if (versions.has(k)) return versions.get(k)!
  const h = createHash('sha256')
  for (const [to, from] of helperFiles(os, arch).sort((a, b) => a[0].localeCompare(b[0]))) {
    h.update(to)
    h.update(readFileSync(from))
  }
  versions.set(k, h.digest('hex').slice(0, 16))
  return versions.get(k)!
}

/** what the agent-browser for a machine's system is: its contents */
export function browserVersion(os: string, arch: string) {
  const k = `browser ${os}/${arch}`
  if (!versions.has(k)) {
    const f = browserBinary(os, arch)
    // with where it goes: a machine that has it as bin/agent-browser (from before the wrapper) gets it again
    versions.set(k, f ? createHash('sha256').update('bin/agent-browser-bin').update(readFileSync(f)).digest('hex').slice(0, 16) : '')
  }
  return versions.get(k)!
}

/** a tar of the helper (plain ustar, read by every tar there is) */
function tarOf(files: [string, string][], version: string, browser: string) {
  const blocks: Buffer[] = []
  const header = (name: string, size: number, mode: number) => {
    const b = Buffer.alloc(512)
    b.write(name, 0, 100)
    b.write(`${mode.toString(8).padStart(7, '0')}\0`, 100)
    b.write('0000000\0', 108)
    b.write('0000000\0', 116)
    b.write(`${size.toString(8).padStart(11, '0')}\0`, 124)
    b.write(`${Math.floor(Date.now() / 1000).toString(8).padStart(11, '0')}\0`, 136)
    b.write('        ', 148)
    b.write('0', 156)
    b.write('ustar\0', 257)
    b.write('00', 263)
    let sum = 0
    for (const x of b) sum += x
    b.write(`${sum.toString(8).padStart(6, '0')}\0 `, 148)
    return b
  }
  const add = (name: string, data: Buffer, mode: number) => {
    blocks.push(header(name, data.length, mode), data, Buffer.alloc((512 - (data.length % 512)) % 512))
  }
  for (const [to, from] of files) add(to, readFileSync(from), to.startsWith('bin/') || to.endsWith('.pl') ? 0o755 : 0o644)
  add('version', Buffer.from(`${version}\n`), 0o644)
  if (browser) add('browser-version', Buffer.from(`${browser}\n`), 0o644)
  blocks.push(Buffer.alloc(1024))
  return Buffer.concat(blocks)
}

/** put the helper there (again): the folder's old files are replaced, its hooks, inbox and prompts stay; the
 *  agent-browser (large) only when it changed */
export async function installHelper(target: string, p: Probe) {
  const version = helperVersion(p.os, p.arch)
  const browser = browserVersion(p.os, p.arch)
  const keep = !!browser && p.browser === browser
  const dir = helperDir(p.home)
  const tar = tarOf(helperFiles(p.os, p.arch).filter(([to]) => !(keep && to === 'bin/agent-browser-bin')), version, browser)
  const clear = keep ? '"$d/skill" "$d/bin/shepherdr" "$d/bin/agent-browser"' : '"$d/skill" "$d/bin"'
  const r = await onMachine(target, `set -e; d=${q(dir)}; mkdir -p "$d"; rm -rf ${clear}; tar xf - -C "$d"; mkdir -p "$d/hooks" "$d/inbox" "$d/prompts"`, { stdin: tar, timeoutMs: 180_000 })
  if (r.code !== 0) throw new Error(`Could not put the helper on the machine: ${r.err || `exit ${r.code}`}`)
  return version
}

/** the machine's facts a worker needs (box.ts) */
export function boxInfo(m: Saved, p: Probe): BoxInfo {
  const bin = join(helperDir(p.home), 'bin')
  return { label: m.label, target: m.target, session: m.session, herdr: herdrPath(m.id, p.herdr), home: p.home, path: `${bin}:${p.path}`, os: p.os }
}

/** what a machine lacks for Shepherdr, in words: nothing when all is there */
export function lacks(p: Probe): string[] {
  const out: string[] = []
  if (!p.tools.herdr) out.push('herdr')
  if (!p.tools.git) out.push('git')
  if (!p.tools.perl || !p.tools.jsonpp) out.push('perl (with JSON::PP)')
  if (!p.tools.tar) out.push('tar')
  if (!p.tools.claude && !p.tools.codex && !p.tools.pi) out.push('an agent CLI (Claude Code, Codex or pi)')
  return out
}

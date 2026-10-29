// Another machine, reached over SSH. Shepherdr is installed once, on the main machine; every other machine saved in
// herdr's Remote (herdr machine list) is run from it by a worker process of its own (machines.ts), with SHEPHERDR_BOX
// naming the machine. In that worker, everything that touches the machine goes through here: herdr's API rides herdr's
// own remote bridge (herdr.ts), commands (git, gh, agent-browser) and file reads run over one shared SSH connection
// (a ControlMaster), and the CLIs' logs are mirrored to the main machine as they grow (mirror.ts). The machine itself
// needs only herdr, git, perl and the agent CLIs, plus the small helper Shepherdr puts there (helper/, install.ts).
import { mkdirSync } from 'node:fs'
import { homedir } from 'node:os'
import { join } from 'node:path'
import { readJson } from '../store.ts'

export interface BoxInfo {
  /** herdr's label for it: the machine's name in Shepherdr */
  label: string
  /** the SSH target herdr reaches it by (a host, user@host, an ~/.ssh/config alias) */
  target: string
  /** its herdr session */
  session: string
  /** herdr's path there */
  herdr: string
  /** its home folder */
  home: string
  /** its login PATH, with the helper's bin first */
  path: string
  /** uname -s: Linux, Darwin */
  os: string
}

/** the machine this worker runs, or nothing: this is the main machine's own process */
export const BOX: BoxInfo | null = process.env.SHEPHERDR_BOX ? (JSON.parse(process.env.SHEPHERDR_BOX) as BoxInfo) : null

/** the helper's folder on a machine: its shepherdr command, hooks, inbox, skills (helper/, install.ts) */
export const helperDir = (home: string) => join(home, '.shepherdr', 'helper')

/** the home folder of the machine this process works on */
export const boxHome = () => BOX?.home ?? homedir()

/** where the SSH connections keep their shared master: short, a socket path has a length limit */
const control = () => {
  const dir = join(process.env.SHEPHERDR_MAIN_HOME ?? join(homedir(), '.shepherdr'), 'ssh')
  mkdirSync(dir, { recursive: true, mode: 0o700 })
  return join(dir, '%C')
}

/** what holds an SSH connection open (a tunnel, a forward): it ends with the connection, where a plain sleep would
 *  stay on the machine for good, one more on every restart. A byte written now and then fails once the connection is
 *  gone; watching $PPID can't tell, since exe.dev's sshd hands sessions to a root init that never ends */
export const HOLD = 'while printf . 2>/dev/null; do sleep 30; done'

/** single-quoted for a POSIX shell */
export const q = (s: string) => `'${s.replace(/'/g, `'\\''`)}'`

/** ssh to a target with the shared connection; the remote command runs under /bin/sh, whatever the login shell is */
export function sshArgs(target: string, script: string, opts: { tty?: boolean; extra?: string[] } = {}): string[] {
  return [
    'ssh',
    // first: ssh keeps the first value it is given
    ...(opts.extra ?? []),
    '-o', 'BatchMode=yes',
    '-o', 'ConnectTimeout=10',
    '-o', 'ServerAliveInterval=15',
    '-o', 'ServerAliveCountMax=4',
    '-o', 'ControlMaster=auto',
    '-o', 'ControlPersist=600',
    '-o', `ControlPath=${control()}`,
    opts.tty ? '-tt' : '-T',
    target,
    `/bin/sh -c ${q(script)}`,
  ]
}

/** a command line on the box: its PATH (the helper's bin first), in cwd, with env */
export function script(b: BoxInfo, argv: string[], o: { cwd?: string; env?: Record<string, string> } = {}) {
  const env = Object.entries({ PATH: b.path, ...o.env })
    .map(([k, v]) => `${k}=${q(v)}`)
    .join(' ')
  return `${o.cwd ? `cd ${q(o.cwd)} || exit 1; ` : ''}export ${env}; exec ${argv.map(q).join(' ')}`
}

export interface Ran {
  code: number
  out: string
  err: string
}

/** run a command on the box (or here, when there is no box) */
export async function run(argv: string[], o: { cwd?: string; env?: Record<string, string>; stdin?: string | Uint8Array; timeoutMs?: number } = {}): Promise<Ran> {
  const cmd = BOX ? sshArgs(BOX.target, script(BOX, argv, o)) : argv
  const p = Bun.spawn(cmd, { cwd: BOX ? undefined : o.cwd, stdin: o.stdin === undefined ? 'ignore' : Buffer.from(o.stdin), stdout: 'pipe', stderr: 'pipe', env: BOX ? process.env : { ...process.env, ...o.env } })
  let timedOut = false
  const t = o.timeoutMs ? setTimeout(() => ((timedOut = true), p.kill()), o.timeoutMs) : null
  const [out, err, code] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text(), p.exited])
  if (t) clearTimeout(t)
  return { code: timedOut ? 124 : code, out, err: timedOut ? `${err}\ntimed out` : err }
}

/** the same, waiting for it: only for the few reads that code expects at once (a file's text, whether it is there) */
export function runSync(argv: string[], o: { cwd?: string; env?: Record<string, string>; stdin?: string | Uint8Array; timeoutMs?: number } = {}): { code: number; out: Buffer; err: string } {
  const cmd = BOX ? sshArgs(BOX.target, script(BOX, argv, o)) : argv
  const r = Bun.spawnSync(cmd, { cwd: BOX ? undefined : o.cwd, stdin: o.stdin === undefined ? 'ignore' : Buffer.from(o.stdin), stdout: 'pipe', stderr: 'pipe', timeout: o.timeoutMs ?? 30_000, env: BOX ? process.env : { ...process.env, ...o.env } })
  return { code: r.exitCode ?? 1, out: r.stdout ?? Buffer.alloc(0), err: r.stderr?.toString() ?? '' }
}

/** a long-running command on the box (a log mirror, a tunnel): its process */
export function spawnOn(b: BoxInfo, argv: string[], o: { extra?: string[]; stdin?: 'pipe' | 'ignore' } = {}) {
  return Bun.spawn(sshArgs(b.target, script(b, argv), { extra: o.extra }), { stdin: o.stdin ?? 'ignore', stdout: 'pipe', stderr: 'pipe' })
}

/** who an agent is to the shepherdr command, and where that command is: here Shepherdr's own bin and server; on
 *  another machine the helper's bin first on its own PATH (its command finds the way back by itself) */
export function agentEnv(chat: string, root: string): Record<string, string> {
  if (BOX) return { SHEPHERDR_CHAT: chat, SHEPHERDR_URL: '', SHEPHERDR_COMMAND: join(helperDir(BOX.home), 'bin', 'shepherdr'), SHEPHERDR_MACHINE: BOX.label, PATH: BOX.path }
  // The listening live server owns these agents, never an inherited CLI target (which may be a demo).
  return { SHEPHERDR_CHAT: chat, SHEPHERDR_COMMAND: join(root, 'bin', 'shepherdr'), SHEPHERDR_URL: `http://127.0.0.1:${readJson<{ port?: number }>('server.json', {}).port ?? process.env.SHEPHERDR_PORT ?? 7787}`, PATH: `${join(root, 'bin')}:${process.env.PATH ?? ''}` }
}

/** a port on the box, reached here: a local port forwarded over SSH to its 127.0.0.1 (a browser's stream, its
 *  DevTools). Kept while it is used; close() ends it */
export async function forward(b: BoxInfo, port: number): Promise<{ port: number; close(): void }> {
  const free = Bun.listen({ hostname: '127.0.0.1', port: 0, socket: { data() {} } })
  const local = free.port
  free.stop(true)
  const p = Bun.spawn(sshArgs(b.target, HOLD, { extra: ['-o', 'ControlPath=none', '-o', 'ExitOnForwardFailure=yes', '-L', `127.0.0.1:${local}:127.0.0.1:${port}`] }), { stdin: 'ignore', stdout: 'ignore', stderr: 'ignore' })
  // ready once the port answers
  for (let i = 0; i < 50; i++) {
    const ok = await new Promise<boolean>((res) => {
      Bun.connect({ hostname: '127.0.0.1', port: local, socket: { open: (s) => (s.end(), res(true)), error: () => res(false), connectError: () => res(false), data() {} } }).catch(() => res(false))
    })
    if (ok) break
    if (p.exitCode !== null) throw new Error(`Could not reach port ${port} on ${b.label}`)
    await Bun.sleep(100)
  }
  return { port: local, close: () => p.kill() }
}

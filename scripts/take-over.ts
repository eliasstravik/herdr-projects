// Once, when herdr has installed Shepherdr in place of herdr-projects (`herdr plugin update`, same plugin id): the old
// herdr-projects is tidied away, and nothing of it is brought over. ~/.herdr-projects is never changed, so going back
// to herdr-projects (docs/upgrading.md, Going back) finds everything as it was. Before anything outside it changes, a copy goes
// to ~/.herdr-projects-backup-<date>/: herdr-projects' agent skill (it tells agents to run commands that are gone now)
// and herdr's config.toml (its marked sidebar rows show herdr-projects' own fields, which are gone too; its marked
// key stays, as it opens Shepherdr now). Then its keeper and page server stop, and a tailnet link its page server took
// over points back where it pointed before. What was done is said once in the home space's chat (live/source.ts).
import { spawnSync } from 'node:child_process'
import { copyFileSync, cpSync, existsSync, mkdirSync, readdirSync, readFileSync, realpathSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { basename, dirname, join, sep } from 'node:path'

export const OLD_ID = 'herdr-projects'
/** written once it ran: never again on this machine */
export const TOOK_OVER = 'took-over-herdr-projects.json'
const GOING_BACK = 'https://github.com/eliasstravik/shepherdr#going-back-to-herdr-projects'

export interface TookOver {
  at: number
  backup: string
  /** what was done, for the chat */
  text: string
  /** said in the home space's chat */
  said?: boolean
}

export interface Sys {
  /** the program a process runs, with its arguments (none: not running) */
  args(pid: number): string | null
  /** every process of this user, with its program and arguments */
  list(): { pid: number; args: string }[]
  kill(pid: number, sig: NodeJS.Signals): void
  /** tailscale's output, or null when it isn't there or failed */
  tailscale(args: string[]): string | null
  /** herdr reads its config again */
  reload(): void
  sleep(ms: number): void
}

const live: Sys = {
  args(pid) {
    const r = spawnSync('ps', ['-p', String(pid), '-o', 'args='], { encoding: 'utf8' })
    return r.status === 0 && r.stdout.trim() ? r.stdout.trim() : null
  },
  list() {
    const r = spawnSync('ps', ['-u', String(process.getuid?.() ?? ''), '-o', 'pid=,args='], { encoding: 'utf8' })
    return r.status !== 0 ? [] : r.stdout.split('\n').flatMap((l) => {
      const m = l.trim().match(/^(\d+)\s+(.*)$/)
      return m ? [{ pid: Number(m[1]), args: m[2] }] : []
    })
  },
  kill(pid, sig) {
    try {
      process.kill(pid, sig)
    } catch {}
  },
  tailscale(args) {
    const r = spawnSync('tailscale', args, { encoding: 'utf8', timeout: 15_000 })
    return r.status === 0 ? r.stdout : null
  },
  reload() {
    spawnSync(process.env.HERDR_BIN_PATH || 'herdr', ['server', 'reload-config'], { stdio: 'ignore', timeout: 15_000 })
  },
  sleep(ms) {
    Bun.sleepSync(ms)
  },
}

const real = (p: string) => {
  try {
    return realpathSync(p)
  } catch {
    return p
  }
}
const isDir = (p: string) => {
  try {
    return statSync(p).isDirectory()
  } catch {
    return false
  }
}
const readJson = <T>(f: string): T | null => {
  try {
    return JSON.parse(readFileSync(f, 'utf8')) as T
  } catch {
    return null
  }
}
const short = (p: string, home: string) => (p === home || p.startsWith(home + sep) ? `~${p.slice(home.length)}` : p)

/** herdr runs this as herdr-projects from a folder it installed from GitHub: herdr-projects was replaced. A checkout
 *  someone linked by hand (a developer's, or Shepherdr's own while it is still called shepherdr elsewhere) never is. */
export function replacedOld(root: string, env: NodeJS.ProcessEnv): boolean {
  const plugins = pluginsDir(env)
  if (env.HERDR_PLUGIN_ID !== OLD_ID || !plugins) return false
  const at = real(root)
  return at.startsWith(plugins + sep) && !at.startsWith(join(plugins, 'config') + sep)
}

/** where herdr keeps its plugins (the parent of each plugin's config folder) */
const pluginsDir = (env: NodeJS.ProcessEnv) => (env.HERDR_PLUGIN_CONFIG_DIR ? real(join(env.HERDR_PLUGIN_CONFIG_DIR, '..', '..')) : '')

export function herdrConfig(env: NodeJS.ProcessEnv, home: string) {
  return env.HERDR_CONFIG_PATH || join(env.XDG_CONFIG_HOME || join(home, '.config'), 'herdr', 'config.toml')
}

/** herdr-projects' marked sidebar block taken out of herdr's config; everything else stays as it was */
export function withoutSidebar(text: string): string {
  const next = text.replace(/^# >>> herdr-projects sidebar[^\n]*\n[\s\S]*?^# <<< herdr-projects sidebar[^\n]*(\n|$)/m, '')
  return next === text ? text : next.replace(/\n{3,}/g, '\n\n')
}

/** the folders herdr-projects wrote its skill to: the ones it wrote down, and each agent program's own */
function skillDirs(home: string, env: NodeJS.ProcessEnv, old: string): string[] {
  const listed = readJson<{ skillPaths?: unknown }>(join(old, 'data', 'extras.json'))?.skillPaths
  const dirs = Array.isArray(listed) ? listed.filter((d): d is string => typeof d === 'string') : []
  dirs.push(
    join(env.CLAUDE_CONFIG_DIR || join(home, '.claude'), 'skills', OLD_ID),
    join(env.CODEX_HOME || join(home, '.codex'), 'skills', OLD_ID),
    join(env.PI_CODING_AGENT_DIR || join(home, '.pi', 'agent'), 'skills', OLD_ID),
  )
  // only this home's (a path written down on another machine is not this one's), and only folders that hold just its
  // own SKILL.md
  return [...new Set(dirs)].filter((d) => {
    if (!d.startsWith(home + sep) || basename(d) !== OLD_ID || !isDir(d)) return false
    try {
      return readdirSync(d).every((f) => f === 'SKILL.md') && /^name: herdr-projects\s*$/m.test(readFileSync(join(d, 'SKILL.md'), 'utf8'))
    } catch {
      return false
    }
  })
}

/** herdr-projects' own programs: its keeper and page server (by the files they write), and anything else running its
 *  program from herdr's plugin folder (a board left open in a tab, its voice and remote helpers) */
function stopOld(old: string, plugins: string, sys: Sys): string[] {
  const named = (args: string | null) => !!args && basename(args.split(/\s+/)[0]).startsWith(OLD_ID)
  const pids = new Map<number, string>()
  for (const file of ['keeper.json', 'serve.json', 'front-door.json']) {
    const pid = Number(readJson<{ pid?: number }>(join(old, 'data', file))?.pid)
    const args = pid > 1 ? sys.args(pid) : null
    if (named(args)) pids.set(pid, args!)
  }
  for (const { pid, args } of sys.list()) if (pid !== process.pid && named(args) && args.startsWith(plugins + sep)) pids.set(pid, args)
  for (const pid of pids.keys()) sys.kill(pid, 'SIGTERM')
  for (let i = 0; i < 30 && [...pids.keys()].some((p) => sys.args(p)); i++) sys.sleep(100)
  for (const pid of pids.keys()) if (sys.args(pid)) sys.kill(pid, 'SIGKILL')
  // by what each was running as, in this order
  const as = new Map([['keeper', 'keeper'], ['keeper-work', 'keeper'], ['serve', 'page server'], ['board', 'board']])
  const kinds = new Set([...pids.values()].map((a) => as.get(a.split(/\s+/)[1]) ?? 'helpers'))
  return ['keeper', 'page server', 'board', 'helpers'].filter((k) => kinds.has(k))
}

interface ServeEntry {
  https: number
  port: number
}
/** each HTTPS port tailscale serves on this machine, and the local port its root goes to */
function served(sys: Sys): ServeEntry[] | null {
  const out = sys.tailscale(['serve', 'status', '--json'])
  if (out === null) return null
  try {
    const web = (JSON.parse(out) as { Web?: Record<string, { Handlers?: Record<string, { Proxy?: string }> }> }).Web ?? {}
    return Object.entries(web).flatMap(([hostPort, w]) => {
      const port = Number(w.Handlers?.['/']?.Proxy?.match(/:(\d+)\/?$/)?.[1])
      return port ? [{ https: Number(hostPort.split(':').pop()), port }] : []
    })
  } catch {
    return null
  }
}

/** tailnet links herdr-projects' page server held: the web UI's link it took over goes back to the web UI, and its
 *  own link for evidence pages goes */
function untangleLinks(old: string, sys: Sys, backup: string): string[] {
  const links = readJson<{ webUiLinks?: { https?: number; was?: number; now?: string }[] }>(join(old, 'data', 'extras.json'))?.webUiLinks ?? []
  const own = readJson<{ ts?: { port?: number; target?: number } }>(join(old, 'data', 'serve.json'))?.ts
  if (!links.length && !own?.port) return []
  const now = served(sys)
  if (!now) return []
  const status = sys.tailscale(['serve', 'status', '--json'])
  if (status) writeFileSync(join(backup, 'tailscale-serve.json'), status)
  const done: string[] = []
  for (const l of links) {
    const door = Number(l.now?.match(/:(\d+)\/?$/)?.[1])
    if (!l.https || !l.was || !door || !now.some((e) => e.https === l.https && e.port === door)) continue
    if (sys.tailscale(['serve', '--bg', `--https=${l.https}`, `http://127.0.0.1:${l.was}`]) !== null) done.push(`port ${l.https} goes to the herdr web UI again`)
  }
  if (own?.port && own.target && now.some((e) => e.https === own.port && e.port === own.target))
    if (sys.tailscale(['serve', `--https=${own.port}`, 'off']) !== null) done.push(`herdr-projects' own port ${own.port} is off`)
  return done
}

const and = (l: string[]) => (l.length > 1 ? `${l.slice(0, -1).join(', ')} and ${l.at(-1)}` : (l[0] ?? ''))
const day = (ms: number) => new Date(ms).toISOString().slice(0, 10)

/** Tidy herdr-projects away once, when it was replaced here; a no-op anywhere else. Returns what it did. */
export function takeOver(root: string, o: { env?: NodeJS.ProcessEnv; home?: string; now?: number; sys?: Sys } = {}): TookOver | null {
  const env = o.env ?? process.env
  const home = o.home ?? homedir()
  const sys = o.sys ?? live
  const old = join(home, '.herdr-projects')
  const mine = env.SHEPHERDR_HOME || join(home, '.shepherdr')
  const marker = join(mine, TOOK_OVER)
  if (!replacedOld(root, env) || !isDir(old) || existsSync(marker)) return null
  const now = o.now ?? Date.now()

  let backup = join(home, `.herdr-projects-backup-${day(now)}`)
  for (let n = 2; existsSync(backup); n++) backup = join(home, `.herdr-projects-backup-${day(now)}-${n}`)
  mkdirSync(backup, { recursive: true })
  const moved: string[] = []

  // copies first, of everything that is about to change
  const skills = skillDirs(home, env, old)
  skills.forEach((d, i) => {
    const to = join(backup, 'skills', `${i + 1}-${basename(dirname(dirname(d))).replace(/^\./, '')}`)
    cpSync(d, to, { recursive: true })
    moved.push(`${short(join(to, 'SKILL.md'), home)} was ${short(join(d, 'SKILL.md'), home)}`)
  })
  const config = herdrConfig(env, home)
  let text: string | null = null
  try {
    text = readFileSync(config, 'utf8')
  } catch {}
  const next = text === null ? null : withoutSidebar(text)
  if (text !== null && next !== text) {
    copyFileSync(config, join(backup, 'config.toml'))
    moved.push(`${short(join(backup, 'config.toml'), home)} was ${short(config, home)}`)
  }

  // then the tidying
  for (const d of skills) rmSync(d, { recursive: true, force: true })
  if (text !== null && next !== null && next !== text) {
    writeFileSync(config, next)
    sys.reload()
  }
  const links = untangleLinks(old, sys, backup)
  const stopped = stopOld(old, pluginsDir(env), sys)

  const did = [
    stopped.length ? `stopped its ${and(stopped)}` : '',
    skills.length ? `took its agent skill out of ${and(skills.map((d) => short(dirname(d), home)))}` : '',
    next !== text ? `took its sidebar rows out of herdr's config.toml${text?.includes('# >>> herdr-projects key') ? ' (its key opens Shepherdr now)' : ''}` : '',
  ].filter(Boolean)
  const b = short(backup, home)
  writeFileSync(
    join(backup, 'README.txt'),
    [
      `Shepherdr ${day(now)}: herdr-projects was replaced by Shepherdr. ${short(old, home)} was left exactly as it was.`,
      `Before tidying, Shepherdr copied here everything it then changed or removed:`,
      ...(moved.length ? moved.map((m) => `  ${m}`) : ['  (nothing: there was nothing of herdr-projects outside its folder)']),
      ...(existsSync(join(backup, 'tailscale-serve.json')) ? [`  tailscale-serve.json is \`tailscale serve status --json\` before`] : []),
      `What it did: ${did.join('; ') || 'nothing beyond this'}.${links.length ? ` On your tailnet, ${and(links)}.` : ''}`,
      `To go back to herdr-projects 1.2.27: ${GOING_BACK}`,
      '',
    ].join('\n'),
  )
  const rec: TookOver = {
    at: now,
    backup,
    text:
      `Shepherdr replaced herdr-projects. Nothing was brought over: your old tasks and settings stay untouched in ${short(old, home)}.\n\n` +
      (did.length ? `Shepherdr ${and(did)}.${links.length ? ` On your tailnet, ${and(links)}.` : ''} A copy of everything it changed is in ${b}.\n\n` : '') +
      `To go back to herdr-projects 1.2.27: [Going back to herdr-projects](${GOING_BACK}).`,
  }
  mkdirSync(mine, { recursive: true })
  writeFileSync(marker, JSON.stringify(rec, null, 2))
  return rec
}

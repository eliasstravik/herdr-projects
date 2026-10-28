// Every machine: Shepherdr is installed once, on the main machine, and runs every other machine saved in herdr's Remote
// (herdr machine list) from there. Each gets a worker of its own: the same Shepherdr, in a process of its own, whose
// herdr, files and commands are that machine's, reached over SSH (box.ts), with its records under
// ~/.shepherdr/machines/<name>. The main process keeps a live link to each worker, shows what it shows, and passes
// actions, chats and terminals through. Before a worker starts, its machine gets Shepherdr's small helper
// (install.ts), and again whenever Shepherdr's copy changed. A machine that can't be reached keeps its last snapshot
// and chats, and what you send it waits until it is back.
//
// A restart of this server (an update) is quick for every machine: what each showed last is kept on disk and shown at
// once as connecting, and its worker outlives the restart and is taken back when its code is still the same; one whose
// code changed is started again at once with what the machine had last time, its look and helper following.
import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, openSync, readdirSync, readFileSync, rmSync, statSync } from 'node:fs'
import { join, relative } from 'node:path'
import type { Action, Alert, Chat, Snapshot, TermScreen } from '../../shared/types.ts'
import { Emitter, type Overview } from '../source.ts'
import { HOME, readJson, writeJson } from '../store.ts'
import { boxInfo, browserVersion, helperVersion, installHelper, lacks, machineStatus, onMachine, probe, savedMachines, type Probe, type Saved } from './install.ts'
import { mirrorHomes } from './mirror.ts'
import { q, sshArgs } from './box.ts'
import { MERGE_DEADLINE } from './engine/engine.ts'

export interface MachineRec {
  machine: string
  /** its worker's address on this machine, once it runs */
  url: string
  lastSeen: number
}

type Events = { snapshot: []; chat: [key: string, chat: Chat]; terminal: [key: string, screen: TermScreen]; alert: [alert: Alert]; news: [machine: string, news: Overview] }

interface Link {
  rec: MachineRec
  saved: Saved
  snap?: Snapshot
  chats: Map<string, Chat>
  ws?: WebSocket
  watching: { chats: string[]; term: string | null }
  reachable: boolean
  down?: number
  /** what you sent while it was out of reach */
  waiting: Action[]
  /** its worker: started here, or one from before a restart taken back */
  worker?: Worker
  /** how it is going, in a few words: Ready, Putting the helper there…, Can't reach it: … */
  state: string
  /** what it lacks, from the last check */
  lacks: string[]
  probe?: Probe
  busy?: boolean
  /** what went wrong putting Chrome there for the shared browser */
  chrome?: string
  /** not reached yet since this server started: shown as connecting, not as out of reach, for a while */
  connecting: boolean
}

interface Worker {
  pid: number
  kill(): void
  exited: Promise<unknown>
}

/** what of a machine is kept across a restart: herdr's record of it, the last look, and what it showed last (its
 *  places, tasks, and the chats on screen) */
interface Kept {
  saved: Saved
  probe?: Probe
  snap?: Snapshot
  chats?: Chat[]
}

/** a machine not reached within this long of the start is out of reach, no longer connecting */
const CONNECTING_FOR = 60_000

/** the server's code: a worker running the same code is kept across a restart of the main server */
let code = ''
export function codeVersion(dir = join(import.meta.dir, '..', '..')) {
  if (code) return code
  const h = createHash('sha256')
  const walk = (d: string) => {
    for (const n of readdirSync(d).sort()) {
      const f = join(d, n)
      if (statSync(f).isDirectory()) walk(f)
      else if (/\.(ts|json)$/.test(n)) h.update(relative(dir, f)).update(readFileSync(f))
    }
  }
  for (const d of ['server', 'shared', 'plugins', 'generated']) if (existsSync(join(dir, d))) walk(join(dir, d))
  for (const f of ['package.json', 'bun.lock', 'plugins.json']) if (existsSync(join(dir, f))) h.update(f).update(readFileSync(join(dir, f)))
  code = h.digest('hex').slice(0, 16)
  return code
}

/** a worker from before this server started, still running: its process, if it is one of Shepherdr's */
function running(pid: number | undefined): Worker | undefined {
  if (!pid || pid === process.pid) return undefined
  try {
    process.kill(pid, 0)
  } catch {
    return undefined
  }
  const cmd = Bun.spawnSync(['ps', '-o', 'command=', '-p', String(pid)]).stdout.toString()
  if (!cmd.includes('server/main.ts')) return undefined
  const exited = new Promise((done) => {
    const t = setInterval(() => {
      try {
        process.kill(pid, 0)
      } catch {
        clearInterval(t)
        done(undefined)
      }
    }, 2000)
  })
  return { pid, kill: () => process.kill(pid, 'SIGTERM'), exited }
}

/** a machine's own folder of records on this machine */
export const machineHome = (label: string) => join(HOME, 'machines', dirOf(label))

export class Machines extends Emitter<Events> {
  private links = new Map<string, Link>()
  private timers: ReturnType<typeof setInterval>[] = []
  private stopped = false

  /** saved: herdr's list of machines; none (tests) runs only the workers adopt() names */
  constructor(private saved: (() => Promise<Saved[]>) | null = savedMachines) {
    super()
    // from the time Shepherdr was installed on every machine: the machines that announced themselves
    rmSync(join(HOME, 'peers.json'), { force: true })
    if (saved) {
      // what each machine showed before a restart, at once; its worker is taken back or started again
      for (const k of kept()) this.resume(k)
      void this.sync()
      this.timers.push(setInterval(() => void this.sync(), 15_000))
    }
    this.timers.push(setInterval(() => void this.pollAll(), 5000))
  }

  /** this server stops: the workers go on (the next start takes them back), and what each machine showed is kept */
  stop() {
    this.stopped = true
    this.timers.forEach(clearInterval)
    for (const l of this.links.values()) {
      l.ws?.close()
      this.keep(l, true)
    }
  }

  /** a machine as it was before a restart: shown as connecting, its worker taken back when its code is the same */
  private resume(k: Kept) {
    const l = this.link(k.saved)
    l.probe = k.probe
    l.snap = k.snap
    for (const c of k.chats ?? []) l.chats.set(c.key, c)
    const was = readJson<{ port?: number; pid?: number; code?: string }>(join('machines', dirOf(k.saved.label), 'server.json'), {})
    const w = running(was.pid)
    if (w && was.port && was.code === codeVersion()) {
      this.own(l, w)
      l.rec.url = `http://127.0.0.1:${was.port}`
      l.state = 'Connecting…'
      void this.poll(l)
      // a newer helper goes there meanwhile, without stopping the worker
      void this.refresh(l)
      return
    }
    if (!w) return void this.start(l)
    // its old worker finishes a merge on its way first (main.ts): the new one starts once it is gone, never beside it
    l.busy = true
    w.kill()
    void Promise.race([w.exited, Bun.sleep(MERGE_DEADLINE + 30_000)]).then(() => {
      l.busy = false
      void this.start(l)
    })
  }

  private link(m: Saved): Link {
    const l: Link = { rec: { machine: m.label, url: '', lastSeen: 0 }, saved: m, chats: new Map(), watching: { chats: [], term: null }, reachable: false, waiting: [], state: 'Connecting…', lacks: [], connecting: true }
    this.links.set(m.label, l)
    setTimeout(() => {
      if (!l.connecting) return
      l.connecting = false
      this.emit('snapshot')
    }, CONNECTING_FOR)
    return l
  }

  /** what a machine shows is kept on disk, a few seconds after it changes (at once as this server stops) */
  private keepTimers = new Map<string, ReturnType<typeof setTimeout>>()
  private keep(l: Link, now = false) {
    if (!this.saved) return
    const write = () => {
      this.keepTimers.delete(l.rec.machine)
      const chats = l.watching.chats.map((k) => l.chats.get(k)).filter((c): c is Chat => !!c)
      try {
        writeJson(join('machines', dirOf(l.rec.machine), 'last.json'), { saved: l.saved, probe: l.probe, snap: l.snap, chats } satisfies Kept)
      } catch {
        /* a full disk: the next start shows it once it answers */
      }
    }
    const t = this.keepTimers.get(l.rec.machine)
    if (now) {
      if (t) clearTimeout(t)
      return write()
    }
    if (!t) this.keepTimers.set(l.rec.machine, setTimeout(write, 5000))
  }

  private own(l: Link, w: Worker) {
    l.worker = w
    w.exited.then(() => {
      if (l.worker !== w) return
      l.worker = undefined
      l.rec.url = ''
      l.state = 'Its worker stopped; starting it again'
      this.emit('snapshot')
    })
  }

  list() {
    return [...this.links.values()]
  }

  /** the machines herdr has saved: a new one is set up and started, a removed one stopped */
  async sync() {
    if (this.stopped || !this.saved) return
    const saved = await this.saved()
    const status = await machineStatus()
    for (const m of saved) {
      let l = this.links.get(m.label)
      if (!l) {
        l = this.link(m)
        this.emit('snapshot')
      }
      l.saved = m
      const st = status[m.label]
      if (st && st.status !== 'reachable' && !l.worker) {
        l.state = `herdr can't reach it: ${st.error ?? st.status}`
        l.connecting = false
        this.emit('snapshot')
        continue
      }
      if (!l.worker && !l.busy) void this.start(l)
    }
    for (const [label, l] of this.links)
      if (!saved.some((m) => m.label === label)) {
        l.ws?.close()
        l.worker?.kill()
        this.links.delete(label)
        rmSync(join(machineHome(label), 'last.json'), { force: true })
        this.emit('snapshot')
      }
  }

  /** a worker that already runs, at this address (tests) */
  adopt(label: string, url: string) {
    const l = this.link({ id: label, label, target: label, session: 'default', enabled: true })
    l.rec.url = url.replace(/\/+$/, '')
    void this.poll(l)
    return l
  }

  /** check the machine, put the helper there when it lacks it or it is older, then start its worker; when nothing
   *  changed there since the last look, the worker starts at once and the look follows */
  async start(l: Link, force = false) {
    if (l.busy) return
    l.busy = true
    const home = machineHome(l.saved.label)
    mkdirSync(home, { recursive: true })
    const last = !force && l.probe && fits(l.probe) ? l.probe : undefined
    if (last) {
      this.spawn(l, last, home)
      l.state = 'Starting…'
    }
    try {
      const p = await this.look(l, force)
      if (!p) return
      if (p.oldInstall) await this.takeOver(l, home)
      if (last && l.worker && sameBox(l.saved, last, p)) return
      l.worker?.kill()
      this.spawn(l, p, home)
      l.state = 'Starting…'
    } catch (e) {
      l.state = `Can't reach it: ${(e as Error).message}`
      l.connecting = false
    } finally {
      l.busy = false
      this.emit('snapshot')
    }
  }

  /** a worker taken back after a restart: the machine is looked at again and gets a newer helper, the worker goes on */
  private async refresh(l: Link) {
    if (l.busy) return
    l.busy = true
    try {
      await this.look(l)
    } catch {
      /* the next restart or the doctor looks again */
    } finally {
      l.busy = false
      this.emit('snapshot')
    }
  }

  /** look at the machine, and put the helper there when it lacks it or it is older: null when it lacks a tool */
  private async look(l: Link, force = false): Promise<Probe | null> {
    const p = await probe(l.saved.target)
    l.probe = p
    l.lacks = lacks(p)
    if (l.lacks.length) {
      l.state = `Needs ${l.lacks.join(', ')}`
      l.connecting = false
      return null
    }
    if (force || p.helper !== helperVersion(p.os, p.arch)) {
      if (!l.worker) l.state = 'Putting the helper there…'
      this.emit('snapshot')
      p.helper = await installHelper(l.saved.target, p)
      p.browser = browserVersion(p.os, p.arch)
      // Chrome for the shared browser, by the helper's agent-browser (nothing to do when it is there)
      void onMachine(l.saved.target, `${q(join(p.home, '.shepherdr', 'helper', 'bin', 'agent-browser'))} install`, { timeoutMs: 600_000 }).then((r) => {
        l.chrome = r.code === 0 ? undefined : `Chrome didn't install there: ${r.err.split('\n').pop() || r.out.trim().split('\n').pop()}`
      })
    }
    this.keep(l)
    return p
  }

  /**
   * The machine had Shepherdr installed on it, reporting to this one (before one install ran every machine): its
   * projects, tasks and chats come here once, and that copy is turned off in its herdr (herdr plugin disable shepherdr),
   * so only this one runs its agents. Its files stay where they were.
   */
  private async takeOver(l: Link, home: string) {
    if (!existsSync(join(home, 'spaces.json')) && !existsSync(join(home, 'projects.json'))) {
      const out = ['./helper', './browser', './inbox', './hooks', './run', './.releases', './config.json', './server.json'].map((x) => `--exclude=${x}`).join(' ')
      const from = Bun.spawn(sshArgs(l.saved.target, `cd "$HOME/.shepherdr" && tar cf - ${out} .`), { stdout: 'pipe', stderr: 'ignore' })
      const to = Bun.spawn(['tar', 'xf', '-', '-C', home], { stdin: from.stdout, stdout: 'ignore', stderr: 'ignore' })
      await Promise.all([from.exited, to.exited])
    }
    await onMachine(l.saved.target, `PATH=${q(l.probe?.path ?? '/usr/bin:/bin')}; herdr --session ${q(l.saved.session)} plugin disable shepherdr >/dev/null 2>&1; mv "$HOME/.shepherdr/config.json" "$HOME/.shepherdr/config.json.before-one-install" 2>/dev/null; true`)
  }

  private spawn(l: Link, p: Probe, home: string) {
    const box = boxInfo(l.saved, p)
    // a worker left from before (another version of the code): one worker per machine
    running(readJson<{ pid?: number }>(join('machines', dirOf(l.saved.label), 'server.json'), {}).pid)?.kill()
    rmSync(join(home, 'server.json'), { force: true })
    l.rec.url = ''
    const log = openSync(join(home, 'worker.log'), 'a')
    const w = Bun.spawn([process.execPath, join(import.meta.dir, '..', 'main.ts')], {
      env: {
        ...process.env,
        ...mirrorHomes(home, box),
        SHEPHERDR_BOX: JSON.stringify(box),
        SHEPHERDR_HOME: home,
        SHEPHERDR_MACHINE: box.label,
        SHEPHERDR_PORT: '0',
        SHEPHERDR_MAIN_HOME: HOME,
        SHEPHERDR_SHARED_HOME: HOME,
      },
      stdout: log,
      stderr: log,
    })
    this.own(l, w)
    // its address, once it serves
    const look = setInterval(() => {
      const port = readJson<{ port?: number }>(join('machines', dirOf(l.saved.label), 'server.json'), {}).port
      if (!port || l.worker !== w) return
      clearInterval(look)
      l.rec.url = `http://127.0.0.1:${port}`
      void this.poll(l)
    }, 300)
    setTimeout(() => clearInterval(look), 60_000)
  }

  private async pollAll() {
    for (const l of this.links.values()) await this.poll(l)
  }

  private async poll(l: Link): Promise<void> {
    if (!l.rec.url && !this.moved(l)) return
    try {
      const r = await fetch(`${l.rec.url}/api/snapshot`, { signal: AbortSignal.timeout(4000) })
      if (!r.ok) throw new Error(String(r.status))
      l.snap = (await r.json()) as Snapshot
      l.rec.lastSeen = Date.now()
      const herdrUp = !l.snap.machines?.some((m) => m.status === 'off')
      l.state = herdrUp ? 'Ready' : "Its herdr isn't answering"
      l.connecting = false
      this.keep(l)
      // a worker started again: the live link is made again
      if (!l.reachable || !l.ws) {
        l.reachable = true
        l.down = undefined
        this.connect(l)
        // what waited goes in now
        const queued = l.waiting.splice(0)
        for (const a of queued) await this.forward(l, a).catch(() => l.waiting.push(a))
      }
      this.emit('snapshot')
    } catch {
      if (this.moved(l)) return this.poll(l)
      if (l.reachable || l.down === undefined) {
        l.reachable = false
        l.down = l.down ?? Date.now()
        this.emit('snapshot')
      }
    }
  }

  /** Its worker was started again out of this server's hands (shepherdr doctor while another server ran, by hand)
   *  and serves at another address: its server.json says where, and that worker is taken as this machine's. */
  private moved(l: Link): boolean {
    if (l.busy) return false
    const s = readJson<{ port?: number; pid?: number }>(join('machines', dirOf(l.saved.label), 'server.json'), {})
    const url = s.port ? `http://127.0.0.1:${s.port}` : ''
    if (!url || url === l.rec.url) return false
    const w = s.pid === l.worker?.pid ? l.worker : running(s.pid)
    if (!w) return false
    if (w !== l.worker) this.own(l, w)
    l.rec.url = url
    // the live link is made again at the new address on its first answer
    l.reachable = false
    return true
  }

  private connect(l: Link) {
    l.ws?.close()
    const url = new URL(`${l.rec.url}/api/ws`)
    url.protocol = url.protocol.replace('http', 'ws')
    const ws = new WebSocket(url)
    l.ws = ws
    ws.onopen = () => ws.send(JSON.stringify({ type: 'watch', ...l.watching, link: true }))
    ws.onmessage = (e) => {
      const m = JSON.parse(String(e.data))
      if (m.type === 'chat') {
        l.chats.set(m.chat.key, m.chat)
        this.keep(l)
        this.emit('chat', m.chat.key, m.chat)
      } else if (m.type === 'terminal') this.emit('terminal', m.chat, { ansi: m.ansi, cols: m.cols, rows: m.rows })
      else if (m.type === 'snapshot') {
        l.snap = m.snapshot
        this.keep(l)
        this.emit('snapshot')
      } else if (m.type === 'alert') this.emit('alert', m.alert)
      else if (m.type === 'news') this.emit('news', l.rec.machine, m.news)
    }
    ws.onclose = () => {
      if (l.ws === ws) l.ws = undefined
    }
  }

  /* ---------- who owns what ---------- */

  /** the machine a place, task or chat key belongs to */
  owner(key: string): Link | undefined {
    const m = key.match(/^[mp]:([^:]+)/)
    if (m) return this.links.get(m[1])
    const task = key.split('/')[0]
    for (const l of this.links.values()) if (l.snap?.tasks.some((t) => t.id === task)) return l
    return undefined
  }

  ownerOfAction(a: Action): Link | undefined {
    const k = 'chat' in a ? a.chat : 'place' in a ? a.place : undefined
    return k ? this.owner(k) : undefined
  }

  byName(machine: string) {
    return [...this.links.values()].find((l) => l.rec.machine.toLowerCase() === machine.toLowerCase())
  }

  async act(l: Link, a: Action): Promise<unknown> {
    if (!l.reachable) {
      l.waiting.push(a)
      return { waiting: true }
    }
    return this.forward(l, a)
  }

  private async forward(l: Link, a: Action) {
    const r = await fetch(`${l.rec.url}/api/action`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(a), signal: AbortSignal.timeout(20_000) })
    return r.json()
  }

  watch(chats: string[], term: string | null) {
    for (const l of this.links.values()) {
      const mine = chats.filter((k) => this.owner(k) === l)
      const t = term && this.owner(term) === l ? term : null
      const next = { chats: mine, term: t }
      if (JSON.stringify(next) === JSON.stringify(l.watching)) continue
      l.watching = next
      if (l.ws?.readyState === WebSocket.OPEN) l.ws.send(JSON.stringify({ type: 'watch', ...next, link: true }))
    }
  }

  chat(key: string): Chat | undefined {
    return this.owner(key)?.chats.get(key)
  }

  /** pass a request (pages, files) through to the machine that has it */
  async fetch(l: Link, path: string, init?: RequestInit) {
    if (!l.rec.url) throw new Error(`${l.rec.machine} isn't running yet.`)
    return fetch(`${l.rec.url}${path}`, init)
  }

  /* ---------- shepherdr doctor ---------- */

  /** check every machine and repair what is missing: the helper put there again, its worker started again */
  async doctor(): Promise<string[]> {
    await this.sync()
    const out: string[] = []
    for (const l of this.links.values()) {
      if (l.worker) {
        l.worker.kill()
        await l.worker.exited
        l.worker = undefined
      }
      await this.start(l, true)
      const p = l.probe
      const missing = p ? [!p.tools.gh && 'gh (pull requests of GitHub repos)', !p.tools.claude && 'Claude Code', !p.tools.codex && 'Codex'].filter(Boolean) : []
      out.push(`${l.rec.machine} (${l.saved.target}): ${l.lacks.length ? `needs ${l.lacks.join(', ')}` : l.state.startsWith("Can't") || l.state.startsWith('herdr') ? l.state : 'helper put there again, worker started again'}${missing.length ? `; not there: ${missing.join(', ')}` : ''}${l.chrome ? `; ${l.chrome}` : ''}`)
    }
    return out.length ? out : ['No other machines saved in herdr (herdr machine add <ssh target> adds one).']
  }

  /** each machine's line for Settings → Machines */
  rows() {
    return [...this.links.values()].map((l) => ({ name: l.rec.machine, state: l.reachable && l.state === 'Ready' ? `Ready · ${l.saved.target}` : l.reachable ? l.state : l.connecting ? 'Connecting…' : l.down && l.rec.url ? `Can't reach since ${clock(l.down)}` : l.state }))
  }

  /** the worker's own log, for the doctor */
  log(label: string) {
    try {
      return readFileSync(join(machineHome(label), 'worker.log'), 'utf8').split('\n').slice(-20).join('\n')
    } catch {
      return ''
    }
  }
}

const clock = (t: number) => new Date(t).toTimeString().slice(0, 5)

/** a machine's folder name under machines/ */
const dirOf = (label: string) => label.replace(/[^\w.-]+/g, '_')

/** what every machine showed before this server started */
function kept(): Kept[] {
  const dir = join(HOME, 'machines')
  if (!existsSync(dir)) return []
  return readdirSync(dir).flatMap((d) => {
    const k = readJson<Kept | null>(join('machines', d, 'last.json'), null)
    return k?.saved?.label ? [k] : []
  })
}

/** the machine as last looked at is ready for a worker: nothing missing, and the helper there is this one's */
const fits = (p: Probe) => !lacks(p).length && !p.oldInstall && p.helper === helperVersion(p.os, p.arch)

/** a worker started from one look runs the same as from the other */
const sameBox = (m: Saved, a: Probe, b: Probe) => JSON.stringify(boxInfo(m, a)) === JSON.stringify(boxInfo(m, b))

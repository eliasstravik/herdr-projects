// Every agent Shepherdr talks to is an ordinary herdr agent. This keeps which herdr agent and which log files belong
// to each chat, starts and resumes agents, and sends what you write: at once, even while the agent works (Claude Code,
// Codex and pi take it mid-turn, as steering). It waits only while the CLI shows a menu or permission prompt, where typed
// text could pick an option, or is still starting; then it goes in as soon as it can.
import { randomUUID } from 'node:crypto'
import { mkdirSync, watch, type FSWatcher } from 'node:fs'
import { dirname, join } from 'node:path'
import { Emitter } from '../source.ts'
import { readJson, writeJson } from '../store.ts'
import type { TermScreen } from '../../shared/types.ts'
import { herdr, Subscription, type AgentInfo, type AgentStatus, type HerdrError, type HerdrEvent } from './herdr.ts'
import { ClaudeSessions, Reviewers, screenBackground, sessionBackground, type Background } from './readers/background.ts'
import { claudeHome, claudeLogPath, findClaudeLog, findCodexLog, findLog, findPiLog, logKind } from './readers/find.ts'
import { effortFor, modelArg } from './models.ts'
import { tag } from './chatlines.ts'
import { hookSettings, HookLogs, type Said } from './readers/hooks.ts'
import * as task from './mfs.ts'
import { boxDir, mirrored } from './mfs.ts'
import { runSync } from './box.ts'
import { leftAtPrompt } from './commands.ts'
import { paneWrong, type Wrong } from './revive.ts'

/** a log's name (its session id), wherever it lies: one named both by its mirrored path and by its own is one log */
const logName = (f: string) => f.slice(f.lastIndexOf('/') + 1)

/** the logs with each one once, where it first came, by the path it was named by last (a session carried to another
 *  folder goes on there): the same log twice is read twice, and a big one holds up the server */
export function uniqueLogs(logs: string[]): string[] {
  const at = new Map<string, number>()
  const out: string[] = []
  for (const f of logs) {
    const i = at.get(logName(f))
    if (i === undefined) (at.set(logName(f), out.length), out.push(f))
    else out[i] = f
  }
  return out
}

/** adds a log the record doesn't have yet; true when it was new */
function addLog(r: AgentRecord, f: string): boolean {
  if (r.logs.some((x) => logName(x) === logName(f))) return false
  r.logs.push(f)
  return true
}

/** a Claude Code session, copied into the project folder of the cwd it resumes in (Claude Code looks for it only
 *  there) from wherever it was written on that machine; its log path, or nothing when there is none */
export function carryClaudeSession(id: string, cwd: string): string | undefined {
  const to = claudeLogPath(cwd, id)
  const sh = `[ -e "$1" ] && exit 0; src=$(ls -t "$2"/*/"$3.jsonl" 2>/dev/null | head -n 1); [ -n "$src" ] || exit 1; mkdir -p "$(dirname "$1")" && cp "$src" "$1"`
  return runSync(['/bin/sh', '-c', sh, 'carry', task.unmirrored(to), task.unmirrored(join(claudeHome(), 'projects')), id]).code === 0 ? to : undefined
}

export interface AgentRecord {
  /** chat key */
  key: string
  /** herdr agent name */
  name: string
  /** herdr agent kind: claude, codex, pi */
  kind: string
  cwd: string
  /** herdr workspace label it lives in */
  workspace: string
  tab?: string
  model?: string
  effort?: string
  /** instructions appended to the CLI's system prompt (or sent first, hidden, where it has no such option) */
  system?: string
  /** log files, oldest first: one per session */
  logs: string[]
  /** the CLI's own session id (claude, pi), for resuming */
  sessionId?: string
  started: number
  /** set when the agent's run ended for good (task done): writing resumes it */
  finished?: boolean
  /** 'ask' keeps the CLI's own approvals; otherwise (task agents) it runs without asking */
  access?: 'full' | 'ask'
  /** environment for its terminal: who it is to the shepherdr command */
  env?: Record<string, string>
  /** where its hooks record what it reports of itself (Claude Code: readers/hooks.ts) */
  hooks?: string
  /** the CLI, model and effort Settings said for its role when it last started on them: a change since is offered as
   *  a restart in its chat (restart.ts) */
  set?: { kind: string; model?: string; effort?: string }
}

export interface OutItem {
  id: string
  key: string
  text: string
  /** a reaction line: shown as a faded pill until it goes in */
  reaction?: { message: string; emoji: string }
  at: number
  /** the screen's own id for it (Message.cid) */
  cid?: string
  /** a CLI command (/model, /compact): goes in only between turns */
  idle?: boolean
  /** a slash command you typed: Shepherdr follows what it does (commands.ts) */
  command?: boolean
}

/** what starts an agent: its record, the first thing it is told, and where its pane comes from when it isn't its
 *  workspace by label (a task's herdr worktree space) */
/** was: the label its workspace had before, renamed in place when that one is still open */
export type Spec = Omit<AgentRecord, 'logs' | 'started'> & { first?: string; pane?: () => Promise<string>; was?: string }

/** why what you send waits: a menu or permission prompt is up, the agent is starting, or (a CLI command) mid-turn */
export type Hold = 'menu' | 'start' | 'turn'

export interface SendOpts {
  /** a reaction line: shown as a faded pill until it goes in */
  reaction?: OutItem['reaction']
  /** starts (or resumes) the agent when it isn't running */
  ensure?: () => Promise<unknown>
  /** the screen's own id for it */
  cid?: string
  /** a CLI command: only between turns */
  idle?: boolean
  /** a slash command you typed (commands.ts) */
  command?: boolean
}

export interface Sent {
  waiting: boolean
  /** why it waits */
  waitsFor?: Hold
  item: OutItem
}

type Events = {
  status: [key: string, status: AgentStatus | 'gone']
  logs: [key: string]
  sent: [item: OutItem]
  /** what a screen sent couldn't go in */
  failed: [key: string]
  output: [key: string]
}

/** where the hooks of the agents Shepherdr starts record their events */
export const HOOKS_DIR = join(boxDir(), 'hooks')

/** where an agent's instructions are kept for its CLI to read */
const promptPath = (name: string) => join(boxDir(), 'prompts', `${name}.md`)

/**
 * The one place an agent's status is decided. What the agent reports of itself wins: its CLI's own hooks today (Claude
 * Code, readers/hooks.ts), OSC 7501's program status once herdr passes that on (then only `said` changes). herdr's
 * reading of its screen counts where the agent reports nothing (Codex, pi, one started before hooks), and for a menu
 * up between turns that no hook tells of (a rate-limit or login menu), where typed text could pick an option.
 */
export function statusFrom(said: Said | undefined, seen: AgentStatus | 'gone'): AgentStatus | 'gone' {
  if (seen === 'gone' || !said) return seen
  if (said.helper) return 'blocked'
  if (said.state !== 'idle') return said.state
  return seen === 'done' || seen === 'blocked' ? seen : 'idle'
}

/** what a refusal from herdr means for what you send: a menu is up, or the agent just started and herdr doesn't count
 *  it as its named agent yet ("agent shep-all is not an active named agent"); anything else, it can't go in */
export function holdFrom(e: unknown): Hold | undefined {
  const code = (e as HerdrError).code
  if (code === 'agent_blocked') return 'menu'
  if (code === 'agent_not_found' || code === 'agent_not_ready' || code === 'agent_not_running') return 'start'
  if (/not an active named agent/.test((e as Error)?.message ?? '')) return 'start'
  return undefined
}

/** what an agent's hooks said is read again at most this often */
const SAID_MS = 250

/** wraps text Shepherdr sends that is not for the chat (instructions, news), in a tag of what it is when it is one
 *  of its lines: <shepherdr><news task="SHE-9" event="merged">…</news></shepherdr>; readers' text is stripped of it */
export const hidden = (s: string, kind?: string, attrs: Record<string, string> = {}) => tag('shepherdr', {}, kind ? tag(kind, attrs, s) : s)
export const stripHidden = (s: string) => s.replace(/<shepherdr(?:\s+[\w-]+="[^"]*")*\s*>[\s\S]*?<\/shepherdr>\s*/g, '').trim()

export class Agents extends Emitter<Events> {
  records: Record<string, AgentRecord> = readJson('agents.json', {})
  outbox: OutItem[] = readJson('outbox.json', [])
  live = new Map<string, AgentInfo>()
  private byName = new Map<string, string>()
  private timer: ReturnType<typeof setInterval> | null = null
  private sub: Subscription
  private starting = new Map<string, Promise<AgentInfo | null>>()
  /** agents being stopped for a restart: what you send waits for the one it starts */
  private stopping = new Set<string>()
  private delivering = new Set<string>()
  /** what went in lately with a screen's id: the chat gives the id back once it reads the message from the log */
  private delivered: OutItem[] = []
  /** everything that went in lately, with when: what a merge checks the agent has read, so kept across a restart */
  private typed: OutItem[] = readJson('typed.json', [])
  /** what a screen sent that waited and then couldn't go in: the screen says so and offers Retry */
  private failures: { key: string; cid: string; error: string; at: number }[] = []
  /** what a screen sent that is on its way now, by its id: sent again meanwhile, it goes in once */
  private inflight = new Map<string, Promise<Sent>>()
  /** since when an agent with screen sends waiting for it is gone, or herdr out of reach */
  private stranded = new Map<string, number>()
  /** how long that may last (an agent starting again) before what waits for it fails */
  strandAfter = 30_000
  /** since when herdr lists an agent that just started but won't take input yet (not yet its named agent): what waits
   *  for it is tried again on each refresh, and fails after unreadyFor */
  private unready = new Map<string, number>()
  unreadyFor = 60_000
  reachable = true
  lastReach = Date.now()
  watchedPanes = new Set<string>()
  sessions = new ClaudeSessions()
  /** headless reviewers of another CLI (codex exec, claude -p) each chat's agent runs: subagents, not shells */
  reviewers = new Reviewers()
  /** what the screen of a Claude Code with no session file says still runs in the background */
  private screens = new Map<string, Background>()
  /** what its hooks, session file or screen say still runs in the background, since when, and after which turn */
  private seen = new Map<string, { what: Background; since: number; turn?: number }>()
  /** the background work last told, so a change is told once */
  private bg = new Map<string, Background | undefined>()
  /** servers alone (a dev server, a preview) keep it at work this long after its turn ended: one it started for you
   *  to look at never wakes it, and what it said then may want an answer */
  serversFor = 10 * 60_000
  /** shells and monitors that end (a test run, a review) keep it at work this long after its turn ended: each wakes it
   *  when it ends, but one may hang */
  shellsFor = 60 * 60_000
  /** each watched pane's size in cells, and when it was read */
  private sizes = new Map<string, { at: number; size: { cols: number; rows: number } | null }>()
  hooks = new HookLogs()
  /** the status last told, by chat key */
  private told = new Map<string, AgentStatus | 'gone'>()
  private hookWatch: FSWatcher | null = null
  private saidAt = new Map<string, { at: number; said?: Said }>()
  private soon: ReturnType<typeof setTimeout> | null = null
  /** whether a menu a slash command opened is up, which herdr doesn't see: typed text could pick an option */
  menuUp?: (key: string) => boolean
  /** Read startup menus herdr may classify as idle before any text can choose from them. */
  beforeInput?: (key: string) => Promise<void>
  /** whether its log shows its turn ended after this time: a turn cut short (Esc, a prompt turned down) runs no hook */
  endedSince?: (key: string, at: number) => boolean

  constructor() {
    super()
    // records written before logs were kept once each
    for (const r of Object.values(this.records)) {
      r.logs = uniqueLogs(r.logs ?? [])
      this.byName.set(r.name, r.key)
    }
    this.sub = new Subscription(
      (e) => this.onEvent(e),
      () => this.refresh(),
    )
  }

  start() {
    this.refresh()
    this.timer = setInterval(() => this.refresh(), 2000)
    // a hook event is read at once, not at the next look
    try {
      mkdirSync(mirrored(HOOKS_DIR), { recursive: true })
      this.hookWatch = watch(mirrored(HOOKS_DIR), () => {
        this.soon ??= setTimeout(() => {
          this.soon = null
          this.refresh()
        }, 150)
      })
    } catch {
      /* read at the next look */
    }
  }

  stop() {
    if (this.timer) clearInterval(this.timer)
    if (this.soon) clearTimeout(this.soon)
    this.hookWatch?.close()
    this.sub.stop()
  }

  save() {
    writeJson('agents.json', this.records)
    writeJson('outbox.json', this.outbox)
  }

  record(key: string) {
    return this.records[key]
  }

  info(key: string) {
    const r = this.records[key]
    return r ? this.live.get(r.name) : undefined
  }

  status(key: string): AgentStatus | 'gone' {
    return statusFrom(this.said(key), this.info(key)?.agent_status ?? 'gone')
  }

  /** what the agent reports of itself, from its hooks; nothing when it reports nothing (see statusFrom). A turn its log
   *  shows ended after its last event was cut short with no hook: it is over. */
  said(key: string): Said | undefined {
    const r = this.records[key]
    if (!r?.hooks || !this.info(key)) return undefined
    // asked many times over for one screen: read at most every SAID_MS
    const now = Date.now()
    const c = this.saidAt.get(key)
    if (c && now - c.at < SAID_MS) return c.said
    let said = this.hooks.read(mirrored(r.hooks)).said
    if (said && said.state !== 'idle' && this.endedSince?.(key, said.at)) said = { state: 'idle', turnAt: said.turnAt, at: said.at, last: said.last, helper: said.helper, paused: said.paused }
    this.saidAt.set(key, { at: now, said })
    return said
  }

  /** a name herdr accepts: [a-z][a-z0-9_-]{0,31} */
  static nameFor(s: string) {
    const n = s
      .toLowerCase()
      .replace(/[^a-z0-9_-]+/g, '-')
      .replace(/^[^a-z]+/, '')
      .slice(0, 32)
      .replace(/-+$/, '')
    return n || `shep-${randomUUID().slice(0, 6)}`
  }

  /* ---------- herdr state ---------- */

  async refresh() {
    this.saidAt.clear()
    try {
      const list = await herdr.agents()
      this.reachable = true
      this.lastReach = Date.now()
      this.live = new Map(list.filter((a) => a.name).map((a) => [a.name!, a]))
      for (const key of this.byName.values()) {
        const now = this.status(key)
        const was = this.told.get(key) ?? 'gone'
        this.told.set(key, now)
        if (was !== now) this.changed(key, now)
      }
      await this.readBackground()
      // learn log files herdr names (its agent integrations report them)
      for (const [name, key] of this.byName) this.learnSession(key, this.live.get(name))
      // and the session its hooks say it is in now, which herdr doesn't name: a /clear starts another
      for (const key of this.byName.values()) this.followSession(key)
      // what waits goes in as soon as its agent can take it
      for (const key of new Set(this.outbox.map((o) => o.key))) if (!this.heldBy(key)) this.deliver(key)
      this.checkStranded()
      this.sub.set([
        { type: 'pane.agent_status_changed' },
        { type: 'pane.closed' },
        { type: 'pane.created' },
        ...[...this.watchedPanes].map((pane_id) => ({ type: 'pane.output_changed', pane_id })),
      ])
    } catch {
      if (this.reachable) {
        this.reachable = false
        this.emit('status', '*', 'gone')
      }
      this.checkStranded()
    }
  }

  /**
   * Its turn ended, but work it started in the background still runs and will wake it: a subagent, a shell, a monitor
   * (Claude Code; see readers/background.ts). Nothing while it works, asks in a menu, or is gone.
   */
  background(key: string, now = Date.now()): Background | undefined {
    const what = this.stillRuns(key)
    if (what !== 'shells' && what !== 'servers') return what
    const s = this.seen.get(key)
    return s?.what === what && now - s.since >= (what === 'servers' ? this.serversFor : this.shellsFor) ? undefined : what
  }

  /** what its hooks said still ran as its turn ended, else its session file, else its screen; a shell that is a
   *  headless reviewer counts as a subagent. However long it has run (background() leaves shells out after a while) */
  stillRuns(key: string): Background | undefined {
    const what = this.ranOn(key)
    return what && what !== 'agents' && this.reviewers.runs(key) ? 'agents' : what
  }

  private ranOn(key: string): Background | undefined {
    const r = this.records[key]
    const st = this.status(key)
    if (r?.kind !== 'claude' || st === 'working' || st === 'blocked' || st === 'gone') return undefined
    const file = this.sessions.status(r.sessionId, r.cwd)
    // its hooks said what still ran as its turn ended; its session file is newer once that work is gone, or once a
    // subagent runs again (one that paused on shells of its own woke)
    const said = this.said(key)
    if (said) return file === 'idle' ? undefined : said.bg && file === 'busy' ? 'agents' : said.bg
    return file === undefined ? this.screens.get(key) : sessionBackground(file)
  }

  /** what still runs in the background, read again: from the screen where there is no session file. A change is told
   *  as a status change, since the agent may have woken, said a word and stopped again in between herdr's looks. */
  private async readBackground() {
    for (const key of this.byName.values()) {
      const r = this.records[key]
      const st = this.status(key)
      const pane = this.info(key)?.pane_id
      if (r?.kind === 'claude' && pane && (st === 'idle' || st === 'done') && !this.said(key) && this.sessions.status(r.sessionId, r.cwd) === undefined) {
        const seen = screenBackground(await herdr.visible(pane).catch(() => ''))
        if (seen) this.screens.set(key, seen)
        else this.screens.delete(key)
      } else this.screens.delete(key)
      // counted from when it was first seen after its turn ended: a turn that woke and ended again in between herdr's
      // looks, which its hooks tell, starts the count again
      const what = this.stillRuns(key)
      const turn = this.said(key)?.turnAt
      const was = this.seen.get(key)
      if (!what) this.seen.delete(key)
      else if (was?.what !== what || was.turn !== turn) this.seen.set(key, { what, since: Date.now(), turn })
      const now = this.background(key)
      if (now === this.bg.get(key)) continue
      this.bg.set(key, now)
      this.emit('status', key, st)
    }
  }

  /** what a screen sent waits only on a menu or a starting agent: one whose agent is gone (or herdr out of reach) for a
   *  while fails */
  checkStranded(now = Date.now()) {
    const keys = new Set(this.outbox.filter((o) => o.cid).map((o) => o.key))
    for (const key of this.stranded.keys()) if (!keys.has(key)) this.stranded.delete(key)
    for (const key of keys) {
      if (this.reachable && this.status(key) !== 'gone') {
        this.stranded.delete(key)
        continue
      }
      const since = this.stranded.get(key) ?? now
      this.stranded.set(key, since)
      if (now - since < this.strandAfter) continue
      this.stranded.delete(key)
      this.fail(key, this.reachable ? 'The agent stopped before it went in.' : "Lost herdr before it went in.")
    }
  }

  /** what a screen sent that waits for this agent fails: the screen says why and offers Retry */
  private fail(key: string, error: string) {
    const mine = this.outbox.filter((o) => o.key === key && o.cid)
    if (!mine.length) return
    const now = Date.now()
    this.outbox = this.outbox.filter((o) => !mine.includes(o))
    this.save()
    this.failures = [...this.failures.filter((f) => now - f.at < 600_000), ...mine.map((o) => ({ key, cid: o.cid!, error, at: now }))]
    this.emit('failed', key)
  }

  private onEvent(e: HerdrEvent) {
    const type = String(e.data?.type ?? e.event)
    if (type.includes('output_changed')) {
      const pane = String(e.data.pane_id ?? '')
      for (const [name, key] of this.byName) if (this.live.get(name)?.pane_id === pane) this.emit('output', key)
      return
    }
    // status and pane changes: read the authoritative list again
    this.refresh()
  }

  private changed(key: string, status: AgentStatus | 'gone') {
    this.emit('status', key, status)
    if (status !== 'gone' && status !== 'blocked') this.deliver(key)
    if (status !== 'gone') this.emit('logs', key)
  }

  private learnSession(key: string, a?: AgentInfo) {
    const r = this.records[key]
    const s = a?.agent_session
    if (!r || !s) return
    if (s.kind === 'path' && addLog(r, mirrored(s.value))) {
      this.save()
      this.emit('logs', key)
    } else if (s.kind === 'id' && r.kind === 'claude') {
      const p = claudeLogPath(r.cwd, s.value)
      if (addLog(r, p)) {
        r.sessionId = s.value
        this.save()
        this.emit('logs', key)
      }
    }
  }

  /** the session a Claude Code's hooks say it is in now (a /clear, /resume or new one), followed: its transcript
   *  joins the chat's logs, and resuming it resumes that one */
  followSession(key: string) {
    const r = this.records[key]
    if (r?.kind !== 'claude' || !r.hooks) return
    const s = this.hooks.read(mirrored(r.hooks)).session
    // another machine's copy of its events may still be an earlier run's, until the new run's first event comes
    if (!s || s.id === r.sessionId || s.at < r.started - 5000) return
    const log = s.log ? mirrored(s.log) : claudeLogPath(r.cwd, s.id)
    r.sessionId = s.id
    addLog(r, log)
    this.save()
    this.emit('logs', key)
  }

  /** after a command that may start another session (/new, /clear, /resume), for an agent whose hooks don't say so: its
   *  CLI's newest log changed since then, once it is one the chat doesn't read yet */
  follow(key: string, since: number) {
    const r = this.records[key]
    if (!r || (r.kind === 'claude' && r.hooks)) return
    const look = () => {
      const f = findLog(r.kind, r.cwd, since - 1000)
      if (!f || !addLog(r, f)) return
      // Claude Code and pi resume by the id in its name; Codex by its last log
      if (r.kind !== 'codex') r.sessionId = f.match(/([0-9a-f-]{36})\.jsonl$/)?.[1] ?? r.sessionId
      this.save()
      this.emit('logs', key)
    }
    // a menu it opened (Codex's /new asks where) may be answered a while later
    for (const ms of [1500, 5000, 15000, 60000, 180000]) setTimeout(look, ms)
  }

  /** find a log file this CLI started since the agent (re)started, when herdr didn't name one */
  discoverLog(key: string) {
    const r = this.records[key]
    if (!r) return
    const since = r.started - 5000
    const f = r.kind === 'claude' ? (r.sessionId ? claudeLogPath(r.cwd, r.sessionId) : findClaudeLog(r.cwd, since)) : r.kind === 'codex' ? findCodexLog(r.cwd, since) : r.kind === 'pi' ? findPiLog(r.cwd, since) : undefined
    if (f && addLog(r, f)) {
      this.save()
      this.emit('logs', key)
    }
  }

  /** an agent already running in herdr that Shepherdr should treat as this chat */
  adopt(key: string, a: AgentInfo, kind: string, workspace: string) {
    if (this.records[key]?.name === a.name) return
    this.records[key] = { key, name: a.name!, kind, cwd: a.cwd ?? '', workspace, logs: [], started: Date.now() }
    this.byName.set(a.name!, key)
    this.learnSession(key, a)
    this.save()
  }

  /* ---------- starting ---------- */

  /**
   * Start this chat's agent again on what spec says: `before` first (its turn stopped; what it returns is the first
   * message then, read once the turn is over), then its run ends and it starts, resuming its session on the same CLI.
   * Meanwhile anything that would start it waits for this instead, and what you send waits and goes in after the first
   * message. `before` throwing leaves it as it was.
   */
  restart(spec: Spec, before?: () => Promise<string | void>): Promise<AgentInfo | null> {
    const p0 = this.starting.get(spec.key)
    const p = (async () => {
      await p0?.catch(() => undefined)
      this.stopping.add(spec.key)
      try {
        const first = await before?.()
        await this.finish(spec.key)
        return await this.launch({ ...spec, ...(first ? { first } : {}) })
      } finally {
        this.stopping.delete(spec.key)
        this.deliver(spec.key)
      }
    })().finally(() => this.starting.delete(spec.key))
    this.starting.set(spec.key, p)
    return p
  }

  /** start (or resume) this chat's agent; resolves once herdr says it is ready for input */
  ensure(spec: Spec): Promise<AgentInfo | null> {
    const running = this.live.get(spec.name)
    if (running) return Promise.resolve(running)
    const p0 = this.starting.get(spec.key)
    if (p0) return p0
    const p = this.launch(spec).finally(() => this.starting.delete(spec.key))
    this.starting.set(spec.key, p)
    return p
  }

  private async launch({ first, pane: place, was, ...spec }: Spec) {
    const old = this.records[spec.key]
    const resume = old && old.kind === spec.kind && (old.sessionId || old.logs.length)
    const r: AgentRecord = old ?? { ...spec, logs: [], started: Date.now() }
    const from = old?.cwd
    Object.assign(r, { name: spec.name, kind: spec.kind, cwd: spec.cwd, workspace: spec.workspace, tab: spec.tab ?? r.tab, env: spec.env ?? r.env, access: spec.access ?? r.access, model: spec.model ?? r.model, effort: spec.effort ?? r.effort, system: spec.system ?? r.system, set: spec.set ?? r.set, finished: false })
    r.started = Date.now()
    if (!resume) r.sessionId = r.kind === 'codex' ? undefined : randomUUID()
    // a run of its own: what its hooks record starts over
    if (r.hooks) this.hooks.forget(mirrored(r.hooks))
    r.hooks = undefined
    this.records[spec.key] = r
    this.byName.set(r.name, r.key)
    this.save()
    task.mkdirSync(r.cwd, { recursive: true })
    // it starts in another folder now: its session goes with it, where --resume looks for it
    if (resume && r.kind === 'claude' && r.sessionId && from !== r.cwd) {
      const id = r.sessionId
      const log = carryClaudeSession(id, r.cwd)
      if (log) r.logs = [...r.logs.filter((f) => !f.endsWith(`/${id}.jsonl`)), log]
    }
    const pane = place ? await place() : await this.paneFor(r, was)
    const args = this.args(r, !!resume)
    this.save()
    // a new pane needs a moment before its shell is at the prompt
    for (let i = 0; ; i++) {
      try {
        await herdr.start(r.name, r.kind, pane, args)
        break
      } catch (e) {
        const code = (e as { code?: string }).code
        if (code === 'agent_not_ready') break
        if (i >= 8) {
          console.error(`shepherdr: could not start ${r.name}: ${(e as Error).message}`)
          throw e
        }
        await new Promise((res) => setTimeout(res, 750))
      }
    }
    await this.refresh()
    // CLIs without an option for extra instructions get them first, hidden from the chat. Both wait until the CLI is
    // ready, ahead of what you sent meanwhile (a restart)
    this.queue(r.key, [r.kind !== 'claude' && r.kind !== 'pi' && !resume && r.system ? hidden(r.system) : '', first ?? ''].filter(Boolean), true)
    setTimeout(() => this.deliver(r.key), 1000)
    setTimeout(() => this.discoverLog(r.key), 3000)
    setTimeout(() => this.discoverLog(r.key), 15000)
    return this.live.get(r.name) ?? null
  }

  args(r: AgentRecord, resume: boolean): string[] {
    const a: string[] = []
    // a model is kept by its id, or by its name when picked in the chat's menu (Opus 5.5); an effort its model lacks
    // goes in as the nearest it has
    const model = modelArg(r.kind, r.model)
    const effort = effortFor(r.kind, r.effort, r.model)
    if (r.kind === 'claude') {
      if (resume && r.sessionId) a.push('--resume', r.sessionId)
      else if (r.sessionId) a.push('--session-id', r.sessionId)
      // its hooks tell Shepherdr what it does (readers/hooks.ts), from a settings file of its own: yours stay untouched
      const h = hookSettings(HOOKS_DIR, r.name)
      r.hooks = h.events
      a.push('--settings', h.settings)
      if (r.system) a.push('--append-system-prompt-file', this.promptFile(r))
      if (model) a.push('--model', model)
      if (effort) a.push('--effort', effort)
    } else if (r.kind === 'pi') {
      if (r.sessionId) a.push('--session-id', r.sessionId)
      if (r.system) a.push('--append-system-prompt', this.promptFile(r))
      if (model) a.push('--model', model)
      if (effort) a.push('--thinking', effort)
    } else if (r.kind === 'codex') {
      // its own last log: a task it took over keeps the logs of the agent before it too
      const id = resume ? r.logs.findLast((f) => (logKind(f) ?? 'codex') === 'codex')?.match(/([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\.jsonl$/)?.[1] : undefined
      if (id) a.unshift('resume', id)
      if (r.access !== 'ask') a.push('--dangerously-bypass-approvals-and-sandbox')
      if (model) a.push('-m', model)
      if (effort) a.push('-c', `model_reasoning_effort="${effort}"`)
    }
    return a
  }

  /** instructions go in a file: long text with quotes and newlines can't travel as a shell argument */
  private promptFile(r: AgentRecord) {
    const f = promptPath(r.name)
    task.mkdirSync(dirname(f), { recursive: true })
    task.writeFileSync(f, r.system ?? '')
    return f
  }

  /** the files Shepherdr starts its CLI with now, by flag: its hooks' settings and its instructions. One started with
   *  others (herdr restored an older command after it restarted) runs without them */
  filesOf(r: AgentRecord): Record<string, string> {
    if (r.kind === 'claude') return { '--settings': join(HOOKS_DIR, `${r.name}.settings.json`), ...(r.system ? { '--append-system-prompt-file': promptPath(r.name) } : {}) }
    if (r.kind === 'pi' && r.system) return { '--append-system-prompt': promptPath(r.name) }
    return {}
  }

  /** what is wrong with this agent's run: gone from herdr, its CLI exited in its pane, or it runs on outdated files;
   *  nothing while it is fine, null when herdr can't say. One on outdated files is left alone while herdr sees it at
   *  work or asking (a turn is never cut for this); its hooks don't count, as what they last said may be from before */
  async wrong(key: string): Promise<Wrong | undefined | null> {
    const r = this.records[key]
    const a = this.info(key)
    if (!r) return undefined
    if (!a) return 'gone'
    const p = await herdr.processInfo(a.pane_id).catch(() => null)
    // herdr didn't say: neither well nor wrong
    if (!p) return null
    const w = paneWrong(r.kind, p, this.filesOf(r))
    return w === 'stale' && (a.agent_status === 'working' || a.agent_status === 'blocked') ? undefined : w
  }

  /** whether its hooks last said it was at work, even while it is gone: a turn its log shows over is over */
  wasWorking(key: string) {
    const r = this.records[key]
    const said = r?.hooks ? this.hooks.read(mirrored(r.hooks)).said : undefined
    return said?.state === 'working' && !this.endedSince?.(key, said.at)
  }

  /** being started or stopped right now */
  busy(key: string) {
    return this.starting.has(key) || this.stopping.has(key)
  }

  /** started again by `start` (that resumes its session), its pane closed first where it still holds it (its CLI
   *  exited, or runs on outdated files). Meanwhile what you send waits for the one it starts, as in a restart */
  async relaunch(key: string, close: boolean, start: () => Promise<unknown>) {
    this.stopping.add(key)
    try {
      const a = close ? this.info(key) : undefined
      if (a) {
        await herdr.closePane(a.pane_id).catch(() => undefined)
        await this.refresh()
      }
      await start()
    } finally {
      this.stopping.delete(key)
      this.deliver(key)
    }
  }

  /** the workspace labelled so, or the one still labelled `was` (never a repo's), renamed to it */
  async space(label: string, was?: string) {
    const list = await herdr.workspaces()
    const ws = list.find((w) => w.label === label)
    const old = !ws && was ? list.find((w) => w.label === was && !w.worktree) : undefined
    if (old) await herdr.renameWorkspace(old.workspace_id, label)
    return ws ?? old
  }

  private async paneFor(r: AgentRecord, was?: string): Promise<string> {
    const env = r.env ?? {}
    const ws = await this.space(r.workspace, was)
    if (!ws) return (await herdr.createWorkspace(r.workspace, r.cwd, env)).root_pane.pane_id
    return (await herdr.createTab(ws.workspace_id, r.tab ?? r.name, r.cwd, env)).root_pane.pane_id
  }

  /* ---------- sending ---------- */

  /** why what you send would wait now, or nothing: it goes in at once. Typed text could pick an option on a menu or
   *  permission prompt, so it waits for the prompt to go; a starting agent isn't taking input yet; a CLI command waits for
   *  the turn to end. Anything else goes in even mid-turn: the CLI takes it at its next step. */
  holdOf(key: string, item?: Pick<OutItem, 'idle'>): Hold | undefined {
    return this.heldBy(key, item) ?? (this.unready.has(key) ? 'start' : undefined)
  }

  /** the same, without an agent herdr refused as not ready yet: that one is simply tried again */
  private heldBy(key: string, item?: Pick<OutItem, 'idle'>): Hold | undefined {
    const st = this.status(key)
    // stopping for a restart: what you send goes to the agent it starts on
    if (st === 'gone' || this.stopping.has(key) || this.info(key)?.interactive_ready === false) return 'start'
    if (st === 'blocked' || this.menuUp?.(key)) return 'menu'
    if (item?.idle && st === 'working') return 'turn'
    return undefined
  }

  /** what you send: in at once, unless the agent is on a menu or still starting (see holdOf). Returns whether it waits.
   *  What a screen sent (with its id) waits only that long: when the agent can't take it or won't start, the screen hears
   *  so and offers Retry. */
  send(key: string, text: string, o: SendOpts = {}): Promise<Sent> {
    const cid = o.cid
    if (!cid) return this.sendNow(key, text, o)
    // the screen sends again what it heard no answer for, maybe while the first is still on its way: once is enough
    const on = this.inflight.get(cid)
    if (on) return on
    const p = this.sendNow(key, text, o).finally(() => this.inflight.delete(cid))
    this.inflight.set(cid, p)
    return p
  }

  private async sendNow(key: string, text: string, { reaction, ensure, cid, idle, command }: SendOpts): Promise<Sent> {
    const again = cid && this.outbox.find((o) => o.cid === cid)
    if (again) return { waiting: true, waitsFor: this.whyWaits(key), item: again }
    const done = cid && this.delivered.find((o) => o.cid === cid)
    if (done) return { waiting: false, item: done }
    // sent again after it failed: it is on its way once more
    if (cid) this.failures = this.failures.filter((f) => f.cid !== cid)
    const item: OutItem = { id: randomUUID(), key, text, reaction, at: Date.now(), ...(cid ? { cid } : {}), ...(idle ? { idle } : {}), ...(command ? { command } : {}) }
    if (this.status(key) === 'gone' && ensure && cid) {
      try {
        await ensure()
      } catch (e) {
        throw new Error(`The agent didn't start: ${(e as Error).message}`)
      }
      if (this.status(key) === 'gone') throw new Error(this.reachable ? "The agent isn't running." : "Can't reach herdr.")
    }
    await this.beforeInput?.(key)
    // what waits goes first, so nothing overtakes it
    let hold: Hold | undefined
    if (!this.holdOf(key, item) && !this.outbox.some((o) => o.key === key)) {
      try {
        await this.put(key, item)
        this.unready.delete(key)
        this.went(item)
        this.emit('sent', item)
        return { waiting: false, item }
      } catch (e) {
        // a menu came up just now, or the agent just started and herdr doesn't take input for it yet: it waits
        hold = holdFrom(e)
        if (hold === 'start' && !this.unready.has(key)) this.unready.set(key, Date.now())
        if (cid && !hold) throw new Error(`The agent didn't take it: ${(e as Error).message}`)
        /* falls through: it waits */
      }
    }
    this.outbox.push(item)
    this.save()
    const st = this.status(key)
    if (st === 'gone' && ensure) ensure().then(() => this.deliver(key)).catch(() => undefined)
    else this.deliver(key)
    return { waiting: true, waitsFor: hold ?? this.whyWaits(key), item }
  }

  /** one thing into the CLI. A slash command is checked: a CLI may take its Enter for a pick in the list of commands
   *  it shows as you type, so while it still sits in the input, Enter goes again; one that never runs is wiped from
   *  the input and fails, so the chat says so instead of waiting on it. */
  private async put(key: string, item: OutItem) {
    await herdr.prompt(this.records[key].name, item.text)
    const pane = item.command && this.info(key)?.pane_id
    if (!pane) return
    const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))
    const read = async () => leftAtPrompt(await herdr.read(pane, 'text', 30).catch(() => ''), item.text)
    // twice, a moment apart: a screen not yet redrawn must not get an Enter that picks from the menu it opened
    const left = async () => (await read()) && (await sleep(this.commandMs / 2), await read())
    for (let i = 0; i < 3; i++) {
      await sleep(this.commandMs)
      if (!(await left())) return
      if (i < 2) await herdr.paneKeys(pane, ['enter'])
    }
    await herdr.paneKeys(pane, Array(item.text.length + 2).fill('backspace')).catch(() => undefined)
    throw new Error(`${item.text.trim().split(/\s/)[0]} didn't run in the terminal.`)
  }

  /** how long a CLI has to run a slash command before its input is looked at */
  commandMs = 800

  /** why what waits for this agent waits: the first in line holds the rest */
  private whyWaits(key: string) {
    const first = this.outbox.find((o) => o.key === key)
    return first ? this.holdOf(key, first) : undefined
  }

  /** hand over what waited, oldest first, while the agent can take it */
  async deliver(key: string) {
    if (this.delivering.has(key)) return
    this.delivering.add(key)
    try {
      for (;;) {
        const next = this.outbox.find((o) => o.key === key)
        if (!next) return void this.unready.delete(key)
        await this.beforeInput?.(key)
        if (this.heldBy(key, next)) return
        await this.put(key, next)
        this.unready.delete(key)
        this.outbox = this.outbox.filter((o) => o !== next)
        this.save()
        this.went(next)
        this.emit('sent', next)
        // give the CLI a moment to take it before the next item
        await new Promise((r) => setTimeout(r, 1500))
        await this.refresh()
      }
    } catch (e) {
      // a menu came up just now: it goes in once the menu is gone
      const hold = holdFrom(e)
      if (hold === 'menu') return
      // just started, not herdr's named agent yet: tried again on the next refresh, for a while
      if (hold === 'start') {
        const since = this.unready.get(key) ?? Date.now()
        this.unready.set(key, since)
        if (Date.now() - since < this.unreadyFor) return
        this.unready.delete(key)
      }
      // tried again on the next status change; what a screen sent fails now, so it doesn't wait on an agent that can't
      // take it
      this.fail(key, `The agent didn't take it: ${(e as Error).message}`)
    } finally {
      this.delivering.delete(key)
    }
  }

  /** what a screen sent that couldn't go in, in the last ten minutes */
  failedLately(key: string) {
    const now = Date.now()
    return this.failures.filter((f) => f.key === key && now - f.at < 600_000).map(({ cid, error }) => ({ cid, error }))
  }

  /** ahead: before what waits for this agent already */
  private queue(key: string, texts: string[], ahead = false) {
    if (!texts.length) return
    const items = texts.map((text) => ({ id: randomUUID(), key, text, at: Date.now() }))
    const i = ahead ? this.outbox.findIndex((o) => o.key === key) : -1
    if (i < 0) this.outbox.push(...items)
    else this.outbox.splice(i, 0, ...items)
    this.save()
  }

  waiting(key: string) {
    return this.outbox.filter((o) => o.key === key)
  }

  /** what went in with a screen's id in the last ten minutes, oldest first; at is when it went in */
  sentLately(key: string) {
    const now = Date.now()
    return this.delivered.filter((o) => o.key === key && now - o.at < 600_000)
  }

  /** what went in since this time (the last ten minutes at most) */
  typedSince(key: string, at: number) {
    return this.typed.filter((o) => o.key === key && o.at > at)
  }

  private went(item: OutItem) {
    const now = Date.now()
    this.typed = [...this.typed.filter((o) => now - o.at < 600_000), { ...item, at: now }].slice(-200)
    writeJson('typed.json', this.typed)
    if (!item.cid) return
    this.delivered = [...this.delivered.filter((o) => now - o.at < 600_000), { ...item, at: now }].slice(-200)
  }

  async keys(key: string, keys: string[]) {
    const a = this.info(key)
    if (a) await herdr.paneKeys(a.pane_id, keys)
  }

  async text(key: string, text: string) {
    const a = this.info(key)
    if (a) await herdr.paneText(a.pane_id, text)
  }

  /** its terminal's recent lines and the pane's size, so a screen draws them at its real width; the size is read again
   *  at most every few seconds, not on every change on the screen */
  async screen(key: string): Promise<TermScreen> {
    const a = this.info(key)
    if (!a) return { ansi: '' }
    const known = this.sizes.get(a.pane_id)
    const fresh = known && Date.now() - known.at < 3000
    const [ansi, size] = await Promise.all([herdr.read(a.pane_id, 'ansi', 300), fresh ? known.size : herdr.size(a.pane_id).catch(() => null)])
    if (!fresh) this.sizes.set(a.pane_id, { at: Date.now(), size })
    return { ansi, ...size }
  }

  /** its terminal's last lines as plain text */
  async screenText(key: string): Promise<string> {
    const a = this.info(key)
    return a ? herdr.read(a.pane_id, 'text', 60) : ''
  }

  watchTerminals(keys: string[]) {
    const panes = new Set(keys.map((k) => this.info(k)?.pane_id).filter(Boolean) as string[])
    const changed = panes.size !== this.watchedPanes.size || [...panes].some((p) => !this.watchedPanes.has(p))
    this.watchedPanes = panes
    for (const p of this.sizes.keys()) if (!panes.has(p)) this.sizes.delete(p)
    if (changed) this.refresh()
  }

  /** end a task agent's run: its pane closes; its chat stays and writing to it resumes it */
  async finish(key: string) {
    const r = this.records[key]
    const a = this.info(key)
    if (r) r.finished = true
    this.save()
    if (!a) return
    await herdr.closePane(a.pane_id).catch(() => undefined)
    // gone from what herdr runs at once: starting it again (a backup agent taking over) starts a new one
    await this.refresh()
  }

  /** a deleted task's agent, forgotten: its record (so its chat no longer reads its CLI's log) and what waited for it */
  forget(key: string) {
    const r = this.records[key]
    if (r && this.byName.get(r.name) === key) this.byName.delete(r.name)
    delete this.records[key]
    this.outbox = this.outbox.filter((o) => o.key !== key)
    this.failures = this.failures.filter((f) => f.key !== key)
    this.stranded.delete(key)
    this.save()
  }

  /** a turn under way ended cleanly before its run ends (a restart): interrupted, then a few seconds for the CLI to
   *  stop and write what it had; still at work after that, it throws and is left as it is */
  async stopTurn(key: string, ms = 15_000) {
    if (this.status(key) !== 'working') return
    await this.interrupt(key)
    for (const end = Date.now() + ms; this.status(key) === 'working'; ) {
      if (Date.now() > end) throw new Error('It is still working after an interrupt: try again in a moment.')
      await new Promise((r) => setTimeout(r, 500))
      await this.refresh()
    }
    // what it wrote last reaches its log
    await new Promise((r) => setTimeout(r, 500))
  }

  async interrupt(key: string) {
    const a = this.info(key)
    if (a) await herdr.sendKeys(this.records[key].name, ['esc']).catch(() => undefined)
  }
}

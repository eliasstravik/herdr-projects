import { TaskIds } from '../task-ids.ts'
import { trustWorktree } from '../trust.ts'
// The tasks engine. A task is added by a coordinator and at once gets one agent, in its own git worktree and branch
// from main. The coordinator's brief is the agent's first message and decides everything else (wait for a go, plan
// first, ask a reviewer, merge or wait for a yes). Its status comes from that agent alone: working, waiting on you, To
// do (it waits for a go: shepherdr todo), ready (shepherdr ready) or done (shepherdr merge, or closed with nothing to
// merge: shepherdr task close, or shepherdr ready on a branch with no changes). A task may also be queued after other
// tasks (--after): its agent starts all the same and waits until Shepherdr tells it to go. On a GitHub repo gh is
// logged in to, its work goes through a pull request: shepherdr pr pushes its branch and opens one (a draft until it
// reports done), shepherdr checks waits for GitHub's checks, and shepherdr merge squash-merges it on GitHub at the
// commit this machine has, then brings main here up to date. Anywhere else it is squash-merged here.
import { existsSync, statSync } from '../mfs.ts'
import { agentEnv } from '../box.ts'
import { join } from 'node:path'
import { STATUS_MAX, TITLE_MAX, type Question, type Report, type Task, type TaskStatus, type UpdateCard } from '../../../shared/types.ts'
import { Emitter } from '../../source.ts'
import { readJson, writeJson } from '../../store.ts'
import { ROOT } from '../../root.ts'
import { has } from '../../plugins.ts'
import { browserEnv } from '../browser.ts'
import { hidden, ROUTING_FAILURE, type Agents, type Spec } from '../agents.ts'
import type { Chats } from '../chats.ts'
import { HOST, placeId, type Places, type Space } from '../places.ts'
import { privatePrompt, goLine, handOverLine, restartLine, resumedLine, settingsBlock, prNewsLine, queuedLine, splitBrief, taskFirst, taskPrompt } from '../prompts.ts'
import { inMins, type Retry } from '../retry.ts'
import { pastTimes } from '../estimates.ts'
import { CLI_NAME, kindOf, type Settings } from '../settings.ts'
import { fitHarness, harnessWords, recentChat } from '../restart.ts'
import * as G from './git.ts'
import { Worktrees, type HerdrSpaces, type Made } from './worktrees.ts'

export interface TaskRec {
  id: string
  n: number
  project: string
  title: string
  desc: string
  by: string
  /** Exact coordinator chat that requested this task, when known. */
  requester?: string
  created: number
  updated: number
  branch?: string
  worktree?: string
  base?: string
  /** its pull request on GitHub, once one is open */
  pr?: { number: number; url: string }
  /** the commit Shepherdr last pushed to its branch on GitHub, and when: checks a push starts take a moment to show */
  pushed?: { sha: string; at: number }
  /** what its agent knows of its pull request, so news is told once: comments from before `since` (when it first
   *  reported done) and those in `seen` (by id), and the checks that failed at a commit (`failed`) */
  watch?: { since: string; seen?: string[]; failed?: string }
  /** the agent's own words when it reported done */
  said?: string
  /** its facts line (PR, commit, checks, review, not proven), apart from the summary */
  facts?: string
  /** the agent ran shepherdr ready and nothing was asked of it since */
  reported?: boolean
  /** on its way into main since it was reported ready: merging, GitHub's checks, a rebase (Merging) */
  landing?: Landing
  merged?: boolean
  /** the commit its squash merge made on its base: what its branch is checked against when it is put away */
  squash?: string
  /** done with nothing merged (shepherdr task close, or done with no changes): Done all the same */
  closed?: boolean
  /** when it was merged or closed; tasks from before it was kept fall back to their last update */
  doneAt?: number
  /** when its agent first reported done: how long tasks take, for agents' guesses of their time left (estimates.ts) */
  reportedAt?: number
  /** its place was put away once done: agent, space, worktree and branch (what lost nothing) */
  cleaned?: boolean
  /** what putting it away kept, and why ("~/x (uncommitted changes)") */
  kept?: string[]
  /** starting it went wrong */
  trouble?: string
  plan?: string[]
  /** its agent waits for a go (shepherdr todo); anything written to it clears it */
  todo?: boolean
  /** queued: the tasks it waits for ("SHE-12", or "vm-str:SHE-12" on another machine); cleared by the go Shepherdr
   *  sends once they are all done, or by anything written to it */
  after?: string[]
  /** the go sent to its agent, until the agent is seen at work: a restart in between sends it again */
  going?: string
  /** held by Shepherdr until this machine has room for it (Settings → the machine's limits): why it waits (all its
   *  slots are taken, or the machine is too loaded) and since when. go: the message its agent gets then, and why it
   *  starts (none: it has no agent yet, and starts then on its brief). Anything you write to it lets it go at once */
  held?: { why: 'slots' | 'machine'; since: number; go?: string; goWhy?: string; plain?: boolean }
  /** shepherdr task stop closed its agent and space; writing to it again clears it */
  stopped?: boolean
  /** the status coordinators were last told, so each change is told once */
  told?: TaskStatus
  /** the state last posted as a card (waiting on you, or done), so each is posted once */
  posted?: string
  /** when it was archived: out of every list and count, its record and chat kept (search and Archived show it) */
  archived?: number
  /** when it was last brought back: a Done task is archived again only that long after */
  unarchived?: number
  /** paused from the screen or task pause: 'sent' until its agent is seen stopped, then 'idle'. Seen at work again
   *  after that (work it left running in the background woke it), it is no longer paused: it works, and shows so */
  paused?: 'sent' | 'idle'
  /** when it was pinned: it shows in the Pinned group at the top of its lists; archiving or deleting it drops the pin */
  pinned?: number
  /** the CLI, model and effort its agent first starts with: from Settings, or task add's flags */
  agent?: Harness
  /** the CLI, model and effort its brief says reviews its change: from Settings, or task add's --reviewer flags */
  reviewer?: Harness
  /** shepherdr merge waits for GitHub's checks, as its brief's Settings block said when it was added (Settings, or
   *  task add --set); none: a task from before, which follows Settings as they are now */
  waitChecks?: boolean
  /** what its agent last said of its own progress (shepherdr status): kept while it works or waits on you */
  report?: Reported
  /** its backup agent took over once its agent couldn't go on (retry.ts): from which CLI, to which, and why */
  tookOver?: { from: string; to: string; why: string; at: number }
  /** coordinators were told it stalled: its last sign of life then (`life`), and the report (by its time) it ran far
   *  past (`over`); each is told once, until it shows life again or reports anew */
  stalled?: { life?: number; over?: number }
}

/**
 * A ready task on its way into main, from the first shepherdr merge until it is merged or back at work: what it waits
 * for (GitHub's checks, its agent's rebase, the merge itself), since when, and when the checks should be through (ms).
 */
export interface Landing {
  step: 'checks' | 'rebase' | 'merge'
  at: number
  due?: number
  /** its agent, at rest with the checks through, was told to merge once */
  nudged?: boolean
}

/** a landing task's line: "Checks running · ~3m", "Rebasing on main", "Merging" */
export function landingLine(l: Landing, base = 'main', now = Date.now()) {
  if (l.step === 'rebase') return `Rebasing on ${base}`
  if (l.step === 'merge') return 'Merging'
  const left = l.due === undefined ? 0 : Math.ceil((l.due - now) / 60_000)
  return left > 0 ? `Checks running · ~${left}m` : 'Checks running'
}

/** a report as kept: its line, % done and minutes left as said `at` then; `pausedAt` while it waits on you */
export interface Reported {
  msg: string
  progress: number
  left?: number
  at: number
  pausedAt?: number
}

/** how an agent starts: its herdr kind (claude, codex, pi, …), model and effort; nothing for the CLI's own default */
export interface Harness {
  kind: string
  model?: string
  effort?: string
}

/** what a task that waits on you shows instead of its agent's message: a few words on why, never the message itself */
export const NEED = {
  plan: 'Plan to approve',
  permission: 'Asks permission',
  login: 'Signed out',
  question: 'Question for you',
  reply: 'Waits for your reply',
} as const

/** a CLI's own short error when its login ran out (Claude Code's API errors, its menu): never a reply about logins */
const LOGIN = /^\s*API Error\b.*(authentication_error|oauth token has expired|invalid api key|please run \/login)|^\s*(invalid api key|oauth token has expired)\b.*please run \/login/is
const loginGone = (text = '') => text.length < 400 && LOGIN.test(text)

/** A status line cut to STATUS_MAX characters at a word, so it ends cleanly: no half word, no dangling comma or "and". */
export function fitStatus(text: string) {
  const msg = text.replace(/\s+/g, ' ').trim()
  if (msg.length <= STATUS_MAX) return msg
  let cut = msg.slice(0, STATUS_MAX + 1).replace(/\s+\S*$/, '')
  if (cut.length > STATUS_MAX) return msg.slice(0, STATUS_MAX) // one word longer than the line
  for (let was = ''; was !== cut; ) {
    was = cut
    cut = cut.replace(/[\s,;:.(&+–—-]+$/, '').replace(/\s+(and|or|but|then|to|the|a|an|of|for|with|in|on|at|from|by|into)$/i, '')
  }
  return cut
}

/**
 * The report a task shows: while it works, its minutes left count down from what its agent said; while it waits on
 * you, all of it stays as it was. Nothing once its job is over (Ready, Done, To do, Queued).
 */
export function reportOf(r: Reported | undefined, status: TaskStatus, now = Date.now()): Report | undefined {
  if (!r || (status !== 'work' && status !== 'need')) return undefined
  const spent = ((r.pausedAt ?? now) - r.at) / 60000
  return { msg: r.msg, progress: r.progress, eta: r.left === undefined ? undefined : Math.max(0, Math.round(r.left - spent)), paused: status === 'need' || undefined }
}

/** a question mark that ends a sentence, not one in code (a?.b) or a path */
const ASKS = /\?(?=[\s"'”’)\]*_]|$)/

/**
 * The question an agent's last words ask you, as a status line: the last sentence that asks in its last paragraph
 * (links and code aside), cut to fit. Nothing when they ask nothing.
 */
export function askOf(text = ''): string | undefined {
  const last = text.replace(/```[\s\S]*?```/g, '').replace(/\S+:\/\/\S+/g, '').trim().split(/\n\s*\n/).at(-1) ?? ''
  const q = last.split(/\n+|(?<=[.!?])\s+(?=\S)/).findLast((s) => ASKS.test(s.replace(/`[^`]*`/g, '')))
  return q ? fitStatus(q.replace(/^\s*(?:[-*+>]|\d+[.)])\s+/, '').replace(/[*_`]+/g, '')) || undefined : undefined
}

/** finished: merged, or closed with nothing to merge */
export const ended = (t: TaskRec) => !!(t.merged || t.closed)

const STATE: Record<TaskStatus, string> = { need: 'Needs you', ready: 'Ready', merging: 'Merging', work: 'Working', queued: 'Queued', todo: 'To do', done: 'Done' }

/** one of the tasks a queued task waits for, as it stands: done, still open, ended without being done, gone, or on a
 *  machine that can't be reached right now */
type DepState = 'done' | 'open' | 'stopped' | 'failed' | 'gone' | 'away'

/** another machine's task, as it last showed itself: 'away' while that machine can't be reached, 'unknown' for a
 *  machine this one doesn't know, nothing when that machine has no such task */
export type PeerTask = (machine: string, id: string) => Task | 'away' | 'unknown' | undefined

/** "vm-str:SHE-12" is another machine's task; this machine's own name leads to its own */
const splitRef = (ref: string): [string | undefined, string] => {
  const i = ref.indexOf(':')
  if (i < 0) return [undefined, ref]
  const machine = ref.slice(0, i).toLowerCase()
  return [machine === HOST.toLowerCase() ? undefined : machine, ref.slice(i + 1)]
}

/** while load decides, held tasks start one at a time this far apart, so each one shows in the load first */
const START_GAP = 30_000

/** the line a held task shows */
export const HELD = { slots: 'Waiting for a free slot', machine: 'Waiting for the machine' } as const

/** queued tasks are first looked at once herdr has told which agents are alive: a go sent before would start a second */
const RECHECK_FIRST = 15_000

/** the branch every task's worktree gets */
const branchFor = (t: TaskRec) => `work/${t.n}-${G.slug(t.title)}`

/** its herdr space's name */
const labelOf = (t: TaskRec) => `${t.id} ${t.title}`

/** "7 days", "1 day", "off": how long a Done task stays before it archives itself (tasks.archiveAfter); null: never */
export function archiveAfterMs(v: string) {
  const m = v.trim().match(/^(\d+)(\s*days?)?$/i)
  return m && Number(m[1]) > 0 ? Number(m[1]) * 86_400_000 : null
}

/** leftovers of finished tasks are swept up this long after starting, and then every hour */
const SWEEP_FIRST = 20_000
const SWEEP_EVERY = 3_600_000

/** how long shepherdr checks waits for GitHub's checks at most, and how often it looks */
const CHECKS_WAIT = 9 * 60_000
const CHECKS_EVERY = 10_000
/** checks a repo's workflows will start may take this long to show on a pull request just pushed */
const CHECKS_SHOW = 90_000

/** shepherdr merge answers within this long: under the two minutes after which an agent's shell (Claude Code's)
 *  moves a command to the background. The shepherdr command itself gives up a little later (server/cli.ts, the
 *  helper's shepherdr) */
export const MERGE_DEADLINE = 90_000

/** done tasks' pull requests are looked at for news (failed checks, comments) this long after starting, then this often */
const WATCH_FIRST = 30_000
const WATCH_EVERY = 120_000
/** a working task with no sign of life this long (no output, status report or commit) has stalled; looked at this often */
export const STALL_QUIET = 20 * 60_000
const STALL_EVERY = 60_000
/** main's checks on GitHub are watched this long after a task merged through its pull request */
const MAIN_WATCH = 3 * 3_600_000

/** the failed checks at a commit, as one key: the same failure is told once */
const failKey = (head: string, c: G.PrChecks) => `${head}:${c.failed.map((f) => f.name).sort().join(',')}`

/** what someone wrote, as quoted to an agent: indented, and cut when long */
const quote = (s: string, max = 1500) => (s.length > max ? `${s.slice(0, max)}…` : s).split('\n').map((l) => `    ${l}`.trimEnd()).join('\n')

const COMMENT: Record<string, string> = { APPROVED: 'Approved', CHANGES_REQUESTED: 'Changes requested', COMMENTED: 'Review' }

/** now, as GitHub writes times (to the second): what is compared with them */
const ghNow = () => new Date().toISOString().replace(/\.\d+Z$/, 'Z')

/** a pull request's footer, as the agent's CLI signs its work */
const SIGNED: Record<string, string> = { claude: '🤖 Generated with [Claude Code](https://claude.com/claude-code)', codex: '🤖 Generated with [Codex](https://openai.com/codex)' }

const troubleOf = (e: unknown) => ((e as Error)?.message ?? String(e)).split('\n')[0].slice(0, 90)

/** a title fits one row: one too long (from an automation saved before the limit) is cut at a word */
const short = (title: string) => (title.length > TITLE_MAX ? `${title.slice(0, TITLE_MAX).replace(/\s+\S*$/, '').slice(0, TITLE_MAX - 1)}…` : title)

/** A title someone writes is refused when it would not fit one line of the tasks list on a phone. */
export function checkTitle(title: string) {
  const t = title.trim()
  if (t.length > TITLE_MAX)
    throw new Error(`That title is ${t.length} characters: write a shorter one, ${TITLE_MAX} at most, so it fits one line on a phone (the rest goes in the brief)`)
  return t
}

export function age(t: number) {
  const s = Math.max(0, (Date.now() - t) / 1000)
  if (s < 60) return 'now'
  if (s < 3600) return `${Math.floor(s / 60)}m`
  if (s < 86400) return `${Math.floor(s / 3600)}h`
  return `${Math.floor(s / 86400)}d`
}

const ago = (t: number) => (age(t) === 'now' ? 'just now' : `${age(t)} ago`)

function clock(t: number) {
  const d = new Date(t)
  const today = new Date().toDateString() === d.toDateString()
  const hm = d.toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' })
  return today ? `today ${hm}` : `${d.toLocaleDateString('en-GB', { day: 'numeric', month: 'short' })} ${hm}`
}

type Events = {
  changed: []
  /** news for coordinators: a task was added, waits on you, is ready, merged or closed; main went red on GitHub */
  /** card: one for the project's chat that no task's status posts (main went red) */
  news: [project: string, text: string, kind: 'added' | 'ready' | 'need' | 'done' | 'red' | 'stalled', about: { task: string; event: 'added' | 'ready' | 'needs-you' | 'merged' | 'closed' | 'main-red' | 'stalled' }, card?: Omit<UpdateCard, 'id'>]
  /** a card for its project's chat: the task now waits on you (needs you, or ready), or it is done */
  card: [task: TaskRec, status: TaskStatus, sub: string]
  /** a task was stopped, merged, closed or deleted: what it had running beside its agent (its shared browser) goes;
   *  for good (forget) unless it was only stopped */
  gone: [task: TaskRec, forget: boolean]
}

/** a message you wrote makes the agent busy a moment later: until then it counts as working */
const ANSWER_GRACE = 6000

/** an agent that never gets to work on its brief leaves To do after this long anyway (and then waits on you) */
const BRIEF_WAIT = 120_000

/** what a task for later (task add --later) is told: its agent reads the brief and waits for a go */
const LATER = "Don't start yet: wait for a go."

export class Engine extends Emitter<Events> {
  /** main: by project path, the failing commit on GitHub's main its coordinator was told of, until main is green */
  private data = readJson<{ next: Record<string, number>; tasks: TaskRec[]; red?: Record<string, string> }>('tasks.json', { next: {}, tasks: [] })
  readonly taskIds: TaskIds
  private starting = new Set<string>()
  /** started, its brief not yet taken up: To do until its agent works on it */
  private briefing = new Set<string>()
  private wrote = new Map<string, number>()
  /** when its agent last reported its progress (shepherdr status) */
  private heard = new Map<string, number>()
  private later = new Map<string, ReturnType<typeof setTimeout>>()
  /** one merge at a time per repo: each one's main checkout is shared */
  private merging = new Map<string, Promise<unknown>>()
  /** each task's merge while it runs */
  private landings = new Map<string, Promise<{ ok: boolean; error?: string; note?: string }>>()
  /** how long shepherdr merge may take before it answers that it isn't through yet (tests shorten it) */
  mergeDeadline = MERGE_DEADLINE
  /** stopping: no merge starts */
  private closing = false
  /** how long GitHub's checks took lately, per repo, and when that was asked */
  private took = new Map<string, { ms?: number; at: number }>()
  /** tasks whose place is being put away */
  private cleaning = new Set<string>()
  private sweeps: ReturnType<typeof setTimeout>[] = []
  private wt: Worktrees
  /** other machines' tasks, for a task queued after one (the main copy only: it is the one that sees them) */
  peerTask?: PeerTask
  /** its agent's turn the provider cut off (retry.ts): waited out, or given up */
  retry?: (key: string) => Retry | undefined
  /** its backup agent is taking over */
  private handing = new Set<string>()
  /** a CLI is installed here (a test says) */
  installed = (kind: string) => !!Bun.which(kind)
  /** how loaded this machine is now: over a start limit (health.ts busy), and whether herdr answers; none until first
   *  measured, which holds nothing by load */
  machine?: () => { busy?: 'cpu' | 'memory'; herdr: boolean } | undefined
  /** when a held task last started while load decided */
  private lastStart = 0
  private releasing = false
  private checking = false
  /** a look at done tasks' pull requests is under way */
  private watching = false
  private watchingMain = false

  constructor(
    private agents: Agents,
    private chats: Chats,
    private places: Places,
    private settings: Settings,
    h?: HerdrSpaces,
  ) {
    super()
    this.taskIds = new TaskIds(this.places.spaces, this.data, (name) => placeId.space(name))
    this.places.takenTaskKeys = () => this.taskIds.keys()
    this.wt = new Worktrees(h)
    let snapped = false
    // tasks from before statuses came from the agent: a finished one is merged, and was cleaned up then
    for (const t of this.data.tasks as (Omit<TaskRec, 'after'> & { stage?: string; after?: string | string[] })[]) {
      if (t.stage === 'done') Object.assign(t, { merged: true, cleaned: true })
      delete t.stage
      // Freeze older queued records before any space setting can change.
      if (!t.base) {
        t.base = this.places.space(t.project)?.base
        snapped ||= !!t.base
      }
      // from an older way a task waited on another one's merge
      if (typeof t.after === 'string') delete t.after
      // from when the Watcher put what a task asks in a few words
      delete (t as { gist?: unknown }).gist
    }
    if (snapped) this.save()
    queueMicrotask(() => {
      // saved for later with no agent, before every task got one at once: its agent starts now and waits for a go
      let chain = Promise.resolve()
      for (const t of this.tasks.filter((t) => t.told === 'todo' && !t.worktree && !ended(t) && !t.trouble)) {
        if (!t.desc.includes(LATER)) t.desc = `${t.desc || t.title}\n\n${LATER}`
        chain = chain.then(() => this.startTask(t)).catch((e) => this.troubled(t, e))
      }
    })
    const recheck = setTimeout(() => this.resumeQueue(), RECHECK_FIRST)
    recheck.unref?.()
    this.sweeps.push(recheck)
    // a repo that gained its GitHub remote after it was added shows it from the start
    queueMicrotask(() => Promise.all(this.places.spaces.map((p) => this.places.recheckGithub(p))).then(() => this.emit('changed'), () => undefined))
    // once herdr has told which agents are at work: a restart (main updated by a merge) cuts a clean-up short
    const first = setTimeout(() => {
      this.autoArchive()
      this.sweep().catch(() => undefined)
    }, SWEEP_FIRST)
    const every = setInterval(() => {
      this.autoArchive()
      this.sweep().catch(() => undefined)
    }, SWEEP_EVERY)
    first.unref?.()
    every.unref?.()
    this.sweeps.push(first, every)
    const watch = () => this.watchPrs().then(() => this.watchMains()).catch(() => undefined)
    const watchFirst = setTimeout(watch, WATCH_FIRST)
    const watchEvery = setInterval(watch, WATCH_EVERY)
    watchFirst.unref?.()
    watchEvery.unref?.()
    this.sweeps.push(watchFirst, watchEvery)
    const stalls = setInterval(() => this.checkStalls(), STALL_EVERY)
    stalls.unref?.()
    this.sweeps.push(stalls)
  }

  get tasks() {
    return this.data.tasks
  }

  task(id: string) {
    return this.data.tasks.find((t) => t.id.toUpperCase() === id.toUpperCase())
  }

  private save() {
    writeJson('tasks.json', this.data)
    this.emit('changed')
  }

  private proj(t: TaskRec): Space {
    const p = this.places.space(t.project)
    if (!p) throw new Error(`${t.id}: its project is gone`)
    return p
  }

  private key(t: TaskRec) {
    return `${t.id}/Agent`
  }

  stop() {
    this.taskIds.close()
    for (const x of this.later.values()) clearTimeout(x)
    for (const x of this.sweeps) clearTimeout(x)
  }

  /* ---------- status: from the agent alone ---------- */

  /** the one status the task list, the task's own chat, the place dots and the command all show */
  statusOf(t: TaskRec): TaskStatus {
    if (ended(t)) return 'done'
    if (t.trouble) return 'need'
    const key = this.key(t)
    const st = this.agents.status(key)
    if (st === 'blocked' || this.asks(key)) return 'need'
    // queued: until they are all done, or one ends without being done (then it waits for you to decide)
    if (this.queued(t)) return this.halts(t).length ? 'need' : 'queued'
    // held until the machine has room
    if (this.holding(t)) return 'queued'
    if (t.todo || this.starting.has(t.id) || this.briefing.has(t.id)) return 'todo'
    // on its way into main, nothing for you to do: unless its agent stopped to ask you something (then it is Ready)
    if (t.reported && t.landing && (st === 'working' || !this.asks(key))) return 'merging'
    if (st === 'working' || this.handing.has(t.id)) return 'work'
    if (this.agents.waiting(key).length || Date.now() - (this.wrote.get(t.id) ?? 0) < ANSWER_GRACE) return 'work'
    if (t.reported) return 'ready'
    if (this.retrying(t)) return 'work'
    // its turn ended while work it started in the background (a subagent, a shell, a monitor) still runs and will wake
    // it: a pause, not a question, unless what it said last asks you something
    if (this.agents.background(key) && !this.asks(key)) return 'work'
    // its turn ended (or it stopped) without reporting done: it waits for you
    return 'need'
  }

  /** the provider cut its last turn off and it will be told to carry on, or was just now */
  private retrying(t: TaskRec) {
    const r = this.mayRetry(this.key(t)) ? this.retry?.(this.key(t)) : undefined
    return !!r && !r.over && (!!r.due || Date.now() - (r.nudged ?? 0) < 60_000)
  }

  /** a task's agent the provider cut off may be told to carry on: not stopped, paused, waiting for a go, archived or
   *  finished since */
  mayRetry(key: string) {
    const t = this.task(key.split('/')[0])
    return !!t && key === this.key(t) && !t.stopped && !t.todo && !t.archived && !ended(t)
  }

  /** it asks you something: a choice card or the CLI's own question still open (facts, never a guess from its words) */
  private asks(key: string) {
    return !!(this.chats.openCard(key) || this.chats.openQuestion(key))
  }

  /**
   * the second line of a list row: what the status is about, in one short standard line, never the brief. While it
   * works, its agent's own status line (shepherdr status) takes its place on screen and in the CLI (source.ts).
   */
  subOf(t: TaskRec, status = this.statusOf(t)): string {
    const key = this.key(t)
    const s = (() => {
      // a few words, never the summary: that is in its chat's card
      if (status === 'done') return t.closed ? 'Nothing to merge' : 'Merged'
      if (status === 'need' && (this.agents.status(key) === 'blocked' || this.asks(key))) return this.reasonOf(key) ?? NEED.question
      if (t.trouble) return fitStatus(`Went wrong: ${t.trouble}`)
      // a To do row is its title alone
      if (status === 'todo') return ''
      if (status === 'queued') return this.holding(t) ? HELD[t.held!.why] : this.waitLine(t)
      if (status === 'need' && this.queued(t) && this.agents.status(key) !== 'blocked') return fitStatus(this.halts(t).join(' · '))
      if (status === 'need') return this.reasonOf(key) ?? NEED.reply
      if (status === 'ready') return 'Ready to merge'
      if (status === 'merging') return landingLine(t.landing!, t.base)
      if (this.handing.has(t.id) && t.tookOver) return fitStatus(`${CLI_NAME[t.tookOver.to] ?? t.tookOver.to} takes over: ${t.tookOver.why}`)
      const r = this.retry?.(key)
      if (r?.due) return `${r.reason} · retry in ${inMins(r.due)}`
      return 'Working'
    })()
    return s.length > 90 ? `${s.slice(0, 89).trimEnd()}…` : s
  }

  /**
   * why its agent waits on you: a standard reason, or the agent's own question cut to a status line. From what the agent
   * reports (its hooks, its log, its cards), never a model's reading of it.
   */
  private reasonOf(key: string): string | undefined {
    const cut = this.retry?.(key)
    if (cut?.over) return fitStatus(cut.reason)
    const said = this.agents.said(key)
    if (this.agents.status(key) === 'blocked') {
      if (said?.state === 'blocked') return said.kind === 'plan' ? NEED.plan : said.kind === 'question' ? (said.ask && fitStatus(said.ask)) || NEED.question : NEED.permission
      // a subagent of its own asks for a permission
      if (said?.helper) return NEED.permission
      const q = this.chats.openQuestion(key)
      if (q) return fitStatus(q.items[0]?.question ?? '') || NEED.question
      return loginGone(this.chats.menu(key)?.title) ? NEED.login : NEED.permission
    }
    const card = this.chats.openCard(key)
    if (card) return card === 'plan' ? NEED.plan : fitStatus(this.chats.openCardTitle(key) ?? '') || NEED.question
    const question = this.chats.openQuestion(key)
    if (question) return fitStatus(question.items[0]?.question ?? '') || NEED.question
    const last = this.chats.lastReply(key)
    if (said?.kind === 'auth' || loginGone(last)) return NEED.login
    return askOf(last)
  }

  /** settle one task, then the tasks queued after others: what it did may be what one of them waits for */
  private settle(t: TaskRec) {
    this.settleOne(t)
    this.recheck()
    // what it did may free room for a held one
    this.release()
  }

  /** tell coordinators and the screen when the status changed; a card when it now waits on you or is done */
  private settleOne(t: TaskRec) {
    const status = this.statusOf(t)
    this.pace(t, status)
    if (status === t.told) return this.emit('changed')
    // archived: put away quietly, no card, news or alert
    if (t.archived) return this.quiet(t, status)
    const before = t.told
    t.told = status
    const sub = this.subOf(t, status)
    // a task from before this was told: what it shows now is where it starts, not news
    if (before === undefined) {
      t.posted = `${status}|${sub}`
      return this.save()
    }
    t.updated = Date.now()
    // back at work: the next time it waits is news again, even with the same words
    if (status === 'work') t.posted = undefined
    if (status === 'need' || status === 'ready') {
      const key = `${status}|${sub}`
      if (t.posted !== key) {
        t.posted = key
        this.emit('card', t, status, sub)
        this.emit('news', t.project, `${t.id} ${status === 'ready' ? `is ready to merge${t.said ? `: ${t.said}` : ''}${t.facts ? ` (Facts: ${t.facts})` : ''}` : `needs you: ${sub}`}`, status === 'ready' ? 'ready' : 'need', { task: t.id, event: status === 'ready' ? 'ready' : 'needs-you' })
      }
    }
    if (status === 'done' && before !== 'done') {
      t.posted = `done|${sub}`
      this.emit('card', t, status, sub)
      this.emit('news', t.project, `${t.id} ${t.closed ? `is done, nothing to merge${t.said ? `: ${t.said}` : ''}` : 'was merged'}`, 'done', { task: t.id, event: t.closed ? 'closed' : 'merged' })
    }
    this.save()
  }

  /** its report follows its status: its clock stops while it waits on you, and the report goes once its job is over */
  private pace(t: TaskRec, status: TaskStatus) {
    const r = t.report
    if (!r) return
    if (status === 'need') r.pausedAt ??= Date.now()
    else if (status !== 'work') t.report = undefined
    else if (r.pausedAt) {
      r.at += Date.now() - r.pausedAt
      r.pausedAt = undefined
    }
  }

  /** what it shows now is taken as told and posted: nothing about it is news */
  private quiet(t: TaskRec, status = this.statusOf(t)) {
    t.told = status
    t.posted = `${status}|${this.subOf(t, status)}`
    this.save()
  }

  /** a turn that ends may be followed at once by the next: wait a moment before telling anyone it waits */
  private settleSoon(t: TaskRec, ms = 4000) {
    clearTimeout(this.later.get(t.id))
    this.later.set(
      t.id,
      setTimeout(() => {
        this.later.delete(t.id)
        const t2 = this.task(t.id)
        if (t2) this.settle(t2)
      }, ms),
    )
  }

  /* ---------- the view the screen draws ---------- */

  view(t: TaskRec): Task {
    const status = this.statusOf(t)
    const key = this.key(t)
    const rec = this.agents.record(key)
    const cli = CLI_NAME[rec?.kind ?? t.agent?.kind ?? ''] ?? this.settings.get('task.agent', this.nameOf(t))
    const sub = this.subOf(t, status)
    const doneAt = status === 'done' ? (t.doneAt ?? t.updated) : undefined
    return {
      id: t.id,
      name: t.title,
      status,
      place: t.project,
      sub,
      age: age(t.archived ?? doneAt ?? (status === 'todo' ? t.created : t.updated)),
      doneAt,
      branch: t.branch,
      base: t.base,
      pr: t.pr,
      plan: t.plan,
      agent: rec || t.worktree ? { chat: key, role: 'Agent', cli, status, age: age(rec?.started ?? t.updated), note: sub } : undefined,
      desc: t.desc || 'No brief yet.',
      said: t.said,
      facts: t.facts,
      state: t.archived ? `Archived · ${ago(t.archived)}` : status === 'todo' ? `To do · added ${ago(t.created)}` : t.closed ? `Done · nothing to merge · ${ago(doneAt ?? t.updated)}` : status === 'done' ? `Done · merged ${ago(doneAt ?? t.updated)}` : `${STATE[status]} · ${ago(doneAt ?? t.updated)}`,
      on: cli,
      created: `${clock(t.created)} by ${t.by}`,
      after: this.queued(t) ? this.open(t) : undefined,
      outcome: this.outcomeOf(t),
      archived: t.archived,
      pinned: t.pinned,
      // waiting out the provider, or a backup taking over: that line shows, not what its agent last reported
      report: this.retrying(t) || this.handing.has(t.id) ? undefined : reportOf(t.report, status),
    }
  }

  /* ---------- stalled ---------- */

  /**
   * Working tasks that went quiet (no sign of life for STALL_QUIET) or ran far past the time their agent said was left:
   * news for their coordinator, once per stall. Only time and liveness, which agents can't see of themselves: what to
   * do about it is the coordinator's call.
   */
  checkStalls(now = Date.now()) {
    for (const t of this.tasks) {
      if (t.archived || t.paused || this.statusOf(t) !== 'work' || this.retrying(t) || this.handing.has(t.id)) continue
      const life = this.lifeOf(t)
      const r = t.report
      // shown life since: its next stall is news again
      if (t.stalled?.life !== undefined && life > t.stalled.life) t.stalled.life = undefined
      const quiet = now - life >= STALL_QUIET && t.stalled?.life === undefined
      const over = r?.left !== undefined && r.at !== t.stalled?.over && now - r.at > (r.left + Math.max(r.left, 20)) * 60_000
      if (!quiet && !over) continue
      t.stalled = { life: quiet ? life : t.stalled?.life, over: r?.at }
      const mins = Math.max(0, Math.round((now - (quiet ? life : r!.at + r!.left! * 60_000)) / 60_000))
      this.emit('card', t, 'work', quiet ? `may be stuck: quiet for ${mins}m` : `may be stuck: ${mins}m past its time left`)
      this.emit('news', t.project, this.stallLine(t, now, quiet ? life : undefined), 'stalled', { task: t.id, event: 'stalled' })
      this.save()
    }
  }

  /** its last sign of life: its status changed, it was written to or reported, its hooks or its CLI's log moved */
  private lifeOf(t: TaskRec) {
    const key = this.key(t)
    const logs = (this.agents.record(key)?.logs ?? []).slice(-2).map((p) => {
      try {
        return statSync(p).mtimeMs
      } catch {
        return 0
      }
    })
    return Math.max(t.updated, this.wrote.get(t.id) ?? 0, this.heard.get(t.id) ?? 0, this.agents.said(key)?.at ?? 0, ...logs)
  }

  /** what Shepherdr saw of a stall: how long quiet (or past its estimate), its last status line, whether its agent runs */
  private stallLine(t: TaskRec, now: number, quietSince?: number) {
    const key = this.key(t)
    const mins = (ms: number) => `${Math.max(0, Math.round(ms / 60_000))}m`
    const r = t.report
    const bg = this.agents.status(key) !== 'working' && this.agents.background(key)
    const ran = { agents: 'a subagent', shells: 'a shell', servers: 'a server' }
    const why =
      quietSince === undefined
        ? `it has worked ${mins(now - r!.at)} since its agent said ${r!.left}m were left`
        : bg
          ? `its agent's turn ended while ${ran[bg]} it started runs on, and nothing has changed for ${mins(now - quietSince)}`
          : `its agent shows as working, but nothing has changed for ${mins(now - quietSince)}: no new output, status report or commit`
    const heard = this.heard.get(t.id)
    const last = r ? `Its last status: "${r.msg}", ${r.progress}% done${heard ? `, ${mins(now - heard)} ago` : ''}` : 'It has reported no status'
    const info = this.agents.info(key)
    const alive = !info ? 'herdr shows no pane for its agent' : info.agent ? 'its agent process is running' : 'herdr sees no agent CLI running in its pane'
    return `${t.id} may have stalled: ${why}. ${last}; ${alive}. Nudge it (shepherdr task tell ${t.id} "…"), stop it, or tell the human`
  }

  /* ---------- queued after other tasks ---------- */

  /** it waits for other tasks (a stopped task waits for nothing until it is written to again) */
  private queued(t: TaskRec) {
    return !!t.after?.length && !ended(t) && !t.stopped
  }

  /** how it ended, for a task queued after it: merged, or closed with nothing to merge, is done */
  outcomeOf(t: TaskRec): Task['outcome'] {
    if (ended(t)) return 'done'
    if (t.trouble) return 'failed'
    if (t.stopped) return 'stopped'
  }

  private depState(ref: string): DepState {
    const [machine, id] = splitRef(ref)
    if (machine) {
      const x = this.peerTask?.(machine, id)
      if (x === 'away') return 'away'
      if (!x || x === 'unknown') return 'gone'
      return x.outcome ?? (x.status === 'done' ? 'done' : 'open')
    }
    const d = this.task(id)
    if (!d || (!ended(d) && !this.places.space(d.project))) return 'gone'
    return this.outcomeOf(d) ?? 'open'
  }

  /** the tasks it still waits for */
  private open(t: TaskRec) {
    return (t.after ?? []).filter((ref) => this.depState(ref) !== 'done')
  }

  /** why it can't go on by itself: a task it waits for was stopped, went wrong or is gone */
  private halts(t: TaskRec) {
    const why: Partial<Record<DepState, string>> = { stopped: 'was stopped', failed: 'went wrong', gone: 'is gone' }
    return (t.after ?? []).flatMap((ref) => {
      const w = why[this.depState(ref)]
      return w ? [`${ref} ${w}`] : []
    })
  }

  /** a queued task's line: what it still waits for ("After SHE-12, SHE-13"), and a machine that can't be reached */
  private waitLine(t: TaskRec) {
    const open = this.open(t)
    const away = [...new Set(open.filter((ref) => this.depState(ref) === 'away').map((ref) => splitRef(ref)[0]))]
    return `After ${open.join(', ')}${away.length ? ` · can't reach ${away.join(', ')}` : ''}`
  }

  /**
   * Queued tasks: each whose tasks are all done is told to go; the rest show what they wait for, or that one of them
   * ended without being done (then it waits for you).
   */
  recheck() {
    if (this.checking) return
    this.checking = true
    try {
      for (const t of this.tasks.filter((t) => this.queued(t))) {
        if (!this.open(t).length) this.goWhenRoom(t, goLine(t.after!, this.notes(t.after!)), `${t.after!.join(', ')} ${t.after!.length === 1 ? 'is' : 'are'} done`)
        else if (this.statusOf(t) !== t.told) this.settleOne(t)
      }
    } finally {
      this.checking = false
    }
  }

  /**
   * After a start: a go that never reached its agent (a restart cut it off) is sent again; then the queue is checked.
   */
  resumeQueue() {
    for (const t of this.tasks.filter((t) => t.going && !ended(t) && !t.stopped && this.agents.status(this.key(t)) !== 'working'))
      this.agents.send(this.key(t), hidden(t.going!, 'go'), { ensure: () => this.launch(t) }).catch(() => undefined)
    this.recheck()
  }

  /** what each task it waited for said when it was done */
  private notes(refs: string[]) {
    return refs.flatMap((ref) => {
      const [machine, id] = splitRef(ref)
      const x = machine ? this.peerTask?.(machine, id) : this.task(id)
      if (!x || typeof x === 'string') return []
      return x.said ? [`${ref} (${'title' in x ? x.title : x.name}): ${x.said}`] : []
    })
  }

  /** the go: its agent hears it (hidden from the chat, where a small card says why it started) and starts on the work */
  private go(t: TaskRec, text: string, why: string, plain = false) {
    // archived while it waited: it starts work, so it shows again
    if (t.archived) this.unarchive(t.id)
    t.after = undefined
    t.held = undefined
    t.todo = undefined
    t.reported = false
    this.briefing.delete(t.id)
    // a coordinator's own words go in as they are, and are not sent again after a restart
    t.going = plain ? undefined : text
    this.wrote.set(t.id, Date.now())
    const key = this.key(t)
    const id = `go:${t.id}:${Date.now()}`
    this.chats.post(key, { id, card: { id, task: t.id, status: 'work', text: `started: ${why}` } })
    this.save()
    this.agents.send(key, plain ? text : hidden(text, 'go'), { ensure: () => this.launch(t) }).catch(() => undefined)
    this.settleOne(t)
    this.settleSoon(t, ANSWER_GRACE + 500)
  }

  /**
   * What a task waits for, as written ("SHE-12", "she-12", "vm-str:SHE-12"), checked: each one there, never itself, and
   * no circle (SHE-3 waits for SHE-5, which waits for SHE-3). Kept by their ids as shown.
   */
  checkAfter(self: string | undefined, refs: string[]): string[] {
    const out: string[] = []
    for (const raw of refs.flatMap((r) => r.split(/[\s,]+/)).filter(Boolean)) {
      const [machine, id] = splitRef(raw)
      let ref: string
      if (machine) {
        if (!this.peerTask) throw new Error(`${raw}: only the main machine sees other machines' tasks, so only its tasks can wait for them`)
        const x = this.peerTask(machine, id)
        if (x === 'unknown') throw new Error(`${raw}: no machine called ${machine}`)
        if (x === 'away') throw new Error(`${raw}: can't reach ${machine} right now, so it can't be checked; try again later`)
        if (!x) throw new Error(`${raw}: ${machine} has no task ${id}`)
        ref = `${machine}:${x.id}`
      } else {
        const d = this.task(id)
        if (!d) throw new Error(`No task ${id}`)
        if (self && d.id.toUpperCase() === self.toUpperCase()) throw new Error(`${d.id} can't wait for itself`)
        ref = d.id
      }
      if (!out.includes(ref)) out.push(ref)
    }
    if (self)
      for (const ref of out) {
        const loop = this.circle(self, ref, [self])
        if (loop) throw new Error(`That would wait in a circle: ${loop.join(' → ')}`)
      }
    return out
  }

  /** the path back to `self` through what `ref` waits for in turn, if there is one */
  private circle(self: string, ref: string, path: string[]): string[] | undefined {
    if (splitRef(ref)[0]) return undefined
    const d = this.task(ref)
    if (!d) return undefined
    if (d.id.toUpperCase() === self.toUpperCase()) return [...path, d.id]
    if (path.includes(d.id)) return undefined
    for (const next of this.queued(d) ? d.after! : []) {
      const loop = this.circle(self, next, [...path, d.id])
      if (loop) return loop
    }
  }

  /**
   * Queue a task after others, change what it waits for, or (none) clear it. Its agent is told: to hold on (it may be
   * at work), what it waits for now, or to go. `self`: its own agent asked, and needs no message for it.
   */
  queue(id: string, refs: string[], opts: { self?: boolean } = {}): { queued: string[]; went?: boolean } {
    const t = this.req(id)
    if (ended(t)) throw new Error(`${t.id} is Done already (${t.closed ? 'nothing to merge' : 'merged'})`)
    const after = this.checkAfter(t.id, refs)
    const was = this.queued(t) ? t.after! : []
    const open = after.filter((ref) => this.depState(ref) !== 'done')
    if (!open.length) {
      if (!was.length) return { queued: [] }
      if (after.length) this.goWhenRoom(t, goLine(after, this.notes(after)), `${after.join(', ')} ${after.length === 1 ? 'is' : 'are'} done`)
      else this.goWhenRoom(t, goLine(was, [], true), `no longer waits for ${was.join(', ')}`)
      return { queued: [], went: true }
    }
    t.after = after
    t.stopped = undefined
    clearTimeout(this.later.get(t.id))
    const key = this.key(t)
    const cid = `queued:${t.id}:${Date.now()}`
    this.chats.post(key, { id: cid, card: { id: cid, task: t.id, status: 'queued', text: `queued after ${open.join(', ')}` } })
    this.save()
    if (!opts.self && this.agents.status(key) !== 'gone') this.agents.send(key, hidden(queuedLine(open, was.length ? 'changed' : 'later', t.id), 'queue'), { ensure: () => this.launch(t) }).catch(() => undefined)
    this.settleOne(t)
    return { queued: open }
  }

  /* ---------- room on this machine ---------- */

  /** held until this machine has room (and nothing it waits for is still open) */
  private holding(t: TaskRec) {
    return !!t.held && !ended(t) && !t.stopped && !this.queued(t)
  }

  /** a task at work, or on its way to it: those count against the machine's limit */
  private busy(t: TaskRec) {
    if (ended(t) || t.archived || t.stopped || this.holding(t) || !this.places.space(t.project)) return false
    // a go sent just now counts before its agent is seen at work; one left from long ago (its agent gone) doesn't
    const going = !!t.going && Date.now() - (this.wrote.get(t.id) ?? 0) < BRIEF_WAIT
    return this.starting.has(t.id) || going || (this.briefing.has(t.id) && !t.todo) || this.statusOf(t) === 'work'
  }

  /** how many tasks are at work here now */
  working() {
    return this.tasks.filter((t) => this.busy(t)).length
  }

  /** whether one more task may start now, and if not, why: its slots are all taken, or the machine is too loaded */
  room(): { why?: 'slots' | 'machine' } {
    const l = this.settings.machine()
    if (l.max !== null && this.working() >= l.max) return { why: 'slots' }
    const m = this.machine?.()
    if (m && (m.busy || !m.herdr)) return { why: 'machine' }
    return {}
  }

  /** tasks held now, oldest first */
  heldTasks() {
    return this.tasks.filter((t) => this.holding(t) && !t.archived).sort((a, b) => a.held!.since - b.held!.since)
  }

  /** hold it until there's room: its line says why, and its chat gets a small card */
  private hold(t: TaskRec, why: 'slots' | 'machine', go?: { text: string; why: string; plain?: boolean }) {
    const l = this.settings.machine()
    t.held = { why, since: t.held?.since ?? Date.now(), ...(go ? { go: go.text, goWhy: go.why, plain: go.plain } : {}) }
    t.after = undefined
    t.todo = undefined
    const id = `held:${t.id}:${Date.now()}`
    const words = why === 'slots' ? `waiting for a free slot (${this.working()} of ${l.max} working)` : 'waiting for the machine: it is busy'
    this.chats.post(this.key(t), { id, card: { id, task: t.id, status: 'queued', text: words } })
    this.save()
    this.settleOne(t)
  }

  /** the go, now if this machine has room, else once it has */
  private goWhenRoom(t: TaskRec, text: string, why: string, plain = false) {
    const r = this.room()
    if (r.why) return this.hold(t, r.why, { text, why, plain })
    this.go(t, text, why, plain)
  }

  /**
   * Start held tasks while this machine has room, oldest first. While load decides, one at a time, START_GAP apart:
   * the load average takes a moment to show what a task adds. Called on each health sample and status change.
   */
  release(now = Date.now()) {
    if (this.releasing) return
    this.releasing = true
    try {
      const byLoad = this.settings.machine().startLoad !== null
      for (const t of this.heldTasks()) {
        const r = this.room()
        if (r.why) {
          // each one waits for the same reason: its line says the one that holds them now
          const stale = this.heldTasks().filter((x) => x.held!.why !== r.why)
          for (const x of stale) x.held!.why = r.why
          if (stale.length) this.save()
          return
        }
        if (byLoad && now - this.lastStart < START_GAP) return
        this.lastStart = now
        const why = t.held!.why === 'slots' ? 'a slot is free' : 'the machine has room'
        if (t.held!.go) this.go(t, t.held!.go, `${t.held!.goWhy ?? why} · ${why}`, t.held!.plain)
        else {
          t.held = undefined
          const id = `go:${t.id}:${now}`
          this.chats.post(this.key(t), { id, card: { id, task: t.id, status: 'work', text: `started: ${why}` } })
          this.save()
          this.startTask(t).catch((e) => this.troubled(t, e))
        }
      }
    } finally {
      this.releasing = false
    }
  }

  /* ---------- adding and starting ---------- */

  /**
   * A new task: its agent starts on the brief at once. The brief ends with the default way of working from Settings;
   * its CLI, model and effort are Settings' too, unless the coordinator chose others for it.
   */
  add(o: { project: string; base?: string; title: string; desc?: string; by: string; requester?: string; after?: string[]; later?: boolean; agent?: Partial<Harness>; reviewer?: Partial<Harness>; set?: Record<string, string> }) {
    const p = this.places.byName(o.project) ?? this.places.byKey(o.project) ?? this.places.space(o.project)
    if (!p) throw new Error(`No project called ${o.project}`)
    // a new task is in no circle; those already done are not waited for
    const after = this.checkAfter(undefined, o.after ?? []).filter((ref) => this.depState(ref) !== 'done')
    const title = short(o.title)
    const space = placeId.space(p.name)
    this.taskIds.ensure(space, p.key)
    const { id, n, prefix } = this.taskIds.allocate(space)
    p.key = prefix
    this.data.next[prefix] = n
    // its project's defaults: its own, else its machine's, else All machines'
    const reviewer = this.harness(o.reviewer, 'reviewer', p.name)
    const agent = this.harness(o.agent, 'task', p.name)
    const set = o.set && Object.keys(o.set).length ? o.set : undefined
    // each line that differs from Shepherdr's own says who set it: the coordinator's flags, or a level of Settings
    const by = { ...this.settings.setBy(p.name, set), ...(Object.keys(o.reviewer ?? {}).length ? { reviewer: 'task' as const } : {}), ...(Object.keys(o.agent ?? {}).length ? { agent: 'task' as const } : {}) }
    const ways = this.settings.ways(p.name, set)
    const how = settingsBlock(id, { ways, reviewer, agent, by })
    const desc = [o.desc ?? (title !== o.title ? o.title : ''), o.later ? LATER : '', how].filter(Boolean).join('\n\n')
    const t: TaskRec = {
      id,
      n,
      project: placeId.space(p.name),
      base: o.base ?? p.base,
      title,
      desc,
      by: o.by,
      requester: o.requester,
      created: Date.now(),
      updated: Date.now(),
      told: after.length ? 'queued' : 'work',
      agent,
      reviewer,
      waitChecks: ways.waitChecks,
    }
    if (after.length) t.after = after
    // one that starts work now waits for room; one for later only reads its brief, so it starts at once
    const room = after.length || o.later ? {} : this.room()
    this.data.tasks.push(t)
    if (room.why) {
      t.told = 'queued'
      this.hold(t, room.why)
    } else this.save()
    this.emit('news', t.project, `${t.id} was added: ${t.title}`, 'added', { task: t.id, event: 'added' })
    if (!room.why) this.startTask(t).catch((e) => this.troubled(t, e))
    return t
  }

  /** Settings' CLI, model and effort for tasks (or their reviewer), with what the coordinator chose for this one on
   *  top. Another CLI drops Settings' model, which names one of the first CLI's models; its effort goes in as the
   *  nearest level the other CLI takes (models.ts). */
  private harness(chosen: Partial<Harness> = {}, role: 'task' | 'reviewer' = 'task', project?: string): Harness {
    const h = this.settings.harness(role, project)
    return { ...h, ...(chosen.kind && chosen.kind !== h.kind ? { model: undefined } : {}), ...chosen }
  }

  /** its agent waits for a go: To do until something is written to it (the go, or a change to the brief) */
  todo(id: string) {
    const t = this.req(id)
    if (ended(t)) throw new Error(`${t.id} is Done already (${t.closed ? 'nothing to merge' : 'merged'})`)
    t.todo = true
    // waiting for a go of its own: no longer just paused
    t.paused = undefined
    clearTimeout(this.later.get(t.id))
    this.settle(t)
    return t
  }

  private env(key: string) {
    return {
      ...(has('browser') ? browserEnv(key.split('/')[0]) : {}),
      ...agentEnv(key, ROOT),
    }
  }

  private async startTask(t: TaskRec) {
    this.starting.add(t.id)
    t.trouble = undefined
    this.emit('changed')
    try {
      const p = this.proj(t)
      const branch = branchFor(t)
      // Old records without a snapshot keep the existing fallback; new tasks snapshot at add.
      t.base ??= p.base
      this.save()
      await G.validateBase(p.path, t.base)
      const made = await this.wt.create({ repo: p.path, name: p.name }, branch, t.base, labelOf(t)).catch((e) => {
        throw new Error(`Could not make a worktree: ${(e as Error).message}`)
      })
      Object.assign(t, { branch, worktree: made.path })
      this.save()
      await this.launch(t, this.queued(t) ? `${taskFirst(t)}\n\n${queuedLine(this.open(t), 'start', t.id)}` : taskFirst(t), made)
      this.briefing.add(t.id)
      clearTimeout(this.later.get(`${t.id}:brief`))
      const x = setTimeout(() => {
        this.later.delete(`${t.id}:brief`)
        if (this.briefing.delete(t.id)) this.settle(t)
      }, BRIEF_WAIT)
      x.unref?.()
      this.later.set(`${t.id}:brief`, x)
    } catch (e) {
      // went wrong before its worktree was there: it shows so
      t.trouble = troubleOf(e)
      throw e
    } finally {
      this.starting.delete(t.id)
      this.begun(t)
      this.settle(t)
    }
  }

  /**
   * Its agent, in its herdr space. A worktree that is gone (a stop with no work put it away) is made again first, in
   * the same folder, so its agent's session (kept by folder) is found again; never a finished task's.
   */
  private async launch(t: TaskRec, first?: string, made?: Made, as?: Harness, restart?: () => Promise<string | void>) {
    const key = this.key(t)
    const rec = this.agents.record(key)
    const p = this.proj(t)
    const home = { repo: p.path, name: p.name }
    if (!made && !ended(t) && !existsSync(t.worktree!) && this.agents.status(key) === 'gone') {
      made = await this.wt.create(home, t.branch!, t.base!, labelOf(t), t.worktree)
      t.worktree = made.path
      this.save()
    }
    const env = this.env(key)
    // a task from before tasks kept their own CLI starts on Settings'
    const h = t.agent ?? this.settings.harness('task', this.nameOf(t))
    trustWorktree(t.worktree!, as?.kind ?? rec?.kind ?? h.kind, env)
    const spec: Spec = {
      access: 'full',
      key,
      name: rec?.name ?? `shep-${t.id.toLowerCase()}`,
      kind: as?.kind ?? rec?.kind ?? h.kind,
      // only its first start (or a backup's, on its CLI's own model): later, its own chat's pickers are what count
      ...(as ? { model: as.model ?? 'default', effort: as.effort } : rec ? {} : { model: h.model, effort: h.effort }),
      // what Settings said when it started on them: a change since is offered as a restart (restart.ts)
      ...(as || !rec ? { set: this.settings.harness('task', this.nameOf(t)) } : {}),
      cwd: t.worktree!,
      workspace: labelOf(t),
      tab: 'Agent',
      system: taskPrompt({ id: t.id, branch: t.branch!, base: t.base! }, pastTimes(this.data.tasks, t)) + '\n\n' + privatePrompt(this.settings.instructions('task', this.nameOf(t)), this.settings.instructions('reviewer', this.nameOf(t))),
      env,
      first,
      pane: () => this.wt.agentPane({ home, wt: t.worktree!, label: labelOf(t), tab: 'Agent', env, made }),
    }
    return restart ? this.agents.restart(spec, restart) : this.agents.ensure(spec)
  }

  /** writing to a task whose agent stopped starts it again, with its memory */
  resume(id: string) {
    const t = this.task(id)
    if (!t?.worktree) return Promise.resolve(null)
    return this.launch(t)
  }

  /**
   * Its agent started again on this CLI, model and effort (the restart sheet, shepherdr restart). Mid-turn it is
   * interrupted first. The same CLI resumes its session; another takes over with the brief, the branch's commits,
   * what lies uncommitted and the chat's last messages. What you sent meanwhile waits for it; its worktree and branch
   * stay as they are; a card in its chat marks it.
   */
  async restart(id: string, to: Harness) {
    const t = this.req(id)
    if (ended(t)) throw new Error(`${t.id} is Done already (${t.closed ? 'nothing to merge' : 'merged'}): it has no agent to restart.`)
    if (!t.worktree) throw new Error(`${t.id} hasn't started yet: it starts on what its brief says.`)
    if (this.handing.has(t.id)) throw new Error(`${t.id}'s agent is starting again already.`)
    if (!this.installed(to.kind)) throw new Error(`${CLI_NAME[to.kind] ?? to.kind} isn't installed on this machine.`)
    const key = this.key(t)
    const h = fitHarness(to)
    const from = this.agents.record(key)?.kind ?? t.agent?.kind ?? ''
    const name = (k: string) => CLI_NAME[k] ?? k
    const words = harnessWords(h)
    this.handing.add(t.id)
    try {
      // what it is told, read once its turn has stopped: what it did meanwhile counts
      const first = async () => {
        await this.agents.stopTurn(key)
        if (from === h.kind) return hidden(resumedLine(words), 'restart')
        return hidden(`${taskFirst(t)}\n\n${restartLine({ from: name(from), to: words, ...(await this.soFar(t, key, name(from))) })}`, 'restart')
      }
      await this.launch(t, undefined, undefined, { ...h, model: h.model ?? '', effort: h.effort ?? '' }, first)
      // a pause or a stop is over: it carries on
      t.stopped = undefined
      t.paused = undefined
      t.agent = h
      const card = `restart:${t.id}:${Date.now()}`
      this.chats.post(key, { id: card, card: { id: card, restart: true, text: `Restarted on ${words}` } })
    } finally {
      this.handing.delete(t.id)
      this.save()
      this.settle(t)
    }
    return words
  }

  /** its agent works and its brief has gone in: it is at work on it, no longer To do */
  private begun(t: TaskRec) {
    const key = this.key(t)
    if (!this.briefing.has(t.id) || this.agents.status(key) !== 'working' || this.agents.waiting(key).length) return
    this.briefing.delete(t.id)
    clearTimeout(this.later.get(`${t.id}:brief`))
    this.later.delete(`${t.id}:brief`)
  }

  /** something went in to an agent: its brief, maybe, which it may already be at work on */
  sent(key: string) {
    const t = this.task(key.split('/')[0])
    if (!t || !this.briefing.has(t.id)) return
    this.begun(t)
    this.settle(t)
  }

  /**
   * Its agent can't go on (retry.ts): out of allowance, signed out, or its provider still down after the last try. The
   * backup agent (Settings' task.fallback) takes over once, in the same worktree and branch, with the brief and what
   * was done so far; else, or when that one can't go on either, it waits for you.
   */
  async cantGoOn(id: string, why: string) {
    const t = this.task(id)
    if (!t || !this.mayRetry(this.key(t)) || this.handing.has(t.id)) return
    const key = this.key(t)
    const from = this.agents.record(key)?.kind ?? t.agent?.kind ?? ''
    const to = kindOf(this.settings.get('task.fallback', this.nameOf(t)))
    if (!to || to === from || t.tookOver || !t.worktree || !this.installed(to)) return this.settle(t)
    const name = (k: string) => CLI_NAME[k] ?? k
    this.handing.add(t.id)
    t.tookOver = { from, to, why: why.charAt(0).toLowerCase() + why.slice(1), at: Date.now() }
    this.settle(t)
    try {
      const first = `${taskFirst(t)}\n\n${handOverLine({ from: name(from), why: t.tookOver.why, ...(await this.soFar(t, key, name(from))) })}`
      await this.agents.finish(key)
      // Settings' Backup model and effort (its project's), or else the task's own effort, as near as the backup's
      // model takes it
      const own = this.settings.modelOf('backup', to, this.nameOf(t))
      await this.launch(t, first, undefined, { kind: to, model: own.model, effort: own.effort ?? (t.agent ?? this.settings.harness('task', this.nameOf(t))).effort })
      const card = `takeover:${t.id}:${t.tookOver.at}`
      this.chats.post(key, { id: card, card: { id: card, text: `${name(to)} took over from ${name(from)}: ${t.tookOver.why}` } })
    } catch (e) {
      // the backup didn't start: it went wrong, and a later hand-over may try again
      t.trouble = troubleOf(e)
      t.tookOver = undefined
    } finally {
      this.handing.delete(t.id)
      this.settle(t)
    }
  }

  /** what a backup agent is told was done: the branch's commits, what lies uncommitted, the chat's last messages */
  private async soFar(t: TaskRec, key: string, them: string) {
    const out = async (...args: string[]) => {
      const r = await G.git(t.worktree!, ...args)
      return r.ok ? r.out.split('\n').filter(Boolean).slice(0, 20).join('\n') : ''
    }
    return { commits: await out('log', '--oneline', `${t.base}..HEAD`), changes: await out('status', '--short'), chat: recentChat(this.chats.chat(key).messages, them, taskFirst(t).trim()) }
  }

  private troubled(t: TaskRec, e: unknown) {
    t.trouble = troubleOf(e)
    this.settle(t)
  }

  /* ---------- what agents report ---------- */

  /** the agent's plan: a card in its chat; your answer goes back to it as a chat line */
  async plan(id: string, steps: string[]) {
    const t = this.req(id)
    t.plan = steps
    const q: Question = {
      id: `plan:${t.id}:${Date.now()}`,
      title: `Plan for ${t.id}`,
      kind: 'Plan',
      steps,
      choices: [{ label: 'Approve the plan' }, { label: 'Send it back with a note' }],
      noteRequiredFor: 1,
      to: `${t.id} agent`,
    }
    this.chats.post(this.key(t), { id: q.id, question: q })
    this.save()
  }

  /** the agent's own progress (shepherdr status): its line, how far through the task, the minutes left if it said */
  report(id: string, msg: string, progress: number, left?: number) {
    const t = this.req(id)
    if (ended(t)) throw new Error(`${t.id} is Done already (${t.closed ? 'nothing to merge' : 'merged'})`)
    // reported done: nothing shows, and a line kept now would come back stale once it works again
    if (t.reported) throw new Error(`${t.id} is Ready: no progress shows until it works again.`)
    const at = Date.now()
    this.heard.set(t.id, at)
    t.report = { msg: fitStatus(msg), progress, left, at, pausedAt: this.statusOf(t) === 'need' ? at : undefined }
    this.save()
    return t.report
  }

  /**
   * The agent reports done: Ready, waiting for review or merge. A branch with no changes at all (a research task, an
   * answer) has nothing to merge: it is closed, Done at once, and put away once the agent's turn ends.
   */
  async done(id: string, summary: string, facts?: string): Promise<'ready' | 'closed' | 'ended'> {
    const t = this.req(id)
    if (ended(t)) return 'ended'
    if (summary) t.said = summary
    if (facts) t.facts = facts
    t.reported = true
    t.reportedAt ??= Date.now()
    t.landing = undefined
    t.going = undefined
    // news on its pull request from now on is told to it
    t.watch ??= { since: ghNow() }
    // its job is over: so is what it said of its progress
    t.report = undefined
    if (t.worktree && t.base && existsSync(t.worktree) && !(await G.dirty(t.worktree)) && (await G.aheadCount(t.worktree, t.base)) === 0) {
      this.markClosed(t)
      if (this.agents.status(this.key(t)) !== 'working') await this.cleanup(t)
      return 'closed'
    }
    this.save()
    // told once its turn ends: an agent that merges right after never shows as waiting
    this.settleSoon(t)
    // its pull request gets the summary and is ready for review; GitHub away changes nothing here
    if (t.pr && !t.merged) this.readyForReview(t).catch(() => undefined)
    return 'ready'
  }

  private markClosed(t: TaskRec) {
    Object.assign(t, { closed: true, doneAt: Date.now(), reported: true, landing: undefined, trouble: undefined, stopped: undefined, todo: undefined })
    clearTimeout(this.later.get(t.id))
    this.settle(t)
    this.emit('gone', t, true)
  }

  /**
   * Close a task with nothing to merge: Done with its summary (what it said when ready, else "Nothing to merge"). Its agent
   * stops and its space closes; its worktree and branch go when they hold nothing unsaved or unmerged, else they are
   * kept and said so.
   */
  async close(id: string, summary?: string, facts?: string): Promise<{ kept: string[] }> {
    const t = this.req(id)
    if (ended(t)) throw new Error(`${t.id} is Done already (${t.closed ? 'nothing to merge' : 'merged'})`)
    const key = this.key(t)
    if (this.agents.status(key) === 'working') await this.agents.interrupt(key)
    if (summary) t.said = summary
    if (facts) t.facts = facts
    this.markClosed(t)
    // putting it away closes the agent too, after looking at its work
    return { kept: (await this.cleanup(t, true))?.kept ?? [] }
  }

  /** what a task's worktree and branch hold that its base lacks, in words ("~/x (uncommitted changes)"); none: nothing is lost */
  async holds(t: TaskRec): Promise<string[]> {
    const p = this.places.space(t.project)
    if (!p) return []
    const on = t.base ?? p.base
    return this.wt.holds({ repo: p.path, wt: t.worktree, branch: t.branch, base: on, squashes: await this.squashes(t, p.path, on) })
  }

  /**
   * Pause a working task: its agent is interrupted mid-work (as Esc does; it stays alive) and the task is To do, as
   * with shepherdr todo, until something is written to it. Its work, worktree and chat stay.
   */
  async pause(id: string) {
    const t = this.req(id)
    if (this.statusOf(t) !== 'work') throw new Error(`${t.id} isn't working: only a working task is paused`)
    // To do first: the turn the interrupt ends is no question for you
    t.todo = true
    t.paused = this.agents.status(this.key(t)) === 'working' ? 'sent' : 'idle'
    clearTimeout(this.later.get(t.id))
    await this.agents.interrupt(this.key(t))
    this.settle(t)
    return t
  }

  private req(id: string) {
    const t = this.task(id)
    if (!t) throw new Error(`No task ${id}`)
    return t
  }

  /* ---------- GitHub: pull requests ---------- */

  /**
   * The project's GitHub repo when gh is logged in to its host, read live: a repo may gain its remote after it was
   * added. Each form of origin is tried (as git rewrites it, as written) for one gh is logged in to. None: the task
   * merges here. Logged in but GitHub not answering throws: that never falls back to merging here.
   */
  private async hub(p: Space): Promise<G.GitHub | null> {
    const was = p.github
    await this.places.recheckGithub(p)
    if (p.github !== was) this.emit('changed')
    let down: string | undefined
    for (const g of await G.githubs(p.path)) {
      const l = await G.ghLogin(g, p.path)
      if (l.state === 'yes') return g
      if (l.state === 'down') down ??= `${g.host}: ${l.said}`
    }
    if (down) throw new Error(`gh can't reach GitHub now (${down}). Nothing was done: try again in a while, or see gh auth status.`)
    return null
  }

  /** the pull request's words: the agent's summary, the brief's gist, and the footer its CLI signs with */
  private prBody(t: TaskRec) {
    const kind = this.agents.record(this.key(t))?.kind ?? t.agent?.kind ?? this.settings.harness('task', this.nameOf(t)).kind
    // the coordinator's words, not the default way of working Shepherdr ended the brief with
    const brief = (splitBrief(t.desc)[0] || t.title).trim()
    const gist = brief.length > 700 ? `${brief.slice(0, 700).replace(/\s+\S*$/, '')} …` : brief
    return [
      t.said || 'Work in progress: the summary comes when the task is done.',
      ...(t.facts ? [`Facts: ${t.facts}`] : []),
      `**Brief** (${t.id})\n\n${gist
        .split('\n')
        .map((l) => `> ${l}`.trimEnd())
        .join('\n')}`,
      SIGNED[kind] ?? `🤖 Generated with ${CLI_NAME[kind] ?? kind}`,
    ].join('\n\n')
  }

  /** its pull request as GitHub has it now: the one recorded, else the branch's own (one opened by hand) */
  private async prOf(t: TaskRec, g: G.GitHub, cwd: string) {
    const known = t.pr ? await G.viewPr(g, cwd, t.pr.number) : null
    // one closed on GitHub is no longer its pull request: the branch may have another
    const pr = known && known.state !== 'CLOSED' ? known : await G.findPr(g, cwd, t.branch!)
    if ((pr?.number ?? null) !== (t.pr?.number ?? null) && (pr || known?.state === 'CLOSED')) {
      t.pr = pr ? { number: pr.number, url: pr.url } : undefined
      this.save()
    }
    return pr
  }

  private async readyForReview(t: TaskRec, hub?: G.GitHub) {
    const p = this.proj(t)
    const g = hub ?? (await this.hub(p))
    if (!g || !t.pr) return
    await G.editPr(g, p.path, t.pr.number, this.prBody(t))
    await G.readyPr(g, p.path, t.pr.number)
  }

  /** push the task's branch as it is committed, to GitHub: forced, since it is rebased on main and only its agent's */
  private async push(t: TaskRec) {
    const sha = (await G.git(t.worktree!, 'rev-parse', 'HEAD')).out
    const r = await G.git(t.worktree!, 'push', '--quiet', '--force', 'origin', `HEAD:refs/heads/${t.branch}`)
    if (r.ok) {
      t.pushed = { sha, at: Date.now() }
      this.save()
    }
    return r
  }

  /** a GitHub step that could not reach GitHub says so, and did nothing */
  private async onGitHub<T extends { ok: boolean }>(f: () => Promise<T>, failed: (why: string) => T): Promise<T> {
    try {
      return await f()
    } catch (e) {
      return failed((e as Error).message)
    }
  }

  /**
   * shepherdr pr: push the branch and open its pull request (a draft until the agent reports done), or bring the open
   * one up to the branch. Its title is the task's, its words the summary and the brief's gist.
   */
  openPr(id: string): Promise<{ ok: boolean; text: string }> {
    return this.onGitHub(() => this.openPrNow(this.req(id)), (text) => ({ ok: false, text }))
  }

  private async openPrNow(t: TaskRec): Promise<{ ok: boolean; text: string }> {
    if (ended(t)) return { ok: false, text: `${t.id} is Done already (${t.closed ? 'nothing to merge' : 'merged'}).` }
    if (!t.worktree || !t.branch || !t.base) return { ok: false, text: `${t.id} has no branch yet.` }
    const p = this.proj(t)
    const g = await this.hub(p)
    if (!g) return { ok: true, text: `No pull request: ${p.name} has no GitHub remote, or gh isn't logged in to it (gh auth status). shepherdr merge ${t.id} squash-merges it here.` }
    if ((await G.aheadCount(t.worktree, t.base)) === 0) return { ok: false, text: `${t.id} has no commits yet: commit your work, then run shepherdr pr ${t.id} again.` }
    const dirty = await G.dirty(t.worktree)
    const pushed = await this.push(t)
    if (!pushed.ok) return { ok: false, text: `Could not push ${t.branch} to GitHub: ${(pushed.err || pushed.out).split('\n')[0]}` }
    let pr = await this.prOf(t, g, t.worktree)
    if (pr && pr.baseRefName !== t.base) return { ok: false, text: `PR #${pr.number} targets ${pr.baseRefName}, but ${t.id} targets ${t.base}. Correct the pull request target on GitHub first.` }
    let opened = false
    if (pr?.state === 'MERGED') return { ok: false, text: `PR #${pr.number} is merged on GitHub already: run shepherdr merge ${t.id} to mark it merged and clean up.` }
    if (!pr) {
      const o = await G.openPr(g, t.worktree, { branch: t.branch, base: t.base, title: t.title, body: this.prBody(t), draft: !t.reported })
      if ('error' in o) return { ok: false, text: `Pushed ${t.branch}, but GitHub did not open a pull request: ${o.error}` }
      t.pr = { number: o.number, url: o.url }
      this.save()
      opened = true
    } else if (t.reported) await this.readyForReview(t, g)
    const head = (await G.git(t.worktree, 'rev-parse', '--short', 'HEAD')).out
    return {
      ok: true,
      text: `${opened ? `Opened PR #${t.pr!.number}${t.reported ? '' : ' as a draft'}` : `Pushed to PR #${t.pr!.number}`} at ${head}: ${t.pr!.url}${dirty ? '\nUncommitted changes are not in it: commit them and run shepherdr pr again.' : ''}\n${(t.waitChecks ?? this.settings.ways(p.name).waitChecks) ? `Next: shepherdr checks ${t.id} waits for GitHub's checks (run it with a long timeout, up to 10 minutes).` : `Merging doesn't wait for GitHub's checks (shepherdr checks ${t.id} waits for any that run on it, if you want to see them).`}`,
    }
  }

  /**
   * GitHub's checks on the task's pull request, waited for while they run (up to `waitMs`). A commit Shepherdr just
   * pushed, in a repo with workflows that run on pull requests, shows none for a moment: that moment is waited out, never taken as passed (after
   * it, none is none: the repo's workflows may not run on pull requests). GitHub not answering three times running
   * is said, not waited on.
   */
  private async checksOf(t: TaskRec, g: G.GitHub, n: number, waitMs: number): Promise<G.PrChecks> {
    const t0 = Date.now()
    const head = (await G.git(t.worktree!, 'rev-parse', 'HEAD')).out
    const expect = await G.hasPrWorkflows(t.worktree!)
    let errors = 0
    for (;;) {
      const c = await G.prChecks(g, t.worktree!, n)
      errors = c.state === 'error' ? errors + 1 : 0
      const early = c.state === 'none' && expect && t.pushed?.sha === head && Date.now() - t.pushed.at < CHECKS_SHOW
      if ((c.state !== 'pending' && c.state !== 'error' && !early) || errors >= 3) return c
      if (Date.now() - t0 + CHECKS_EVERY > waitMs) return early ? { ...c, state: 'pending' } : c
      await Bun.sleep(CHECKS_EVERY)
    }
  }

  /** a ready task's next step into main (Merging), or none: Ready again, or back at work */
  private land(t: TaskRec, step?: Landing['step'], due?: number) {
    if (!t.reported || ended(t)) return
    t.landing = step ? { step, at: Date.now(), due } : undefined
    this.save()
    this.settle(t)
  }

  /** when GitHub's checks at `head` should be through: from when it was pushed, as long as they took lately */
  private async checksDue(t: TaskRec, p: Space, g: G.GitHub, head: string) {
    let took = this.took.get(p.path)
    if (!took || Date.now() - took.at > 30 * 60_000) {
      took = { ms: await G.checksTook(g, p.path).catch(() => undefined), at: Date.now() }
      this.took.set(p.path, took)
    }
    if (took.ms === undefined) return undefined
    return (t.pushed?.sha === head ? t.pushed.at : Date.now()) + took.ms
  }

  /** which of GitHub's checks failed, and where to see why */
  private checksFailed(n: number, c: G.PrChecks) {
    const link = c.failed.find((f) => f.link)?.link
    return `GitHub's checks failed on PR #${n}: ${c.failed.map((f) => f.name).join(', ')}.${link ? ` See ${link} (gh run view --log-failed shows why).` : ''}`
  }

  private fixIt = (t: TaskRec) => `Fix it, commit, push with shepherdr pr ${t.id}, wait with shepherdr checks ${t.id}, then run shepherdr merge ${t.id} again.`

  /** shepherdr checks: wait for GitHub's checks on the task's pull request, at the commit this machine has */
  checks(id: string, waitMs = CHECKS_WAIT): Promise<{ ok: boolean; text: string }> {
    return this.onGitHub(() => this.checksNow(this.req(id), waitMs), (text) => ({ ok: false, text }))
  }

  private async checksNow(t: TaskRec, waitMs: number): Promise<{ ok: boolean; text: string }> {
    if (!t.worktree || !t.branch) return { ok: false, text: `${t.id} has no branch yet.` }
    const p = this.proj(t)
    const g = await this.hub(p)
    if (!g) return { ok: true, text: `No GitHub checks: ${p.name} has no GitHub remote, or gh isn't logged in to it. Run the project's own checks here.` }
    const pr = await this.prOf(t, g, t.worktree)
    if (!pr) return { ok: false, text: `${t.id} has no pull request yet: shepherdr pr ${t.id} pushes the branch and opens one.` }
    const head = (await G.git(t.worktree, 'rev-parse', 'HEAD')).out
    if (pr.headRefOid !== head) return { ok: false, text: `PR #${pr.number} is not at your commit (${head.slice(0, 7)}): push it with shepherdr pr ${t.id} first.` }
    // on its way into main (a merge waits for them): the row says they run, then what comes next
    if (t.landing) this.land(t, 'checks', await this.checksDue(t, p, g, head))
    const c = await this.checksOf(t, g, pr.number, waitMs)
    if (t.landing && c.state !== 'pending') this.land(t, c.state === 'pass' || c.state === 'none' ? 'merge' : undefined)
    if (c.state === 'fail') {
      this.toldFailed(t, head, c)
      return { ok: false, text: `${this.checksFailed(pr.number, c)} ${this.fixIt(t)}` }
    }
    if (c.state === 'pending') return { ok: false, text: `GitHub's checks still run on PR #${pr.number}: run shepherdr checks ${t.id} again.` }
    if (c.state === 'error') return { ok: false, text: `Could not read GitHub's checks on PR #${pr.number}: ${c.error}` }
    return { ok: true, text: c.state === 'none' ? `PR #${pr.number} has no checks on GitHub: nothing to wait for.` : `GitHub's checks passed on PR #${pr.number} (${c.passed}).` }
  }

  /**
   * News on done tasks' pull requests: checks that failed, and what someone wrote on it (a review, a comment) since its
   * agent reported done. Its agent hears each once, in <shepherdr> tags, which wakes it; what to do about it is its own
   * call (the shepherdr-agent skill). Only an agent at rest is told: one at work, or at a prompt, hears it next time.
   */
  async watchPrs() {
    if (this.watching) return
    this.watching = true
    try {
      const hubs = new Map<string, G.GitHub | null>()
      for (const t of this.tasks.filter((t) => t.reported && t.pr && t.worktree && !ended(t) && !t.archived && !t.stopped)) {
        if (['working', 'blocked'].includes(this.agents.status(this.key(t)))) continue
        const p = this.places.space(t.project)
        if (!p || !existsSync(t.worktree!)) continue
        if (!hubs.has(p.path)) hubs.set(p.path, await this.hub(p).catch(() => null))
        const g = hubs.get(p.path)
        if (g) await this.watchPr(t, g).catch(() => undefined)
      }
    } finally {
      this.watching = false
    }
  }

  private async watchPr(t: TaskRec, g: G.GitHub) {
    const pr = await this.prOf(t, g, t.worktree!)
    if (pr?.state !== 'OPEN') return
    const w = (t.watch ??= { since: ghNow() })
    const news: string[] = []
    const what: string[] = []
    const c = await G.prChecks(g, t.worktree!, pr.number)
    const failed = c.state === 'fail' ? failKey(pr.headRefOid, c) : c.state === 'pass' ? undefined : w.failed
    if (c.state === 'fail' && failed !== w.failed) {
      for (const f of c.failed) {
        const log = f.link ? await G.failedLog(g, t.worktree!, f.link) : undefined
        news.push(`- Check failed: ${f.name}${f.link ? ` (${f.link})` : ''}${log ? `. Its log ends:\n${quote(log)}` : ''}`)
      }
      what.push(`${c.failed.map((f) => f.name).join(', ')} failed`)
    }
    const all = await G.prComments(g, t.worktree!, pr.number)
    const seen = new Set(w.seen)
    // an approval or a review with no words of its own (it only carries its line comments) is no news
    const fresh = (all ?? []).filter((x) => !seen.has(x.id) && x.at >= w.since && (x.body || x.state === 'CHANGES_REQUESTED'))
    for (const x of fresh)
      news.push(`- ${x.state ? (COMMENT[x.state] ?? 'Review') : 'Comment'} by ${x.who}${x.where ? ` on ${x.where}` : ''} (${x.url}):\n${quote(x.body || '(no words)')}`)
    if (fresh.length) what.push(fresh.length === 1 ? 'a new comment' : `${fresh.length} new comments`)
    // put to work, or at a prompt, while GitHub was asked: it hears all this next time
    if (news.length && ['working', 'blocked'].includes(this.agents.status(this.key(t)))) return
    w.failed = failed
    if (all) w.seen = all.map((x) => x.id)
    this.save()
    if (!news.length) return this.nudgeToMerge(t, pr.number, c)
    const id = `pr:${t.id}:${Date.now()}`
    this.chats.post(this.key(t), { id, card: { id, task: t.id, status: 'work', text: `woken: ${what.join(' and ')} on PR #${pr.number}`, pr: t.pr } })
    await this.sendBack(t, hidden(prNewsLine(t.id, t.pr!, news), 'github', { task: t.id, pr: String(t.pr!.number) }))
  }

  /**
   * On its way into main, its agent at rest with nothing left running to wake it, and GitHub's checks through: it is
   * told once to merge, so it never shows Merging while nothing happens.
   */
  private async nudgeToMerge(t: TaskRec, n: number, c: G.PrChecks) {
    const key = this.key(t)
    if (!t.landing || t.landing.nudged || (c.state !== 'pass' && c.state !== 'none') || this.agents.background(key)) return
    t.landing.nudged = true
    this.save()
    const text = `GitHub's checks passed on PR #${n}: merge ${t.id} now (shepherdr merge ${t.id}), as your brief says.`
    await this.agents.send(key, hidden(text, 'github', { task: t.id, pr: String(n) }), { ensure: () => this.launch(t) }).catch(() => undefined)
  }

  /**
   * GitHub's checks on main after merges, which a task merges without waiting for (Settings), or that merged cleanly
   * but clash: once a run on main fails, its coordinator hears it once, with the run and the tasks merged since main
   * was last green. What follows (a fix task) is its call. Looked at while a task merged through its pull request
   * lately, or main is red, until it is green again.
   */
  async watchMains() {
    if (this.watchingMain) return
    this.watchingMain = true
    try {
      for (const p of this.places.spaces) await this.watchMain(p).catch(() => undefined)
    } finally {
      this.watchingMain = false
    }
  }

  private async watchMain(p: Space) {
    const red = this.data.red?.[p.path]
    const lately = Date.now() - MAIN_WATCH
    if (!red && !this.tasks.some((t) => t.merged && t.pr && (t.doneAt ?? 0) > lately && this.places.space(t.project) === p)) return
    const g = await this.hub(p).catch(() => null)
    const runs = g && (await G.branchRuns(g, p.path, p.base))
    // the newest commit on main its runs have a verdict on
    const i = runs ? runs.findIndex((r) => r.state !== 'pending') : -1
    if (!g || !runs || i < 0) return
    const now = runs[i]
    if (now.state === 'pass') {
      if (red) delete this.data.red![p.path]
      if (red) this.save()
      return
    }
    // told once while it stays red, whatever else merges meanwhile
    if (red) return
    ;(this.data.red ??= {})[p.path] = now.sha
    this.save()
    const green = runs.slice(i + 1).find((r) => r.state === 'pass')?.sha
    await G.git(p.path, 'fetch', '--quiet', 'origin', p.base)
    const log = await G.git(p.path, 'log', '--format=%s', '-n', '20', green ? `${green}..${now.sha}` : `${now.sha}^!`)
    const merged = log.ok ? log.out.split('\n').filter(Boolean) : []
    const f = now.failed[0]
    const tail = await G.failedLog(g, p.path, f.link)
    const ids = merged.map((m) => m.match(/^([A-Z][A-Z0-9]*-\d+):/)?.[1]).filter((x): x is string => !!x)
    this.emit(
      'news',
      placeId.space(p.name),
      [
        `${p.base} went red on GitHub at ${now.sha.slice(0, 7)}: ${now.failed.map((x) => `${x.name} failed (${x.link})`).join(', ')}.`,
        `Merged since it was last green${green ? ` (${green.slice(0, 7)})` : ''}: ${merged.length ? merged.join('; ') : 'none Shepherdr can tell'}.`,
        tail ? `Its log ends:\n${quote(tail)}` : '',
        'A fix-forward task follows if you decide so: its brief names the failing run and these merges',
      ].filter(Boolean).join('\n'),
      'red',
      { task: ids[0] ?? '', event: 'main-red' },
      { red: p.base, text: `went red: ${now.failed.map((x) => x.name).join(', ')} failed` },
    )
  }

  /** checks that failed at a commit were told to the agent (shepherdr checks, merge): watching won't tell it again */
  private toldFailed(t: TaskRec, head: string, c: G.PrChecks) {
    t.watch = { ...t.watch, since: t.watch?.since ?? ghNow(), failed: failKey(head, c) }
    this.save()
  }

  /* ---------- merging ---------- */

  /**
   * Rebase the task's branch on main, squash-merge it into main and mark it merged: through its pull request on a
   * GitHub repo gh is logged in to, else here. Its agent and worktree are put away once the agent's turn ends (it may
   * be the one asking). A conflict or failed checks stop it with what to do. Anyone but the task's own agent waits
   * until that agent's turn has ended, so no half-done work goes in.
   */
  merge(id: string, opts: { self?: boolean } = {}): Promise<{ ok: boolean; error?: string; note?: string }> {
    const t = this.req(id)
    if (this.closing) return Promise.resolve({ ok: false, error: `Shepherdr is restarting: nothing was merged. Run shepherdr merge ${t.id} again in a minute.` })
    // asked again while its merge still runs: the same one is waited for, never a second beside it
    let run = this.landings.get(t.id)
    if (!run) {
      const repo = this.places.space(t.project)?.path ?? ''
      const now = (this.merging.get(repo) ?? Promise.resolve())
        .then(() => this.onGitHub(() => this.mergeNow(t, !!opts.self), (error) => ({ ok: false, error: `Can't merge ${t.id}: ${error}` })))
        // stopped short with nothing more to wait for (failed checks, GitHub refused): Ready again, or back at work
        .then((r) => (!r.ok && t.landing?.step === 'merge' && this.land(t), r))
      run = now
      this.landings.set(t.id, now)
      const tail = now.catch(() => undefined)
      tail.then(() => this.landings.get(t.id) === now && this.landings.delete(t.id))
      // the next merge into this repo waits until this one is really through, however long: two at once would write
      // over each other's main. Its caller still hears by the deadline
      this.merging.set(repo, tail)
      tail.then(() => this.merging.get(repo) === tail && this.merging.delete(repo))
    }
    // its caller hears by the deadline, whatever hangs on the way (git, gh, GitHub): never a command that waits on and
    // on until the agent's shell moves it to the background, where its answer is lost. The merge itself goes on.
    let timer: ReturnType<typeof setTimeout> | undefined
    const late = new Promise<{ ok: boolean; error: string }>((done) => {
      timer = setTimeout(
        () => done({ ok: false, error: `${t.id}'s merge isn't through after ${Math.round(this.mergeDeadline / 1000)} s (git or GitHub is slow, or another merge into this repo still runs); Shepherdr goes on with it. Run shepherdr merge ${t.id} again in a minute: it waits for this one, then says it's merged or what stopped it.` }),
        this.mergeDeadline,
      )
    })
    return Promise.race([run, late]).finally(() => clearTimeout(timer))
  }

  /**
   * Shepherdr stops (an update restarts it, as one right after a merge into its own repo does): the merges on their
   * way finish first, at most their deadline, so none is cut halfway (merged on GitHub, never marked, its agent never
   * told) and each caller gets its answer. No new one starts meanwhile. True when there was one to wait for.
   */
  async drain(): Promise<boolean> {
    this.closing = true
    const on = [...this.landings.values()]
    if (!on.length) return false
    await Promise.race([Promise.allSettled(on), Bun.sleep(this.mergeDeadline)])
    return true
  }

  private async mergeNow(t: TaskRec, self: boolean): Promise<{ ok: boolean; error?: string; note?: string }> {
    if (t.merged) return { ok: false, error: `${t.id} is already merged` }
    if (t.closed) return { ok: false, error: `${t.id} is Done: it had nothing to merge` }
    if (!t.worktree || !t.branch || !t.base) return { ok: false, error: `${t.id} has no branch yet` }
    if (!self && this.agents.status(this.key(t)) === 'working') return { ok: false, error: `${t.id}'s agent is working: wait for its turn to end, or ask it to merge (shepherdr task tell ${t.id} "merge it").` }
    // a late note from you or a coordinator, not read yet, may change what goes in
    if (this.chats.unseen(this.key(t)).length)
      return { ok: false, error: self ? 'You have a new message in your chat: read and handle it, then merge again.' : `${t.id}'s agent has a new message it hasn't read yet: let it handle that first.` }
    this.land(t, 'merge')
    const p = this.proj(t)
    const wt = t.worktree
    // main here first catches up with the remote's, so the branch is rebased on what main really is
    const up = await G.catchUp(p.path, t.base)
    if (!up.ok) return { ok: false, error: `Can't merge ${t.id}: ${up.err}` }
    await G.commitAll(wt, `${t.id}: ${t.title}`)
    const g = await this.hub(p)
    if (g) return this.mergeOnGitHub(t, p, g, self)
    // its pull request is open, but gh can't use GitHub here now: it is never merged around it
    if (t.pr) return { ok: false, error: `Can't merge ${t.id}: its PR #${t.pr.number} is on GitHub, but gh isn't signed in to it here (gh auth login), so nothing was merged.` }
    // nothing ahead of main and no work ever made here; work merged by a merge commit (a pull request) is behind it too
    if ((await G.aheadCount(wt, t.base)) === 0 && !(await G.workedOn(wt, t.branch))) return { ok: false, error: `${t.id} has no changes to merge` }
    let note: string | undefined
    // merged already, by hand or as its pull request on GitHub (squashed, or with a merge commit), even with main
    // moved on since: it only needs marking
    if (await G.landed(wt, 'HEAD', t.base)) note = `it was in ${t.base} already`
    else {
      const rb = await G.rebase(wt, t.base)
      if (!rb.ok) {
        this.land(t, 'rebase')
        return { ok: false, error: `Conflict with ${t.base}: rebase ${t.branch} on ${t.base} by hand, commit, then run shepherdr merge ${t.id} again.` }
      }
      const message = `${t.id}: ${t.title}${t.said ? `\n\n${t.said}` : ''}${t.facts ? `\n\nFacts: ${t.facts}` : ''}`
      const m = await G.mergeHere(p.path, t.branch, t.base, message, true)
      if (!m.ok) {
        if (/conflict/i.test(`${m.out}${m.err}`)) this.land(t, 'rebase')
        return { ok: false, error: /conflict/i.test(`${m.out}${m.err}`) ? `Conflict with ${t.base}` : `Merge failed: ${(m.err || m.out).split('\n')[0]}` }
      }
      t.squash = (await G.git(p.path, 'rev-parse', '--verify', '--quiet', `refs/heads/${t.base}`)).out || undefined
    }
    // a GitHub remote gh can't use (not logged in): main is pushed as it is here
    if (p.github) {
      const push = await G.git(p.path, 'push', '--quiet', 'origin', t.base)
      const pushed = push.ok ? `pushed ${t.base} to GitHub` : `could not push ${t.base} to GitHub: ${push.err.split('\n')[0]}`
      note = note ? `${note}; ${pushed}` : pushed
    }
    return this.markMerged(t, note)
  }

  /**
   * Through the pull request, as herdr-projects lands one: the branch pushed as it is, rebased on GitHub's main only
   * when it conflicts with it (its pull request opened if the agent didn't), GitHub's checks passed at exactly that
   * commit, then squash-merged on GitHub only while its head is still that commit, its branch there deleted, and main here brought up to GitHub's (which
   * is what Shepherdr's own update watches). Failed checks go back to the agent.
   */
  private async mergeOnGitHub(t: TaskRec, p: Space, g: G.GitHub, self: boolean): Promise<{ ok: boolean; error?: string; note?: string }> {
    const wt = t.worktree!
    const base = `origin/${t.base}`
    if (!(await G.git(p.path, 'rev-parse', '--verify', '--quiet', base)).ok) return { ok: false, error: `GitHub has no ${t.base} yet: push it once (git push origin ${t.base}), then merge again.` }
    // main here holds commits GitHub lacks: merging on GitHub would split the two, so they go up first, by you
    const ahead = Number((await G.git(p.path, 'rev-list', '--count', `${base}..${t.base}`)).out || 0)
    if (ahead) return { ok: false, error: `${t.base} here has ${ahead} commit${ahead === 1 ? '' : 's'} GitHub lacks: push ${t.base} (git push origin ${t.base}) or move them off it, then merge ${t.id} again. Nothing was merged.` }
    let pr = await this.prOf(t, g, wt)
    if (pr && pr.baseRefName !== t.base) return { ok: false, error: `PR #${pr.number} targets ${pr.baseRefName}, but ${t.id} targets ${t.base}. Correct the pull request target on GitHub first. Nothing was merged.` }
    // merged on GitHub already (by hand, or a merge whose answer was lost): it only needs main here and marking
    if (pr?.state === 'MERGED') return this.mergedOnGitHub(t, p, `PR #${pr.number} was merged on GitHub already`)
    if ((await G.aheadCount(wt, t.base!)) === 0) return { ok: false, error: `${t.id} has no changes to merge` }
    // GitHub squash-merges a branch behind main as it is: rebasing it would make a new commit, and all its checks run
    // again (again each time main moves meanwhile). Only a real conflict is rebased here; a clash that merges cleanly
    // but breaks is caught by the checks on main.
    let rebased = false
    if (!(await G.mergesCleanly(wt, base, 'HEAD'))) {
      const rb = await G.rebase(wt, base)
      if (!rb.ok) {
        this.land(t, 'rebase')
        return { ok: false, error: `Conflict with ${t.base}: rebase ${t.branch} on ${base} by hand, commit, push with shepherdr pr ${t.id}, then run shepherdr merge ${t.id} again.` }
      }
      rebased = true
    }
    // main has this work already (merged another way): its pull request would change nothing
    if (await G.landed(wt, 'HEAD', base)) {
      // closed, with its branch there
      const closed = pr ? (await G.closePr(g, wt, pr.number, `Closed by Shepherdr: ${t.base} has this work already.`)).ok : false
      if (closed) await G.git(p.path, 'push', '--quiet', 'origin', '--delete', t.branch!)
      return this.mergedOnGitHub(t, p, `it was in ${t.base} already${pr ? (closed ? `; closed PR #${pr.number}` : `; could not close PR #${pr.number}: close it on GitHub`) : ''}`)
    }
    const head = (await G.git(wt, 'rev-parse', 'HEAD')).out
    const pushing = !pr || pr.headRefOid !== head
    if (pushing) {
      const push = await this.push(t)
      if (!push.ok) return { ok: false, error: `Could not push ${t.branch} to GitHub: ${(push.err || push.out).split('\n')[0]}` }
    }
    if (!pr) {
      const o = await G.openPr(g, wt, { branch: t.branch!, base: t.base!, title: t.title, body: this.prBody(t), draft: false })
      if ('error' in o) return { ok: false, error: `GitHub did not open a pull request for ${t.branch}: ${o.error}` }
      t.pr = { number: o.number, url: o.url }
      this.save()
    } else {
      // its words carry the summary, whether or not the agent reported done before
      if (t.said) await G.editPr(g, wt, pr.number, this.prBody(t))
      if (pr.isDraft) await G.readyPr(g, wt, pr.number)
    }
    const n = t.pr!.number
    const wait = t.waitChecks ?? this.settings.ways(p.name).waitChecks
    // not waited for (Settings): its own quick checks passed, and GitHub's run on, here and then on main
    // (watchMain); only a failure GitHub already shows at this very commit stops it
    const now = wait || pushing ? undefined : await G.prChecks(g, wt, n)
    const c = wait ? await this.checksOf(t, g, n, 0) : now?.state === 'fail' ? now : { state: 'pass' as const, passed: 0, failed: [] }
    if (c.state === 'pending') {
      this.land(t, 'checks', await this.checksDue(t, p, g, head))
      return { ok: false, error: `GitHub's checks are running on PR #${n}${pushing ? ` (just pushed${rebased ? `, rebased on ${t.base} for a conflict` : ''})` : ''}: wait for them with shepherdr checks ${t.id}, then run shepherdr merge ${t.id} again.` }
    }
    if (c.state === 'error') return { ok: false, error: `Could not read GitHub's checks on PR #${n}: ${c.error}. Nothing was merged.` }
    if (c.state === 'fail') {
      const why = this.checksFailed(n, c)
      if (self) {
        this.toldFailed(t, head, c)
        return { ok: false, error: `${why} ${this.fixIt(t)}` }
      }
      this.toldFailed(t, head, c)
      await this.sendBack(t, `${why} ${this.fixIt(t)}`)
      return { ok: false, error: `${why} Sent back to ${t.id}'s agent to fix.` }
    }
    const m = await G.mergePr(g, wt, n, head, `${t.id}: ${t.title} (#${n})`, [t.said, t.facts && `Facts: ${t.facts}`].filter(Boolean).join('\n\n'))
    if ((await G.viewPr(g, wt, n))?.state !== 'MERGED') {
      const said = (m.err || m.out || 'it did not say why').split('\n')[0]
      // main moved on GitHub since it was fetched: merging again rebases it if it now conflicts (Merging meanwhile)
      if (/conflict|not mergeable/i.test(said)) this.land(t, 'rebase')
      return { ok: false, error: /conflict|not mergeable/i.test(said) ? `Conflict with ${t.base} on GitHub: run shepherdr merge ${t.id} again, which rebases ${t.branch} on ${base} (or rebase it by hand, push with shepherdr pr ${t.id}, then merge again).` : `GitHub did not merge PR #${n}: ${said}` }
    }
    await G.git(p.path, 'push', '--quiet', 'origin', '--delete', t.branch!)
    return this.mergedOnGitHub(t, p, `merged PR #${n} on GitHub${wait ? '' : ', not waiting for its checks'}`)
  }

  /** main here comes up to GitHub's, so the merge is here too (and Shepherdr's own update sees it), then it is marked */
  private async mergedOnGitHub(t: TaskRec, p: Space, note: string) {
    const up = await G.catchUp(p.path, t.base!)
    return this.markMerged(t, up.ok ? note : `${note}; ${t.base} here is not up to date: ${up.err}`)
  }

  private async markMerged(t: TaskRec, note?: string) {
    t.merged = true
    t.doneAt = Date.now()
    t.reported = true
    t.landing = undefined
    t.trouble = undefined
    t.stopped = undefined
    this.settle(t)
    this.emit('gone', t, true)
    if (this.agents.status(this.key(t)) !== 'working') await this.cleanup(t)
    return { ok: true, note }
  }

  /** what its agent must fix, written to it: it is at work again */
  private async sendBack(t: TaskRec, text: string) {
    await this.agents.send(this.key(t), text, { ensure: () => this.launch(t) }).catch(() => undefined)
    this.youWrote(this.key(t))
  }

  /**
   * Put a finished task's place away: agent, space, worktree and branch, keeping whatever holds unsaved or unmerged
   * work (said in news, and in shepherdr status). Marked cleaned only once done, so one cut short (a restart) or put
   * off (something still runs in its space) is tried again.
   */
  private async cleanup(t: TaskRec, quiet = false) {
    const p = this.places.space(t.project)
    if (!p || this.cleaning.has(t.id)) return
    this.cleaning.add(t.id)
    try {
      const r = await this.putAway(t, p.path, p.base)
      if (r.busy) return r
      const news = r.kept.length && JSON.stringify(r.kept) !== JSON.stringify(t.kept)
      t.kept = r.kept.length ? r.kept : undefined
      t.cleaned = true
      this.save()
      if (news && !quiet) this.emit('news', t.project, `${t.id} ${t.closed ? 'is done, nothing to merge' : 'was merged'}; kept ${r.kept.join(', ')}`, 'done', { task: t.id, event: t.closed ? 'closed' : 'merged' })
      return r
    } finally {
      this.cleaning.delete(t.id)
    }
  }

  /** a merged task's branch is checked against its squash merge: the one recorded, else (merged before one was) the
   *  commits its subject names */
  private async putAway(t: TaskRec, repo: string, base: string) {
    const key = this.key(t)
    const on = t.base ?? base
    return this.wt.putAway({ repo, wt: t.worktree, branch: t.branch, base: on, squashes: await this.squashes(t, repo, on), label: labelOf(t), agentPane: this.agents.info(key)?.pane_id, closeAgent: () => this.agents.finish(key) })
  }

  private async squashes(t: TaskRec, repo: string, on: string) {
    return !t.merged ? [] : t.squash ? [t.squash] : G.squashesOf(repo, on, t.id)
  }

  /**
   * Leftovers of finished tasks, put away: at startup and every hour, so none pile up. Only tasks Shepherdr made and
   * merged or closed, never one whose agent is at work (or that you talk to again after its clean-up), and never what holds
   * unsaved or unmerged work. Nothing is touched while herdr does not answer.
   */
  async sweep() {
    const spaces = await this.wt.workspaces().catch(() => null)
    if (!spaces) return
    for (const t of this.tasks.filter(ended)) {
      const p = this.places.space(t.project)
      const st = this.agents.status(this.key(t))
      if (!p || st === 'working' || (t.cleaned && st !== 'gone')) continue
      const left = !t.cleaned || (!!t.worktree && existsSync(t.worktree)) || (!!t.branch && (await G.branchExists(p.path, t.branch))) || !!(await this.wt.spaceOf(t.worktree, labelOf(t), spaces))
      if (left) await this.cleanup(t, true).catch(() => undefined)
    }
    await this.wt.closeRepoSpaces()
  }

  /** finished tasks that kept a worktree or branch, and why: for shepherdr status */
  keptLeftovers() {
    return this.tasks.filter((t) => ended(t) && t.kept?.length).map((t) => `${t.id} ${t.kept!.join(', ')}`)
  }

  /* ---------- you and coordinators ---------- */

  /**
   * A message to a task (task tell, or yours in its chat). `gate`: a coordinator's go to a task that waits (To do, or
   * held) waits for room on this machine; it says so. Yours always goes in at once. One queued after other tasks gets
   * it at once and stays queued: its agent clears the queue itself if the words are a go.
   */
  async tell(id: string, text: string, { gate = false } = {}): Promise<{ held?: 'slots' | 'machine' }> {
    const t = this.req(id)
    const waits = this.statusOf(t) === 'todo' || this.holding(t)
    const r = gate && waits && !ended(t) ? this.room() : {}
    if (r.why) {
      if (!t.worktree) {
        // its brief takes the words, as below; it starts on it once there's room
        const [work, how] = splitBrief(t.desc)
        t.desc = [work, text, how].filter(Boolean).join('\n\n')
        this.hold(t, r.why)
      } else this.hold(t, r.why, { text: t.held?.go ? `${t.held.go}\n\n${text}` : text, why: 'its coordinator said go', plain: true })
      return { held: r.why }
    }
    if (!t.worktree && !this.starting.has(t.id)) {
      // above the defaults Shepherdr ended the brief with, so it wins over them like the rest of the brief
      const [work, how] = splitBrief(t.desc)
      t.desc = [work, text, how].filter(Boolean).join('\n\n')
      // a message is the go, as for one at work
      t.held = undefined
      await this.startTask(t)
      return {}
    }
    await this.agents.send(this.key(t), text, { ensure: () => this.launch(t) })
    this.youWrote(this.key(t))
    return {}
  }

  /** a task held before it ever started: it has no agent yet, so what you write starts it (tell) */
  unstarted(id: string) {
    const t = this.task(id)
    return !!t && !t.worktree && !this.starting.has(t.id) && !ended(t)
  }

  /**
   * Stop a task: its agent and its herdr space close. Its work stays: a worktree with unsaved or unmerged work is kept,
   * one with none goes with its branch. Writing to it again opens its space (or makes its worktree) and resumes it.
   */
  async stopTask(id: string): Promise<string[]> {
    const t = this.req(id)
    t.stopped = true
    this.briefing.delete(t.id)
    this.settle(t)
    const key = this.key(t)
    await this.agents.interrupt(key)
    // closed before anything is looked at, so it changes nothing more
    await this.agents.finish(key)
    this.emit('gone', t, false)
    const p = this.places.space(t.project)
    if (!p || !t.worktree || ended(t) || this.cleaning.has(t.id)) return []
    this.cleaning.add(t.id)
    try {
      return (await this.putAway(t, p.path, p.base)).kept
    } finally {
      this.cleaning.delete(t.id)
    }
  }

  /**
   * Delete a task for good (only when the human plainly asks): its agent stops, its herdr space closes, its worktree
   * and branch go, its chat log and its record too, so it leaves every list. Nothing is posted. Refused while its
   * worktree or branch holds work its base lacks, unless forced; a merged task's commits stay on main. Returns what
   * could not be removed.
   */
  async deleteTask(id: string, opts: { force?: boolean } = {}): Promise<string[]> {
    const t = this.req(id)
    if (this.starting.has(t.id) || this.cleaning.has(t.id)) throw new Error(`${t.id} is starting or being put away: try again in a moment`)
    const p = this.places.space(t.project)
    const key = this.key(t)
    // a merge under way in its repo goes first: never its worktree removed under it
    if (p) await this.merging.get(p.path)
    if (!this.task(t.id)) throw new Error(`No task ${id}`)
    const holds = async (stopped = '') => {
      if (opts.force) return
      const held = await this.holds(t)
      // what it holds goes with the error too, for the screen to ask whether to lose it
      if (held.length) throw Object.assign(new Error(`Not deleted: ${t.id} holds ${held.join(', ')}.${stopped} Merge or save that work first, or delete with --force to lose it.`), { held, stopped: !!stopped })
    }
    await holds()
    this.cleaning.add(t.id)
    try {
      await this.agents.interrupt(key)
      await this.agents.finish(key)
      // its agent may have written something before it closed: it stays stopped, and writing to it resumes it
      await holds(' Its agent was stopped (writing to it starts it again).').catch((e) => {
        t.stopped = true
        this.save()
        throw e
      })
      const left = p ? await this.wt.remove({ repo: p.path, wt: t.worktree, branch: t.branch, label: labelOf(t) }) : []
      if (!p && t.worktree && existsSync(t.worktree)) left.push(`${t.worktree} (its project is gone)`)
      clearTimeout(this.later.get(t.id))
      for (const m of [this.later, this.wrote]) m.delete(t.id)
      this.data.tasks = this.data.tasks.filter((x) => x !== t)
      this.emit('gone', t, true)
      this.chats.forget(key)
      this.agents.forget(key)
      this.save()
      // a task queued after it waits on you now
      this.recheck()
      return left
    } finally {
      this.cleaning.delete(t.id)
    }
  }

  /* ---------- archiving ---------- */

  /**
   * Archive a task: it leaves every list and count, keeping its record, chat and summary (search and its place's
   * Archived view find it). A task whose agent runs is stopped first, as with task stop: its work stays.
   */
  async archive(id: string): Promise<{ kept: string[]; stopped: boolean }> {
    const t = this.req(id)
    if (t.archived) return { kept: [], stopped: false }
    t.archived = Date.now()
    t.pinned = undefined
    clearTimeout(this.later.get(t.id))
    this.quiet(t)
    const stopped = this.agents.status(this.key(t)) !== 'gone' && !ended(t)
    const kept = stopped ? await this.stopTask(t.id) : []
    // its shared browser closes too (stopping it already did)
    if (!stopped) this.emit('gone', t, ended(t))
    // a finished task's agent may still be about: put away as merging would
    if (ended(t) && !t.cleaned && this.agents.status(this.key(t)) !== 'working') this.cleanup(t, true).catch(() => undefined)
    return { kept, stopped }
  }

  /** bring an archived task back into its lists, as it is now: nothing about it is news */
  unarchive(id: string) {
    const t = this.req(id)
    if (!t.archived) return t
    t.archived = undefined
    t.unarchived = Date.now()
    this.quiet(t)
    return t
  }

  /** pin a task to the top of its lists (on) or unpin it; an archived one can't be pinned */
  pin(id: string, on: boolean) {
    const t = this.req(id)
    if (on && t.archived) throw new Error(`${t.id} is archived: unarchive it first (shepherdr task unarchive ${t.id})`)
    if (!!t.pinned === on) return t
    t.pinned = on ? Date.now() : undefined
    this.save()
    return t
  }

  /** a task's project's name, for its defaults */
  private nameOf(t: TaskRec) {
    return this.places.space(t.project)?.name
  }

  /** Done tasks archive themselves once tasks.archiveAfter has passed since they finished (or were last brought back) */
  autoArchive(now = Date.now()) {
    // each after its own project's tasks.archiveAfter (off: never); never one whose agent you are talking to (it
    // works): its row would vanish mid-conversation
    const after = (t: TaskRec) => archiveAfterMs(this.settings.get('tasks.archiveAfter', this.nameOf(t)))
    const due = this.tasks.filter((t) => {
      const ms = after(t)
      return ms !== null && !t.archived && this.statusOf(t) === 'done' && this.agents.status(this.key(t)) !== 'working' && now - Math.max(t.doneAt ?? t.updated, t.unarchived ?? 0) >= ms
    })
    for (const t of due) {
      t.archived = now
      t.pinned = undefined
    }
    if (due.length) this.save()
    return due.length
  }

  /** a message sent to a task agent asks something of it: it is working again, not done. `go`: false for an answer in
   *  its CLI's own menu or question, which is no go for a queued task */
  youWrote(key: string, { go = true } = {}) {
    const [id] = key.split('/')
    const t = this.task(id)
    if (!t) return
    // writing to an archived task brings it back: you talk to it again
    if (t.archived) this.unarchive(t.id)
    if (ended(t)) return
    t.reported = false
    t.landing = undefined
    t.stopped = undefined
    // talking to it is the go, or a change to the brief: it says so again if it still waits. Queued, it stays so: its
    // agent decides from the words whether they are a go, and clears its queue itself (task queue --after none)
    t.todo = undefined
    t.paused = undefined
    t.going = undefined
    // what it was held with is said already, or overtaken by what you wrote
    if (go) t.held = undefined
    this.wrote.set(t.id, Date.now())
    this.settle(t)
    this.settleSoon(t, ANSWER_GRACE + 500)
  }

  /** A launch error is superseded only by verified private routing and a live owning agent.
   * Progress reports, queued messages and old hooks alone are not evidence of recovery. */
  routeRecovered(key: string) {
    const t = this.task(key.split('/')[0])
    if (!t || key !== this.key(t)) return

    const st = this.agents.status(key)
    if (t.trouble !== ROUTING_FAILURE || t.stopped || ended(t) || !this.agents.routed(key) || (st !== 'working' && st !== 'blocked')) return false
    t.trouble = undefined
    this.settle(t)
    this.save()
  }

  /** an agent's state changed in herdr */
  agentStatus(key: string) {
    const [id, role] = key.split('/')
    const t = this.task(id)
    if (!t || !role) return
    const st = this.agents.status(key)
    // a reviewer or attempt from before tasks had one agent only: put it away
    if (role !== 'Agent') {
      if (st !== 'gone' && !this.agents.record(key)?.finished) this.agents.finish(key).catch(() => undefined)
      return
    }
    if (ended(t)) {
      if (st !== 'working' && !t.cleaned) this.cleanup(t).catch(() => undefined)
      return
    }
    this.begun(t)
    // at work: the go reached it
    if (st === 'working' && t.going) t.going = undefined
    // paused, it stopped; at work again on its own after that, it isn't paused any more
    if (t.paused && st !== 'working') t.paused = 'idle'
    else if (t.paused === 'idle' && st === 'working') {
      t.paused = undefined
      t.todo = undefined
    }
    if (st === 'working' || st === 'blocked') {
      clearTimeout(this.later.get(t.id))
      this.settle(t)
    } else this.settleSoon(t)
  }

  /** what the provider did to its agent's turn changed (retry.ts): waiting, trying again, or given up */
  retried(key: string) {
    const t = this.task(key.split('/')[0])
    if (t && key === this.key(t)) this.settle(t)
  }

  /** automations: tasks of a project, by status (archived ones left out) */
  count(project: string, status: TaskStatus) {
    return this.tasks.filter((t) => t.project === project && !t.archived && this.statusOf(t) === status).length
  }
}

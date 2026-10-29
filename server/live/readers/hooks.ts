// What a Claude Code agent reports of itself, from its own hooks. Every Claude Code Shepherdr starts gets a settings
// file of its own (--settings: added to the human's settings for that one launch, which stay untouched) whose hooks
// append each event, cut down to what matters, to a file per agent. Folded in order, those events say whether its
// turn runs, ended (and what still runs in the background), or waits on a permission prompt, a question or a plan.
// Captured on Claude Code 2.1.310: Stop carries last_assistant_message and background_tasks; a subagent's own tool
// calls carry its agent_id; a background job that finishes wakes the agent with a UserPromptSubmit; a permission
// prompt or plan turned down, or Esc, ends the turn with no hook at all (its transcript says so: readers/claude.ts).
// A background subagent that ends its own turn while shells or monitors of its own still run is "completed" to Claude
// Code: Stop's background_tasks then lists only those shells, and its session file says "shell", though they will
// wake that subagent, not end. Only the notice that woke the agent says so ("is waiting on its own background work"),
// and the same task id notifies again once that subagent is really done.
import { appendFileSync, closeSync, mkdirSync, openSync, readSync, renameSync, statSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { BOX, helperDir } from '../box.ts'
import * as task from '../mfs.ts'
import type { Background } from './background.ts'

/** one hook event, as the recorder writes it */
export interface HookEvent {
  at: number
  model?: string
  effort?: string
  event: string
  tool?: string
  /** set when a subagent did it, not the agent itself */
  agent?: string
  /** Notification's notification_type, SessionStart's source */
  type?: string
  /** Stop's last message, a Notification's text, StopFailure's error */
  msg?: string
  /** AskUserQuestion's first question */
  question?: string
  /** what still runs in the background as the turn ends (Stop): subagent, shell, monitor... */
  bg?: string[]
  /** subagents whose notice woke it (UserPromptSubmit) saying they stopped with background work of their own still
   *  running, by task id */
  paused?: string[]
  /** tasks whose notice woke it saying they are over (done, failed, stopped), by task id */
  over?: string[]
  /** the CLI's session id (SessionStart, UserPromptSubmit): a /clear, /resume or new session changes it */
  session?: string
  /** that session's transcript (SessionStart) */
  log?: string
}

/** what the agent says it is doing: OSC 7501's states and blocked kinds (https://mitchellh.com/writing/program-status-osc7501) */
export interface Said {
  state: 'idle' | 'working' | 'blocked'
  /** what it waits on you for: a permission prompt, a question, a plan, a login */
  kind?: 'permission' | 'question' | 'plan' | 'auth'
  /** its own question, when it asked one with its question tool */
  ask?: string
  /** the last thing it said, as its turn ended */
  last?: string
  /** what still runs in the background since its turn ended */
  bg?: Background
  /** a subagent of its own waits on a permission prompt, whatever the agent itself does: that subagent's id */
  helper?: string
  /** subagents of its own that stopped with background work of their own still running: that work wakes them again,
   *  by task id */
  paused?: string[]
  /** when its current (or last) turn began */
  turnAt: number
  /** its last event */
  at: number
}

/** the hook events recorded */
export const HOOK_EVENTS = ['SessionStart', 'UserPromptSubmit', 'PreToolUse', 'PostToolUse', 'PostToolUseFailure', 'PermissionRequest', 'Notification', 'Stop', 'StopFailure', 'SessionEnd'] as const

/** a hook's input cut down to a line of the events file. Self-contained: the recorder script carries its source. */
export function pickHook(input: any, at: number): HookEvent | undefined {
  if (!input || typeof input !== 'object' || typeof input.hook_event_name !== 'string') return undefined
  const cut = (s: unknown, n = 2000) => (typeof s === 'string' && s ? s.slice(0, n) : undefined)
  const e: HookEvent = { at, event: input.hook_event_name }
  const set = (k: keyof HookEvent, v: unknown) => {
    if (v !== undefined && v !== null && v !== '') (e as any)[k] = v
  }
  set('model', cut(input.model, 160))
  set('effort', cut(input.effort?.level, 40))
  set('tool', cut(input.tool_name, 80))
  set('agent', cut(input.agent_id, 80))
  set('type', cut(input.notification_type ?? input.source, 80))
  set('msg', cut(input.last_assistant_message ?? input.message ?? input.error ?? input.error_details))
  const q = input.tool_name === 'AskUserQuestion' && Array.isArray(input.tool_input?.questions) ? input.tool_input.questions[0]?.question : undefined
  set('question', cut(q, 300))
  if (input.hook_event_name === 'SessionStart' || input.hook_event_name === 'UserPromptSubmit') set('session', cut(input.session_id, 80))
  if (input.hook_event_name === 'SessionStart') set('log', cut(input.transcript_path, 1000))
  // a shell that serves or watches (a dev server, a preview, tail -f) never ends to wake it: a "server", by what its
  // description or command says
  const serves = (t: any) =>
    /\b(server|serve|serving|preview)\b/i.test(String(t.description ?? '')) ||
    /\b(npm|pnpm|yarn|bun)( run)? (dev|start|serve|preview)\b|\b(vite|next|astro|nuxt|remix|wrangler|vercel|netlify) dev\b|\bvite( preview)?\s*($|[;&|])|http\.server|\b(uvicorn|gunicorn|flask run|rails s(erver)?)\b|\bnpx serve\b|--(watch|hot)\b|\btail -[fF]\b|sleep infinity/.test(String(t.command ?? ''))
  if (Array.isArray(input.background_tasks))
    e.bg = input.background_tasks.filter((t: any) => t && t.status === 'running').map((t: any) => (t.type === 'shell' && serves(t) ? 'server' : String(t.type ?? 'task')))
  // the notices of background tasks that woke it, in order: a subagent waiting on work of its own, or a task that is
  // over. Only Claude Code's own words count, not a command line or a result that quotes them.
  if (input.hook_event_name === 'UserPromptSubmit' && typeof input.prompt === 'string') {
    const last = new Map<string, 'paused' | 'over'>()
    for (const n of input.prompt.split('<task-notification>').slice(1)) {
      const body = n.split('</task-notification>')[0]
      const id = cut(/<task-id>([^<]+)<\/task-id>/.exec(body)?.[1], 80)
      if (!id) continue
      const tag = (t: string) => new RegExp(`<${t}>([\\s\\S]*?)</${t}>`).exec(body)?.[1] ?? ''
      last.delete(id)
      last.set(id, /is waiting on its own background work$/.test(tag('summary').trim()) || /^This agent stopped with background work of its own still running\b/.test(tag('note').trim()) ? 'paused' : 'over')
    }
    for (const [id, k] of last) (e[k] ??= []).push(id)
  }
  return e
}

/** the event files grow to about this much, then start over (the one before is kept beside it) */
const ROTATE_AT = 256 * 1024

/** the recorder Claude Code runs for every hook: reads the event from stdin, appends one line */
export const recorderScript = () => `// Written by Shepherdr: records a Claude Code hook event (readers/hooks.ts).
const { appendFileSync, renameSync, statSync } = require('node:fs')
const pickHook = ${pickHook.toString()}
const file = process.argv[2]
let s = ''
process.stdin.setEncoding('utf8')
process.stdin.on('data', (c) => (s += c))
process.stdin.on('end', () => {
  try {
    const e = pickHook(JSON.parse(s), Date.now())
    if (!e) return
    try {
      if (statSync(file).size > ${ROTATE_AT}) renameSync(file, file + '.1')
    } catch {}
    appendFileSync(file, JSON.stringify(e) + '\\n')
  } catch {}
})
`

const q = (s: string) => `'${s.replace(/'/g, `'\\''`)}'`

/** the settings file that makes one Claude Code record its hooks to `events`; returns its path. Starting over: what
 *  an earlier run recorded is gone. */
export function hookSettings(dir: string, name: string, runtime = process.execPath): { settings: string; events: string } {
  task.mkdirSync(dir, { recursive: true })
  const events = join(dir, `${name}.jsonl`)
  task.rmSync(events, { force: true })
  task.rmSync(`${events}.1`, { force: true })
  let command: string
  if (BOX) {
    // another machine has no Shepherdr to run the recorder: the helper's perl one appends the event as it came, with
    // its time, and the main machine cuts it down as it reads the mirrored file (parseHookLines)
    command = `perl ${q(join(helperDir(BOX.home), 'hook.pl'))} ${q(events)}`
  } else {
    // other agents' hooks may be running it: it is replaced whole, and only when it changed
    const script = join(dir, 'record.cjs')
    const text = recorderScript()
    if (!task.existsSync(script) || task.readFileSync(script, 'utf8') !== text) {
      task.writeFileSync(`${script}.${process.pid}`, text)
      task.renameSync(`${script}.${process.pid}`, script)
    }
    command = `${q(runtime)} ${q(script)} ${q(events)}`
  }
  const one = [{ hooks: [{ type: 'command', command, timeout: 10 }] }]
  const settings = join(dir, `${name}.settings.json`)
  task.writeFileSync(settings, JSON.stringify({ hooks: Object.fromEntries(HOOK_EVENTS.map((e) => [e, one])) }, null, 2))
  return { settings, events }
}

/** what still runs, from Stop's list: a subagent (maybe with shells), shells and monitors that end, or only servers.
 *  Shells while a subagent of its own is paused may be that subagent's, and wake it: a subagent still runs. */
const backgroundOf = (bg: string[] | undefined, paused: string[] | undefined): Background | undefined =>
  !bg?.length ? undefined : bg.some((t) => /agent/i.test(t)) || paused?.length ? 'agents' : bg.every((t) => t === 'server') ? 'servers' : 'shells'

/** its paused subagents, after the notices that woke it: one that is over, or notified again, is no longer paused */
const pausedAfter = (was: string[] | undefined, e: HookEvent): string[] | undefined => {
  const now = [...(was ?? []).filter((id) => !e.over?.includes(id) && !e.paused?.includes(id)), ...(e.paused ?? [])]
  return now.length ? now : undefined
}

/** one more event, folded into what the agent says. A subagent's events don't start or end its turn; only its
 *  permission prompt counts, until that subagent goes on. */
export function foldHook(s: Said | undefined, e: HookEvent): Said | undefined {
  if (e.agent) {
    if (!s) return s
    if (e.event === 'PermissionRequest') return { ...s, helper: e.agent }
    return s.helper === e.agent && /^(Pre|Post)ToolUse/.test(e.event) ? { ...s, helper: undefined } : s
  }
  const base: Said = s ?? { state: 'idle', turnAt: e.at, at: e.at }
  const was = { ...base, at: e.at }
  const go = (patch: Partial<Said>): Said => ({ state: was.state, turnAt: was.turnAt, at: e.at, last: was.last, bg: was.bg, helper: was.helper, paused: was.paused, ...patch })
  switch (e.event) {
    case 'SessionStart':
      // a compaction mid-turn starts the session again too; the turn goes on
      return e.type === 'compact' ? was : { state: 'idle', turnAt: e.at, at: e.at }
    case 'UserPromptSubmit':
      return { state: 'working', turnAt: e.at, at: e.at, paused: pausedAfter(was.paused, e) }
    case 'PreToolUse':
      if (e.tool === 'AskUserQuestion') return go({ state: 'blocked', kind: 'question', ask: e.question, bg: undefined })
      if (e.tool === 'ExitPlanMode') return go({ state: 'blocked', kind: 'plan', bg: undefined })
      return go({ state: 'working', bg: undefined })
    case 'PermissionRequest':
      // the question tool and the plan ask first too; a prompt for anything else is a permission
      if (was.state === 'blocked' && (was.kind === 'question' || was.kind === 'plan')) return was
      return go({ state: 'blocked', kind: e.tool === 'AskUserQuestion' ? 'question' : e.tool === 'ExitPlanMode' ? 'plan' : 'permission', bg: undefined })
    case 'Notification':
      // a permission prompt's notice comes seconds after its PermissionRequest, maybe after it was answered: not news
      if (e.type === 'elicitation_dialog') return go({ state: 'blocked', kind: 'question' })
      return was
    case 'PostToolUse':
    case 'PostToolUseFailure':
      return go({ state: 'working', bg: undefined })
    case 'Stop':
      return { state: 'idle', turnAt: was.turnAt, at: e.at, last: e.msg, bg: backgroundOf(e.bg, was.paused), helper: was.helper, paused: was.paused }
    case 'StopFailure':
      return { state: 'idle', turnAt: was.turnAt, at: e.at, last: e.msg, kind: /auth|login|oauth|api key/i.test(e.msg ?? '') ? 'auth' : undefined, paused: was.paused }
    case 'SessionEnd':
      return undefined
    default:
      return was
  }
}

export const parseHookLines = (text: string): HookEvent[] =>
  text
    .split('\n')
    .flatMap((l) => {
      try {
        const e = l.trim() && JSON.parse(l)
        // as another machine's helper writes it: the hook's whole input and when it came, cut down here
        if (e && typeof e.raw === 'object' && typeof e.at === 'number') {
          const picked = pickHook(e.raw, e.at)
          return picked ? [picked] : []
        }
        return e && typeof e.event === 'string' && typeof e.at === 'number' ? [e as HookEvent] : []
      } catch {
        return []
      }
    })

/** the CLI session an agent is in now, as its hooks last said: its id, and its transcript where it said */
export interface HookSession {
  id: string
  log?: string
  /** when it said so */
  at: number
}

/** every agent's events file, read on from where it was last read, and what each says now */
export class HookLogs {
  private files = new Map<string, { size: number; carry: string; said?: Said; seen: boolean; session?: HookSession; runtime?: { model?: string; effort?: string; at: number; session?: string } }>()

  /** what the agent recording to this file says now, or nothing: no event yet (an agent started before hooks, or
   *  still starting), or its session ended; and the session it is in */
  read(file: string): { said?: Said; seen: boolean; session?: HookSession; runtime?: { model?: string; effort?: string; at: number; session?: string } } {
    let f = this.files.get(file)
    let size = 0
    try {
      size = statSync(file).size
    } catch {
      this.files.delete(file)
      return { seen: false }
    }
    if (!f) this.files.set(file, (f = { size: 0, carry: '', seen: false }))
    // it grew too big and turned over: what was folded stays, the new file is read from its start
    else if (size < f.size) Object.assign(f, { size: 0, carry: '' })
    if (size > f.size) {
      let fresh = f.carry
      try {
        const fd = openSync(file, 'r')
        try {
          const buf = Buffer.alloc(size - f.size)
          fresh += buf.subarray(0, readSync(fd, buf, 0, buf.length, f.size)).toString('utf8')
        } finally {
          closeSync(fd)
        }
      } catch {
        return { said: f.said, seen: f.seen, session: f.session, runtime: f.runtime }
      }
      const cut = fresh.lastIndexOf('\n')
      f.carry = cut < 0 ? fresh : fresh.slice(cut + 1)
      for (const e of parseHookLines(cut < 0 ? '' : fresh.slice(0, cut))) {
        if (!e.agent) {
          if (e.event === 'SessionStart' && e.type !== 'compact') f.runtime = undefined
          if (e.model || e.effort) f.runtime = { ...(e.model && f.runtime?.model && e.model !== f.runtime.model ? {} : f.runtime), ...(e.model ? { model: e.model } : {}), ...(e.effort ? { effort: e.effort } : {}), at: e.at, session: e.session ?? f.session?.id }
        }
        f.said = foldHook(f.said, e)
        f.seen = true
        if (e.session && !e.agent) f.session = { id: e.session, log: e.log ?? (f.session?.id === e.session ? f.session.log : undefined), at: e.at }
      }
      f.size = size
    }
    return { said: f.said, seen: f.seen, session: f.session, runtime: f.runtime }
  }

  forget(file: string) {
    this.files.delete(file)
  }
}

/** for tests and tools: record one event the way the recorder does */
export function record(file: string, input: unknown, at = Date.now()) {
  const e = pickHook(input, at)
  if (!e) return
  mkdirSync(dirname(file), { recursive: true })
  try {
    if (statSync(file).size > ROTATE_AT) renameSync(file, `${file}.1`)
  } catch {
    /* not there yet */
  }
  appendFileSync(file, `${JSON.stringify(e)}\n`)
}

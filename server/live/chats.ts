// Chats: one chat per agent across all its sessions, read from the CLI's own log files, plus what Shepherdr itself
// posts (update cards, choice cards, plans, artifacts, merge cards) kept in its own small log per chat.
import { createHash } from 'node:crypto'
import { existsSync, rmSync } from 'node:fs'
import { join } from 'node:path'
import { apart, cardMessage, type ActivityAnswer, type ActivityAsk, type Chat, type ChatControls, type ArtifactPost, type Job, type Message, type Question, type Reaction, type SecretAsk, type UpdateCard } from '../../shared/types.ts'
import { HOME, readJson, writeJson } from '../store.ts'
import { stripHidden, type Agents, type OutItem } from './agents.ts'
import { agentLines, excerpt, findQuoted, markerSoFar, parseBrowserNotes, parseReply, parseUserLine, userLines, type Parsed } from './chatlines.ts'
import { shortUrl } from '../../shared/browser.ts'
import { herdr } from './herdr.ts'
import { logKind } from './readers/find.ts'
import { LogTail, readerFor } from './readers/index.ts'
import type { LogEvent } from './readers/types.ts'
import type { Cut } from './retry.ts'
import { findModel, modelsOf, nearest } from './models.ts'
import { sameCommand } from './commands.ts'
import { sessionRuntime } from './runtime.ts'
import { launchHarness } from './restart.ts'
import { CLI_NAME } from './settings.ts'
import { replyOnScreen } from './streaming.ts'
import { activityOf, codexThreadLog, cutOutput, headlessJob, JobLogs, lastLine, lineOf, outputAt, shown as shownStep, tailOf, type JobRec } from './activity.ts'

/** a chat line of yours that shows as words: an answer, a page's Send and words about an artifact under what they
 *  answer, a note with what it is on */
function shownAs(p: Parsed): { title?: string; text: string } | undefined {
  if (p.kind === 'answer') return { title: `Answered “${p.title}”`, text: p.answer }
  if (p.kind === 'secret') return { title: 'Gave a secret', text: `${p.name}${p.where ? `, for ${p.where}` : ''}` }
  if (p.kind === 'artifact-send') return { title: `Sent from "${p.title}"`, text: p.text }
  if (p.kind === 'artifact-say') return { title: `About "${p.title}"`, text: p.text }
  if (p.kind === 'note') return { text: `[note on ${p.target}] ${p.note}` }
  return undefined
}

/** a line of yours the agent will answer: words of yours, not Shepherdr's hidden news and not only reactions */
const yours = (text: string) =>
  stripHidden(text)
    .split('\n')
    .some((ln) => ln.trim() && !['react', 'unreact'].includes(parseUserLine(ln.trim())?.kind ?? ''))

/** The turn the agent is in, by its log: one your message started ('yours'), one it started itself on Shepherdr's news,
 *  an automation or its background work waking it ('own', with that turn's id), or none ('over': its reply ended the
 *  last one, and what herdr still sees working is background work: a subagent, a shell). No turn marks: 'unknown'. */
export function turnOf(ev: LogEvent[], sent: { at: number }[]): { kind: 'yours' | 'own' | 'over' | 'unknown'; id?: string } {
  if (!ev.some((e) => e.kind === 'turn-start' || e.kind === 'turn-end')) return { kind: 'unknown' }
  const end = ev.findLastIndex((e) => e.kind === 'turn-end')
  // what you sent from a screen since the last turn ended went in before the CLI wrote it down: its turn is on its way
  if (sent.some((o) => o.at > (end < 0 ? -Infinity : ev[end].at))) return { kind: 'yours' }
  const now = ev.slice(end + 1)
  const start = now.find((e) => e.kind === 'turn-start')
  if (!start) return { kind: 'over' }
  // yours once a message of yours is in it, even one taken in mid-turn
  return now.some((e) => e.kind === 'user' && yours(e.text)) ? { kind: 'yours' } : { kind: 'own', id: start.id }
}

/** a reply you will see that already landed in the agent's own turn (a hidden <no-reply> is none) */
export const landed = (ev: LogEvent[]) => {
  const start = ev.findLastIndex((e) => e.kind === 'turn-start')
  return start >= 0 && ev.slice(start).some((e) => e.kind === 'agent' && agentLines(stripHidden(e.text)).text)
}

/** Do the typing dots show: is a message you will see on its way? In a turn you started, at once (an agent never
 *  answers you with a hidden <no-reply>). In one it started itself, only once its reply shows: on its screen as it
 *  streams (`shown`) or landed in its log; until then it may be thinking, running tools, or about to say <no-reply>. */
export function replying(turn: ReturnType<typeof turnOf>, shown: boolean, ev: LogEvent[]): boolean {
  if (turn.kind === 'yours' || turn.kind === 'unknown') return true
  return turn.kind === 'own' && (shown || landed(ev))
}

/** what the screen showed of the reply in a turn the agent started itself */
interface Shown {
  turn: string
  /** the reply's text at the last read; it must grow once before it counts, so a still line (a job's "completed") never does */
  last?: string
  on: boolean
  at: number
}

/** something Shepherdr posted into a chat */
export interface Posted {
  id: string
  at: number
  card?: UpdateCard
  question?: Question
  artifact?: ArtifactPost
  /** a secret the agent asked for: its masked box */
  secret?: SecretAsk
  /** text shown with it, when it stands alone */
  text?: string
  /** a line of yours Shepherdr shows for you (an answer typed as keys, or a command, which the log doesn't keep) */
  who?: 'you'
  answered?: string
  /** answered on the question's copy in a space's chat: a link there */
  elsewhere?: Question['elsewhere']
  /** the screen's own id for it: the log's line, when it has one after all, shows instead */
  cid?: string
}

interface ChatLog {
  posted: Posted[]
  /** the choice you sent for each question, by question id */
  answers: Record<string, number>
  /** the words a question was answered with instead of a pick, by question id; chat: a message in the chat; space: on
   *  its copy in that space's chat */
  said?: Record<string, { text?: string; chat?: true; space?: string }>
}

/** a card Shepherdr posted, with its answer: a choice card answered without a pick and no words typed on it was
 *  answered by a message in the chat (closeAsks; older logs kept no words) */
function withAnswer(q: Question, l: ChatLog, place: (key: string) => Question['elsewhere']): Question {
  const sent = q.sent ?? l.answers[q.id]
  const s = l.said?.[q.id]
  const elsewhere = s?.space ? place(s.space) : undefined
  if (sent !== -1) return { ...q, sent, elsewhere }
  return { ...q, sent, said: s?.text, via: s?.chat || (!s && q.id.startsWith('ask:')) ? 'chat' : undefined, elsewhere }
}

const safe = (key: string) => key.replace(/[^a-zA-Z0-9_.-]+/g, '_')

/** a menu the CLI shows in its terminal (permission prompts and the like), read from the screen */
export interface ScreenMenu {
  title: string
  choices: string[]
  /** what the CLI shows under each choice, in grey (Claude Code's own questions): none, or one per choice */
  descriptions?: (string | undefined)[]
  /** the choice the CLI's cursor is on */
  cursor: number
  /** Direct keys for y/n prompts; otherwise move the menu cursor and confirm. */
  keys?: string[][]
  multi?: boolean
  selected?: number[]
  reject?: string
  /** The visible prompt, including security context that must not be lost. */
  screen?: string
}

/** A card's identity excludes the cursor, but includes the full permission being granted. */
export function menuId(key: string, m: ScreenMenu) {
  const screen = m.multi
    ? m.screen?.split('\n').map((l) => l.trim().replace(/^[❯›>▶]\s*/, '').replace(/^\[[✔✓x ]\]/, '[ ]')).join('\n')
    : m.screen?.replace(/^(\s*)[❯›>▶](?=\s)/gm, '$1 ')
  return `menu:${key}:${createHash('sha256').update(JSON.stringify([m.title, m.choices, screen])).digest('hex').slice(0, 16)}`
}

export function menuKeys(m: ScreenMenu, choice: number, choices?: number[]): string[] | undefined {
  if (m.multi) {
    if (!choices) return
    if (!choices.length && choice === -1) return ['escape']
    if (choices.some((i) => !Number.isInteger(i) || i < 0 || i >= m.choices.length)) return
    const keys: string[] = []
    let cursor = m.cursor
    const move = (to: number) => { const d = to - cursor; keys.push(...Array(Math.abs(d)).fill(d > 0 ? 'down' : 'up')); cursor = to }
    for (let i = 0; i < m.choices.length; i++) if (choices.includes(i) !== !!m.selected?.includes(i)) { move(i); keys.push('space') }
    move(m.choices.length)
    keys.push('enter')
    return keys
  }
  if (!Number.isInteger(choice) || choice < 0 || choice >= m.choices.length) return
  const d = choice - m.cursor
  return m.keys?.[choice] ?? [...Array(Math.abs(d)).fill(d > 0 ? 'down' : 'up'), 'enter']
}

/** Drop shell startup and old output above the prompt's separator. Keep all of the consent text. */
function promptScreen(lines: string[], first: number) {
  for (let i = first - 1; i >= 0; i--) if (/^\s*[╭┌│|]?[─━─]{5,}/.test(lines[i])) return lines.slice(i + 1).join('\n').trim()
  return lines.join('\n').trim()
}

/** Unnumbered arrow menus and single-line confirmations, as used at startup. */
function startupMenu(lines: string[]): ScreenMenu | null {
  const screen = lines.join('\n').trim()
  const clean = (s: string) => s.replace(/^[\s─╭╰]+|[\s─╮╯]+$/g, '').trim()
  // Claude's multi-server MCP consent is a checkbox list followed by a submit row.
  if (/new MCP servers found in this project/i.test(screen) && /Space to select.*Esc to reject all/i.test(screen)) {
    const first = lines.findIndex((l) => /^\s*[❯›>▶]?\s*\[[✔✓x ]\]\s+/.test(l))
    const rows = lines.slice(first).filter((l) => /^\s*[❯›>▶]?\s*\[[✔✓x ]\]\s+/.test(l))
    if (first >= 0 && rows.length) {
      const cursor = rows.findIndex((l) => /[❯›>▶]/.test(l))
      return { title: lines.find((l) => /new MCP servers found in this project/i.test(l))!.trim(), choices: rows.map((l) => l.replace(/^\s*[❯›>▶]?\s*\[[✔✓x ]\]\s+/, '').trim()), cursor: cursor < 0 ? rows.length : cursor, multi: true, selected: rows.flatMap((l, i) => /\[[✔✓x]\]/.test(l) ? [i] : []), reject: 'Reject all', screen: promptScreen(lines, first) }
    }
  }
  const cursor = lines.findLastIndex((l) => /^\s*[❯›>▶]\s+\S/.test(l))
  if (cursor >= 0 && /enter.*(?:confirm|select|continue)|[↑↓]|arrow keys/i.test(screen)) {
    const col = lines[cursor].search(/[❯›>▶]/) + 2
    const isOption = (i: number) => i === cursor || (!!lines[i]?.trim() && lines[i].search(/\S/) === col && !/^(?:enter|esc|press |[↑↓]|arrow keys).*?(?:confirm|select|continue|cancel|navigate|back)/i.test(lines[i].trim()))
    let first = cursor, last = cursor
    while (first > 0 && isOption(first - 1)) first--
    while (last + 1 < lines.length && isOption(last + 1)) last++
    if (last > first) {
      const choices = lines.slice(first, last + 1).map((l) => l.replace(/^\s*[❯›>▶]?\s*/, '').trim())
      const context = lines.slice(0, first).map(clean).filter(Boolean)
      const title = context.findLast((l) => /\?|MCP servers? found|trust|accessing workspace/i.test(l)) ?? context.at(-1) ?? 'Choose an option'
      return { title, choices, cursor: cursor - first, screen: promptScreen(lines, first) }
    }
  }
  const yn = screen.match(/([^\n]*?)(?:\[y\/n\]|\(y\/n\)|\[yes\/no\]|\(yes\/no\))\s*[:?]?\s*$/i)
  if (yn) return { title: yn[1].trim() || 'Continue?', choices: ['Yes', 'No'], cursor: 0, keys: [['y', 'enter'], ['n', 'enter']], screen }
  return null
}

export function parseMenu(screen: string): ScreenMenu | null {
  const lines = screen.split('\n').map((l) => l.replace(/\s+$/, '').replace(/^[\s│|]+|[│|]+$/g, (m) => m.replace(/[│|]/g, ' ')))
  const OPT = /^(\s*)([❯›>▶*]\s*)?(\d{1,2})[.)]\s+(.+)$/
  // the last numbered list on screen that starts at 1
  let first = -1
  for (let i = lines.length - 1; i >= Math.max(0, lines.length - 40); i--) {
    const m = lines[i].match(OPT)
    if (m && m[3] === '1') {
      first = i
      break
    }
  }
  if (first < 0) return startupMenu(lines)
  const opts: { label: string; description?: string; on: boolean }[] = []
  let indent = 0
  // the terminal's width, by its longest line: a line wraps only once the next word would pass it
  const width = Math.max(...lines.map((l) => l.trimEnd().length))
  for (let i = first; i < lines.length; i++) {
    const l = lines[i]
    const m = l.match(OPT)
    if (m && Number(m[3]) === opts.length + 1) {
      opts.push({ label: m[4].trim(), on: !!m[2] })
      indent = m[1].length + (m[2]?.length ?? 0)
      continue
    }
    // an indented line under a choice: the choice wrapping on, or (under a line that had room for more) its description
    if (opts.length && l.trim() && l.search(/\S/) > indent) {
      const o = opts[opts.length - 1]
      const wraps = lines[i - 1].trimEnd().length + 1 + l.trim().split(' ')[0].length > width
      if (o.description !== undefined || !wraps) o.description = o.description === undefined ? l.trim() : `${o.description} ${l.trim()}`
      else o.label += ` ${l.trim()}`
      continue
    }
    if (!l.trim() && opts.length) continue
    break
  }
  if (opts.length < 2) return null
  for (const o of opts) o.label = o.label.replace(/\s*\((esc|enter|tab|[a-z])\)$/i, '').trim()
  // the question: the nearest line above the list with a "?"
  const clean = (x: string) => x.replace(/^[\s╭╰─]+|[\s╮╯─]+$/g, '').trim()
  const asks = /would you like|do you want|allow|proceed|run the following|run this/i
  const marked = [...Array(Math.min(14, first)).keys()].map((k) => first - 1 - k).filter((i) => lines[i].includes('?'))
  const qi = marked.find((i) => asks.test(lines[i])) ?? marked[0] ?? -1
  if (qi < 0) return null
  // a question that wraps: its lines above, while each goes on from the one before (starts in lower case)
  let top = qi
  while (top > 0 && /^[a-z]/.test(clean(lines[top])) && clean(lines[top - 1])) top--
  const qline = lines.slice(top, qi + 1).map(clean).join(' ')
  const qmark = qline.indexOf('?')
  const question = qline.slice(Math.max(qline.lastIndexOf('. ', qmark) + 1, 0), qmark + 1).trim()
  // the command being asked about, when the CLI shows one between the question and the list
  const shown = lines
    .slice(qi + 1, first)
    .map(clean)
    .filter(Boolean)
  const at = shown.findIndex((l) => l.startsWith('$ '))
  const cmd = (at >= 0 ? shown.slice(at).filter((l, k) => k === 0 || !/[?:]$/.test(l)).join(' ').slice(2) : undefined) ?? lines.slice(Math.max(0, qi - 4), qi).map(clean).filter((l) => l && !/[.:]$/.test(l) && !/^(Bash|Edit|Write|Read)( command)?$/i.test(l)).at(-1)
  const asksToRun = asks.test(question)
  const title = asksToRun && cmd ? `Allow: ${cmd.replace(/\?$/, '')}?` : question
  const descriptions = opts.some((o) => o.description) ? opts.map((o) => o.description) : undefined
  return { title, screen: promptScreen(lines, first), choices: opts.map((o) => o.label), ...(descriptions ? { descriptions } : {}), cursor: Math.max(0, opts.findIndex((o) => o.on)) }
}

/** words as typed, whatever the spacing a CLI's log keeps */
const words = (s: string) => s.replace(/\s+/g, ' ').trim()

/** the message each reply sent from a screen answers, by the words of its line: a quote alone may fit a newer message
    that starts the same way ("Done."). Kept until a restart; after one the quote finds it. */
const replied = new Map<string, string>()

export function rememberReply(line: string, id: string) {
  replied.set(words(line), id)
  if (replied.size > 500) replied.delete(replied.keys().next().value!)
}

/** the agent's reply of the same turn, written at or after `at`, before you wrote again */
function hostFor(msgs: Message[], at: number, p: { question?: unknown; artifact?: unknown }, hosting: Set<Message>): Message | undefined {
  for (const m of msgs) {
    if ((m.at ?? 0) < at - 1000) continue
    if (m.who === 'you') return undefined
    if (m.text && !m.card && !(p.question && m.question) && !(p.artifact && hosting.has(m))) return m
  }
  return undefined
}

/** one more read of the screen: the reply counts once it grew since the last read and, past its reaction and reply
 *  lines, starts with two letters or more that aren't a hidden marker (<no-reply>, an older [[...]]), whole or still coming */
export function sawReply(s: Pick<Shown, 'last' | 'on'>, text: string | undefined): Pick<Shown, 'last' | 'on'> {
  const words = text === undefined ? '' : agentLines(text).text
  const ok = words.length >= 2 && !markerSoFar(words)
  return { on: s.on || (ok && s.last !== undefined && text !== s.last), last: ok ? text : undefined }
}

/** how much of a chat's logs one pull reads: the rest is read in the background a piece at a time, so a log of a
 *  hundred MB (a long-lived coordinator's) never holds up the server, at a restart or as it grows */
const PIECE = 2 << 20

export class Chats {
  private tails = new Map<string, LogTail>()
  private events = new Map<string, LogEvent[]>()
  /** where each reply and usage is in its chat's events, by kind and id: a reply read in two pieces replaces the first */
  private where = new Map<string, Map<string, number>>()
  /** chats whose logs are still being read in the background */
  private reading = new Set<string>()
  /** a chat's logs were read in the background to their end: what it shows changed */
  onRead?: (key: string) => void
  private logs = new Map<string, ChatLog>()
  private menus = new Map<string, ScreenMenu>()
  private menuAt = new Map<string, number>()
  private shown = new Map<string, Shown>()
  private jobLogs = new JobLogs()
  private threadLogs = new Map<string, string>()
  private jobsSeen = new Map<string, string>()

  constructor(
    private agents: Agents,
    private nameOf: (key: string) => string,
    private user: () => string,
  ) {}

  /* ---------- Shepherdr's own log per chat ---------- */

  log(key: string): ChatLog {
    let l = this.logs.get(key)
    if (!l) {
      l = readJson<ChatLog>(`chatlog/${safe(key)}.json`, { posted: [], answers: {} })
      this.logs.set(key, l)
    }
    return l
  }

  saveLog(key: string) {
    writeJson(`chatlog/${safe(key)}.json`, this.log(key))
  }

  /** a deleted task's chat: its log goes, and all that was read of it */
  forget(key: string) {
    for (const m of [this.tails, this.events, this.where, this.reading, this.logs, this.menus, this.menuAt, this.shown]) m.delete(key)
    rmSync(join(HOME, 'chatlog', `${safe(key)}.json`), { force: true })
  }

  /** told of each new post: the chat has something new (unread.ts) */
  onPost?: (key: string) => void

  post(key: string, p: Omit<Posted, 'at'> & { at?: number }) {
    const l = this.log(key)
    const i = l.posted.findIndex((x) => x.id === p.id)
    const item = { at: Date.now(), ...p }
    if (i >= 0) l.posted[i] = { ...l.posted[i], ...item, at: l.posted[i].at }
    else l.posted.push(item)
    this.saveLog(key)
    if (i < 0 && !p.who) this.onPost?.(key)
  }

  /** a page posted before artifacts: now the artifact with its id */
  repostPage(key: string, id: string) {
    const l = this.log(key)
    const p = l.posted.find((x) => x.id === `page:${id}`) as (Posted & { page?: unknown }) | undefined
    if (!p) return
    delete p.page
    p.artifact = { id, v: 1 }
    this.saveLog(key)
  }

  /** other: the words typed on the card in place of a pick; space: answered on its copy in that space's chat */
  answer(key: string, qid: string, choice: number, other?: string, space?: string) {
    const l = this.log(key)
    l.answers[qid] = choice
    if (other || space) (l.said ??= {})[qid] = { text: other, space }
    this.saveLog(key)
  }

  /** a reply to the agent's plain question went from its copy in a space's chat: the reply says so */
  repliedFrom(key: string, reply: string, space: string) {
    ;((this.log(key).said ??= {})[`reply:${reply}`] = { space })
    this.saveLog(key)
  }

  /** where a question answered on its copy in a space's chat says it was answered, with a link there */
  answeredIn = (space: string): Question['elsewhere'] => ({ label: 'Answered in', chat: space, name: `${this.nameOf(space)} chat` })

  /** a message you send answers the choice cards still open in that chat: they close as answered in words (-1), and
   *  keep those words to quote */
  closeAsks(key: string, text?: string) {
    const l = this.log(key)
    const open = l.posted.filter((p) => p.question?.id.startsWith('ask:') && p.question.sent === undefined && l.answers[p.question.id] === undefined)
    if (!open.length) return false
    for (const p of open) {
      l.answers[p.question!.id] = -1
      ;(l.said ??= {})[p.question!.id] = { text: text?.trim() || undefined, chat: true }
    }
    this.saveLog(key)
    return true
  }

  /* ---------- reading the CLI's logs ---------- */

  /** read new lines of this chat's log files, up to PIECE in all (the rest follows in the background); true when
   *  anything new came */
  pull(key: string): boolean {
    const { changed, behind } = this.read(key)
    if (behind) this.readOn(key)
    return changed
  }

  private read(key: string): { changed: boolean; behind: boolean } {
    const r = this.agents.record(key)
    if (!r) return { changed: false, behind: false }
    let changed = false
    let behind = false
    let left = PIECE
    const all = this.events.get(key) ?? []
    const where = this.where.get(key) ?? new Map<string, number>()
    for (const f of r.logs) {
      if (!existsSync(f)) continue
      let t = this.tails.get(f)
      if (!t) {
        // each by its own CLI's reader: a backup agent's logs follow the first one's
        const reader = readerFor(logKind(f) ?? r.kind)
        if (!reader) continue
        t = new LogTail(f, reader)
        this.tails.set(f, t)
      }
      const was = t.done
      const ev = t.read(left)
      left -= Math.max(0, t.done - was)
      for (const e of ev) {
        // a reply read in two pieces comes again with the same id and the full text: it replaces the first
        const id = e.kind === 'agent' || e.kind === 'usage' ? `${e.kind}:${e.id}` : undefined
        const i = id === undefined ? undefined : where.get(id)
        if (i !== undefined) all[i] = e
        else {
          if (id !== undefined) where.set(id, all.length)
          all.push(e)
        }
        changed = true
      }
      // the next log once this one is read to its end: the chat keeps its order
      if ((behind = t.behind || left <= 0)) break
    }
    if (changed) (this.events.set(key, all), this.where.set(key, where))
    return { changed, behind }
  }

  /** reads the rest of a chat's logs a piece at a time, letting the server answer between pieces; told once at the end */
  private readOn(key: string) {
    if (this.reading.has(key)) return
    this.reading.add(key)
    const step = () => {
      if (!this.reading.has(key)) return
      if (this.read(key).behind) return void setTimeout(step, 0)
      this.reading.delete(key)
      this.onRead?.(key)
    }
    setTimeout(step, 0)
  }

  /** tokens this chat's agent has used, across all its sessions */
  tokens(key: string): number {
    this.pull(key)
    return (this.events.get(key) ?? []).reduce((n, e) => n + (e.kind === 'usage' ? e.tokens : 0), 0)
  }

  /** what the agent last said, plain */
  lastReply(key: string): string | undefined {
    this.pull(key)
    const ev = this.events.get(key) ?? []
    for (let i = ev.length - 1; i >= 0; i--) {
      const e = ev[i]
      if (e.kind === 'agent') return agentLines(stripHidden(e.text)).text
    }
    return undefined
  }

  /** the newest thing in the chat from its agent (a reply you will see, its question) or Shepherdr (a card, an
   *  artifact): when it came, its words, and whether it is a card about a task (whose own alert tells of it) */
  newest(key: string): { at: number; text?: string; card?: boolean } | undefined {
    this.pull(key)
    const ev = this.events.get(key) ?? []
    const e = ev.findLast((x) => (x.kind === 'agent' && agentLines(stripHidden(x.text)).text) || x.kind === 'question')
    const p = this.log(key).posted.findLast((x) => !x.who)
    const reply = e && { at: e.at, text: e.kind === 'agent' ? agentLines(stripHidden(e.text)).text : undefined }
    const posted = p && { at: p.at, text: p.card?.text ?? p.text ?? p.question?.title, card: !!p.card }
    return !reply || (posted && posted.at > reply.at) ? posted : reply
  }

  /** in a turn the agent started itself, has its reply begun on its screen, not as a hidden <no-reply> (or older [[...]])? Read about
   *  once a second, until it has; true when that changes what the chat shows */
  async readReply(key: string): Promise<boolean> {
    const r = this.agents.record(key)
    const turn = turnOf(this.events.get(key) ?? [], this.agents.sentLately(key))
    if (!r || this.agents.status(key) !== 'working' || turn.kind !== 'own') {
      const had = this.shown.get(key)?.on ?? false
      this.shown.delete(key)
      return had
    }
    const was = this.shown.get(key)
    const s: Shown = was && was.turn === turn.id ? was : { turn: turn.id!, on: false, at: 0 }
    this.shown.set(key, s)
    // a little under the poll's second, so a late tick never skips a read
    if (s.on || Date.now() - s.at < 900) return false
    s.at = Date.now()
    const info = this.agents.info(key)
    if (!info) return false
    Object.assign(s, sawReply(s, replyOnScreen(r.kind, await herdr.visible(info.pane_id).catch(() => ''))))
    return s.on
  }

  /** a blocked agent's terminal menu, or one a slash command opened, read from its screen (refreshed at most every
   *  2s) */
  async readMenu(key: string, force = false): Promise<boolean> {
    const st = this.agents.status(key)
    if (!force && (st === 'working' || st === 'gone') && !this.commandMenus.has(key)) {
      const had = this.menus.delete(key)
      return had
    }
    const last = this.menuAt.get(key) ?? 0
    if (!force && Date.now() - last < 2000) return false
    this.menuAt.set(key, Date.now())
    const info = this.agents.info(key)
    if (!info) return false
    const text = await herdr.visible(info.pane_id)
    // Some herdr versions call startup checkbox menus idle. Require a live menu footer before parsing those.
    const waiting = /(?:enter.*(?:confirm|select|continue)|space to select.*esc to reject|\[y\/n\]|\(y\/n\))/i.test(text.trimEnd().split('\n').slice(-3).join('\n'))
    const parsed = st === 'blocked' || this.commandMenus.has(key) || waiting ? parseMenu(text) : null
    const m = parsed ? { ...parsed, screen: parsed.screen ?? text.trim() } : st === 'blocked' && text.trim() ? { title: 'The terminal is waiting for you', choices: [], cursor: 0, screen: text.trim() } : null
    const before = JSON.stringify(this.menus.get(key) ?? null)
    if (m) this.menus.set(key, m)
    else this.menus.delete(key)
    // the command's menu is answered: what you wrote meanwhile goes in
    if (!m && this.commandMenus.delete(key)) void this.agents.deliver(key)
    return before !== JSON.stringify(m)
  }

  /** menus a slash command opened that herdr doesn't see as blocking the agent: shown as a card all the same, and what
   *  you write waits for them like for any menu */
  private commandMenus = new Set<string>()

  /** a slash command opened a menu: read it as a card; true when there is one */
  async commandMenu(key: string) {
    this.commandMenus.add(key)
    await this.readMenu(key, true).catch(() => false)
    if (!this.menus.has(key)) this.commandMenus.delete(key)
    return this.menus.has(key)
  }

  /** a menu a slash command opened is up */
  commandMenuUp(key: string) {
    return this.commandMenus.has(key) && this.menus.has(key)
  }

  menu(key: string) {
    return this.menus.get(key)
  }

  /** the question the CLI asks with its own tool, still open */
  openQuestion(key: string): { id: string; items: { question: string; options: { label: string }[]; multiSelect: boolean }[] } | undefined {
    const ev = this.events.get(key) ?? []
    const answered = new Set(ev.filter((e) => e.kind === 'answered').map((e) => e.id))
    for (let i = ev.length - 1; i >= 0; i--) {
      const e = ev[i]
      if (e.kind === 'question' && !answered.has(e.id)) return { id: e.id, items: e.items }
    }
    return undefined
  }

  /** a plan or choice card the agent posted since you last wrote to it, still unanswered */
  openCard(key: string): 'plan' | 'ask' | undefined {
    const id = this.openPosted(key)?.id
    return id?.startsWith('plan:') ? 'plan' : id?.startsWith('ask:') ? 'ask' : undefined
  }

  /** that card's question, as the agent wrote it */
  openCardTitle(key: string): string | undefined {
    return this.openPosted(key)?.title
  }

  private openPosted(key: string): Question | undefined {
    this.pull(key)
    const l = this.log(key)
    const since = Math.max(0, ...(this.events.get(key) ?? []).filter((e) => e.kind === 'user').map((e) => e.at), ...l.posted.filter((p) => p.who === 'you').map((p) => p.at))
    for (let i = l.posted.length - 1; i >= 0; i--) {
      const q = l.posted[i].question
      if (l.posted[i].at <= since) continue
      if (!q || q.sent !== undefined || l.answers[q.id] !== undefined) continue
      if (q.id.startsWith('plan:') || q.id.startsWith('ask:')) return q
    }
    return undefined
  }

  /** what cut off the agent's last turn (its provider failed), when nothing began since; never one from before it last
   *  started (started again, or a backup agent that took over, whose own log has nothing yet) */
  cutOff(key: string): Cut | undefined {
    this.pull(key)
    const ev = this.events.get(key) ?? []
    const e = ev.findLast((e) => e.kind === 'error' || e.kind === 'turn-start' || e.kind === 'user')
    if (e?.kind !== 'error' || e.at < (this.agents.record(key)?.started ?? 0)) return undefined
    return { id: e.id, at: e.at, text: e.text, code: e.code, status: e.status }
  }

  /** its log shows a turn that ended after this time */
  endedSince(key: string, at: number): boolean {
    this.pull(key)
    return (this.events.get(key) ?? []).some((e) => e.kind === 'turn-end' && e.at > at)
  }

  /**
   * What you or a coordinator wrote to this agent that it hasn't read yet: still waiting to go in, or typed in since its
   * turn began and not in its log since (Claude Code takes text typed mid-turn at its next step). Reactions and
   * Shepherdr's own hidden lines aside.
   */
  unseen(key: string): OutItem[] {
    this.pull(key)
    const ev = this.events.get(key) ?? []
    const start = ev.findLast((e) => e.kind === 'turn-start')?.at ?? 0
    // the same words (a long paste may be kept as "[Pasted text #1]"), in its log since about when they went in
    const flat = (t: string) => stripHidden(t).replace(/\s+/g, ' ').trim()
    const same = (a: string, b: string) => a.startsWith(b.slice(0, 40)) || b.startsWith(a.slice(0, 40)) || /\[Pasted text #\d/.test(a)
    const read = (o: OutItem) => ev.some((e) => e.kind === 'user' && e.at >= o.at - 5000 && same(flat(e.text), flat(o.text)))
    const mine = (o: OutItem) => !o.reaction && yours(o.text)
    return [...this.agents.waiting(key).filter(mine), ...this.agents.typedSince(key, start).filter((o) => mine(o) && !read(o))]
  }

  /* ---------- what it does: its steps, and its work in the background ---------- */

  /**
   * What it started in the background that still runs, each with what it does now: what its log started and didn't
   * say is over, and the headless agents it runs. Nothing once it is gone; with Claude Code, nothing once its turn is
   * over and its session says nothing runs (a log may miss an end).
   */
  jobs(key: string, act = activityOf(this.events.get(key) ?? [])): JobRec[] {
    const st = this.agents.status(key)
    if (st === 'gone') return []
    const r = this.agents.record(key)
    let list = act.jobs.map((j) => ({ ...j }))
    if (r?.kind === 'claude' && st !== 'working' && st !== 'blocked' && !this.agents.stillRuns(key)) list = []
    const heads = this.agents.reviewers.list(key).map((h) => headlessJob(h, r?.logs ?? []))
    // a headless agent run as a background shell is the agent, once
    if (heads.length) list = list.filter((j) => j.kind !== 'shell' || !/\b(codex (exec|e)|(claude|pi)\b.* (-p|--print))\b/.test(j.title))
    list.push(...heads)
    for (const j of list) {
      if (j.kind === 'agent' && !j.log && r?.kind === 'codex') {
        const f = this.threadLogs.get(j.id) ?? codexThreadLog(j.id)
        if (f) this.threadLogs.set(j.id, (j.log = f))
      }
      if (j.log) j.more = 'steps'
      j.now = j.log ? this.jobLogs.now(j.log) : j.output ? lastLine(j.output) : undefined
    }
    return list
  }

  /** what runs in the background changed since it was last asked (a pass of the watched chats): a job came or went,
   *  or one does something else now */
  jobsChanged(key: string): boolean {
    this.pull(key)
    const sig = JSON.stringify(this.jobs(key).map((j) => [j.id, j.now]))
    if (this.jobsSeen.get(key) === sig) return false
    this.jobsSeen.set(key, sig)
    return true
  }

  /** what the screen opens: a turn's steps, a step's raw output, a background job's own steps or output */
  activity(a: ActivityAsk): ActivityAnswer {
    this.pull(a.chat)
    const act = activityOf(this.events.get(a.chat) ?? [])
    const turns = [...act.turns, ...(act.open ? [act.open] : [])]
    if (a.what === 'turn') {
      const t = turns.find((x) => x.id === a.id)
      if (!t) return { error: 'That turn is no longer in its log.' }
      return { steps: t.steps.slice(-300).map(shownStep), more: t.steps.length > 300 ? t.steps.length - 300 : undefined }
    }
    if (a.what === 'output') {
      const s = turns.flatMap((t) => t.steps).find((x) => x.id === a.id)
      const out = s?.where && outputAt(s.where, s.id)
      return out === undefined || out === '' ? { error: s?.where ? 'It printed nothing.' : 'Its output is not in its log.' } : cutOutput(out)
    }
    const j = this.jobs(a.chat, act).find((x) => x.id === a.id)
    if (!j) return { error: 'That is over now.' }
    if (j.log) return this.jobLogs.steps(j.log)
    const tail = j.output && tailOf(j.output)
    return tail === undefined || tail === '' ? { error: 'Nothing to show for it yet.' } : cutOutput(tail)
  }

  /* ---------- the chat as the screen draws it ---------- */

  chat(key: string): Chat {
    this.pull(key)
    const r = this.agents.record(key)
    const kind = r?.kind ?? 'claude'
    const user = this.user()
    const from = this.nameOf(key)
    const ev = this.events.get(key) ?? []
    const l = this.log(key)
    const msgs: Message[] = []
    const answered = new Set(ev.filter((e) => e.kind === 'answered').map((e) => e.id))
    // a file is named by its path (two may share a name); one quoted by its name alone is the latest of that name
    const byText = (quoted: string, path?: string) => findQuoted(msgs, quoted, path)
    /** an artifact's card, by the version posted (the latest post of it when that version's is gone) */
    const artifactPost = (id: string, v?: number) =>
      msgs.findLast((m) => m.artifact?.id === id && !m.text && (v === undefined || m.artifact.v === v)) ?? msgs.findLast((m) => m.artifact?.id === id && !m.text)
    // artifacts join the chat after what the log says: what quotes one waits for them
    const later: (() => void)[] = []
    let joined = false
    /** a reply's quote of the message it answers: that message when it is still in the chat, else the quote alone */
    const replyTo = (quoted: string, path?: string, line?: string, artifact?: string): Message['reply'] => {
      if (artifact) {
        const q: NonNullable<Message['reply']> = { text: quoted.replace(/”/g, '"'), artifact: true }
        const find = () => {
          const m = artifactPost(artifact, Number(quoted.match(/ · v(\d+)$/)?.[1]) || undefined)
          if (m) Object.assign(q, { id: m.id, who: m.who })
        }
        if (joined) find()
        else later.push(find)
        return q
      }
      const id = line && replied.get(words(line))
      const m = (id && msgs.find((x) => x.id === id)) || byText(quoted, path)
      if (!m) return { text: quoted.replace(/”/g, '"') }
      return { id: m.id, who: m.who, text: m.text ? excerpt(m.text, 160) : (m.file?.name ?? ''), ...(m.text ? {} : { file: true }) }
    }
    const addRx = (m: Message | undefined, emoji: string, who: 'me' | 'them', on: boolean, pending = false) => {
      if (!m) return
      const rx = (m.reactions ??= [])
      const i = rx.findIndex((x) => x.emoji === emoji)
      const cur = i >= 0 ? rx[i].by : undefined
      let next: Reaction['by'] | undefined
      if (on) next = cur && cur !== who ? 'both' : who
      else next = cur === 'both' ? (who === 'me' ? 'them' : 'me') : cur === who ? undefined : cur
      if (i >= 0) rx.splice(i, 1)
      if (next) rx.splice(i >= 0 ? i : rx.length, 0, { emoji, by: next, ...(pending ? { pending } : {}) })
    }
    // what you sent from a screen gets the screen's id back, so the screen swaps the message it showed at once for it:
    // the same words, read from the log once they went in
    const lately = this.agents.sentLately(key).map((o) => ({ cid: o.cid!, at: o.at, words: words(stripHidden(o.text)) }))
    const cidFor = (text: string, at: number) => {
      const i = lately.findIndex((o) => at >= o.at - 3000 && o.words === words(text))
      return i < 0 ? undefined : lately.splice(i, 1)[0].cid
    }
    // Shepherdr's update cards are messages too, among the rest by time, so a reaction can quote one
    const cards = l.posted.filter((p) => p.card).map((p) => cardMessage(p.card!, p.at))
    let sessions = 0
    let newSession = false
    for (const e of ev) {
      while (cards.length && (cards[0].at ?? 0) <= e.at) msgs.push(cards.shift()!)
      if (e.kind === 'session') {
        if (sessions++) newSession = true
        continue
      }
      if (e.kind === 'user') {
        const text = stripHidden(e.text)
        if (!text) continue
        // a reply: its words, then its files; notes from the shared browser show short: each element and its note
        const bn = parseBrowserNotes(text)
        const r = bn ? null : parseReply(text)
        const reply = r ? replyTo(r.to, undefined, r.line, r.artifact) : undefined
        const lines = bn
          ? [`Notes in the browser on ${shortUrl(bn.url)}:`, ...bn.notes.map((n, i) => `${i + 1}. ${n.element}: ${n.note}`), ...bn.rest]
          : r
            ? [...userLines(r.text), ...r.rest]
            : userLines(text)
        const files: { name: string; path: string }[] = []
        const rest: string[] = []
        let answeredTitle: string | undefined
        let consumed = false
        for (const ln of lines) {
          const p = parseUserLine(ln)
          if (!p) {
            rest.push(ln)
            continue
          }
          if (p.kind === 'attach') files.push({ name: p.name, path: p.path })
          else if (p.kind === 'react' || p.kind === 'unreact') {
            const { artifact: a, emoji, kind } = p
            if (a) later.push(() => addRx(artifactPost(a.id, a.v), emoji, 'me', kind === 'react'))
            else addRx(byText(p.to, p.path), p.emoji, 'me', p.kind === 'react')
            consumed = true
          } else {
            const s = shownAs(p)!
            answeredTitle = s.title ?? answeredTitle
            rest.push(s.text)
          }
        }
        const body = rest.join('\n').trim()
        if (consumed && !body && !files.length) continue
        const cid = answeredTitle ? undefined : cidFor(text, e.at)
        if (body || answeredTitle) msgs.push({ id: e.id, who: 'you', at: e.at, text: body || undefined, answered: answeredTitle, cid, newSession: newSession || undefined, reply })
        // each file is a message of its own, right after the text (the first one keeps the message's own id, and a
        // reply with no words)
        files.forEach((file, i) =>
          msgs.push({ id: i || body || answeredTitle ? `${e.id}:${i}` : e.id, who: 'you', at: e.at, file, cid, newSession: (!i && !body && !answeredTitle && newSession) || undefined, reply: !i && !body && !answeredTitle ? reply : undefined }),
        )
        newSession = false
        continue
      }
      if (e.kind === 'agent') {
        const { text, reactions, reply } = agentLines(stripHidden(e.text))
        for (const x of reactions) addRx(byText(x.to, x.path), x.emoji, 'them', true)
        if (!text) continue
        msgs.push({ id: e.id, who: 'agent', at: e.at, text, newSession: newSession || undefined, reply: reply ? replyTo(reply.to, reply.path) : undefined })
        newSession = false
        continue
      }
      if (e.kind === 'question') {
        e.items.forEach((it, i) => {
          const qid = e.items.length > 1 ? `${e.id}:${i}` : e.id
          const sent = l.answers[qid] ?? (answered.has(e.id) ? -1 : undefined)
          // not picked here and no words typed on the card: answered in the terminal
          const said = sent === -1 ? l.said?.[qid]?.text : undefined
          const space = l.said?.[qid]?.space
          msgs.push({
            id: `${e.id}#${i}`,
            who: 'agent',
            at: e.at + i,
            question: {
              id: qid,
              title: it.question,
              kind: 'Question',
              choices: it.options,
              multi: it.multiSelect,
              other: it.allowCustom,
              to: from,
              sent: sent === undefined ? undefined : sent,
              said,
              via: sent === -1 && !said ? 'terminal' : undefined,
              elsewhere: space ? this.answeredIn(space) : undefined,
            },
          })
        })
      }
    }
    msgs.push(...cards)
    msgs.sort((a, b) => (a.at ?? 0) - (b.at ?? 0))
    // what else Shepherdr posted: a question or artifact joins the agent's reply of that turn
    const echoed = new Set<Message>()
    /** the replies an artifact came with: one each */
    const hosting = new Set<Message>()
    for (const p of l.posted) {
      if (p.card) continue
      if (p.who === 'you') {
        // a command as you typed it: once the log has it (by its id, or by its name: /new is /clear to Claude Code), the
        // log's line shows, in your words
        const logged = p.cid ? (msgs.find((m) => m.cid === p.cid) ?? msgs.find((m) => m.who === 'you' && !m.cid && !echoed.has(m) && !!m.text && sameCommand(m.text, p.text ?? '') && Math.abs((m.at ?? 0) - p.at) < 60_000)) : undefined
        if (logged) {
          echoed.add(logged)
          // at when it went in: Claude Code logs a menu's command once the menu closes
          Object.assign(logged, { text: p.text, cid: p.cid, at: Math.min(logged.at ?? p.at, p.at) })
          continue
        }
        msgs.push({ id: p.id, who: 'you', at: p.at, text: p.text, answered: p.answered, elsewhere: p.elsewhere, cid: p.cid })
        continue
      }
      if (p.secret) {
        msgs.push({ id: p.id, who: 'agent', at: p.at, secret: p.secret })
        continue
      }
      const q = p.question ? withAnswer(p.question, l, this.answeredIn) : undefined
      const host = hostFor(msgs, p.at, p, hosting)
      if (host && !p.text) {
        if (q) host.question = q
        // an artifact is a message of its own, just after the words it came with
        if (p.artifact) {
          hosting.add(host)
          msgs.push({ id: p.id, who: 'agent', at: (host.at ?? p.at) + 0.1, artifact: p.artifact })
        }
      } else msgs.push(...apart([{ id: p.id, who: 'agent', at: p.at, text: p.text, question: q, artifact: p.artifact }]))
    }
    // a question that came with them goes after the artifact, as the last thing the chat waits on
    for (const h of hosting) {
      if (!h.question) continue
      msgs.push({ id: `${h.id}:q`, who: 'agent', at: (h.at ?? 0) + 0.2, question: h.question })
      delete h.question
    }
    joined = true
    for (const f of later) f()
    // each turn's steps fold into one line just before its last reply; a turn nobody sees (news it answered with
    // nothing) shows none
    const act = activityOf(ev)
    for (const t of act.turns) {
      if (!t.steps.length) continue
      const reply = t.reply ? msgs.find((m) => m.id === t.reply) : undefined
      if (!reply && !(t.asked && msgs.some((m) => m.id === t.asked))) continue
      msgs.push({ id: `act:${t.id}`, who: 'agent', at: reply ? (reply.at ?? 0) - 0.5 : t.end, activity: lineOf(t) })
    }
    msgs.sort((a, b) => (a.at ?? 0) - (b.at ?? 0))
    // what waits to go in, and why: the first in line holds the rest
    const queue = this.agents.waiting(key)
    const waitsFor = queue.length ? this.agents.holdOf(key, queue[0]) : undefined
    for (const o of queue) {
      if (o.reaction) {
        const m = msgs.find((x) => x.id === o.reaction!.message)
        addRx(m, o.reaction.emoji, 'me', parseUserLine(o.text)?.kind !== 'unreact', true)
      } else {
        const files: { name: string; path: string }[] = []
        const r = parseReply(o.text)
        const reply = r ? replyTo(r.to, undefined, r.line, r.artifact) : undefined
        let answered: string | undefined
        const text = stripHidden(
          (r ? [...userLines(r.text), ...r.rest] : userLines(o.text))
            .flatMap((ln) => {
              const p = parseUserLine(ln)
              if (p?.kind === 'attach') files.push({ name: p.name, path: p.path })
              const s = p && shownAs(p)
              if (s?.title) answered = s.title
              return p?.kind === 'attach' ? [] : [s ? s.text : ln]
            })
            .join('\n'),
        )
        // what Shepherdr itself sends (instructions, news) never shows as yours
        if (text || answered) msgs.push({ id: o.id, who: 'you', at: o.at, text: text || undefined, answered, waiting: true, waitsFor, cid: o.cid, reply })
        files.forEach((file, i) => msgs.push({ id: `${o.id}:${i}`, who: 'you', at: o.at, file, waiting: true, waitsFor, cid: o.cid, reply: !i && !text ? reply : undefined }))
      }
    }
    // your reply to a plain question, sent from its copy in a space's chat: a link there
    for (const [id, s] of Object.entries(l.said ?? {}))
      if (id.startsWith('reply:') && s.space) {
        const m = msgs.find((x) => x.who === 'you' && x.reply?.id === id.slice('reply:'.length))
        if (m) Object.assign(m, { answered: m.answered ?? 'Answered', elsewhere: this.answeredIn(s.space) })
      }
    // a menu in the terminal shows as a question card, posted by Shepherdr
    const menu = this.menus.get(key)
    if (menu && !this.openQuestion(key)) {
      const waitingAt = msgs.findIndex((m) => m.waiting)
      const card: Message = {
        id: menuId(key, menu),
        who: 'agent',
        at: Date.now(),
        question: { id: menuId(key, menu), title: menu.title, screen: menu.screen, terminal: true, noNote: true, multi: menu.multi, reject: menu.reject, kind: 'Form', choices: menu.choices.map((label, i) => ({ label, description: menu.descriptions?.[i] })), to: from, asks: `${CLI_NAME[kind] ?? kind} asks` },
      }
      if (waitingAt >= 0) msgs.splice(waitingAt, 0, card)
      else msgs.push(card)
    }
    const st = this.agents.status(key)
    const failed = this.agents.failedLately(key)
    const active = this.agents.info(key)?.agent_session
    const file = active?.kind === 'path' ? r?.logs.find((f) => f.endsWith('/' + active.value.split('/').at(-1)))
      : r?.sessionId ? r.logs.find((f) => f.endsWith(`${r.sessionId}.jsonl`)) : r?.logs.at(-1)
    let runtime = sessionRuntime(ev, file, r?.started ?? 0)
    if (st !== 'working' || !runtime) runtime = this.agents.runtime(key) ?? runtime
    const hook = this.agents.runtimeHook(key)
    if (hook) runtime = !runtime || hook.at >= runtime.at ? { ...(hook.model && runtime?.model && hook.model !== runtime.model ? {} : runtime), ...hook, source: 'hook' } : { ...(hook.model && runtime.model && hook.model !== runtime.model ? {} : hook), ...runtime }
    if (runtime?.model) runtime.modelLabel = findModel(kind, runtime.model)?.label
    const jobs = this.jobs(key, act).map(({ log: _l, output: _o, ...j }): Job => j)
    return {
      key,
      cli: CLI_NAME[kind] ?? kind,
      agent: r ? { runtime, ...launchHarness({ kind: r.kind, model: r.model, effort: r.effort }), ...(findModel(r.kind, r.model) ? { modelLabel: findModel(r.kind, r.model)!.label } : {}) } : undefined,
      messages: msgs,
      failed: failed.length ? failed : undefined,
      working: st === 'working' || undefined,
      // its turn is over, or ended while work it started still runs: a subagent, a shell
      background: (st === 'working' && !this.replying(key, ev)) || !!this.agents.background(key) || undefined,
      finished: (r?.finished && st === 'gone') || undefined,
      live: st === 'working' && act.open ? lineOf(act.open, true) : undefined,
      jobs: jobs.length ? jobs : undefined,
      controls: this.controls(key, kind),
    }
  }

  private replying(key: string, ev: LogEvent[]) {
    const turn = turnOf(ev, this.agents.sentLately(key))
    const s = this.shown.get(key)
    return replying(turn, !!s?.on && s.turn === turn.id, ev)
  }

  private controls(key: string, kind: string): ChatControls {
    // only Claude Code changes model and effort mid-session (/model, /effort); Codex and pi change them only through
    // their own pickers: their menu only adds files
    const c = kind === 'claude' ? modelsOf(kind) : undefined
    const r = this.agents.record(key)
    const on = findModel(kind, r?.model) ?? findModel(kind, c?.default) ?? c?.models[0]
    if (!c || !on) return { models: [], model: 0, efforts: [], effort: 0 }
    // the models it offers, and an older one it was started on (--model claude-opus-4-8) as well
    const models = c.models.some((m) => m.id === on.id) ? c.models : [...c.models, on]
    // the level it runs at: a level this model lacks shows as the nearest it has, as it goes in
    const effort = nearest(on.efforts, r?.effort) ?? on.effort ?? on.efforts[0]
    return { models: models.map((m) => m.label), model: models.indexOf(on), efforts: on.efforts, effort: Math.max(0, on.efforts.indexOf(effort ?? '')) }
  }

}

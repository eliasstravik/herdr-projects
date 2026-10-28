// How long this space's tasks really took, for a task's agent to guess its time left by: agents guess in a person's
// time (hours that take minutes). Numbers only; the guessing stays the agent's.
import { splitBrief } from './prompts.ts'

/** a task as tasks.json keeps it, as far as its time goes */
export interface Timed {
  id: string
  project: string
  title: string
  desc: string
  created: number
  /** when its agent first reported done */
  reportedAt?: number
  /** when it was merged or closed */
  doneAt?: number
  merged?: boolean
  closed?: boolean
}

/** the last finished tasks it counts, so the numbers follow how agents work now */
const LAST = 50
/** fewer than this, and it says nothing: no numbers beat made-up ones */
const FEW = 5

const STOP = new Set(
  'the and for with from into that this then than when what your you are not but all any can its was out off one our has have more less some only each them they their there here also just like make made show shows add adds use uses new now too'.split(
    ' ',
  ),
)

/** the words that say what a task is about: no short or common ones, a plural as its singular */
function words(t: { title: string; desc: string }) {
  const text = `${t.title} ${splitBrief(t.desc)[0].slice(0, 400)}`.toLowerCase()
  return new Set((text.match(/\p{L}[\p{L}\p{N}]+/gu) ?? []).filter((w) => w.length > 2 && !STOP.has(w)).map((w) => (w.length > 4 ? w.replace(/s$/, '') : w)))
}

/** minutes from start to done: when its agent reported done, else when it was merged or closed */
const took = (t: Timed) => ((t.reportedAt ?? t.doneAt)! - t.created) / 60000

/** 25m, 1h20m */
export function span(min: number) {
  const m = Math.max(1, Math.round(min))
  return m < 60 ? `${m}m` : `${Math.floor(m / 60)}h${m % 60 ? `${m % 60}m` : ''}`
}

const at = (xs: number[], q: number) => xs[Math.min(xs.length - 1, Math.floor(q * xs.length))]

/**
 * What this project's finished tasks took, start to done: the median, the middle half, 9 in 10, and the three that read
 * most like this one (the latest ones when none do). Nothing while it has finished too few.
 */
export function pastTimes(all: Timed[], t: Pick<Timed, 'id' | 'project' | 'title' | 'desc'>) {
  const done = all
    .filter((x) => x.id !== t.id && x.project === t.project && (x.merged || x.closed) && (x.reportedAt ?? x.doneAt) && took(x) > 0)
    .sort((a, b) => b.created - a.created)
    .slice(0, LAST)
  if (done.length < FEW) return ''
  const mins = done.map(took).sort((a, b) => a - b)
  const mine = words(t)
  const score = (x: Timed) => {
    const w = words(x)
    let n = 0
    for (const v of w) if (mine.has(v)) n++
    return n && n / Math.sqrt(w.size * mine.size)
  }
  const like = done
    .map((x, i) => ({ x, s: score(x), i }))
    .filter((l) => l.s)
    .sort((a, b) => b.s - a.s || a.i - b.i)
    .slice(0, 3)
    .map((l) => l.x)
  const which = like.length ? 'The ones that read most like this task' : 'The latest ones'
  const list = (like.length ? like : done.slice(0, 3)).map((x) => `${x.id} "${x.title}" ${span(took(x))}`).join(', ')
  return `How long this space's tasks really took, from start to done: the last ${done.length} took ${span(at(mins, 0.5))} (median), half of them ${span(at(mins, 0.25))} to ${span(at(mins, 0.75))}, 9 in 10 under ${span(at(mins, 0.9))}. ${which}: ${list}.`
}

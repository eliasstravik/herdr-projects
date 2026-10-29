// Plain rules the chat draws by, kept apart so tests can run them without a screen.
import type { Message } from '@shared/types.ts'

/** a day's line between messages: Today, Yesterday, a weekday this week, else its date (and year, if not this one) */
export function dayLabel(at: number, now = new Date()) {
  const d = new Date(at)
  const start = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime()
  const days = Math.round((start(now) - start(d)) / 864e5)
  if (days === 0) return 'Today'
  if (days === 1) return 'Yesterday'
  if (days > 1 && days < 7) return d.toLocaleDateString([], { weekday: 'long' })
  return d.toLocaleDateString([], { weekday: 'short', day: 'numeric', month: 'short', ...(d.getFullYear() !== now.getFullYear() ? { year: 'numeric' } : {}) })
}

/** a reply's words as they read, without its markdown: a link by its words, no marks around them */
const plain = (s: string) => s.replace(/!?\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/[*_`#>~]/g, '')

/** what a message can be found by: its words, a file's name, a card's title */
export const messageText = (m: Message) => [m.text && plain(m.text), m.file?.name, m.question?.title].filter(Boolean).join('\n')
const hay = (m: Message) => messageText(m).toLowerCase()

/** the messages with these words (search in a chat), oldest first; not one still waiting to go in */
export function findIn(msgs: Message[], q: string) {
  const w = q.trim().toLowerCase()
  return w ? msgs.filter((m) => !m.waiting && hay(m).includes(w)).map((m) => m.id) : []
}

// The chat's plain rules: day lines and search in a chat.
import { describe, expect, test } from 'bun:test'
import { dayLabel, findIn } from '../plugins/chat/days.ts'
import type { Message } from '../shared/types.ts'

describe('day lines', () => {
  const now = new Date(2026, 9, 25, 9, 30)
  test('today, yesterday, a weekday this week, else the date; the year only when it is not this one', () => {
    expect(dayLabel(new Date(2026, 9, 25, 0, 5).getTime(), now)).toBe('Today')
    expect(dayLabel(new Date(2026, 9, 24, 23, 59).getTime(), now)).toBe('Yesterday')
    expect(dayLabel(new Date(2026, 9, 21, 12).getTime(), now)).toBe(new Date(2026, 9, 21).toLocaleDateString([], { weekday: 'long' }))
    const old = dayLabel(new Date(2026, 8, 2).getTime(), now)
    expect(old).not.toContain('2026')
    expect(dayLabel(new Date(2025, 8, 2).getTime(), now)).toContain('2025')
  })
})

describe('search in a chat', () => {
  const msgs: Message[] = [
    { id: 'a', who: 'you', text: 'Is the Clock mocked?' },
    { id: 'b', who: 'agent', text: 'No.' },
    { id: 'c', who: 'you', file: { name: 'clock.png', path: '/x/clock.png' } },
    { id: 'd', who: 'you', text: 'clock again', waiting: true },
    { id: 'e', who: 'agent', question: { id: 'q', title: 'Fix the clock?', kind: 'Question', choices: [], to: 'x' } },
  ]
  test('words in any case, file names and card titles; nothing still waiting to go in', () => {
    expect(findIn(msgs, '  CLOCK ')).toEqual(['a', 'c', 'e'])
    expect(findIn(msgs, '')).toEqual([])
    expect(findIn(msgs, 'zebra')).toEqual([])
    // markdown aside: a link by its words, not its address; no ** in the way
    const md: Message[] = [{ id: 'm', who: 'agent', text: 'See **the** [docs](https://example.com/clockwork)' }]
    expect(findIn(md, 'see the docs')).toEqual(['m'])
    expect(findIn(md, 'clockwork')).toEqual([])
  })
})

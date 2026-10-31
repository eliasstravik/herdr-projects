import { expect, test } from 'bun:test'
import { holdNeeds, type HeldNeed } from '../src/needs.ts'
import type { Need } from '../shared/types.ts'

const need = (id: string, at = 1): Need => ({ id, chat: `${id}/Agent`, place: 'p', at })
const ids = (held: HeldNeed[]) => held.map((h) => `${h.need.id}${h.gone ? ' gone' : ''}`)

test('opens oldest first, as the server sent it', () => {
  expect(ids(holdNeeds([], [need('a', 1), need('b', 2)]))).toEqual(['a', 'b'])
})

test('a new need joins the end, even one that is older than what shows', () => {
  const held = holdNeeds([], [need('b', 5), need('c', 6)])
  expect(ids(holdNeeds(held, [need('a', 1), need('b', 5), need('c', 6), need('d', 7)]))).toEqual(['b', 'c', 'a', 'd'])
})

test('one that no longer waits keeps its place above an open one, so nothing slides into a tap', () => {
  const held = holdNeeds([], [need('a'), need('b'), need('c')])
  const after = holdNeeds(held, [need('b'), need('c')])
  expect(ids(after)).toEqual(['a gone', 'b', 'c'])
  // and it stays while a new one arrives below
  expect(ids(holdNeeds(after, [need('b'), need('c'), need('d')]))).toEqual(['a gone', 'b', 'c', 'd'])
})

test('with nothing open below, the kept places go: nothing can move', () => {
  let held = holdNeeds([], [need('a'), need('b')])
  held = holdNeeds(held, [need('b')])
  expect(ids(holdNeeds(held, []))).toEqual([])
  held = holdNeeds(holdNeeds([], [need('a'), need('b'), need('c')]), [need('a'), need('c')])
  expect(ids(holdNeeds(held, [need('a')]))).toEqual(['a'])
})

test('the same id in two chats is two needs, and one answered leaves the other', () => {
  const one: Need = { id: 'attention', chat: 'A/Agent', place: 'p', at: 1 }, two: Need = { ...one, chat: 'B/Agent' }
  expect(ids(holdNeeds(holdNeeds([], [one, two]), [one]))).toEqual(['attention'])
})

test('a snapshot that changes nothing changes nothing', () => {
  const held = holdNeeds(holdNeeds([], [need('a'), need('b')]), [need('b')])
  expect(holdNeeds(held, [need('b')])).toEqual(held)
})

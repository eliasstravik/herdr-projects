// What you send shows at once, then gives way to the message from the agent's log: no gap, no double.
import { describe, expect, test } from 'bun:test'
import { expire, KEEP_SENT, keyOf, partsOf, prune, reasonOf, SEND_TIMEOUT, settle, type Outgoing } from '../src/outbox.ts'
import type { Chat, Message } from '../shared/types.ts'

const T = 1_700_000_000_000
const out = (p: Partial<Outgoing> = {}): Outgoing => ({ cid: 'c1', chat: 'gtm', text: 'Ship it', files: [], at: T, state: 'sending', ...p })
const chat = (...messages: Message[]): Chat => ({ key: 'gtm', cli: 'Claude Code', messages })
const old: Message = { id: 'm0', who: 'you', text: 'Ship it', at: T - 60_000 }

describe('it gives way to the real message', () => {
  test('by its id, once the log has it', () => {
    const o = [out({ state: 'sent', serverAt: T })]
    expect(settle(o, chat(old))).toBe(o)
    expect(settle(o, chat(old, { id: 'm1', who: 'you', text: 'Ship it', at: T + 800, cid: 'c1' }))).toEqual([])
  })
  test('even one that seemed to fail: it got through after all', () => {
    expect(settle([out({ state: 'failed', error: 'x' })], chat({ id: 'm1', who: 'you', text: 'Ship it', cid: 'c1' }))).toEqual([])
  })
  test('another chat, or other pending ones, are left as they are', () => {
    const o = [out(), out({ cid: 'c2', chat: 'vm' })]
    expect(settle(o, chat({ id: 'm1', who: 'you', text: 'Ship it', cid: 'c1' }))).toEqual([o[1]])
  })
  test('waiting for the agent, it hides behind the waiting one, and shows again while it goes in', () => {
    const waiting: Message = { id: 'q1', who: 'you', text: 'Ship it', cid: 'c1', waiting: true, at: T }
    let o = settle([out({ state: 'sending' })], chat(waiting), T + 100)
    expect(o).toMatchObject([{ state: 'sent', waiting: true, queued: true }])
    expect(partsOf(o[0], chat(waiting))).toEqual([])
    // the queue let it go; the log hasn't read it yet
    o = settle(o, chat(), T + 5000)
    expect(o).toMatchObject([{ state: 'sent', waiting: undefined, queued: undefined, wentAt: T + 5000 }])
    expect(partsOf(o[0], chat()).map((m) => [keyOf(m), m.waiting])).toEqual([['c1', undefined]])
    expect(settle(o, chat({ id: 'm1', who: 'you', text: 'Ship it', cid: 'c1', at: T + 6000 }))).toEqual([])
  })
  test('without its id (an older machine): only the very same message, new since you sent it', () => {
    const o = [out({ state: 'sent', serverAt: T, prior: ['m0'] }), out({ cid: 'c2', text: 'And this', state: 'sent', serverAt: T, prior: ['m0'] })]
    const back = chat(old, { id: 'm1', who: 'you', text: 'Ship  it', at: T + 500 })
    expect(settle(o, back)).toEqual([o[1]])
    // an unrelated message of yours, just before or after: not it
    expect(settle(o, chat(old, { id: 'm2', who: 'you', text: 'Something else', at: T + 500 }, { id: 'm3', who: 'you', text: 'Ship it', at: T - 2000 }))).toBe(o)
    // the same words, already in the chat when you sent it: not it either
    const same: Message = { id: 'm0b', who: 'you', text: 'Ship it', at: T - 1000 }
    expect(settle([out({ state: 'sent', serverAt: T, prior: ['m0b'] })], chat(same))).toHaveLength(1)
    // its files must all be there too
    const withFile = out({ state: 'sent', serverAt: T, files: [{ name: 'a.png', path: '/in/a.png' }] })
    expect(settle([withFile], back)).toHaveLength(1)
    expect(settle([withFile], chat(...back.messages, { id: 'm1:0', who: 'you', file: { name: 'a.png', path: '/in/a.png' }, at: T + 500 }))).toEqual([])
    // still on its way: no server time to go by, so only its id counts
    expect(settle([out()], back)).toHaveLength(1)
    // an answer to a card isn't it
    expect(settle([o[0]], chat({ id: 'a', who: 'you', answered: 'Answered', text: 'Ship it', at: T + 500 }))).toHaveLength(1)
  })
})

describe('how it shows', () => {
  test('a bubble for the text and one per file, with the keys the real ones will have', () => {
    const o = out({ files: [{ name: 'a.png', path: '/in/a.png' }] })
    const parts = partsOf(o)
    expect(parts.map(keyOf)).toEqual(['c1', 'c1:/in/a.png'])
    expect(keyOf({ id: 'm1', who: 'you', file: { name: 'a.png', path: '/in/a.png' }, cid: 'c1' })).toBe('c1:/in/a.png')
    expect(keyOf({ id: 'm1', who: 'agent', text: 'Hi' })).toBe('m1')
    expect(partsOf(out({ state: 'sent', waiting: true, waitsFor: 'menu' }))[0]).toMatchObject({ waiting: true, waitsFor: 'menu' })
    expect(partsOf(out({ state: 'sent', waitsFor: 'menu' }))[0].waitsFor).toBeUndefined()
  })
  test('why it failed, in a few words', () => {
    expect(reasonOf(new TypeError('Failed to fetch'))).toBe("Can't reach Shepherdr.")
    expect(reasonOf(new DOMException('t', 'TimeoutError'))).toBe('No answer from Shepherdr.')
    expect(reasonOf(new Error('{"error":"herdr is gone"}'))).toBe('herdr is gone')
    expect(reasonOf(new Error(''))).toBe('Something went wrong.')
  })
})

describe('over time', () => {
  test("what didn't go through in time fails and keeps its words; nothing is thrown away", () => {
    const o = [
      out(),
      out({ cid: 'c2', state: 'sent', files: [{ name: 'a.png', path: '/in/a.png' }] }),
      out({ cid: 'c3', state: 'sent', waiting: true, queued: true }),
      out({ cid: 'c4', state: 'failed', error: 'x' }),
      out({ cid: 'c5', state: 'sent', waiting: true }),
    ]
    expect(expire(o, T + 1000)).toBe(o)
    // still on its way after its time (the screen reloaded meanwhile)
    expect(expire(o, T + SEND_TIMEOUT + 6000).map((x) => [x.cid, x.state])).toEqual([['c1', 'failed'], ['c2', 'sent'], ['c3', 'sent'], ['c4', 'failed'], ['c5', 'sent']])
    // in, but never showed in the chat: failed, with its files, and Retry sends it as new
    const later = expire(o, T + 6 * 60_000)
    expect(later[1]).toMatchObject({ state: 'failed', lost: true, files: [{ path: '/in/a.png' }], error: "It didn't show up in the chat." })
    // waiting on a menu the chat shows, it stays however long; waiting unseen (a machine out of reach) fails
    const hour = expire(o, T + 3_600_000)
    expect(hour.map((x) => [x.cid, x.state])).toEqual([['c1', 'failed'], ['c2', 'failed'], ['c3', 'sent'], ['c4', 'failed'], ['c5', 'failed']])
    expect(hour).toHaveLength(5)
  })
  test('in while the agent works, it may come back only at its next step: the wait counts from when it last worked', () => {
    const o = [out({ state: 'sent', serverAt: T })]
    const working = (on: boolean) => () => on
    // a long step: it keeps showing
    let cur = o
    for (let now = T; now < T + 20 * 60_000; now += 10_000) cur = expire(cur, now, working(true))
    expect(cur[0]).toMatchObject({ state: 'sent' })
    // noted now and then, not on every tick
    expect(expire(cur, cur[0].busyAt! + 10_000, working(true))).toBe(cur)
    // the agent stopped: two minutes more to show up, then it is not delivered
    expect(expire(cur, cur[0].busyAt! + KEEP_SENT - 10_000, working(false))[0].state).toBe('sent')
    expect(expire(cur, cur[0].busyAt! + KEEP_SENT + 10_000, working(false))[0]).toMatchObject({ state: 'failed', lost: true })
  })
  test('an agent that seems to work for good keeps it no more than half an hour', () => {
    let cur = [out({ state: 'sent', serverAt: T })]
    for (let now = T; now < T + 29 * 60_000; now += 10_000) cur = expire(cur, now, () => true)
    expect(cur[0].state).toBe('sent')
    expect(expire(cur, T + 31 * 60_000, () => true)[0]).toMatchObject({ state: 'failed', lost: true })
  })
  test('a screen loading drops ones older than a day, and ones for a chat that is gone', () => {
    const o = [out(), out({ cid: 'c2', at: T - 25 * 3_600_000 }), out({ cid: 'c3', chat: 'gone' })]
    expect(prune(o, T).map((x) => x.cid)).toEqual(['c1', 'c3'])
    expect(prune(o, T, (c) => c === 'gtm').map((x) => x.cid)).toEqual(['c1'])
    const kept = [out()]
    expect(prune(kept, T, () => true)).toBe(kept)
  })
  test('one the server says could not go in after waiting fails; one sent again meanwhile is left on its way', () => {
    const c: Chat = { ...chat(), failed: [{ cid: 'c1', error: "The agent didn't take it" }] }
    expect(settle([out({ state: 'sent', waiting: true, queued: true })], c)).toMatchObject([{ state: 'failed', error: "The agent didn't take it", waiting: undefined, queued: undefined }])
    expect(settle([out({ state: 'sending' })], c)).toMatchObject([{ state: 'sending' }])
  })
  test('one that failed but shows up after all goes, or waits again', () => {
    const lost = out({ state: 'failed', lost: true, error: 'x' })
    expect(settle([lost], chat({ id: 'q', who: 'you', text: 'Ship it', cid: 'c1', waiting: true }))).toMatchObject([{ state: 'sent', waiting: true, error: undefined, lost: undefined }])
    expect(settle([lost], chat({ id: 'm', who: 'you', text: 'Ship it', cid: 'c1' }))).toEqual([])
  })
})

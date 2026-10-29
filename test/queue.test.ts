// What a chat shows waiting in the CLI's own queue (Claude Code holds what you type while it works).
import { describe, expect, test } from 'bun:test'
import { inQueue } from '../server/live/chats.ts'
import type { LogEvent } from '../server/live/readers/types.ts'

const queued = (id: string, at: number, text: string): LogEvent => ({ kind: 'queued', id, at, text })
const ids = (ev: LogEvent[]) => inQueue(ev).map((q) => q.id)

describe('inQueue', () => {
  test('one put in stays until it is taken out', () => {
    expect(ids([queued('a', 1, 'hi'), queued('b', 2, 'there')])).toEqual(['a', 'b'])
    expect(ids([queued('a', 1, 'hi'), queued('b', 2, 'there'), { kind: 'unqueued', id: 'a', at: 3 }])).toEqual(['b'])
  })

  test('its user line, a turn end or a new session takes it out, so none waits for good', () => {
    expect(ids([queued('a', 1, 'hi  there'), { kind: 'user', id: 'u', at: 2, text: 'hi there' }])).toEqual([])
    expect(ids([queued('a', 1, 'hi'), { kind: 'turn-end', id: 't', at: 2 }])).toEqual([])
    expect(ids([queued('a', 1, 'hi'), { kind: 'session', id: 's', at: 2, sessionId: 'x' }])).toEqual([])
    // one put in after the turn ended still waits
    expect(ids([{ kind: 'turn-end', id: 't', at: 1 }, queued('a', 2, 'hi')])).toEqual(['a'])
  })
})

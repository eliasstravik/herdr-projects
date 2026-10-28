// The typing dots follow the agent's real status: on while it works on a reply, off when it stops, waits on the human
// or only its background work runs.
import { describe, expect, test } from 'bun:test'
import { typing } from '../plugins/chat/typing.ts'
import { replying, turnOf } from '../server/live/chats.ts'
import type { LogEvent } from '../server/live/readers/types.ts'
import type { Chat, Message, Question } from '../shared/types.ts'

const chat = (working: boolean | undefined, messages: Message[] = [], more: Partial<Chat> = {}): Chat => ({ key: 'c', cli: 'Claude Code', messages, working, ...more })
const q = (sent?: number): Question => ({ id: 'q', title: 'Rerun CI?', kind: 'Question', choices: [{ label: 'Yes' }, { label: 'No' }], to: 'x', sent })

describe('typing dots', () => {
  test('on while the agent works, whether you wrote or it started on its own', () => {
    expect(typing(chat(true, [{ id: '1', who: 'you', text: 'Go' }]))).toBe(true)
    expect(typing(chat(true, [{ id: '1', who: 'agent', text: 'Done.' }]))).toBe(true)
    expect(typing(chat(true))).toBe(true)
  })
  test('off once it is idle, blocked or finished', () => {
    expect(typing(chat(undefined, [{ id: '1', who: 'you', text: 'Go' }]))).toBe(false)
    expect(typing(chat(true, [], { finished: true }))).toBe(false)
  })
  test('off while its last card waits on the human; on again once answered', () => {
    expect(typing(chat(true, [{ id: '1', who: 'agent', question: q() }]))).toBe(false)
    expect(typing(chat(true, [{ id: '1', who: 'agent', question: q(0) }]))).toBe(true)
    // what you write while its menu is up waits under the card: still waiting on you
    expect(typing(chat(true, [{ id: '1', who: 'agent', question: q() }, { id: '2', who: 'you', text: 'hm', waiting: true }]))).toBe(false)
  })
  test('off while no message you will see is on its way', () => {
    const done = [{ id: '1', who: 'agent' as const, text: 'Research is running.' }]
    expect(typing(chat(true, done, { background: true }))).toBe(false)
  })
})

describe('is a message you will see on its way (the log, and the screen in its own turns)', () => {
  type K = 'turn-start' | 'turn-end' | 'you' | 'news' | 'react' | 'agent' | 'noted'
  const TEXT: Record<string, string> = {
    you: 'Research four options',
    news: '<shepherdr><news task="SHE-9" event="ready">SHE-9 is ready.</news></shepherdr>',
    react: '<reaction from="Elias" emoji="👍" to="Done."/>',
    agent: 'SHE-10 can start now.',
    noted: '<no-reply>SHE-9 is ready, nothing to add</no-reply>',
  }
  const ev = (...kinds: K[]): LogEvent[] =>
    kinds.map((k, i) =>
      k === 'turn-start' || k === 'turn-end'
        ? { kind: k, id: `${i}`, at: i * 10 }
        : { kind: k === 'agent' || k === 'noted' ? 'agent' : 'user', id: `${i}`, at: i * 10, text: TEXT[k] },
    )
  const on = (e: LogEvent[], shown = false, sent: { at: number }[] = []) => replying(turnOf(e, sent), shown, e)

  test('a turn your message started: dots at once, until its reply ends the turn', () => {
    expect(on(ev('turn-start', 'you'))).toBe(true)
    expect(on(ev('turn-start', 'you', 'agent'))).toBe(true)
    expect(on(ev('turn-start', 'you', 'agent', 'turn-end'))).toBe(false)
    // you wrote from a screen and the CLI hasn't written it down yet; what you sent before the last turn ended is done
    expect(on(ev('turn-start', 'you', 'turn-end'), false, [{ at: 25 }])).toBe(true)
    expect(on(ev('turn-start', 'you', 'turn-end'), false, [{ at: 5 }])).toBe(false)
    // you wrote while it was on news: it answers you now
    expect(on(ev('turn-start', 'news', 'you'))).toBe(true)
  })

  test('a turn it started itself (news, a reaction, background work waking it): no dots while it thinks or runs tools', () => {
    expect(on(ev('turn-start', 'you', 'turn-end', 'turn-start', 'news'))).toBe(false)
    expect(on(ev('turn-start', 'you', 'turn-end', 'turn-start', 'react'))).toBe(false)
    expect(on(ev('turn-start', 'you', 'turn-end', 'turn-start'))).toBe(false)
    expect(turnOf(ev('turn-start', 'news'), [])).toEqual({ kind: 'own', id: '0' })
  })

  test('…dots once its reply shows on screen or lands, never for a hidden <no-reply>, gone when the turn ends', () => {
    expect(on(ev('turn-start', 'news'), true)).toBe(true)
    expect(on(ev('turn-start', 'news', 'agent'))).toBe(true)
    expect(on(ev('turn-start', 'news', 'noted'))).toBe(false)
    expect(on(ev('turn-start', 'news', 'agent', 'turn-end'), true)).toBe(false)
  })

  test('background work alone never shows dots; a log without turn marks leaves it to herdr', () => {
    expect(turnOf(ev('turn-start', 'you', 'agent', 'turn-end'), [])).toEqual({ kind: 'over' })
    expect(on(ev('turn-start', 'you', 'agent', 'turn-end'), true)).toBe(false)
    expect(on(ev('you', 'agent'))).toBe(true)
    expect(on([])).toBe(true)
  })
})

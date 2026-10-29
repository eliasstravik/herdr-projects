// Isolated voice tools: no agents, servers, user files, microphone or provider requests.
import { describe, expect, test } from 'bun:test'
import { mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import type { Action, Chat, Place, Snapshot, Task } from '../shared/types.ts'
import { Emitter } from '../server/source.ts'
process.env.SHEPHERDR_HOME = mkdtempSync(join(tmpdir(), 'voice-routing-'))
const { LiveSource } = await import('../server/live/source.ts')
const { Machines } = await import('../server/live/machines.ts')
const { HOST } = await import('../server/live/places.ts')

const place = (machine: string, name = 'project') => ({ id: `p:${machine}:${name}`, chat: `p:${machine}:${name}`, name, machine, status: 'idle' }) as Place
const history = (key: string, texts = ['old']): Chat => ({ key, cli: 'codex', working: true, messages: texts.map((text, i) => ({ id: String(i), who: 'agent', text })) })
function fixture() {
  const local = place(HOST)
  const remote = place('remote')
  const task = { id: 'ABC-1', name: 'Remote job', place: remote.id, state: 'working', agent: { chat: 'ABC-1/Agent' } } as Task
  const peer = { rec: { machine: 'remote', url: 'http://unused' }, reachable: true, snap: { places: [remote], tasks: [task] }, chats: new Map(), watching: { chats: [] as string[], term: null } }
  const peers = Object.assign(new Emitter(), {
    list: () => [peer],
    owner: (key: string) => key === remote.chat || key === 'ABC-1/Agent' ? peer : undefined,
    act: async (_p: unknown, a: Action) => { sent.push(a); return result },
    fetch: async (_p: unknown, path: string) => Response.json({ chat: history(new URL(path, 'http://fake').searchParams.get('chat')!), status: 'working' }),
    subscribeChat: (_p: unknown, key: string, fn: (c: Chat) => void) => {
      watched.add(key)
      const off = events.on('chat', (k, c) => { if (k === key) fn(c) })
      return () => { watched.delete(key); off() }
    },
  })
  const events = new Emitter<{ chat: [string, Chat] }>()
  const sent: Action[] = []
  let result: object = { ok: true }
  const watched = new Set<string>()
  // Skip LiveSource's constructor: none of its timers, disk writes or real machine links runs.
  const src = Object.assign(Object.create(LiveSource.prototype), {
    peers, user: 'Human', engine: { tasks: [] },
    placeList: () => [local, remote],
    snapshot: () => ({ places: [local, remote], tasks: [task] }) as Snapshot,
    chats: { chat: (key: string) => history(key), lastReply: () => 'old' },
    agents: { status: () => 'idle' }, replyFns: new Map(), lastAgentMsg: new Map(),
    actLocal: async (a: Action) => { sent.push(a); return result },
  }) as InstanceType<typeof LiveSource>
  return { src, peer, sent, events, watched, local, remote, setResult: (r: object) => { result = r } }
}

describe('voice routes', () => {
  test('remote coordinator and task sends reach exactly their owner with ordinary send actions', async () => {
    const f = fixture()
    await f.src.voiceTools(f.remote.chat).send('hello')
    await f.src.voiceTools('ABC-1/Agent').send('task hello')
    expect(f.sent).toEqual([{ type: 'send', chat: f.remote.chat, text: 'hello' }, { type: 'send', chat: 'ABC-1/Agent', text: 'task hello' }])
  })
  test('read uses owning chat history and state; overview includes remote places and tasks', async () => {
    const f = fixture()
    for (const key of [f.remote.chat, 'ABC-1/Agent']) {
      const tools = f.src.voiceTools(key)
      expect(await tools.read()).toContain('working')
      expect(await tools.read()).toContain('old')
      expect(await tools.overview()).toContain('remote:ABC-1')
      expect(await tools.overview()).toContain(f.remote.id)
    }
    f.peer.reachable = false
    await expect(f.src.voiceTools(f.remote.chat).read()).rejects.toThrow('unavailable')
  })
  test('names are unambiguous, machine qualifiers work, unknown names never fall back', async () => {
    const f = fixture()
    const tools = f.src.voiceTools(f.local.chat)
    await expect(tools.send('x', 'project')).rejects.toThrow('Several')
    await expect(tools.send('x', 'missing')).rejects.toThrow('No such')
    await tools.send('x', 'remote:project')
    await tools.send('y', 'remote:ABC-1')
    expect(f.sent.map((a) => 'chat' in a && a.chat)).toEqual([f.remote.chat, 'ABC-1/Agent'])
  })
  test('duplicate task IDs require explicit ownership, including call startup', async () => {
    const f = fixture()
    f.src.engine.tasks.push({ id: 'ABC-1' } as never)
    expect(() => f.src.voiceTools('ABC-1/Agent')).toThrow('Several')
    await f.src.voiceTools('remote:ABC-1/Agent').send('remote only')
    expect(f.sent).toEqual([{ type: 'send', chat: 'ABC-1/Agent', text: 'remote only' }])
  })
  test('reviewer chat retains role and owning machine', async () => {
    const f = fixture()
    await f.src.voiceTools('remote:ABC-1/Reviewer').send('review')
    expect(f.sent).toEqual([{ type: 'send', chat: 'ABC-1/Reviewer', text: 'review' }])
  })
  test('offline queue and refusal are reported honestly', async () => {
    const f = fixture()
    f.setResult({ waiting: true })
    expect(await f.src.voiceTools(f.remote.chat).send('x')).toContain('once the agent has started')
    f.peer.reachable = false
    expect(await f.src.voiceTools(f.remote.chat).send('x')).toContain('has not reached')
    f.setResult({ ok: false, error: 'refused' })
    await expect(f.src.voiceTools(f.remote.chat).send('x')).rejects.toThrow('refused')
  })
  test('only new replies from the call chat speak once, and disposal removes watch', async () => {
    const f = fixture()
    const spoken: string[] = []
    const off = f.src.voiceTools(f.remote.chat).onReply((s) => spoken.push(s))
    // The initial owner view is the baseline, never an unsolicited readout of history.
    await Bun.sleep(0)
    f.events.emit('chat', f.local.chat, history(f.local.chat, ['wrong']))
    f.events.emit('chat', f.remote.chat, history(f.remote.chat, ['old', 'new']))
    f.events.emit('chat', f.remote.chat, history(f.remote.chat, ['old', 'new']))
    expect(spoken).toEqual(['new'])
    f.events.emit('chat', f.remote.chat, history(f.remote.chat, ['old', 'new complete']))
    expect(spoken).toEqual(['new', 'new complete'])
    off()
    expect(f.watched.size).toBe(0)
    f.events.emit('chat', f.remote.chat, history(f.remote.chat, ['old', 'new', 'late']))
    expect(spoken).toEqual(['new', 'new complete'])
  })
  test('local send/read/subscription retain their path', async () => {
    const f = fixture()
    const tools = f.src.voiceTools(f.local.chat)
    expect(await tools.send('local')).toBe('Sent.')
    expect(await tools.read()).toContain('idle')
    const off = tools.onReply(() => {})
    expect(f.watched.size).toBe(0)
    off()
    expect(f.sent).toHaveLength(1)
  })
  test('peer watch pins survive screen changes, remain machine scoped and are released', () => {
    const f = fixture()
    const machines = Object.assign(new Emitter(), { links: new Map([['remote', f.peer]]), voiceWatches: new Map(), screenWatch: { chats: [], term: null }, owner: () => f.peer }) as unknown as InstanceType<typeof Machines>
    const seen: string[] = []
    const off = Machines.prototype.subscribeChat.call(machines, f.peer as never, 'ABC-1/Agent', (c) => seen.push(c.key))
    machines.watch([], null)
    expect(f.peer.watching.chats).toEqual(['ABC-1/Agent'])
    machines.emit('peerChat', 'other', 'ABC-1/Agent', history('ABC-1/Agent'))
    machines.emit('peerChat', 'remote', 'ABC-1/Agent', history('ABC-1/Agent'))
    expect(seen).toEqual(['ABC-1/Agent'])
    off()
    off()
    expect(f.peer.watching.chats).toEqual([])
  })
})


test('provider close and explicit call end both release the reply subscription once', async () => {
  const { startVoice } = await import('../server/live/voice.ts')
  const nativeFetch = globalThis.fetch
  const nativeSocket = globalThis.WebSocket
  const previousKey = process.env.ELEVENLABS_API_KEY
  process.env.ELEVENLABS_API_KEY = 'isolated-fake-key'
  writeFileSync(join(process.env.SHEPHERDR_HOME!, 'voice.json'), JSON.stringify({ agent: 'fake' }))
  let socket: FakeSocket
  class FakeSocket {
    static OPEN = 1
    readyState = 1
    onopen?: () => void
    onclose?: () => void
    onmessage?: (e: { data: string }) => void
    constructor() { socket = this }
    send() {}
    close() { this.onclose?.() }
  }
  try {
    globalThis.fetch = (async () => Response.json({ signed_url: 'ws://fake' })) as unknown as typeof fetch
    globalThis.WebSocket = FakeSocket as unknown as typeof WebSocket
    for (const providerEnds of [false, true]) {
      let unsubscriptions = 0
      const call = startVoice({ name: 'remote:test', send: async () => '', read: async () => '', overview: async () => '', onReply: () => () => { unsubscriptions++ } }, { send() {}, close() {} })
      await Bun.sleep(0)
      socket!.onopen?.()
      if (providerEnds) socket!.onclose?.()
      call.end()
      call.end()
      expect(unsubscriptions).toBe(1)
    }
  } finally {
    globalThis.fetch = nativeFetch
    globalThis.WebSocket = nativeSocket
    if (previousKey === undefined) delete process.env.ELEVENLABS_API_KEY
    else process.env.ELEVENLABS_API_KEY = previousKey
  }
})

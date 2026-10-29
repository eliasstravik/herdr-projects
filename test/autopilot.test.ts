// Isolated fixtures only. No real provider, coordinator, login, settings or Herdr process is used.
import { afterEach, describe, expect, spyOn, test } from 'bun:test'
import { mkdtempSync, mkdirSync, writeFileSync, utimesSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
process.env.SHEPHERDR_HOME = mkdtempSync(join(tmpdir(), 'shep-pilot-test-'))
process.env.HERDR_SOCKET_PATH = join(process.env.SHEPHERDR_HOME, 'absent.sock')
const { PilotRuns, AUTOPILOT_KINDS, hostList, pilotCommand, pilotProblem, reserveTraffic } = await import('../server/live/autopilot.ts')
const { sandboxArgs, publicAddress, providerConfig, capturedTranscript } = await import('../server/live/autopilot-sandbox.ts')
const { Settings } = await import('../server/live/settings.ts')
const { herdr } = await import('../server/live/herdr.ts')
const { Agents, isolatedReady, isolatedShell } = await import('../server/live/agents.ts')
const { LiveSource } = await import('../server/live/source.ts')
const { Heartbeats } = await import('../server/live/heartbeat.ts')
const { placeId } = await import('../server/live/places.ts')
const { PilotUsage } = await import('../server/live/autopilot-usage.ts')
const { HOME } = await import('../server/store.ts')
import type { AgentRecord, Spec } from '../server/live/agents.ts'
import type { PilotConfig, PilotRun } from '../server/live/autopilot.ts'

const config = (kind = 'codex'): PilotConfig => ({ harness: { kind }, hosts: ['api.openai.com'], secrets: [], approvals: ['merge'], parallel: 2 })
const cleanup: (() => void)[] = []
afterEach(() => { for (const f of cleanup.splice(0)) f() })

function world(kind = 'codex') {
  const key = placeId.space('fixture')
  const path = mkdtempSync(join(tmpdir(), 'shep-pilot-repo-'))
  const settings = new Settings({ everywhere: { 'coordinator.agent': kind, 'autopilot.agent': 'inherit' }, machine: {}, spaces: {} })
  const pilots = new PilotRuns(); pilots.runs = []
  const records: Record<string, AgentRecord> = {}
  const events: string[] = [], posted: unknown[] = [], specs: Spec[] = []
  let mode = 'regular', fail = false
  const src = Object.create(LiveSource.prototype) as InstanceType<typeof LiveSource>
  const agents = {
    records, record: (k: string) => records[k], on: () => {}, status: () => 'idle', reachable: true,
    pauseDelivery: (_k: string, on: boolean) => events.push(on ? 'hold' : 'release'),
    stopTurn: async () => { events.push('interrupt') },
    finish: async (k: string) => { events.push(`finish:${k}`); if (records[k]) records[k].finished = true },
    restart: async (spec: Spec, before?: () => Promise<string | undefined>) => {
      await before?.(); events.push(`launch:${spec.kind}`); specs.push(spec)
      if (fail) throw new Error('fixture startup failed')
      records[spec.key] = { ...spec, logs: records[spec.key]?.logs ?? [], started: Date.now() }
      return null
    },
    ensure: async () => { throw new Error('unexpected automatic launch') },
    send: async () => ({ waiting: false }), waiting: () => [], background: () => undefined,
  }
  Object.assign(src, {
    settings, pilots, heartbeats: new Heartbeats(), agents,
    pilotSandboxes: new Map(), pilotUsage: new PilotUsage(), timers: [],
    engine: { installed: () => true, tasks: [], task: () => undefined },
    places: {
      coordinator: (k: string) => k === key ? { name: 'fixture-coordinator', label: 'fixture', cwd: path, path, workspace: 'fixture' } : undefined,
      space: (k: string) => k === key ? { name: 'fixture', path } : undefined,
      mode: () => mode, setMode: (_k: string, m: string) => { mode = m },
    },
    chats: { chat: () => ({ messages: [{ id: 'human', who: 'you', text: 'Keep improving the game', at: 1 }] }), post: (_k: string, card: unknown) => posted.push(card) },
    pilotRuntime: { available: async () => '/fake/bwrap' },
    emit: () => {},
  })
  const internals = src as unknown as {
    configureAutopilot(): void
    switchPilot(key: string, mode: 'regular' | 'autopilot', keep?: boolean, replace?: boolean): Promise<void>
    coordinator(key: string): Promise<unknown>
    monitorPilots(): Promise<void>
    pilotCli(r: PilotRun, args: string[], ctx: { chat: string; cwd: string }): Promise<{ code: number; out: string }>
    timers: ReturnType<typeof setInterval>[]
  }
  internals.configureAutopilot()
  cleanup.push(() => internals.timers.forEach(clearInterval))
  return { src, internals, key, path, pilots, records, specs, events, posted, mode: () => mode, fail: () => { fail = true } }
}

describe('capabilities and exact launch settings', () => {
  test.each(['claude', 'codex', 'pi'])('%s launch replaces a profile-altered PATH before starting an agent', async (kind) => {
    const commands: string[] = [], wrapper = `/trusted/launcher/${kind}`
    let polls = 0
    const io = {
      idle: async () => true,
      text: async (_pane: string, text: string) => { commands.push(text) },
      keys: async () => undefined,
      info: async () => { polls++; return { shell_pid: 123 } },
      read: (file: string) => file.endsWith('/cmdline') ? '/bin/sh\0-i\0' : polls < 2 ? 'PATH=/profile/host-binaries\0' : 'PATH=/trusted/launcher:/usr/local/bin:/usr/bin:/bin\0ENV=/dev/null\0',
      pause: async () => {},
    }
    await isolatedShell('pane', wrapper, io)
    expect(polls).toBe(2)
    expect(commands).toEqual(["exec /usr/bin/env PATH='/trusted/launcher:/usr/local/bin:/usr/bin:/bin' ENV=/dev/null /bin/sh -i"])
    await expect(isolatedShell('pane', wrapper, { ...io, read: () => 'unverified' })).rejects.toThrow('No agent was started')
  })
  test.each(['claude', 'codex', 'pi'])('%s only reports startup once that exact adapter accepts input', async (kind) => {
    const seen = [undefined, { agent: kind, interactive_ready: false }, { agent: kind, interactive_ready: true }]
    let polls = 0
    await isolatedReady(kind, async () => { polls++; return seen.shift() }, async () => {}, 3)
    expect(polls).toBe(3)
    await expect(isolatedReady(kind, async () => ({ agent: 'unrelated', interactive_ready: true }), async () => {}, 2)).rejects.toThrow('not started')
    await expect(isolatedReady(kind, async () => undefined, async () => {}, 2)).rejects.toThrow('not started')
  })
  test.each(['claude', 'codex', 'pi'])('%s has a chat adapter; other Herdr CLIs fail closed', (kind) => {
    expect(AUTOPILOT_KINDS).toContain(kind)
    expect(pilotProblem(kind)).toBeUndefined()
    expect(pilotProblem('cursor')).toContain('chat')
  })
  test.each(['claude', 'codex'])('%s inherits normal defaults and accepts explicit overrides', (kind) => {
    const s = new Settings({ everywhere: { 'coordinator.agent': kind, 'coordinator.effort': 'high' }, machine: {}, spaces: {} })
    expect(s.pilot().harness).toMatchObject({ kind, effort: 'high' })
    expect(s.set('autopilot.agent', kind === 'claude' ? 'Codex' : 'Claude Code').ok).toBe(true)
    expect(s.set('autopilot.effort', 'low').ok).toBe(true)
    expect(s.pilot().harness.effort).toBe('low')
    expect(s.set('autopilot.agent', 'Cursor').ok).toBe(false)
  })
  test.each(['claude', 'codex'])('%s fresh launch resets old explicit model/effort to CLI defaults', async (kind) => {
    const a = new Agents(), cwd = mkdtempSync(join(tmpdir(), 'pilot-defaults-'))
    a.records['defaults'] = { key: 'defaults', name: 'defaults', kind, cwd, workspace: 'fixture', model: 'old-model', effort: 'high', logs: [], started: 1 }
    const start = spyOn(herdr, 'start').mockResolvedValue(undefined as never)
    const timer = spyOn(globalThis, 'setTimeout').mockImplementation((() => 0) as never)
    Object.assign(a, { refresh: async () => undefined, bindRoute: async () => undefined, queue: () => {} })
    try {
      await a.ensure({ key: 'defaults', name: 'defaults', kind, cwd, workspace: 'fixture', fresh: true, pane: async () => 'fake-pane' })
      expect(a.record('defaults')?.model).toBe('')
      expect(a.record('defaults')?.effort).toBe('')
      expect(start.mock.calls[0][3]).not.toContain('old-model')
    } finally { start.mockRestore(); timer.mockRestore(); a.stop() }
  })
  test.each(['claude', 'codex'])('%s startup flags apply model/effort within shared OS isolation', (kind) => {
    const a = new Agents(); cleanup.push(() => a.stop())
    const r = { key: 'p:fixture', name: 'fixture', kind, cwd: '/project', workspace: 'fixture', logs: [], started: 1, model: kind === 'claude' ? 'sonnet' : 'gpt-6-sol', effort: 'high', isolation: { run: 'r', home: mkdtempSync(join(tmpdir(), 'pilot-hooks-')), control: mkdtempSync(join(tmpdir(), 'pilot-control-')), path: '/wrapper' } } as AgentRecord
    const flags = a.args(r, false)
    expect(flags).toContain(kind === 'claude' ? '--model' : '-m')
    expect(flags.join(' ')).toContain('high')
    if (kind === 'claude') { expect(flags).toContain('--dangerously-skip-permissions'); expect(flags).not.toContain('--settings') }
    else expect(flags.join(' ')).toContain('http://127.0.0.1:18765')
  })
})

describe('fresh replacement, failures and worker ownership', () => {
  test.each(['send', 'deliver'])('%s rechecks ownership after delayed input preflight', async (path) => {
    const a = new Agents(), key = 'preflight', r: AgentRecord = { key, name: key, kind: 'codex', cwd: '/fixture', workspace: 'fixture', launch: 'old', logs: [], started: 1 }
    a.records[key] = r
    let release!: () => void
    a.beforeInput = () => new Promise<void>((resolve) => { release = resolve })
    a.acceptInput = (item) => !item.generation || item.generation === r.launch
    const prompt = spyOn(herdr, 'prompt').mockResolvedValue(undefined as never)
    try {
      if (path === 'deliver') (a as unknown as { outbox: unknown[] }).outbox = [{ key, id: 'old', launch: 'old', text: 'old startup', at: 1 }]
      const pending = path === 'send' ? a.send(key, 'old wake', { generation: 'old' }) : a.deliver(key)
      await Promise.resolve()
      r.launch = 'new'; release(); await pending
      expect(prompt).not.toHaveBeenCalled()
      expect(a.waiting(key)).toHaveLength(0)
    } finally { prompt.mockRestore(); a.stop() }
  })
  test.each(['claude', 'codex'])('%s main chat excludes newer helpers and headless reviewers', (kind) => {
    const a = new Agents(), captures = mkdtempSync(join(tmpdir(), 'pilot-main-captures-'))
    const id = '00000000-0000-4000-8000-000000000001'
    const main = capturedTranscript(captures, kind, `.config/${id}.jsonl`)
    const helper = capturedTranscript(captures, kind, '.config/subagents/child.jsonl')
    const reviewer = capturedTranscript(captures, kind, '.config/reviewer.jsonl')
    for (const f of [main, helper, reviewer]) mkdirSync(join(f, '..'), { recursive: true })
    const meta = (source: unknown) => JSON.stringify({ type: 'session_meta', payload: { cwd: '/fixture', source } }) + '\n'
    writeFileSync(main, kind === 'codex' ? meta('cli') : '{}\n')
    writeFileSync(helper, kind === 'codex' ? meta({ subagent: {} }) : '{}\n')
    writeFileSync(reviewer, kind === 'codex' ? meta('exec') : '{}\n')
    for (const f of [helper, reviewer]) utimesSync(f, new Date(), new Date(Date.now() + 1000))
    a.records.main = { key: 'main', name: 'main', kind, sessionId: kind === 'claude' ? id : undefined, cwd: '/fixture', workspace: 'fixture', logs: [], started: Date.now(), isolation: { run: 'r', home: '/absent', captures, control: '/absent', path: '/absent' } }
    try { a.discoverLog('main'); expect(a.record('main')!.logs).toEqual([main]) }
    finally { a.stop() }
  })
  test('old startup instructions are discarded on handover while a queued human message survives', async () => {
    const a = new Agents(), key = 'handover'
    const state = a as unknown as { outbox: { key: string; id: string; text: string; at: number; launch?: string; cid?: string }[]; live: Map<string, unknown> }
    a.records[key] = { key, name: key, kind: 'codex', cwd: '/fixture', workspace: 'fixture', launch: 'new', logs: [], started: 1 }
    state.outbox = [
      { key, id: 'old', text: '<shepherdr>act on old Heartbeat</shepherdr>', at: 1, launch: 'old' },
      { key, id: 'new', text: 'regular startup', at: 2, launch: 'new' },
      { key, id: 'human', cid: 'human', text: 'my next message', at: 3 },
    ]
    Object.assign(a, { refresh: async () => {}, heldBy: () => undefined })
    const prompt = spyOn(herdr, 'prompt').mockResolvedValue(undefined as never)
    const timer = spyOn(globalThis, 'setTimeout').mockImplementation(((fn: () => void) => { queueMicrotask(fn); return 0 }) as never)
    try {
      await a.deliver(key)
      expect(prompt.mock.calls.map((call) => call[1])).toEqual(['regular startup', 'my next message'])
      expect(a.waiting(key)).toHaveLength(0)
    } finally { prompt.mockRestore(); timer.mockRestore(); a.stop() }
  })
  test.each(['claude', 'codex'])('%s ON/OFF replace one owner in the same chat', async (kind) => {
    const w = world(kind)
    await w.internals.switchPilot(w.key, 'autopilot')
    expect(w.specs[0]).toMatchObject({ key: w.key, kind, fresh: true })
    expect(w.mode()).toBe('autopilot')
    expect(w.pilots.current(w.key)?.state).toBe('on')
    expect(w.events[0]).toBe('hold')
    expect(w.events.at(-1)).toBe('release')
    await w.internals.switchPilot(w.key, 'regular')
    expect(w.specs.at(-1)).toMatchObject({ key: w.key, fresh: true })
    expect(w.mode()).toBe('regular')
    expect(w.posted).toHaveLength(2)
  })
  test('failed startup stays stopped, retains chat, and never falls back', async () => {
    const w = world(); w.fail()
    await expect(w.internals.switchPilot(w.key, 'autopilot')).rejects.toThrow('fixture startup failed')
    expect(w.mode()).toBe('autopilot')
    expect(w.pilots.current(w.key)?.state).toBe('failed')
    expect(w.specs).toHaveLength(1)
    expect(() => w.internals.coordinator(w.key)).toThrow('Autopilot did not start')
  })
  test('OFF closes only run-owned workers; explicit keep preserves them', async () => {
    for (const keep of [false, true]) {
      const w = world(); await w.internals.switchPilot(w.key, 'autopilot')
      const r = w.pilots.current(w.key)!
      w.records[w.key].isolation = { run: r.id, home: '/fixture', control: '/absent', path: '/wrapper' }
      Object.assign(w.src.engine, { tasks: [{ id: 'OWN-1', pilot: r.id }, { id: 'OTHER-1' }] })
      await w.internals.switchPilot(w.key, 'regular', keep)
      expect(w.events.includes('finish:OWN-1/Agent')).toBe(!keep)
      expect(w.events).not.toContain('finish:OTHER-1/Agent')
    }
  })
  test('OFF closes owned Reviewer records, including ones whose task already ended', async () => {
    const w = world(); await w.internals.switchPilot(w.key, 'autopilot')
    const r = w.pilots.current(w.key)!
    w.records['OWN-1/Reviewer'] = { key: 'OWN-1/Reviewer', name: 'reviewer', kind: 'codex', cwd: '/fixture', workspace: 'fixture', logs: [], started: 1, isolation: { run: r.id, home: '/fixture', control: '/absent', path: '/wrapper' } }
    await w.internals.switchPilot(w.key, 'regular')
    expect(w.events).toContain('finish:OWN-1/Reviewer')
  })
  test('secret names outside the profile never reach the vault', async () => {
    const w = world(); await w.internals.switchPilot(w.key, 'autopilot')
    const result = await w.internals.pilotCli(w.pilots.current(w.key)!, ['secret', 'run', 'OUTSIDE'], { chat: w.key, cwd: w.path })
    expect(result.code).toBe(1)
    expect(result.out).toContain('outside this permission profile')
  })
  test('incoming messages wait during transition and old generations are rejected', async () => {
    const w = world()
    let release!: () => void
    w.src.pilotRuntime.available = () => new Promise((resolve) => { release = () => resolve('/fake/bwrap') })
    const first = w.internals.switchPilot(w.key, 'autopilot')
    await Promise.resolve()
    await expect(w.internals.switchPilot(w.key, 'autopilot')).rejects.toThrow('changing')
    expect(w.events).toEqual(['hold'])
    release(); await first
    const accept = w.src.agents.acceptInput!
    expect(accept({ generation: 'old', key: w.key } as never)).toBe(false)
    expect(accept({ key: w.key } as never)).toBe(true)
    expect(accept({ generation: w.pilots.current(w.key)!.id, key: w.key } as never)).toBe(true)
  })
})

describe('trusted broker, limits and actual namespace configuration', () => {
  test('shared runtime, reported allowance and parallel tasks gate outside prompts', () => {
    const p = new PilotRuns(); p.runs = []
    const r = p.begin('fixture', { ...config(), runtime: 1000, tokens: 10 }, 100)
    r.state = 'on'; p.claim(r, 'OWN-1')
    expect(() => p.check(r, 1, 101)).not.toThrow()
    expect(() => p.check(r, 2, 101)).toThrow('parallel')
    expect(() => p.check(r, 0, 1100)).toThrow('runtime')
    r.used = 10
    expect(() => p.check(r, 0, 101)).toThrow('reported token')
    expect(() => pilotCommand(r, 'fixture', ['task', 'tell', 'OTHER-1', 'stop'])).toThrow('human')
    expect(() => pilotCommand(r, 'fixture', ['merge', 'OWN-1'])).toThrow('human')
    expect(() => pilotCommand(r, 'OWN-1/Agent', ['checkpoint', 'save'])).not.toThrow()
    expect(() => pilotCommand(r, 'fixture', ['artifact', 'new', '/host/secret'])).toThrow('human')
    expect(() => pilotCommand(r, 'fixture', ['set', 'autopilot.access', 'full'])).toThrow('human')
  })
  test('network excludes private/loopback/Tailscale addresses and wildcard hosts', () => {
    for (const ip of ['127.0.0.1', '10.0.0.1', '192.168.1.1', '100.64.0.1', '169.254.169.254', '::1', 'fc00::1', '::ffff:127.0.0.1']) expect(publicAddress(ip)).toBe(false)
    expect(publicAddress('1.1.1.1')).toBe(true)
    expect(() => hostList('*.example.com')).toThrow()
    expect(() => hostList('127.0.0.1')).toThrow()
  })
  test.each(['claude', 'codex', 'pi'])('%s gets the same outer sandbox with no unrestricted shell escape', (kind) => {
    const args = sandboxArgs({ cwd: '/work/task', repo: '/repo', home: '/private-home', state: '/fixture/state', control: '/fixture/control', binary: '/usr/bin/fake', bun: '/usr/bin/bun', kind, writable: true, git: ['/work/task/.git'] })
    expect(args).toContain('--unshare-all')
    expect(args).toContain('--clearenv')
    expect(args).toContain('--cap-drop')
    expect(args).not.toContain('--share-net')
    expect(args).not.toContain('/run/user')
    expect(args.join(' ')).toContain('--ro-bind /work/task/.git /work/task/.git')
    expect(args.join(' ')).toContain('--ro-bind /repo /repo')
  })
  test('trusted shared traffic allowance cannot be bypassed by changing agent logs', () => {
    const p = new PilotRuns(); p.runs = []
    const r = p.begin('traffic', { ...config(), traffic: 10 }); r.state = 'on'
    reserveTraffic(r, 6); reserveTraffic(r, 3)
    expect(() => reserveTraffic(r, 2)).toThrow('network allowance')
    expect(() => p.check(r, -1)).toThrow('network allowance')
  })
  test('only provider settings survive; host MCP servers, hooks and plugins do not', () => {
    const codex = providerConfig('codex', 'model_provider = "fixture"\n[mcp_servers.host_shell]\ncommand = "unrestricted-shell"\n[model_providers.fixture]\nbase_url = "https://api.example.com"\n')
    expect(codex).toContain('api.example.com')
    expect(codex).not.toContain('unrestricted-shell')
    expect(codex).not.toContain('mcp_servers')
    const claude = providerConfig('claude', JSON.stringify({ env: { ANTHROPIC_BASE_URL: 'https://api.example.com', UNRELATED_SECRET: 'fake' }, hooks: { SessionStart: ['unrestricted-shell'] }, mcpServers: { remote: {} } }))
    expect(claude).toContain('api.example.com')
    expect(claude).not.toContain('unrestricted-shell')
    expect(claude).not.toContain('UNRELATED_SECRET')
    expect(claude).not.toContain('mcpServers')
  })
  test('all coordinator/worker/reviewer transcripts contribute without offset-id collisions', () => {
    const run = 'usage-fixture', root = join(HOME, 'autopilot-runtime', run)
    for (const who of ['coordinator', 'worker', 'reviewer']) {
      const dir = join(root, who, 'captures/.codex/sessions/2026/10/29'); mkdirSync(dir, { recursive: true })
      writeFileSync(join(dir, `${who}.jsonl`), JSON.stringify({ timestamp: '2026-10-29T00:00:00Z', type: 'event_msg', payload: { type: 'token_count', info: { total_token_usage: { total_tokens: 7 }, last_token_usage: { total_tokens: 7 } } } }) + '\n')
    }
    const usage = new PilotUsage()
    expect(usage.tokens(run)).toBe(21)
    expect(usage.tokens(run)).toBe(21)
  })
})

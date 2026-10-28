// Background work that outlives an agent's turn (readers/background.ts): Claude Code's session file and its screen, on
// samples captured from real panes and session files (Claude Code 2.1.310).
import { afterAll, describe, expect, test } from 'bun:test'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

process.env.SHEPHERDR_HOME = mkdtempSync(join(tmpdir(), 'shep-bg-'))
process.env.CLAUDE_CONFIG_DIR = mkdtempSync(join(tmpdir(), 'shep-bg-claude-'))
process.env.HERDR_SOCKET_PATH = join(process.env.SHEPHERDR_HOME, 'herdr.sock')
const { ClaudeSessions, Reviewers, headless, screenBackground, sessionBackground } = await import('../server/live/readers/background.ts')
const { Agents } = await import('../server/live/agents.ts')
const { writeJson } = await import('../server/store.ts')
const { record } = await import('../server/live/readers/hooks.ts')

const BOX = '─'.repeat(130)
const screen = (above: string[], footer: string, rows: string[] = [], prompt = '❯ ') => [...above, '', BOX, prompt, BOX, footer, '', ...rows].join('\n')

/** SHE-59: it ended its turn while its fresh-eyes reviewer ran, and the reviewer's own probe (a shell and a monitor) */
const REVIEWER = screen(
  ['✻ Sautéed for 19m 18s · done 10:04 PM · 1 shell, 1 monitor still running'],
  '  ⏵⏵ bypass permissions on · 1 shell, 1 monitor · ← 4 agents · ↓ to manage',
  ['  ● main', '  ◯ general-purpose  Waiting on Bun fetch timeout probe                                                         3m 14s · ↓ 104.7k tokens'],
  '❯ ok, merge when the review\'s done',
)

/** it launched a background agent and ended its turn at once */
const WAITING_AGENT = screen(
  ['● Agent(Run python sleep 100 and reply ok)', '  ⎿  Backgrounded agent (↓ to manage · ctrl+o to expand)', '', '● waiting', '', '✻ Waiting for 1 background agent to finish'],
  '  ⏵⏵ bypass permissions on (shift+tab to cycle) · ← 4 agents · ↓ to manage',
  ['  ● main', '  ◯ general-purpose  Run python sleep 100 and reply ok                                                                                    29s · ↓ 29.3k tokens'],
)

/** the agent finished, woke it, and it answered: the old waiting line is still up in the scrollback */
const WOKE_AND_DONE = screen(
  ['✻ Waiting for 1 background agent to finish', '', '● Agent "Run python sleep 100 and reply ok" finished · 1m 43s', '', '● The agent finished and replied "ok."', '', '✻ Brewed for 1m 46s · done 10:13 PM'],
  '  ⏵⏵ bypass permissions on (shift+tab to cycle) · /tasks to see subagents · ← 4 agents',
)

/** SHE-54: three test runs in background shells */
const SHELLS = screen(
  ['● Waiting on three full test runs to rule out a flaky test.', '', '✻ Baked for 42m 58s · done 10:06 PM · 2 shells still running'],
  '  ⏵⏵ bypass permissions on · 2 shells · ← 4 agents · ↓ to manage',
)

const MONITOR = screen(['  ⎿  Monitor started · task b7csavm37 · timeout 60s', '', '● waiting', '', '✻ Crunched for 3s · done 10:14 PM · 1 monitor still running'], '  ⏵⏵ bypass permissions on · 1 monitor · ← 4 agents · ↓ to manage')

/** SHE-63: its turn ended with a question and nothing in the background */
const NOTHING = screen(
  ['  writing them each time. That way they\'re never forgotten and always match Settings.', '', '✻ Sautéed for 5m 24s · done 10:06 PM'],
  '  ⏵⏵ bypass permissions on (shift+tab to cycle) · ← 4 agents',
  [],
  '❯ Yes, build it',
)

describe("Claude Code's screen", () => {
  test('a subagent still running is background work, with or without shells of its own', () => {
    expect(screenBackground(REVIEWER)).toBe('agents')
    expect(screenBackground(WAITING_AGENT)).toBe('agents')
  })

  test('shells and monitors alone are background work too', () => {
    expect(screenBackground(SHELLS)).toBe('shells')
    expect(screenBackground(MONITOR)).toBe('shells')
  })

  test('nothing in the background: nothing, even with an old waiting line in the scrollback', () => {
    expect(screenBackground(NOTHING)).toBeUndefined()
    expect(screenBackground(WOKE_AND_DONE)).toBeUndefined()
    expect(screenBackground('')).toBeUndefined()
    // "← 4 agents" is the footer's way to the agent list, not agents at work
    expect(screenBackground(screen(['✻ Done'], '  ⏵⏵ bypass permissions on · ← 4 agents'))).toBeUndefined()
  })
})

describe("Claude Code's session file", () => {
  test('busy while a background agent runs, shell while only shells or monitors do', () => {
    expect(sessionBackground('busy')).toBe('agents')
    expect(sessionBackground('shell')).toBe('shells')
    expect(sessionBackground('idle')).toBeUndefined()
    expect(sessionBackground('waiting')).toBeUndefined()
    expect(sessionBackground(undefined)).toBeUndefined()
  })

  test('found by its session id, else by its folder; one whose process is gone is none', () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-sessions-'))
    try {
      const file = (pid: number, o: object) =>
        writeFileSync(join(dir, `${pid}.json`), JSON.stringify({ pid, version: '2.1.310', peerProtocol: 1, kind: 'interactive', entrypoint: 'cli', name: 'work-59', ...o }))
      const me = process.pid
      // as captured from SHE-59 after its turn ended while its reviewer ran
      file(me, { sessionId: '764b1ad0-5551-4161-a30e-3706eb3321bc', cwd: '/w/59', status: 'busy', updatedAt: 1792966090908, statusUpdatedAt: 1792966090908 })
      // a process that is gone: its file is left over
      file(2 ** 22 + 7, { sessionId: 'old', cwd: '/w/60', status: 'busy', updatedAt: 1 })
      // another agent, in its own folder, with only a monitor left running
      writeFileSync(join(dir, '1.json'), JSON.stringify({ pid: process.ppid, sessionId: 'she-61', cwd: '/w/61', kind: 'interactive', status: 'shell', updatedAt: 5 }))
      let now = 1_000_000
      const s = new ClaudeSessions(() => dir)
      expect(s.status('764b1ad0-5551-4161-a30e-3706eb3321bc', '/elsewhere', now)).toBe('busy')
      // one look through the folder finds every agent's file
      expect(s.status('she-61', '/w/61', now)).toBe('shell')
      expect(s.status('resumed-under-a-new-id', '/w/59', (now += 10_000))).toBe('busy')
      expect(s.status('old', '/w/60', (now += 10_000))).toBeUndefined()
      // a live pid that started at another time is a new process that got a dead Claude Code's pid
      file(process.ppid, { sessionId: 'reused', cwd: '/w/62', status: 'busy', procStart: '1' })
      expect(s.status('reused', '/w/62', (now += 10_000))).toBeUndefined()
      // what it says now, once it is no longer fresh
      file(me, { sessionId: '764b1ad0-5551-4161-a30e-3706eb3321bc', cwd: '/w/59', status: 'idle', updatedAt: 1792966200000 })
      expect(s.status('764b1ad0-5551-4161-a30e-3706eb3321bc', '/w/59', (now += 2000))).toBe('idle')
    } finally {
      rmSync(dir, { recursive: true, force: true })
    }
  })
})

describe('an agent whose turn ended', () => {
  // herdr, standing in: two Claude Code agents whose turns ended; the second has no session file, only its screen
  const screens: Record<string, string> = { p1: NOTHING, p2: REVIEWER }
  const server = createServer((c) => {
    let buf = ''
    c.on('data', (d) => {
      buf += d
      const i = buf.indexOf('\n')
      if (i < 0) return
      const { id, method, params } = JSON.parse(buf.slice(0, i))
      buf = buf.slice(i + 1)
      const agent = (name: string, pane_id: string) => ({ agent: 'claude', agent_status: 'done', name, pane_id, tab_id: 't', workspace_id: 'w', cwd: null, interactive_ready: true })
      const result =
        method === 'agent.list' ? { agents: [agent('shep-t-1', 'p1'), agent('shep-t-2', 'p2')] } : method === 'pane.read' ? { read: { text: screens[params.pane_id] ?? '' } } : {}
      c.end(`${JSON.stringify({ id, result })}\n`)
    })
  }).listen(process.env.HERDR_SOCKET_PATH)
  afterAll(() => server.close())

  const sessions = join(process.env.CLAUDE_CONFIG_DIR!, 'sessions')
  mkdirSync(sessions, { recursive: true })
  const session = (status: string) => writeFileSync(join(sessions, `${process.pid}.json`), JSON.stringify({ pid: process.pid, sessionId: 's1', cwd: '/w/1', kind: 'interactive', status, updatedAt: Date.now() }))
  const rec = (key: string, name: string, cwd: string, sessionId: string) => ({ key, name, kind: 'claude', cwd, workspace: 'w', logs: [], sessionId, started: 0 })
  writeJson('agents.json', { 'T-1/Agent': rec('T-1/Agent', 'shep-t-1', '/w/1', 's1'), 'T-2/Agent': rec('T-2/Agent', 'shep-t-2', '/w/2', 's2') })

  test('its session file says what still runs, else its screen; a change is told as a status change', async () => {
    session('busy')
    const a = new Agents()
    const told: string[] = []
    a.on('status', (key) => told.push(key))
    await a.refresh()
    expect(a.background('T-1/Agent')).toBe('agents')
    expect(a.background('T-2/Agent')).toBe('agents')
    // the reviewer finished, woke it, and it stopped again in between herdr's looks: told, though herdr saw nothing
    told.length = 0
    session('idle')
    a.sessions = new ClaudeSessions()
    screens.p2 = WOKE_AND_DONE
    await a.refresh()
    expect(a.background('T-1/Agent')).toBeUndefined()
    expect(a.background('T-2/Agent')).toBeUndefined()
    expect(told.sort()).toEqual(['T-1/Agent', 'T-2/Agent'])
    a.stop()
  })

  test('a shell running a headless reviewer of another CLI keeps it at work like a subagent, however long', async () => {
    session('shell')
    const a = new Agents()
    a.reviewers = { runs: (chat: string) => chat === 'T-1/Agent' } as never
    await a.refresh()
    expect(a.background('T-1/Agent')).toBe('agents')
    expect(a.background('T-1/Agent', Date.now() + a.shellsFor + 1)).toBe('agents')
    // the review is over: its shells alone again, for so long
    a.reviewers = { runs: () => false } as never
    expect(a.background('T-1/Agent')).toBe('shells')
    a.stop()
  })

  test("shells alone keep it at work only so long: a server it started for you never wakes it", async () => {
    session('shell')
    const a = new Agents()
    await a.refresh()
    expect(a.background('T-1/Agent')).toBe('shells')
    expect(a.background('T-1/Agent', Date.now() + a.shellsFor + 1)).toBeUndefined()
    const told: string[] = []
    a.on('status', (key) => told.push(key))
    a.shellsFor = 0
    await a.refresh()
    expect(a.background('T-1/Agent')).toBeUndefined()
    expect(told).toEqual(['T-1/Agent'])
    a.stop()
  })

  test('SHE-90: a subagent paused on shells of its own keeps it at work past the shells alone, and so does a session file that says busy', async () => {
    // its subagent stopped to wait on a shell of its own; it ended its turn with only <no-reply>. Stop lists only the
    // subagent's shell, and the session file says "shell" (a live re-run of SHE-90's case)
    const inputs: any[] = readFileSync(join(import.meta.dir, 'samples', 'claude-paused.jsonl'), 'utf8').trim().split('\n').map((l) => JSON.parse(l))
    const events = join(mkdtempSync(join(tmpdir(), 'shep-bg-hooks-')), 'e.jsonl')
    inputs.slice(0, 4).forEach((input, i) => record(events, input, Date.now() - 1000 + i))
    session('shell')
    const a = new Agents()
    a.records['T-1/Agent'].hooks = events
    await a.refresh()
    expect(a.background('T-1/Agent')).toBe('agents')
    // long past what shells alone may keep it at work
    expect(a.background('T-1/Agent', Date.now() + a.shellsFor + 1)).toBe('agents')
    a.shellsFor = 0
    await a.refresh()
    expect(a.background('T-1/Agent')).toBe('agents')
    // hooks that say only shells, while the session file says a subagent runs: it runs
    const shells = join(mkdtempSync(join(tmpdir(), 'shep-bg-hooks-')), 'e.jsonl')
    record(shells, { hook_event_name: 'Stop', last_assistant_message: 'The preview is up.', background_tasks: [{ id: 'b1', type: 'shell', status: 'running' }] }, Date.now())
    session('busy')
    const b = new Agents()
    b.records['T-1/Agent'].hooks = shells
    b.shellsFor = 0
    await b.refresh()
    expect(b.background('T-1/Agent')).toBe('agents')
    // and once the session file says only shells again, they keep it at work only so long
    session('shell')
    b.sessions = new ClaudeSessions()
    await b.refresh()
    expect(b.background('T-1/Agent')).toBeUndefined()
    a.stop()
    b.stop()
  })

  test('SHE-171: shells that end (a test run, a review) keep it at work long past a server; once they are done it needs you', async () => {
    // REV-4: it ended its turn "only waiting on the final test run and Codex's last review pass", 17 minutes into them
    const stop = (tasks: object[]) => {
      const events = join(mkdtempSync(join(tmpdir(), 'shep-bg-hooks-')), 'e.jsonl')
      record(events, { hook_event_name: 'UserPromptSubmit', prompt: 'what do you need?', session_id: 's1' }, Date.now() - 3000)
      record(events, { hook_event_name: 'Stop', last_assistant_message: "Nothing from you right now. I'm only waiting on the final test run and Codex's last review pass.", background_tasks: tasks }, Date.now() - 1000)
      return events
    }
    const REV4 = [
      { id: 'bs5jno4fa', type: 'shell', status: 'running', description: 'Wait for third Codex review to finish', command: 'while pgrep -f "codex review --base main" >/dev/null; do sleep 15; done; tail -c 2500 /tmp/codex-review3.txt' },
      { id: 'b8g1hkavz', type: 'shell', status: 'running', description: 'Rerun acceptance tests then Codex review', command: 'node --test test/acceptance.test.mjs > /tmp/accept2.log 2>&1; codex review --base main > /tmp/codex-review5.txt 2>&1' },
    ]
    session('shell')
    const a = new Agents()
    a.records['T-1/Agent'].hooks = stop(REV4)
    await a.refresh()
    expect(a.background('T-1/Agent')).toBe('shells')
    // where 10 minutes used to make it "needs you"
    expect(a.background('T-1/Agent', Date.now() + a.serversFor + 1)).toBe('shells')
    expect(a.background('T-1/Agent', Date.now() + 30 * 60_000)).toBe('shells')
    // one that hangs: not for ever
    expect(a.background('T-1/Agent', Date.now() + a.shellsFor + 1)).toBeUndefined()
    // they ended and woke it, and it stopped again with nothing in the background, not ready: it needs you
    session('idle')
    a.sessions = new ClaudeSessions()
    await a.refresh()
    expect(a.background('T-1/Agent')).toBeUndefined()
    a.stop()

    // a dev server it left for you to look at never wakes it: only so long
    session('shell')
    const b = new Agents()
    b.records['T-1/Agent'].hooks = stop([{ id: 'b1', type: 'shell', status: 'running', description: 'Start the dev server', command: 'bun run dev --port 5173' }])
    await b.refresh()
    expect(b.background('T-1/Agent')).toBe('servers')
    expect(b.background('T-1/Agent', Date.now() + b.serversFor + 1)).toBeUndefined()
    b.stop()
  })

  test('SHE-171: the count starts again at each turn that ends, even one herdr never saw at work', async () => {
    const events = join(mkdtempSync(join(tmpdir(), 'shep-bg-hooks-')), 'e.jsonl')
    const turn = (at: number) => {
      record(events, { hook_event_name: 'UserPromptSubmit', prompt: 'status?', session_id: 's1' }, at)
      record(events, { hook_event_name: 'Stop', last_assistant_message: 'Still waiting on the tests.', background_tasks: [{ id: 'b1', type: 'shell', status: 'running', command: 'bun test' }] }, at + 1)
    }
    turn(Date.now() - 5000)
    session('shell')
    const a = new Agents()
    a.records['T-1/Agent'].hooks = events
    a.shellsFor = 60_000
    await a.refresh()
    expect(a.background('T-1/Agent')).toBe('shells')
    // seen two minutes ago: past what it may wait
    ;(a as any).seen.get('T-1/Agent').since -= 120_000
    await a.refresh()
    expect(a.background('T-1/Agent')).toBeUndefined()
    // you asked how it goes; it answered in between herdr's looks and stopped again: counted from now
    turn(Date.now() - 2000)
    await a.refresh()
    expect(a.background('T-1/Agent')).toBe('shells')
    expect(a.background('T-1/Agent', Date.now() + 60_001)).toBeUndefined()
    a.stop()
  })
})

describe('headless reviewers', () => {
  test('codex exec, claude -p and pi -p are; an interactive CLI or another command is not', () => {
    expect(headless(['/usr/local/bin/codex', 'exec', '-s', 'read-only', '-'])).toBe(true)
    expect(headless(['node', '/usr/lib/node_modules/@openai/codex/bin/codex.js', 'exec', '-'])).toBe(true)
    expect(headless(['claude', '-p', '--model', 'opus'])).toBe(true)
    expect(headless(['pi', '--print'])).toBe(true)
    expect(headless(['claude', '--settings', 'x.json'])).toBe(false)
    expect(headless(['codex', 'resume'])).toBe(false)
    expect(headless(['grep', 'codex', 'exec'])).toBe(false)
  })
  test("they are found in /proc by their command, and are the chat's their environment names", () => {
    const proc = mkdtempSync(join(tmpdir(), 'shep-proc-'))
    const put = (pid: number, argv: string[], env: string[]) => {
      mkdirSync(join(proc, String(pid)))
      writeFileSync(join(proc, String(pid), 'cmdline'), `${argv.join('\0')}\0`)
      writeFileSync(join(proc, String(pid), 'environ'), `${env.join('\0')}\0`)
    }
    put(10, ['codex', 'exec', '-'], ['HOME=/h', 'SHEPHERDR_CHAT=T-1/Agent'])
    put(11, ['claude', '--settings', 'x'], ['SHEPHERDR_CHAT=T-2/Agent'])
    put(12, ['claude', '-p'], ['HOME=/h'])
    mkdirSync(join(proc, 'self'))
    const r = new Reviewers(proc)
    expect(r.runs('T-1/Agent')).toBe(true)
    expect(r.runs('T-2/Agent')).toBe(false)
  })
  test('with no /proc (macOS) they are found in ps, which prints each command with its environment after it', () => {
    const ps = [
      '/usr/local/bin/codex exec -s workspace-write - TERM=xterm SHEPHERDR_CHAT=T-1/Agent HOME=/Users/e',
      'claude --settings /h/x.json SHEPHERDR_CHAT=T-2/Agent',
      'claude -p --model opus HOME=/Users/e',
      'grep codex exec SHEPHERDR_CHAT=T-3/Agent',
    ].join('\n')
    const r = new Reviewers('/no/proc/here', () => ps)
    expect(r.runs('T-1/Agent')).toBe(true)
    expect(r.runs('T-2/Agent')).toBe(false)
    expect(r.runs('T-3/Agent')).toBe(false)
    expect(new Reviewers('/no/proc/here', () => { throw new Error('no ps') }).runs('T-1/Agent')).toBe(false)
  })
})

describe('the logs an agent keeps', () => {
  test('each once: an old record with one twice reads it once, and the same log found again is not added', () => {
    const p = (id: string, dir = join(process.env.CLAUDE_CONFIG_DIR!, 'projects', '-w-3')) => join(dir, `${id}.jsonl`)
    // SHE-253: a coordinator's record held its 95 MB log twice
    writeJson('agents.json', { 'T-3/Agent': { key: 'T-3/Agent', name: 'shep-t-3', kind: 'claude', cwd: '/w/3', workspace: 'w', logs: [p('a'), p('b'), p('a'), p('c')], sessionId: 'c', started: 0 } })
    const a = new Agents()
    expect(a.record('T-3/Agent')!.logs).toEqual([p('a'), p('b'), p('c')])
    // found again: by its session id, and by the path herdr names it by
    a.discoverLog('T-3/Agent')
    ;(a as unknown as { learnSession: (k: string, i: object) => void }).learnSession('T-3/Agent', { agent_session: { kind: 'path', value: p('a', '/elsewhere') } })
    expect(a.record('T-3/Agent')!.logs).toEqual([p('a'), p('b'), p('c')])
    a.save()
    expect(JSON.parse(readFileSync(join(process.env.SHEPHERDR_HOME!, 'agents.json'), 'utf8'))['T-3/Agent'].logs).toEqual([p('a'), p('b'), p('c')])
    a.stop()
  })
})

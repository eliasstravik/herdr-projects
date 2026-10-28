// What a Claude Code agent reports of itself (readers/hooks.ts), on hook events and a transcript captured from a real
// session (Claude Code 2.1.310): a background subagent and shell, a permission prompt turned down, its question tool,
// a permission granted and a plain question, a plan, the end of the session.
import { describe, expect, test } from 'bun:test'
import { spawnSync } from 'node:child_process'
import { copyFileSync, mkdtempSync, readFileSync, writeFileSync, appendFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

process.env.SHEPHERDR_HOME ??= mkdtempSync(join(tmpdir(), 'shep-hooks-'))
const { foldHook, hookSettings, HookLogs, HOOK_EVENTS, pickHook, record } = await import('../server/live/readers/hooks.ts')
const { Agents, statusFrom } = await import('../server/live/agents.ts')
const { Chats } = await import('../server/live/chats.ts')
type Said = import('../server/live/readers/hooks.ts').Said

const SAMPLES = join(import.meta.dir, 'samples')
/** the captured hook inputs, in the order Claude Code ran them */
const INPUTS: any[] = readFileSync(join(SAMPLES, 'claude-hooks.jsonl'), 'utf8').trim().split('\n').map((l) => JSON.parse(l))

/** folds the captured events from..to (1-based lines of the sample), a second apart; returns what it said after each */
function run(from: number, to: number, start?: Said) {
  let s = start
  const out: (Said | undefined)[] = []
  INPUTS.slice(from - 1, to).forEach((input, i) => {
    const e = pickHook(input, 1000 * (from + i))
    s = e ? foldHook(s, e) : s
    out.push(s)
  })
  return out
}
const brief = (s?: Said) => (s ? [s.state, s.kind, s.ask, s.bg].filter(Boolean).join(' ') : 'none')

describe('folded hook events say what the agent does', () => {
  test('a turn that ends with a subagent and a shell still running, then each one waking it', () => {
    const said = run(1, 16)
    expect(said.map(brief)).toEqual([
      'idle', // SessionStart
      'working', // your prompt
      'working', // it starts a subagent
      'working', // the subagent's own start
      'working',
      'working', // a shell in the background
      'working',
      'idle agents', // its turn ends: a subagent and a shell run
      'idle agents', // the subagent's own tool calls are not its turn
      'idle agents',
      'idle agents',
      'idle agents',
      'working', // the subagent finished and woke it
      'idle shells', // only the shell runs now
      'working', // the shell finished and woke it
      'idle', // nothing runs
    ])
    expect(said[7]?.last).toBe('started')
    expect(said[15]?.last).toBe('The 25-second background sleep finished with exit code 0. Both background tasks are done.')
  })

  test('a permission prompt waits on you', () => {
    expect(run(17, 20, run(1, 16).at(-1)).map(brief)).toEqual(['working', 'working', 'blocked permission', 'blocked permission'])
  })

  test('its question tool waits on you with its own question; your answer sets it going again', () => {
    expect(run(21, 26).map(brief)).toEqual([
      'working',
      'blocked question Which colour should the button be?',
      'blocked question Which colour should the button be?',
      'blocked question Which colour should the button be?',
      'working',
      'idle',
    ])
  })

  test('a permission granted, then a turn that ends asking in plain words', () => {
    const said = run(27, 31)
    expect(said.map(brief)).toEqual(['working', 'working', 'blocked permission', 'working', 'idle'])
    expect(said.at(-1)?.last).toBe('I created `approved.txt`. Should I delete it?')
  })

  test('a plan waits on you, and the notice of it stays a plan', () => {
    expect(run(32, 39).map(brief)).toEqual(['working', 'working', 'working', 'working', 'working', 'blocked plan', 'blocked plan', 'blocked plan'])
  })

  test('the idle notice and the end of the session', () => {
    const said = run(40, 47)
    expect(said.map(brief)).toEqual(['working', 'idle', 'idle', 'working', 'working', 'working', 'idle', 'none'])
  })

  test("a subagent's permission prompt waits on you until that subagent goes on, whatever the agent does meanwhile", () => {
    const at = (event: string, more: object = {}) => ({ hook_event_name: event, ...more })
    const steps = [
      at('UserPromptSubmit'),
      at('PreToolUse', { tool_name: 'Agent' }),
      at('PermissionRequest', { tool_name: 'Bash', agent_id: 'sub1' }),
      at('PostToolUse', { tool_name: 'Agent' }),
      at('Stop', { last_assistant_message: 'A helper is on it.', background_tasks: [{ type: 'subagent', status: 'running' }] }),
      // the notice of it, seconds later: nothing new
      at('Notification', { notification_type: 'permission_prompt' }),
      at('PostToolUse', { tool_name: 'Bash', agent_id: 'sub1' }),
    ]
    let s: Said | undefined
    const seen = steps.map((input, i) => {
      s = foldHook(s, pickHook(input, i + 1)!)
      return statusFrom(s, 'idle')
    })
    expect(seen).toEqual(['working', 'working', 'blocked', 'blocked', 'blocked', 'blocked', 'idle'])
    expect(s?.bg).toBe('agents')
  })

  test('a late notice of a prompt already answered does not block it again', () => {
    expect(run(27, 30).map(brief)).toEqual(['working', 'working', 'blocked permission', 'working'])
    const late = foldHook(run(27, 30).at(-1), pickHook(INPUTS[19], 99)!)
    expect(brief(late)).toBe('working')
  })

  test('a lost login ends the turn waiting on you', () => {
    const s = foldHook({ state: 'working', turnAt: 1, at: 1 }, pickHook({ hook_event_name: 'StopFailure', error: 'authentication_failed', error_details: 'OAuth token has expired' }, 2)!)
    expect(brief(s)).toBe('idle auth')
    expect(brief(foldHook({ state: 'working', turnAt: 1, at: 1 }, pickHook({ hook_event_name: 'StopFailure', error: 'rate_limit' }, 2)!))).toBe('idle')
  })

  test('SHE-90: a subagent paused on shells of its own still runs, though Stop lists only the shells; until its last notice', () => {
    // a live re-run of SHE-90's case: a background subagent started a shell of its own and ended its turn; the notice
    // woke the agent and it answered with only <no-reply>; the shell ended, the subagent finished, its notice woke it
    const paused: any[] = readFileSync(join(SAMPLES, 'claude-paused.jsonl'), 'utf8').trim().split('\n').map((l) => JSON.parse(l))
    let s: Said | undefined
    const said = paused.map((input, i) => (s = foldHook(s, pickHook(input, 1000 * (i + 1))!)))
    expect(said.map(brief)).toEqual([
      'working', // your prompt
      'idle agents', // it started the subagent and ended its turn
      'working', // the subagent's notice woke it: it is waiting on its own background work
      'idle agents', // it ended its turn with only <no-reply>: Stop lists only the subagent's shell
      'working', // the subagent's last notice: it finished
      'idle', // nothing runs
    ])
    expect(said[3]?.last).toBe('<no-reply>reviewer still running</no-reply>')
    expect(said[3]?.paused).toEqual(['aea8329397f764f6e'])
    expect(said.at(-1)?.paused).toBeUndefined()
    expect(pickHook(paused[2], 1)).toEqual({ at: 1, event: 'UserPromptSubmit', paused: ['aea8329397f764f6e'], session: paused[2].session_id })
    expect(pickHook(paused[4], 2)).toEqual({ at: 2, event: 'UserPromptSubmit', over: ['aea8329397f764f6e'], session: paused[4].session_id })
    const stop = (...types: string[]) => pickHook({ hook_event_name: 'Stop', background_tasks: types.map((type, i) => ({ id: `t${i}`, type, status: 'running' })) }, 9)!
    // once it is over, a shell of its own is only a shell again
    expect(foldHook(said[4], stop('shell'))?.bg).toBe('shells')
    // a server that never ends alone is "servers"; with a shell that ends, "shells"
    expect(foldHook(said[4], stop('server'))?.bg).toBe('servers')
    expect(foldHook(said[4], stop('server', 'shell'))?.bg).toBe('shells')
    // a paused subagent with nothing left running can't wake: nothing runs
    expect(foldHook(said[2], stop())?.bg).toBeUndefined()
    // a new session forgets it
    expect(foldHook(said[3], pickHook({ hook_event_name: 'SessionStart', source: 'startup' }, 9)!)?.paused).toBeUndefined()
  })

  test('SHE-171: a shell that serves or watches is a server, one that ends (a test run, a review, a wait) is a shell', () => {
    const kind = (command: string, description = '') => pickHook({ hook_event_name: 'Stop', background_tasks: [{ id: 'b', type: 'shell', status: 'running', command, description }] }, 1)!.bg![0]
    for (const c of ['bun run dev', 'npm start', 'pnpm dev --port 3000', 'npx vite', 'vite preview', 'next dev -p 4000', 'python3 -m http.server 8000', 'uvicorn app:main', 'bun --hot server.ts', 'bun test --watch', 'tail -f /tmp/log', 'npx serve dist'])
      expect([c, kind(c)]).toEqual([c, 'server'])
    expect(kind('bun server/index.ts', 'Start the Shepherdr server')).toBe('server')
    expect(kind('python3 app.py', 'Serve the preview on 8080')).toBe('server')
    for (const c of ['bun run test', 'node --test test/acceptance.test.mjs; codex review --base main > /tmp/r.txt', 'while pgrep -f "codex review" >/dev/null; do sleep 15; done', 'gh run watch 123', 'vite build', 'npm run build', 'sleep 300', 'tailscale serve status'])
      expect([c, kind(c, 'Run the tests then a review')]).toEqual([c, 'shell'])
    expect(pickHook({ hook_event_name: 'Stop', background_tasks: [{ id: 'm', type: 'monitor', status: 'running', command: 'tail -f x' }] }, 1)!.bg).toEqual(['monitor'])
  })

  test("only Claude Code's own words pause a subagent, and of one task's notices in a prompt the last counts", () => {
    const notice = (id: string, summary: string, rest = '') => `<task-notification>\n<task-id>${id}</task-id>\n<status>completed</status>\n<summary>${summary}</summary>\n${rest}</task-notification>`
    const PAUSED = notice('a1', 'Agent "Review" is waiting on its own background work', '<note>This agent stopped with background work of its own still running. It may resume on its own.</note>\n<result>Monitor still watching.</result>\n')
    const FINISHED = notice('a1', 'Agent "Review" finished', '<result>Done.</result>\n')
    const prompt = (...n: string[]) => pickHook({ hook_event_name: 'UserPromptSubmit', prompt: n.join('\n') }, 1)
    // a shell whose command quotes the words, and a finished subagent whose result does: both are over
    expect(prompt(notice('bk2a9x0qz', `Background command "grep -rn 'waiting on its own background work' server" completed`))).toEqual({ at: 1, event: 'UserPromptSubmit', over: ['bk2a9x0qz'] })
    expect(prompt(notice('a2', 'Agent "Grep" finished', '<result>Found "is waiting on its own background work" in hooks.ts. This agent stopped with background work of its own still running</result>\n'))).toEqual({ at: 1, event: 'UserPromptSubmit', over: ['a2'] })
    // paused, then finished, in one prompt: over; and the other way round: paused
    expect(prompt(PAUSED, notice('b1', 'Background command "bun test" completed'), FINISHED)).toEqual({ at: 1, event: 'UserPromptSubmit', over: ['b1', 'a1'] })
    expect(prompt(FINISHED, PAUSED)).toEqual({ at: 1, event: 'UserPromptSubmit', paused: ['a1'] })
  })

  test('a compaction mid-turn goes on with the turn', () => {
    const s = foldHook({ state: 'working', turnAt: 1, at: 1 }, pickHook({ hook_event_name: 'SessionStart', source: 'compact' }, 2)!)
    expect(s?.state).toBe('working')
    expect(s?.turnAt).toBe(1)
  })

  test('a recorded event keeps what matters and drops the bulk', () => {
    const e = pickHook({ ...INPUTS[21], tool_input: { ...INPUTS[21].tool_input, big: 'x'.repeat(50_000) } }, 5)
    expect(e).toEqual({ at: 5, event: 'PreToolUse', tool: 'AskUserQuestion', question: 'Which colour should the button be?' })
    expect(pickHook(INPUTS[7], 6)).toEqual({ at: 6, event: 'Stop', msg: 'started', bg: ['subagent', 'shell'] })
    expect(pickHook(INPUTS[8], 7)?.agent).toBe('a7e51fa529f034c98')
    expect(pickHook({ nothing: 1 }, 8)).toBeUndefined()
  })
})

describe('the one mapping: what it reports over what herdr reads off its screen', () => {
  const said = (state: Said['state']): Said => ({ state, turnAt: 0, at: 0 })
  test('its report wins', () => {
    expect(statusFrom(said('working'), 'idle')).toBe('working')
    expect(statusFrom(said('blocked'), 'working')).toBe('blocked')
    // its turn ended with helpers running: herdr's screen may read that as work, it is not
    expect(statusFrom(said('idle'), 'working')).toBe('idle')
    expect(statusFrom(said('idle'), 'done')).toBe('done')
  })
  test("herdr's reading counts where it reports nothing, for a menu between turns, and when it is gone", () => {
    expect(statusFrom(undefined, 'working')).toBe('working')
    expect(statusFrom(said('idle'), 'blocked')).toBe('blocked')
    expect(statusFrom(said('working'), 'gone')).toBe('gone')
  })
})

describe('recording', () => {
  test('a settings file of its own runs the recorder on every event; each launch starts over', () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-rec-'))
    const { settings, events } = hookSettings(dir, "it's-1")
    const s = JSON.parse(readFileSync(settings, 'utf8'))
    expect(Object.keys(s.hooks)).toEqual([...HOOK_EVENTS])
    const cmd: string = s.hooks.Stop[0].hooks[0].command
    expect(cmd).toContain(process.execPath)
    // the whole raw input goes in on stdin, as Claude Code passes it; one short line comes out
    for (const input of INPUTS.slice(0, 8)) {
      const r = spawnSync('/bin/sh', ['-c', cmd], { input: JSON.stringify({ ...input, transcript_path: '/x.jsonl', cwd: '/x' }) })
      expect(r.status).toBe(0)
    }
    const lines = readFileSync(events, 'utf8').trim().split('\n')
    expect(lines.length).toBe(8)
    const logs = new HookLogs()
    expect(brief(logs.read(events).said)).toBe('idle agents')
    // the recorder reads a paused subagent's notice too
    const paused = hookSettings(dir, 'paused')
    for (const l of readFileSync(join(SAMPLES, 'claude-paused.jsonl'), 'utf8').trim().split('\n').slice(0, 4)) {
      expect(spawnSync('/bin/sh', ['-c', JSON.parse(readFileSync(paused.settings, 'utf8')).hooks.Stop[0].hooks[0].command], { input: l }).status).toBe(0)
    }
    expect(logs.read(paused.events).said?.paused).toEqual(['aea8329397f764f6e'])
    // a launch again: what it recorded before is gone
    hookSettings(dir, "it's-1")
    expect(logs.read(events)).toEqual({ seen: false })
  })

  test('a file grown too big turns over: what it said stays, and the new file is read from its start', () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-rec-'))
    const f = join(dir, 'a.jsonl')
    const logs = new HookLogs()
    record(f, INPUTS[1], 1)
    // pad it past the limit
    appendFileSync(f, `${'{}\n'.repeat(100_000)}`)
    record(f, INPUTS[17], 2)
    expect(brief(logs.read(f).said)).toBe('working')
    record(f, INPUTS[18], 3)
    expect(readFileSync(`${f}.1`, 'utf8').length).toBeGreaterThan(256 * 1024)
    expect(brief(logs.read(f).said)).toBe('blocked permission')
  })

  test('the file is read on from where it was, a half-written line waits for its end', () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-rec-'))
    const f = join(dir, 'a.jsonl')
    const logs = new HookLogs()
    record(f, INPUTS[1], 1)
    expect(brief(logs.read(f).said)).toBe('working')
    const line = `${JSON.stringify(pickHook(INPUTS[18], 2))}\n`
    appendFileSync(f, line.slice(0, 10))
    expect(brief(logs.read(f).said)).toBe('working')
    appendFileSync(f, line.slice(10))
    expect(brief(logs.read(f).said)).toBe('blocked permission')
  })
})

/** an Agents with one Claude Code agent herdr says runs, recording to `events`, its transcript `log` */
function oneAgent(events: string, log: string, herdrSays = 'idle') {
  const a = new Agents()
  const key = 'T-9/Agent'
  a.records[key] = { key, name: 't-9', kind: 'claude', cwd: '/x', workspace: 'w', logs: [log], started: 0, hooks: events }
  a.live.set('t-9', { agent: 'claude', agent_status: herdrSays as never, cwd: '/x', name: 't-9', pane_id: 'p', tab_id: 't', workspace_id: 'w' })
  const chats = new Chats(a, (k) => k, () => 'You')
  a.endedSince = (k, at) => chats.endedSince(k, at)
  return { a, key, chats }
}

describe('a turn cut short runs no hook: its transcript says it is over', () => {
  test('a permission prompt turned down', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-cut-'))
    const events = join(dir, 'e.jsonl')
    const log = join(dir, 'log.jsonl')
    const lines = readFileSync(join(SAMPLES, 'claude-denied.jsonl'), 'utf8').trim().split('\n')
    // up to the prompt: your message, its tool call
    writeFileSync(log, `${lines.slice(0, 6).join('\n')}\n`)
    const at = (iso: string) => Date.parse(iso)
    record(events, INPUTS[16], at('2026-10-25T22:36:26.440Z'))
    record(events, INPUTS[17], at('2026-10-25T22:36:27.700Z'))
    record(events, INPUTS[18], at('2026-10-25T22:36:27.720Z'))
    const { a, key } = oneAgent(events, log, 'blocked')
    expect(a.status(key)).toBe('blocked')
    expect(a.said(key)?.kind).toBe('permission')
    // you said no: the tool result, the interrupt, the turn's end, and not one hook
    appendFileSync(log, `${lines.slice(6).join('\n')}\n`)
    a.live.get('t-9')!.agent_status = 'idle'
    // read again a moment later
    await Bun.sleep(300)
    expect(a.status(key)).toBe('idle')
    expect(a.said(key)?.kind).toBeUndefined()
  })

  test('an agent whose hooks said nothing yet is read off its screen', () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-cut-'))
    const { a, key } = oneAgent(join(dir, 'none.jsonl'), join(dir, 'none-log.jsonl'), 'working')
    expect(a.said(key)).toBeUndefined()
    expect(a.status(key)).toBe('working')
  })

  test('its turn ended with helpers running: idle, with background work, though herdr reads work off its screen', () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-bg-'))
    const events = join(dir, 'e.jsonl')
    INPUTS.slice(0, 8).forEach((input, i) => record(events, input, 1000 + i))
    const { a, key } = oneAgent(events, join(dir, 'log.jsonl'), 'working')
    expect(a.status(key)).toBe('idle')
    expect(a.background(key)).toBe('agents')
  })
})

describe('what it has not read yet', () => {
  const words = (o: { text: string }[]) => o.map((x) => x.text)
  test('what waits to go in, and what was typed in during its turn until its log has it', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-unseen-'))
    const log = join(dir, 'log.jsonl')
    const lines = readFileSync(join(SAMPLES, 'claude-denied.jsonl'), 'utf8').trim().split('\n')
    copyFileSync(join(SAMPLES, 'claude-denied.jsonl'), log)
    writeFileSync(log, `${lines.slice(0, 6).join('\n')}\n`)
    const turn = Date.parse('2026-10-25T22:36:26.434Z')
    const typed = [
      { id: '1', key: 'T-9/Agent', text: 'Run the bash command: touch made-by-test.txt', at: turn - 50 },
      { id: '2', key: 'T-9/Agent', text: '<reaction from="Elias" emoji="👍" to="started"/>', reaction: { message: 'started', emoji: '👍' }, at: turn + 1000 },
      { id: '3', key: 'T-9/Agent', text: '<shepherdr><news task="SHE-1" event="merged">SHE-1 was merged</news></shepherdr>', at: turn + 1500 },
      { id: '4', key: 'T-9/Agent', text: 'Also add a line to the README', at: turn + 2000 },
    ]
    let waiting: typeof typed = []
    const agents = {
      record: () => ({ key: 'T-9/Agent', name: 't-9', kind: 'claude', cwd: '/x', workspace: 'w', logs: [log], started: 0 }),
      waiting: () => waiting,
      typedSince: (_k: string, at: number) => typed.filter((o) => o.at > at),
    }
    const chats = new Chats(agents as never, (k) => k, () => 'Elias')
    // typed in mid-turn, not taken in yet; a reaction and Shepherdr's hidden news don't count
    expect(words(chats.unseen('T-9/Agent'))).toEqual(['Also add a line to the README'])
    // it took it in at its next step: the log has it
    appendFileSync(
      log,
      `${JSON.stringify({ type: 'attachment', uuid: 'q1', isSidechain: false, timestamp: new Date(turn + 2500).toISOString(), attachment: { type: 'queued_command', commandMode: 'prompt', prompt: 'Also add a line to the README', timestamp: new Date(turn + 2500).toISOString() } })}\n`,
    )
    expect(chats.unseen('T-9/Agent')).toEqual([])
    // one still waiting for the agent to take it
    waiting = [{ id: '5', key: 'T-9/Agent', text: 'And merge after', at: turn + 3000 }]
    expect(words(chats.unseen('T-9/Agent'))).toEqual(['And merge after'])
  })
})

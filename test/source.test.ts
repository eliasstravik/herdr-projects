// The shepherdr command: who may merge, choice cards, files, modes, adding projects.
import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { execSync } from 'node:child_process'
import { appendFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, symlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

process.env.SHEPHERDR_HOME = mkdtempSync(join(tmpdir(), 'shep-source-'))
process.env.HERDR_SOCKET_PATH = join(process.env.SHEPHERDR_HOME, 'no-herdr.sock')
delete process.env.SHEPHERDR_MAIN
const { LiveSource } = await import('../server/live/source.ts')
const { HOST, NAME, placeId, Places } = await import('../server/live/places.ts')
const { herdr } = await import('../server/live/herdr.ts')
const { hidden } = await import('../server/live/agents.ts')
const { isAgentProcess } = await import('../server/live/agent-process.ts')
const { heartbeatFile } = await import('../server/live/heartbeat.ts')
const { carryOn } = await import('../server/live/retry.ts')
const { Worktrees } = await import('../server/live/engine/worktrees.ts')
const { fakeHerdr } = await import('./fake-herdr.ts')

const src = new LiveSource()
// herdr makes task worktrees: a stand-in on real git; no terminal opens for their agents (no herdr is there)
const fake = fakeHerdr()
fake.h.createTab = async () => {
  throw new Error('herdr is not here')
}
Object.assign(src.engine, { wt: new Worktrees(fake.h as never) })
afterAll(() => src.stop())

/** what a test waits for has come true, however slow the machine (a timer, another process) */
const until = async (f: () => boolean, ms = 8000) => {
  for (const t0 = Date.now(); !f(); await Bun.sleep(20)) if (Date.now() - t0 > ms) throw new Error('timed out')
}

/** a project with a task whose agent has committed work on its branch (herdr isn't there: its agent can't start) */
async function workedTask(name: string) {
  const dir = mkdtempSync(join(tmpdir(), 'shep-src-repo-'))
  execSync('git init -q -b main && git config user.email t@t && git config user.name T && echo hi > README.md && git add -A && git commit -qm init', { cwd: dir, shell: '/bin/sh' })
  const p = await src.places.add(dir, name)
  const t = src.engine.add({ project: p.name, title: 'Worked', by: 'test' })
  for (let i = 0; !t.worktree && i < 200; i++) await Bun.sleep(20)
  // never in the folder the tests run from
  if (!t.worktree) throw new Error(`${t.id} got no worktree: ${t.trouble}`)
  execSync('echo x > x.txt && git add -A && git commit -qm work', { cwd: t.worktree!, shell: '/bin/sh' })
  return { t, dir, place: placeId.space(p.name) }
}

describe('merging from the command', () => {
  test("a task's agent merges its own task, and only its own", async () => {
    const { t, dir } = await workedTask('own')
    const other = await workedTask('other')
    const no = await src.cli(['merge', other.t.id], { chat: `${t.id}/Agent` })
    expect(no.code).toBe(1)
    expect(no.out).toContain('only its own task')
    expect(other.t.merged).toBeFalsy()
    const r = await src.cli(['merge', t.id], { chat: `${t.id}/Agent` })
    expect(r.out).toContain(`Merged ${t.id} into main`)
    expect(r.out).toContain('do nothing more in them')
    expect(existsSync(join(dir, 'x.txt'))).toBe(true)
    expect(src.engine.statusOf(t)).toBe('done')
  })
  test("shepherdr pr and checks: a task's agent works on its own only, and a repo with no GitHub says so", async () => {
    const { t } = await workedTask('prs')
    const other = await workedTask('prs2')
    const no = await src.cli(['pr', other.t.id], { chat: `${t.id}/Agent` })
    expect(no.code).toBe(1)
    expect(no.out).toContain('its own task')
    const r = await src.cli(['pr', t.id], { chat: `${t.id}/Agent` })
    expect(r.code).toBe(0)
    expect(r.out).toContain('No pull request')
    expect((await src.cli(['checks', t.id], { chat: `${t.id}/Agent` })).out).toContain('No GitHub checks')
  })
  test("a Done card links the task's pull request", async () => {
    const { t, place } = await workedTask('prcard')
    const pr = { number: 7, url: 'https://github.com/acme/app/pull/7' }
    ;(src as unknown as { onCard(t: unknown, s: string, sub: string): void }).onCard({ ...t, pr, posted: 'done|x' }, 'done', 'x')
    expect(src.chats.chat(place).messages.find((m) => m.card?.status === 'done')?.card?.pr).toEqual(pr)
  })
  test("a merged task gets a Done card in its project's chat, which takes reactions like a message", async () => {
    const { t, place } = await workedTask('carded')
    await src.engine.done(t.id, 'Added x.txt.')
    expect((await src.cli(['merge', t.id], {})).out).toContain('Merged')
    // (herdr isn't here, so its start went wrong: that card comes first)
    const cards = () => src.chats.chat(place).messages.filter((m) => m.card?.status === 'done')
    // one line of a few words: its summary is in the task
    expect(cards().map((m) => [m.card!.status, m.card!.text, m.text])).toEqual([['done', 'is merged', `${t.id} is merged`]])
    // a tap on it sends the reaction as a chat line quoting the card
    const sent: string[] = []
    const send = src.agents.send
    src.agents.send = (async (_k: string, text: string) => (sent.push(text), { waiting: false })) as typeof send
    try {
      expect(await src.act({ type: 'react', chat: place, message: cards()[0].id, emoji: '🎉' })).toEqual({})
    } finally {
      src.agents.send = send
    }
    expect(sent).toEqual([`<reaction from="${src.user}" emoji="🎉" to="${t.id} is merged"/>`])
    // read back from the log, it lands on the card
    const ev = (src.chats as unknown as { events: Map<string, unknown[]> }).events
    ev.set(place, [...(ev.get(place) ?? []), { kind: 'user', id: 'r1', at: Date.now() + 10, text: sent[0] }])
    expect(cards()[0].reactions).toEqual([{ emoji: '🎉', by: 'me' }])
  })
  test("a coordinator's <no-reply> after the news shows nothing in the chat; its words do", async () => {
    const { t, place } = await workedTask('noted')
    expect((await src.cli(['merge', t.id], {})).out).toContain('Merged')
    const ev = (src.chats as unknown as { events: Map<string, unknown[]> }).events
    const at = Date.now() + 10
    ev.set(place, [
      ...(ev.get(place) ?? []),
      { kind: 'user', id: 'n1', at, text: hidden(`${t.id} was merged.`, 'news', { task: t.id, event: 'merged' }) },
      { kind: 'agent', id: 'a1', at: at + 1, text: '<no-reply>merged, nothing next</no-reply>' },
    ])
    const agentSaid = () => src.chats.chat(place).messages.filter((m) => m.who === 'agent' && !m.card)
    expect(agentSaid()).toEqual([])
    expect(src.chats.lastReply(place)).toBe('')
    ev.set(place, [...(ev.get(place) ?? []), { kind: 'agent', id: 'a2', at: at + 2, text: '<no-reply>merged</no-reply>\nNext: start SHE-9.' }])
    expect(agentSaid().map((m) => m.text)).toEqual(['<no-reply>merged</no-reply>\nNext: start SHE-9.'])
  })
  test('a coordinator merges when the human asks, and you from a terminal', async () => {
    const a = await workedTask('coord')
    expect((await src.cli(['task', 'merge', a.t.id], { chat: a.place })).out).toContain('Merged')
    const b = await workedTask('term')
    expect((await src.cli(['merge', b.t.id], {})).out).toContain('Merged')
    expect(existsSync(join(b.dir, 'x.txt'))).toBe(true)
  })
  test('task list shows Done tasks newest finished first, after the rest', async () => {
    const a = await workedTask('first')
    const b = await workedTask('second')
    const open = await workedTask('open')
    expect((await src.cli(['merge', a.t.id], {})).out).toContain('Merged')
    await Bun.sleep(5)
    expect((await src.cli(['merge', b.t.id], {})).out).toContain('Merged')
    const ids = [a.t.id, b.t.id, open.t.id]
    const listed = (JSON.parse((await src.cli(['task', 'list', '--json'], {})).out) as { id: string }[]).map((t) => t.id).filter((id) => ids.includes(id))
    expect(listed).toEqual([open.t.id, b.t.id, a.t.id])
    expect(src.engine.view(b.t).doneAt).toBe(b.t.doneAt!)
    expect(src.engine.view(open.t).doneAt).toBeUndefined()
  })
  test('the old task settings and their flags are gone', async () => {
    expect((await src.cli(['set', 'merge.by', 'by itself'], {})).code).toBe(1)
    expect((await src.cli(['review', 'X-1', '--verdict', 'pass'], {})).code).toBe(1)
    expect((await src.cli(['task', 'pick', 'X-1', 'A'], {})).code).toBe(1)
  })
})

describe('closing from the command', () => {
  /** a project with a task whose agent made no changes */
  async function emptyTask(name: string) {
    const dir = mkdtempSync(join(tmpdir(), 'shep-src-repo-'))
    execSync('git init -q -b main && git config user.email t@t && git config user.name T && echo hi > README.md && git add -A && git commit -qm init', { cwd: dir, shell: '/bin/sh' })
    const p = await src.places.add(dir, name)
    const t = src.engine.add({ project: p.name, title: 'Answer', by: 'test' })
    for (let i = 0; !t.worktree && i < 200; i++) await Bun.sleep(20)
    if (!t.worktree) throw new Error(`${t.id} got no worktree: ${t.trouble}`)
    return { t, dir, place: placeId.space(p.name) }
  }

  test('a coordinator closes a task with nothing to merge: Done, and its card says so', async () => {
    const { t, place } = await emptyTask('closed')
    const other = await emptyTask('notyours')
    const no = await src.cli(['task', 'close', t.id], { chat: `${other.t.id}/Agent` })
    expect(no.code).toBe(1)
    expect(no.out).toContain('Coordinators close tasks')
    const r = await src.cli(['task', 'close', t.id, '--summary', 'Answered in chat.'], { chat: place })
    expect(r.code).toBe(0)
    expect(r.out).toContain(`Closed ${t.id}: it is Done`)
    expect(r.out).toContain('its working folder and branch are gone')
    expect(src.engine.statusOf(t)).toBe('done')
    expect(src.engine.view(t).sub).toBe('Nothing to merge')
    const cards = src.chats.chat(place).messages.filter((m) => m.card?.status === 'done')
    expect(cards.map((m) => m.text)).toEqual([`${t.id} is done, nothing to merge`])
    expect((await src.cli(['task', 'close', t.id], { chat: place })).out).toContain('Done already (nothing to merge)')
    expect((await src.cli(['merge', t.id], { chat: place })).out).toContain('nothing to merge')
  })

  test("a task agent's ready on a branch with no changes is Done at once; done, its old name, still works", async () => {
    const { t, place } = await emptyTask('research')
    const r = await src.cli(['ready', t.id, '--summary', 'The answer is 42.'], { chat: `${t.id}/Agent` })
    expect(r.out).toContain('nothing to merge')
    expect(r.out).toContain('do nothing more in them')
    expect(src.engine.statusOf(t)).toBe('done')
    const cards = src.chats.chat(place).messages.filter((m) => m.card?.status === 'done')
    expect(cards.map((m) => m.card!.text)).toEqual(['is done, nothing to merge'])
    expect((await src.cli(['done', t.id], { chat: `${t.id}/Agent` })).out).toContain('Done already (nothing to merge)')
  })

  test('a summary past 160 characters is refused, for the agent to rewrite; the facts go apart, kept off the card', async () => {
    const { t, place } = await emptyTask('short')
    const long = await src.cli(['ready', t.id, '--summary', 'x'.repeat(161)], { chat: `${t.id}/Agent` })
    expect(long.code).toBe(1)
    expect(long.out).toContain('That summary is 161 characters: write a shorter one, 160 at most')
    expect(src.engine.statusOf(t)).not.toBe('done')
    // a facts line written at the summary's end comes apart, and doesn't count
    const r = await src.cli(['ready', t.id, '--summary', `Cards are short. Facts: branch work/1-x · ${'checks passed · '.repeat(12)}`], { chat: `${t.id}/Agent` })
    expect(r.code).toBe(0)
    expect(src.engine.view(t).said).toBe('Cards are short.')
    expect(src.engine.view(t).facts).toStartWith('branch work/1-x · checks passed')
    const card = src.chats.chat(place).messages.find((m) => m.card?.status === 'done')!.card!
    expect(JSON.stringify(card)).not.toContain('Cards are short')
    expect(JSON.stringify(card)).not.toContain('checks passed')
  })
})

describe('deleting a task from the command', () => {
  test("only a coordinator deletes; work main lacks is refused unless --force; then it is gone from every list", async () => {
    const { t, dir, place } = await workedTask('deleting')
    const key = `${t.id}/Agent`
    const no = await src.cli(['task', 'delete', t.id], { chat: key })
    expect(no.code).toBe(1)
    expect(no.out).toContain('only a coordinator')
    const held = await src.cli(['task', 'delete', t.id], { chat: place })
    expect(held.code).toBe(1)
    expect(held.out).toContain(`Not deleted: ${t.id} holds`)
    expect(held.out).toContain('--force')
    expect(src.engine.task(t.id)).toBe(t)
    // its chat had a log and a file
    src.chats.post(key, { id: 'x', text: 'A card' })
    const file = await src.upload(key, 'a.txt', new TextEncoder().encode('a'))
    const r = await src.cli(['task', 'delete', '--force', t.id], { chat: place })
    expect(r.out).toContain(`Deleted ${t.id}`)
    expect(src.engine.task(t.id)).toBeUndefined()
    expect(existsSync(t.worktree!)).toBe(false)
    expect(execSync('git branch --list "work/*"', { cwd: dir, encoding: 'utf8' }).trim()).toBe('')
    expect(existsSync(join(process.env.SHEPHERDR_HOME!, 'chatlog', `${key.replace('/', '_')}.json`))).toBe(false)
    expect(existsSync(file)).toBe(false)
    expect(src.agents.record(key)).toBeUndefined()
    expect((await src.cli(['task', 'list'], {})).out).not.toContain(t.id)
  })

  test('a merged task can be deleted; its commits stay on main', async () => {
    const { t, dir } = await workedTask('delmerged')
    expect((await src.cli(['merge', t.id], {})).out).toContain('Merged')
    const r = await src.cli(['task', 'delete', t.id], {})
    expect(r.out).toContain(`Its commits stay on main`)
    expect(src.engine.task(t.id)).toBeUndefined()
    expect(existsSync(join(dir, 'x.txt'))).toBe(true)
  })
})

describe('To do from the command', () => {
  test('task add starts its agent; shepherdr todo shows it as To do, and a message to its agent is the go', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-src-repo-'))
    execSync('git init -q -b main && git config user.email t@t && git config user.name T && echo hi > README.md && git add -A && git commit -qm init', { cwd: dir, shell: '/bin/sh' })
    const p = await src.places.add(dir, 'later')
    const place = placeId.space(p.name)
    // a title too long for one line on a phone is refused, and nothing is added
    const before = src.engine.tasks.length
    const long = await src.cli(['task', 'add', 'Reply to any message, plus what the best chats do', '--desc', 'x'], { chat: place })
    expect(long.code).toBe(1)
    expect(long.out).toContain('49 characters')
    expect(long.out).toContain('32 at most')
    expect(src.engine.tasks.length).toBe(before)
    expect((await src.cli(['automation', 'add', 'Nightly', '--task', 'x'.repeat(33), '--when', 'FREQ=DAILY'], { chat: place })).code).toBe(1)
    const add = await src.cli(['task', 'add', 'Noted', '--desc', 'For later.', '--later'], { chat: place })
    expect(add.out).toContain('Its agent (Claude Code) reads the brief now and waits for a go')
    const id = add.out.match(/([A-Z]+-\d+)/)![1]
    const other = await src.cli(['task', 'add', 'Next', '--desc', 'x'], { chat: place })
    expect(src.engine.task(add.out.match(/([A-Z]+-\d+)/)![1])!.desc).toStartWith("For later.\n\nDon't start yet: wait for a go.")
    // the old ways are gone
    expect((await src.cli(['task', 'start', id], { chat: place })).code).toBe(1)
    expect((await src.cli(['task', 'edit', id, '--desc', 'x'], { chat: place })).code).toBe(1)
    // herdr isn't here: its agents can't start. Let that settle before the tests after this one
    const t = src.engine.task(id)!
    const next = src.engine.task(other.out.match(/([A-Z]+-\d+)/)![1])!
    for (let i = 0; !(t.trouble && next.trouble) && i < 250; i++) await Bun.sleep(20)
    expect((await src.cli(['todo', id], { chat: `${next.id}/Agent` })).code).toBe(1)
    expect((await src.cli(['todo', id], { chat: `${id}/Agent` })).out).toContain('To do until it gets a go')
    expect(t.todo).toBe(true)
    // the go is a message to its agent: it waits for the agent, and the task is no longer To do
    await src.cli(['task', 'tell', id, 'Go.'], { chat: place })
    expect(src.agents.waiting(`${id}/Agent`).map((o) => o.text)).toEqual(['Go.'])
    expect(t.todo).toBeUndefined()
    src.agents.outbox = src.agents.outbox.filter((o) => o.key !== `${id}/Agent`)
  })
})

describe('Queued from the command', () => {
  test('task add --after queues it; task queue changes or clears it; what is not there, or a circle, is refused', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-src-repo-'))
    execSync('git init -q -b main && git config user.email t@t && git config user.name T && echo hi > README.md && git add -A && git commit -qm init', { cwd: dir, shell: '/bin/sh' })
    const p = await src.places.add(dir, 'queue')
    const place = placeId.space(p.name)
    const idOf = (out: string) => out.match(/([A-Z]+-\d+)/)![1]
    const first = idOf((await src.cli(['task', 'add', 'First', '--desc', 'x'], { chat: place })).out)
    const add = await src.cli(['task', 'add', 'Then', '--desc', 'y', '--after', first], { chat: place })
    expect(add.out).toContain(`queued after ${first}`)
    const then = idOf(add.out)
    expect(src.engine.task(then)!.after).toEqual([first])
    expect((await src.cli(['task', 'add', 'Bad', '--desc', 'y', '--after', 'NOPE-1'], { chat: place })).out).toContain('No task NOPE-1')
    expect((await src.cli(['task', 'add', 'Bad', '--desc', 'y', '--after'], { chat: place })).out).toContain('--after needs the tasks')
    expect((await src.cli(['task', 'add', 'Bad', '--desc', 'y', '--after', 'vm-x:ELA-1'], { chat: place })).out).toContain('no machine called vm-x')
    const loop = await src.cli(['task', 'queue', first, '--after', then], { chat: place })
    expect(loop.code).toBe(1)
    expect(loop.out).toContain(`circle: ${first} → ${then} → ${first}`)
    expect((await src.cli(['task', 'queue', then], { chat: place })).code).toBe(1)
    expect((await src.cli(['task', 'queue', then, '--after', 'none'], { chat: place })).out).toContain('waits for nothing more')
    expect(src.engine.task(then)!.after).toBeUndefined()
    expect((await src.cli(['help'], {})).out).toContain('shepherdr task queue <id> --after')
    // another machine's task, as that machine last showed it: refused while it can't be reached
    const links = (src.peers as unknown as { links: Map<string, unknown> }).links
    const theirs = { id: 'ELA-3', name: 'Their API', status: 'work', place: 'p:vm-x:api', age: '1m', state: 'Working', on: 'Claude Code', created: '' }
    links.set('vm-x', { rec: { machine: 'vm-x', url: 'http://127.0.0.1:9', lastSeen: 0 }, reachable: false, snap: { places: [], tasks: [theirs] }, chats: new Map(), watching: { chats: [], term: null }, waiting: [] })
    expect((await src.cli(['task', 'add', 'Use it', '--desc', 'y', '--after', 'vm-x:ELA-3'], { chat: place })).out).toContain("can't reach vm-x")
    ;(links.get('vm-x') as { reachable: boolean }).reachable = true
    expect((await src.cli(['task', 'add', 'Use it', '--desc', 'y', '--after', 'vm-x:ELA-9'], { chat: place })).out).toContain('vm-x has no task ELA-9')
    const used = await src.cli(['task', 'add', 'Use it', '--desc', 'y', '--after', 'Vm-X:ela-3'], { chat: place })
    expect(used.out).toContain('queued after vm-x:ELA-3')
    const use = src.engine.task(idOf(used.out))!
    theirs.status = 'done'
    src.engine.recheck()
    expect(use.after).toBeUndefined()
    links.delete('vm-x')
    // herdr isn't here: its agents can't start. Let that settle before the tests after this one
    const ts = [first, then, use.id].map((id) => src.engine.task(id)!)
    for (let i = 0; !ts.every((t) => t.trouble) && i < 250; i++) await Bun.sleep(20)
    src.agents.outbox = src.agents.outbox.filter((o) => !ts.some((t) => o.key === `${t.id}/Agent`))
  })
})

describe("a task's CLI, model and effort", () => {
  test('task add takes --agent, --model and --effort for one task; what makes no sense is refused', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-src-repo-'))
    execSync('git init -q -b main && git config user.email t@t && git config user.name T && echo hi > README.md && git add -A && git commit -qm init', { cwd: dir, shell: '/bin/sh' })
    const p = await src.places.add(dir, 'harness-source')
    const place = placeId.space(p.name)
    const before = src.engine.tasks.length
    for (const [args, says] of <[string[], string][]>[
      [['--agent', 'nope'], 'No CLI called nope'],
      [['--effort', 'extreme'], '--effort takes default, low, medium, high, xhigh, max on Claude Code'],
      [['--effort', 'ultra', '--model', 'sonnet'], '--effort takes default, low, medium, high, xhigh, max on Sonnet 5.5'],
      [['--model', 'opus 9'], '--model opus 9: Claude Code has no such model. It takes claude-fable-5-1, claude-opus-5-5, claude-sonnet-5-5, claude-haiku-5-5, or default'],
      [['--model', 'opus;rm'], 'Write a model id'],
      [['--prio', 'high'], 'Task add takes no --prio'],
      [['--effort'], '--effort needs a value'],
      [['--agent', 'gemini', '--model', 'x'], 'Claude Code, Codex and pi only'],
    ]) {
      const r = await src.cli(['task', 'add', 'Odd', '--desc', 'x', ...args], { chat: place })
      expect(r.code).toBe(1)
      expect(r.out).toContain(says)
    }
    expect(src.engine.tasks.length).toBe(before)
    const r = await src.cli(['task', 'add', 'On Codex', '--desc', 'x', '--agent', 'codex', '--model', 'GPT-6-Sol', '--effort', 'Ultra'], { chat: place })
    expect(r.out).toContain('Its agent (Codex, gpt-6-sol, ultra effort) starts on the brief now')
    const t = src.engine.task(r.out.match(/([A-Z]+-\d+)/)![1])!
    expect(t.agent).toEqual({ kind: 'codex', model: 'gpt-6-sol', effort: 'ultra' })
    // default: the CLI's own, whatever Settings say
    src.settings.set('task.model', 'opus')
    const d = await src.cli(['task', 'add', 'Plain', '--desc', 'x', '--model', 'default'], { chat: place })
    expect(src.engine.task(d.out.match(/([A-Z]+-\d+)/)![1])!.agent).toEqual({ kind: 'claude', model: '', effort: undefined })
    src.settings.set('task.model', null)
    for (let i = 0; !t.trouble && i < 250; i++) await Bun.sleep(20)
  })
  test("its brief's Reviewer line names the reviewer: Settings', or task add's --reviewer flags for one task", async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-src-repo-'))
    execSync('git init -q -b main && git config user.email t@t && git config user.name T && echo hi > README.md && git add -A && git commit -qm init', { cwd: dir, shell: '/bin/sh' })
    const place = placeId.space((await src.places.add(dir, 'reviewer-source')).name)
    for (const [args, says] of <[string[], string][]>[
      [['--reviewer', 'gemini'], '--reviewer gemini: Pick one of: Codex · Claude Code · pi.'],
      [['--reviewer-effort', 'extreme'], '--reviewer-effort takes default, low, medium, high, xhigh, max, ultra on Codex'],
      [['--reviewer-model', 'opus'], 'Codex has no such model'],
    ]) {
      const r = await src.cli(['task', 'add', 'Odd', '--desc', 'x', ...args], { chat: place })
      expect(r.code).toBe(1)
      expect(r.out).toContain(says)
    }
    const id = (out: string) => out.match(/([A-Z]+-\d+)/)![1]
    const plain = src.engine.task(id((await src.cli(['task', 'add', 'Plain review', '--desc', 'x'], { chat: place })).out))!
    expect(plain.reviewer).toEqual({ kind: 'codex', model: undefined, effort: undefined })
    expect(plain.desc).toContain('- Reviewer: a headless Codex or a Codex subagent (its default model and effort).\n')
    src.settings.set('reviewer.model', 'gpt-6-sol')
    const own = src.engine.task(id((await src.cli(['task', 'add', 'Claude review', '--desc', 'x', '--reviewer', 'claude', '--reviewer-effort', 'High'], { chat: place })).out))!
    // another CLI drops Settings' model, which names one of Codex's
    expect(own.reviewer).toEqual({ kind: 'claude', model: undefined, effort: 'high' })
    expect(own.desc).toContain('- Reviewer: a headless Claude Code or a Claude Code subagent (its default model, high effort). (set by coordinator for this task)')
    const codex = src.engine.task(id((await src.cli(['task', 'add', 'Codex review', '--desc', 'x', '--reviewer-effort', 'low'], { chat: place })).out))!
    expect(codex.desc).toContain('- Reviewer: a headless Codex or a Codex subagent (model gpt-6-sol, low effort). (set by coordinator for this task)')
    src.settings.set('reviewer.model', null)
  })
  test('task add --set changes a line of its Settings block for that task alone, marked as set by its coordinator', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-src-repo-'))
    execSync('git init -q -b main && git config user.email t@t && git config user.name T && echo hi > README.md && git add -A && git commit -qm init', { cwd: dir, shell: '/bin/sh' })
    const place = placeId.space((await src.places.add(dir, 'set-source')).name)
    const bad = await src.cli(['task', 'add', 'Odd', '--desc', 'x', '--set', 'merge=whenever'], { chat: place })
    expect(bad.code).toBe(1)
    expect(bad.out).toContain('--set merge: Pick one of')
    const id = (out: string) => out.match(/([A-Z]+-\d+)/)![1]
    const t = src.engine.task(id((await src.cli(['task', 'add', 'One round', '--desc', 'x', '--set', 'review.rounds=1', '--set', 'merge=after GitHub checks', '--set', 'reviewer=claude'], { chat: place })).out))!
    // shepherdr merge waits for GitHub's checks, as the brief says, whatever Settings say later
    expect(t.waitChecks).toBe(true)
    expect(t.reviewer?.kind).toBe('claude')
    expect(t.desc).toContain('- Review rounds: 1. One round, no review again. (set by coordinator for this task)')
    expect(t.desc).toContain("- Merge: after GitHub checks. Merge it yourself once it works and GitHub's checks pass (shepherdr checks " + t.id)
    expect(t.desc).toContain('- Reviewer: a headless Claude Code or a Claude Code subagent (its default model and effort). (set by coordinator for this task)')
    // the rest stays Settings', unmarked
    expect(t.desc).toContain('- Must fix: major. Findings are tagged critical/major/minor/trivial; fix critical and major.\n')
  })
  test('shepherdr models lists what each CLI and model take', async () => {
    const out = (await src.cli(['models'], {})).out
    expect(out).toContain('Claude Code (--agent claude), effort levels low, medium, high, xhigh, max:\n  claude-fable-5-1')
    expect(out).toContain('claude-haiku-5-5             Haiku 5.5: low, medium, high, xhigh, max')
    expect(out).toContain('gpt-6-luna                   GPT-6-Luna: low, medium, high, xhigh, max\n')
    expect(out).toContain('pi (--agent pi), effort levels off, minimal, low, medium, high, xhigh, max:\n  any model id it knows')
    expect((await src.cli(['models', 'gemini'], {})).out).toContain('Shepherdr passes it no model or effort')
  })
  test('shepherdr show prints the settings and the lines every brief ends with', async () => {
    const out = (await src.cli(['show'], {})).out
    expect(out).toContain('brief.review           on')
    expect(out).toContain('review.mustFix         major')
    expect(out).toContain('review.rounds          3 rounds')
    expect(out).not.toContain('brief.plan')
    expect(out).toContain('reviewer.agent         Codex')
    expect(out).toContain('reviewer.model         default')
    expect(out).toContain('reviewer.effort        default')
    expect(out).toContain('- Reviewer: a headless Codex or a Codex subagent (its default model and effort).')
    expect(out).toContain('What agents are told (')
    expect(out).toContain('How Shepherdr runs:')
    expect(out).toContain('--set review.rounds=1')
    expect(out).toContain('coordinator.agent      Claude Code')
    expect(out).toContain("Every new task's brief ends with:\nSettings:")
  })
  test("a coordinator first starts on Settings' CLI, model and effort for coordinators", async () => {
    // one that ran before keeps what it runs on until it is restarted (SHE-249): a new space's has never run
    const seen: { kind: string; model?: string; effort?: string }[] = []
    const ensure = src.agents.ensure
    src.agents.ensure = (async (spec: { kind: string; model?: string; effort?: string }) => (seen.push(spec), null)) as never
    try {
      const dir = mkdtempSync(join(tmpdir(), 'shep-src-repo-'))
      execSync('git init -q -b main && git config user.email t@t && git config user.name T && echo hi > README.md && git add -A && git commit -qm init', { cwd: dir, shell: '/bin/sh' })
      const place = placeId.space((await src.places.add(dir, 'coord-settings')).name)
      expect(src.agents.record(place)).toBeUndefined()
      src.settings.set('coordinator.agent', 'Codex')
      src.settings.set('coordinator.effort', 'high')
      await (src as unknown as { coordinator(id: string): Promise<unknown> }).coordinator(place)
      expect(seen.at(-1)).toMatchObject({ kind: 'codex', model: '', effort: 'high' })
    } finally {
      src.agents.ensure = ensure
      src.settings.set('coordinator.agent', null)
      src.settings.set('coordinator.effort', null)
    }
  })
  test('each CLI is started with its own flags for model and effort', () => {
    const rec = (kind: string, model?: string, effort?: string) => ({ key: 'k', name: 'n', kind, cwd: '/', workspace: 'w', logs: [], started: 0, model, effort, access: 'full' as const })
    // Claude Code also gets its own hooks' settings file (readers/hooks.ts)
    const hooks = ['--settings', join(process.env.SHEPHERDR_HOME!, 'hooks', 'n.settings.json')]
    expect(src.agents.args(rec('claude', 'claude-opus-5-5', 'xhigh'), false)).toEqual([...hooks, '--model', 'claude-opus-5-5', '--effort', 'xhigh'])
    // picked in its chat's menu by name
    expect(src.agents.args(rec('claude', 'Sonnet 5.5', 'lowest'), false)).toEqual([...hooks, '--model', 'claude-sonnet-5-5', '--effort', 'low'])
    expect(src.agents.args(rec('claude', 'default', 'default'), false)).toEqual(hooks)
    // a level its model lacks goes in as the nearest it has
    expect(src.agents.args(rec('claude', 'Haiku 5.5', 'ultra'), false)).toEqual([...hooks, '--model', 'claude-haiku-5-5', '--effort', 'max'])
    expect(src.agents.args(rec('codex', 'gpt-6-luna', 'ultra'), false).slice(2)).toEqual(['--dangerously-bypass-approvals-and-sandbox', '-m', 'gpt-6-luna', '-c', 'model_reasoning_effort="max"'])
    for (const key of ['space-key', 'SHE-9/Agent', 'SHE-9/Reviewer']) {
      for (const resume of [false, true]) {
        const args = src.agents.args({ ...rec('codex', '', ''), key, env: { SHEPHERDR_CHAT: 'ST-1/Reviewer', SHEPHERDR_URL: 'http://127.0.0.1:7787' } }, resume)
        const policy = args.find((a) => a.startsWith('shell_environment_policy.set='))!
        expect(policy).toContain(`"SHEPHERDR_CHAT"="${key}"`)
        expect(policy).toContain('"SHEPHERDR_COMMAND"=')
        expect(policy).toContain('"PATH"=')
      }
    }
    expect(src.agents.args(rec('pi', 'openai/gpt-5.5', 'lowest'), false)).toEqual(['--model', 'openai/gpt-5.5', '--thinking', 'minimal'])
    expect(src.agents.args(rec('gemini', 'x', 'high'), false)).toEqual([])
  })
})

describe('choice cards', () => {
  test('fast answers require an explicit ordinary question declaration', async () => {
    const chat = placeId.home()
    for (const ordinary of [false, true]) {
      const r = await src.cli(['ask', 'Which colour?', ...(ordinary ? ['--ordinary'] : []), '--choice', 'Blue', '--choice', 'Green'], { chat })
      expect(r.code).toBe(0)
      const q = src.chats.log(chat).posted.at(-1)!.question!
      expect(q.ordinary).toBe(ordinary || undefined)
      expect(q.choices.map((c) => c.label)).toEqual(['Blue', 'Green'])
      src.chats.answer(chat, q.id, 0)
    }
  })

  test('a typed reply closes the open card: the dot stops waiting and the card shows answered', async () => {
    const chat = placeId.home()
    const r = await src.cli(['ask', 'Rerun CI on main?', '--choice', 'Rerun it', '--choice', 'Leave it'], { chat })
    expect(r.out).toContain('Asked')
    const qid = src.asks()[chat]
    expect(qid).toStartWith('ask:')
    // herdr isn't running here, so the message itself may not go in; the card closes all the same
    await src.act({ type: 'send', chat, text: 'Leave it, I will look later' }).catch(() => undefined)
    expect(src.asks()[chat]).toBeUndefined()
    expect(src.chats.log(chat).answers[qid]).toBe(-1)
    const card = src.chats.chat(chat).messages.find((m) => m.question?.id === qid)
    // answered in chat, quoting your words: not as if an option was picked
    expect(card?.question).toMatchObject({ sent: -1, said: 'Leave it, I will look later', via: 'chat' })
  })

  test('an Other answer typed on the card is quoted, and sent from the card', async () => {
    const chat = placeId.home()
    await src.cli(['ask', 'Which branch?', '--choice', 'main', '--choice', 'next', '--other'], { chat })
    const qid = src.asks()[chat]
    await src.act({ type: 'answer', chat, question: qid, choice: -1, other: 'release-2' }).catch(() => undefined)
    const card = src.chats.chat(chat).messages.find((m) => m.question?.id === qid)
    expect(card?.question).toMatchObject({ sent: -1, said: 'release-2' })
    expect(card?.question?.via).toBeUndefined()
  })
})

describe("a coordinator's question", () => {
  test('sends no alert of its own (the main machine\'s home space picks what buzzes); a second answer is refused', async () => {
    const chat = placeId.home()
    const alerts: import('../shared/types.ts').Alert[] = []
    const off = src.on('alert', (a) => void alerts.push(a))
    try {
      await src.cli(['ask', 'Rerun CI?', '--choice', 'Rerun it', '--choice', 'Leave it'], { chat })
      const qid = src.asks()[chat]
      expect(alerts.filter((a) => a.tag === `ask:${chat}` || a.card)).toEqual([])
      await src.act({ type: 'answer', chat, question: qid, choice: 1 }).catch(() => undefined)
      expect(src.chats.log(chat).answers[qid]).toBe(1)
      // an old alert's button: refused, the answer stays
      expect(await src.act({ type: 'answer', chat, question: qid, choice: 0 })).toMatchObject({ ok: false })
      expect(src.chats.log(chat).answers[qid]).toBe(1)
    } finally {
      off()
    }
  })
})

describe("an alert's button", () => {
  test('answers only the card the chat still ends on; the app may still answer it', async () => {
    const chat = placeId.home()
    await src.cli(['ask', 'Ship it?', '--choice', 'Ship', '--choice', 'Wait'], { chat })
    const qid = src.asks()[chat]
    // something came after the card (an answer in words that leaves it open)
    src.chats.post(chat, { id: 'later', who: 'you', text: 'Hold on, one more thing' })
    expect(await src.act({ type: 'answer', chat, question: qid, choice: 0, fromAlert: true })).toMatchObject({ ok: false })
    expect(src.chats.log(chat).answers[qid]).toBeUndefined()
    await src.act({ type: 'answer', chat, question: qid, choice: 1 }).catch(() => undefined)
    expect(src.chats.log(chat).answers[qid]).toBe(1)
  })
})

describe("a coordinator's alert about a task", () => {
  test('says what the task needs, then the why, in place of the task\'s own alert', async () => {
    const { t } = await workedTask('alerted')
    for (let i = 0; src.engine.statusOf(t) !== 'need' && i < 100; i++) await Bun.sleep(20)
    expect(src.engine.statusOf(t)).toBe('need')
    const alerts: import('../shared/types.ts').Alert[] = []
    const off = src.on('alert', (a) => void alerts.push(a))
    try {
      await src.cli(['alert', 'Needs a look', '--body', 'It could not start', '--task', t.id], { chat: placeId.home() })
    } finally {
      off()
    }
    const a = alerts.at(-1)!
    expect(a).toMatchObject({ from: 'Shepherdr', tag: t.id, go: { task: t.id } })
    expect(a.title).toStartWith(`${t.id} · `)
    expect(a.body).toEndWith('\nhome: Needs a look: It could not start')
  })
})

describe("a task's question on its needs-you card in its space's chat", () => {
  const engine = () => src.engine as unknown as { emit(e: string, ...a: unknown[]): void }
  const told = (place: string) => {
    const said: string[] = []
    const send = src.agents.send
    src.agents.send = (async (k: string, text: string) => (k === place && said.push(text), { waiting: false })) as never
    return { said, off: () => void (src.agents.send = send) }
  }

  test('a choice card answered on the card: it goes to the task, both copies say where, the coordinator hears it', async () => {
    const { t, place } = await workedTask('askcopy')
    await until(() => src.engine.statusOf(t) === 'need')
    const key = `${t.id}/Agent`
    await src.cli(['ask', 'Which colour?', '--choice', 'Blue', '--choice', 'Green'], { chat: key })
    const qid = src.chats.log(key).posted.findLast((p) => p.question)!.question!.id
    engine().emit('card', t, 'need', 'Question for you')
    // kept on the card as it goes up, before anyone opens the space's chat
    expect(src.chats.log(place).posted.findLast((p) => p.card?.task === t.id)?.card?.ask?.questions?.[0]?.id).toBe(qid)
    const card = () => src.chat(place)!.messages.findLast((m) => m.card?.task === t.id)!.card!
    expect(card().ask).toMatchObject({ chat: key, open: true, questions: [{ id: qid, title: 'Which colour?', choices: [{ label: 'Blue' }, { label: 'Green' }] }] })
    const { said, off } = told(place)
    try {
      await src.act({ type: 'answer', chat: key, question: qid, choice: 1, space: place }).catch(() => undefined)
    } finally {
      off()
    }
    expect(src.chats.log(key).answers[qid]).toBe(1)
    expect(card().ask).toMatchObject({ open: false, answered: { where: 'space' }, questions: [{ sent: 1, elsewhere: { label: 'Answered here ·', chat: key, name: `${t.id} chat` } }] })
    expect(src.chats.chat(key).messages.find((m) => m.question?.id === qid)?.question).toMatchObject({ sent: 1, elsewhere: { label: 'Answered in', chat: place } })
    expect(said.join('\n')).toContain("don't answer it too")
    // answered once: the task's own chat can't answer it again
    expect(await src.act({ type: 'answer', chat: key, question: qid, choice: 0 })).toMatchObject({ ok: false })
  })

  test('answered in the task chat instead: the card follows, and says where', async () => {
    const { t, place } = await workedTask('askthere')
    await until(() => src.engine.statusOf(t) === 'need')
    const key = `${t.id}/Agent`
    await src.cli(['ask', 'Ship it?', '--choice', 'Ship', '--choice', 'Wait'], { chat: key })
    const qid = src.chats.log(key).posted.findLast((p) => p.question)!.question!.id
    engine().emit('card', t, 'need', 'Question for you')
    const card = () => src.chat(place)!.messages.findLast((m) => m.card?.task === t.id)!.card!
    expect(card().ask?.open).toBe(true)
    await src.act({ type: 'answer', chat: key, question: qid, choice: 0 }).catch(() => undefined)
    expect(card().ask).toMatchObject({ open: false, answered: { where: 'task' }, questions: [{ sent: 0, elsewhere: { label: 'Answered in', chat: key } }] })
    // a card from another space can't answer for it
    expect(await src.act({ type: 'answer', chat: key, question: qid, choice: 1, space: placeId.home() })).toMatchObject({ ok: false })
  })

  test('a plain question: its words and a reply that goes to the task', async () => {
    const { t, place } = await workedTask('askplain')
    await until(() => src.engine.statusOf(t) === 'need')
    const key = `${t.id}/Agent`
    src.chats.post(key, { id: 'plain-q', text: 'Keep the old API around?' })
    // its question cut to a status line, as the task's row shows it
    engine().emit('card', t, 'need', 'Keep the old API around?')
    const card = () => src.chat(place)!.messages.findLast((m) => m.card?.task === t.id)!.card!
    expect(card().ask).toMatchObject({ chat: key, open: true, reply: { id: 'plain-q', text: 'Keep the old API around?' } })
    const { said, off } = told(place)
    try {
      await src.act({ type: 'send', chat: key, text: 'Yes, one more release', reply: 'plain-q', space: place }).catch(() => undefined)
    } finally {
      off()
    }
    expect(card().ask).toMatchObject({ open: false, answered: { where: 'space', text: 'Yes, one more release' } })
    expect(said.join('\n')).toContain("don't answer it too")
    // another screen still showing the card can't send it twice
    expect(await src.act({ type: 'send', chat: key, text: 'Yes', reply: 'plain-q', space: place })).toMatchObject({ ok: false })
  })
})

describe('waiting on you', () => {
  test("a coordinator blocked on its CLI's own menu waits on you, like an open card: the sidebar counts it", () => {
    const chat = placeId.home()
    const agents = src.agents as unknown as { status: (k: string) => string }
    const status = agents.status
    agents.status = (k) => (k === chat ? 'blocked' : status.call(src.agents, k))
    try {
      const s = src.snapshot()
      expect(s.asks[chat]).toBe('blocked')
      expect(s.places[0].status).toBe('need')
    } finally {
      agents.status = status
    }
    expect(src.snapshot().asks[chat]).toBeUndefined()
  })
})

describe("a task's own progress (shepherdr status)", () => {
  test('its agent reports a line, % done and time left; the task shows it while it works, frozen while it waits on you, gone once done', async () => {
    const { t } = await workedTask('reported')
    const other = await workedTask('reported2')
    const key = `${t.id}/Agent`
    const agents = src.agents as unknown as { status: (k: string) => string }
    const { status } = agents
    let st = 'working'
    agents.status = (k) => (k === key ? st : status.call(src.agents, k))
    const view = () => src.snapshot().tasks.find((x) => x.id === t.id)!
    try {
      // herdr isn't here, so its agent couldn't start: say it did
      t.trouble = undefined
      src.engine.agentStatus(key)
      expect(view().status).toBe('work')
      const no = await src.cli(['status', other.t.id, 'Fixing', '--done', '10'], { chat: key })
      expect(no.code).toBe(1)
      expect(no.out).toContain('only its own task')
      expect((await src.cli(['status', t.id, 'Fixing'], { chat: key })).out).toContain('--done <0-100>')
      expect((await src.cli(['status', t.id, 'Fixing', '--done', '140'], { chat: key })).code).toBe(1)
      expect((await src.cli(['status', t.id, 'Fixing', '--done', '10', '--left', 'soon'], { chat: key })).out).toContain('--left takes a time')
      expect((await src.cli(['status', t.id, 'Fixing', '--done', '10', '--left', '200h'], { chat: key })).out).toContain('up to 99h')
      expect((await src.cli(['status', t.id, 'Fixing', '--done', '10', '--left'], { chat: key })).code).toBe(1)
      expect((await src.cli(['status', t.id, ',,, ,,,', '--done', '10'], { chat: key })).out).toContain('in words')
      const r = await src.cli(['status', t.id, 'Fixing the two failing tests', '--done', '40%', '--left', '1h20m'], { chat: key })
      expect(r.out).toBe(`Noted: ${t.id} shows "Fixing the two failing tests", 40% done, about 80 min left.\n`)
      expect(view().report).toEqual({ msg: 'Fixing the two failing tests', progress: 40, eta: 80, paused: undefined })
      // the CLI's task list shows its own line while it works
      expect((await src.cli(['task', 'list'], {})).out).toContain(`${t.id}    work  Worked  · Fixing the two failing tests`)
      // the minutes count down while it works
      t.report!.at -= 5 * 60_000
      expect(view().report!.eta).toBe(75)
      // waiting on you: frozen, and the time it waits does not count
      st = 'blocked'
      src.engine.agentStatus(key)
      expect(view().report).toMatchObject({ progress: 40, eta: 75, paused: true })
      // ten minutes go by while it waits
      t.report!.at -= 10 * 60_000
      t.report!.pausedAt! -= 10 * 60_000
      expect(view().report!.eta).toBe(75)
      st = 'working'
      src.engine.agentStatus(key)
      expect(view().report).toEqual({ msg: 'Fixing the two failing tests', progress: 40, eta: 75, paused: undefined })
      // a line too long is cut at a word, and it says so; no --left, no minutes
      const long = await src.cli(['status', t.id, 'Rebasing onto main and running every check again', '--done', '90'], { chat: key })
      expect(long.out).toBe(`Noted: ${t.id} shows "Rebasing onto main and running every" (cut to 40 characters), 90% done.\n`)
      expect(view().report!.eta).toBeUndefined()
      // done: its job is over, and so is its report
      st = 'idle'
      await src.cli(['done', t.id, '--summary', 'Fixed'], { chat: key })
      expect(view().status).toBe('ready')
      expect(view().report).toBeUndefined()
      expect(t.report).toBeUndefined()
      // Ready: a report would show nothing now, and come back stale later
      expect((await src.cli(['status', t.id, 'Tidying up', '--done', '99'], { chat: key })).out).toContain('is Ready')
      expect(t.report).toBeUndefined()
      // with no id it still says how Shepherdr is
      expect((await src.cli(['status'], {})).out).toContain('Shepherdr on ')
    } finally {
      agents.status = status
    }
  })
})

describe('attachments', () => {
  const chat = placeId.home()
  /** what the human sent, as the CLI's log has it */
  const said = (id: string, text: string) => {
    const ev = (src.chats as unknown as { events: Map<string, unknown[]> }).events
    ev.set(chat, [...(ev.get(chat) ?? []), { kind: 'user', id, at: Date.now(), text }])
  }
  const attach = (name: string, path: string) => `<attachment from="Elias" name="${name}" path="${path}"/>`

  test('each file is a message of its own, right after the text it came with', async () => {
    const shot = await src.upload(chat, 'shot.png', new Uint8Array([1, 2, 3]))
    const notes = await src.upload(chat, 'notes.pdf', new Uint8Array([4]))
    said('u1', `Look at this\n${attach('shot.png', shot)}\n${attach('notes.pdf', notes)}`)
    // the same name again, a new file
    const again = await src.upload(chat, 'shot.png', new Uint8Array([9]))
    expect(again).not.toBe(shot)
    said('u2', attach('shot.png', again))
    const msgs = src.chats.chat(chat).messages.filter((m) => m.id.startsWith('u'))
    expect(msgs.map((m) => [m.id, m.text ?? m.file?.name])).toEqual([
      ['u1', 'Look at this'],
      ['u1:0', 'shot.png'],
      ['u1:1', 'notes.pdf'],
      ['u2', 'shot.png'],
    ])
    expect(msgs[1].file?.path).toBe(shot)
  })
  test('a reaction to a file lands on that file, not on a later one of the same name', async () => {
    const first = src.chats.chat(chat).messages.find((x) => x.id === 'u1:0')!
    // what a tap sends names the file by its path
    const sent: string[] = []
    const send = src.agents.send
    src.agents.send = (async (_k: string, text: string) => (sent.push(text), { waiting: false })) as typeof send
    try {
      expect(await src.act({ type: 'react', chat, message: first.id, emoji: '❤️' })).toEqual({})
    } finally {
      src.agents.send = send
    }
    expect(sent).toEqual([`<reaction from="${src.user}" emoji="❤️" file="shot.png" path="${first.file!.path}"/>`])
    // and read back from the log, it marks the first shot.png (u2 is a later one)
    said('u3', sent[0])
    // the agent's own reaction to it, too
    const ev = (src.chats as unknown as { events: Map<string, unknown[]> }).events
    ev.get(chat)!.push({ kind: 'agent', id: 'a3', at: Date.now(), text: `<react emoji="👀" file="shot.png" path="${first.file!.path}"/>\nLooking.` })
    const msgs = src.chats.chat(chat).messages
    expect(msgs.find((x) => x.id === 'u1:0')?.reactions).toEqual([{ emoji: '❤️', by: 'me' }, { emoji: '👀', by: 'them' }])
    expect(msgs.find((x) => x.id === 'u2')?.reactions).toBeUndefined()
    // a line naming only the file (an older one) goes to the latest of that name
    said('u3c', '[Elias reacted 🎉 to "notes.pdf"]')
    expect(src.chats.chat(chat).messages.find((x) => x.id === 'u1:1')?.reactions).toEqual([{ emoji: '🎉', by: 'me' }])
  })
  test('a reply goes in quoting the message it answers, and reads back as a quote that jumps to it', async () => {
    const ev = (src.chats as unknown as { events: Map<string, unknown[]> }).events
    ev.get(chat)!.push({ kind: 'agent', id: 'r1', at: Date.now(), text: 'Tests pass on the branch. Want me to merge?' })
    const sent: string[] = []
    const send = src.agents.send
    src.agents.send = (async (_k: string, text: string) => (sent.push(text), { waiting: false })) as typeof send
    try {
      await src.act({ type: 'send', chat, text: 'Yes\nmerge it', reply: 'r1', cid: 'cr1' })
      // a message no longer there: the words go plain
      await src.act({ type: 'send', chat, text: 'Plain', reply: 'gone' })
    } finally {
      src.agents.send = send
    }
    expect(sent).toEqual([`<reply from="${src.user}" to="Tests pass on the branch. Want me to merge?">Yes\nmerge it</reply>`, 'Plain'])
    said('r2', sent[0])
    // the agent answers your reply with one of its own, quoting your words
    ev.get(chat)!.push({ kind: 'agent', id: 'r3', at: Date.now(), text: '<reply to="Yes merge it"/>\nMerged.' })
    // a reply to a file, with a file of its own and no words
    said('r4', `<reply from="Elias" to="shot.png"></reply>\n${attach('b.png', '/inbox/b.png')}`)
    const msgs = src.chats.chat(chat).messages
    const r2 = msgs.find((m) => m.id === 'r2')!
    expect(r2.text).toBe('Yes\nmerge it')
    expect(r2.reply).toEqual({ id: 'r1', who: 'agent', text: 'Tests pass on the branch. Want me to merge?' })
    expect(msgs.find((m) => m.id === 'r3')).toMatchObject({ text: 'Merged.', reply: { id: 'r2', who: 'you', text: 'Yes merge it' } })
    expect(msgs.find((m) => m.id === 'r4')).toMatchObject({ file: { name: 'b.png' }, reply: { who: 'you', text: 'shot.png', file: true } })
    // a quote of something no longer in the chat still shows, without a way to jump
    // (an older bracket line, from a log kept before tags)
    said('r5', '[Elias replied to "Long gone": Still?]')
    expect(src.chats.chat(chat).messages.find((m) => m.id === 'r5')).toMatchObject({ text: 'Still?', reply: { text: 'Long gone' } })
    expect(src.chats.chat(chat).messages.find((m) => m.id === 'r5')?.reply?.id).toBeUndefined()
    // a reply to an older "Done." keeps to it, though a newer one starts the same way
    ev.get(chat)!.push({ kind: 'agent', id: 'd1', at: Date.now(), text: 'Done.' }, { kind: 'agent', id: 'd2', at: Date.now(), text: 'Done.' })
    const sent2: string[] = []
    src.agents.send = (async (_k: string, text: string) => (sent2.push(text), { waiting: false })) as typeof send
    try {
      await src.act({ type: 'send', chat, text: 'Which one?', reply: 'd1' })
    } finally {
      src.agents.send = send
    }
    said('d3', sent2[0])
    expect(src.chats.chat(chat).messages.find((m) => m.id === 'd3')?.reply?.id).toBe('d1')
  })
  test('only files attached in that chat, from the inbox, are served', async () => {
    const shot = src.chats.chat(chat).messages.find((x) => x.id === 'u1:0')!.file!.path
    expect(await (await src.attachment(chat, shot))?.bytes()).toEqual(new Uint8Array([1, 2, 3]))
    // not in this chat
    expect(await src.attachment(placeId.space('elsewhere'), shot)).toBeNull()
    // never attached
    expect(await src.attachment(chat, join(process.env.SHEPHERDR_HOME!, 'config.json'))).toBeNull()
    // attached, but leading out of the inbox: by dots or by a link
    const secret = join(process.env.SHEPHERDR_HOME!, 'secret.txt')
    writeFileSync(secret, 'no')
    const dots = join(process.env.SHEPHERDR_HOME!, 'inbox', 'x', '..', '..', 'secret.txt')
    mkdirSync(join(process.env.SHEPHERDR_HOME!, 'inbox', 'x'), { recursive: true })
    const link = join(process.env.SHEPHERDR_HOME!, 'inbox', 'x', 'mf1-link.txt')
    symlinkSync(secret, link)
    said('u4', `${attach('secret.txt', dots)}\n${attach('link.txt', link)}`)
    expect(await src.attachment(chat, dots)).toBeNull()
    expect(await src.attachment(chat, link)).toBeNull()
  })
  test('a file still in the message box shows there before it is sent, in its own chat only', async () => {
    const waiting = await src.upload(chat, 'draft.png', new Uint8Array([5]))
    expect(await (await src.attachment(chat, waiting))?.bytes()).toEqual(new Uint8Array([5]))
    expect(await src.attachment(placeId.space('elsewhere'), waiting)).toBeNull()
    // another file in that chat's inbox folder that leads out of it by a link is still refused
    const secret = join(process.env.SHEPHERDR_HOME!, 'secret2.txt')
    writeFileSync(secret, 'no')
    const link = join(waiting, '..', 'mf2-link.txt')
    symlinkSync(secret, link)
    expect(await src.attachment(chat, link)).toBeNull()
  })
})

describe('what you send shows at once', () => {
  const chat = placeId.home()
  const said = (id: string, text: string, at = Date.now()) => {
    const ev = (src.chats as unknown as { events: Map<string, unknown[]> }).events
    ev.set(chat, [...(ev.get(chat) ?? []), { kind: 'user', id, at, text }])
  }
  /** the agent's state for a while, as herdr would say it */
  const as = async <T,>(st: string, fn: () => Promise<T>) => {
    const status = src.agents.status
    src.agents.status = () => st as ReturnType<typeof status>
    try {
      return await fn()
    } finally {
      src.agents.status = status
    }
  }
  test("an agent that won't start, or won't take it, is an error the screen shows, not a wait", async () => {
    // what earlier tests left waiting
    src.agents.outbox = src.agents.outbox.filter((o) => o.key !== chat)
    // herdr isn't running here: the agent can't be started
    await expect(src.act({ type: 'send', chat, text: 'Hello?', cid: 'c0' })).rejects.toThrow(/herdr|agent/)
    // ready, but typing it in fails
    await as('idle', async () => {
      await expect(src.act({ type: 'send', chat, text: 'Hello?', cid: 'c0' })).rejects.toThrow("The agent didn't take it")
    })
    expect(src.agents.waiting(chat).some((o) => o.cid === 'c0')).toBe(false)
  })
  test('one that waited and then could not go in fails in the chat; sent again, it is on its way once more', async () => {
    src.agents.outbox = src.agents.outbox.filter((o) => o.key !== chat)
    expect(await as('blocked', () => src.act({ type: 'send', chat, text: 'Later', cid: 'c9' }))).toMatchObject({ waiting: true })
    // the agent is ready, but typing it in fails
    await as('idle', () => src.agents.deliver(chat))
    const c = src.chats.chat(chat)
    expect(c.messages.some((m) => m.cid === 'c9')).toBe(false)
    expect(c.failed).toEqual([{ cid: 'c9', error: expect.stringContaining("The agent didn't take it") }])
    await as('blocked', () => src.act({ type: 'send', chat, text: 'Later', cid: 'c9' }))
    expect(src.chats.chat(chat).failed).toBeUndefined()
    src.agents.outbox = src.agents.outbox.filter((o) => o.key !== chat)
  })
  test("it comes back with the screen's id: while it waits, and once the log has it", async () => {
    // an agent on a menu: it waits, and says why
    const r = (await as('blocked', () => src.act({ type: 'send', chat, text: 'Ship  it\nnow', cid: 'c1' }))) as { waiting: boolean; at: number }
    expect(r.waiting).toBe(true)
    expect(Math.abs(r.at - Date.now())).toBeLessThan(5000)
    expect(r).toMatchObject({ waitsFor: 'menu' })
    const shown = await as('blocked', async () => src.chats.chat(chat).messages.filter((m) => m.cid === 'c1'))
    expect(shown.map((m) => [m.text, m.waiting, m.waitsFor])).toEqual([['Ship  it\nnow', true, 'menu']])
    // the screen heard no answer and sends it again: it still goes in once
    await as('blocked', () => src.act({ type: 'send', chat, text: 'Ship  it\nnow', cid: 'c1' }))
    expect(src.agents.waiting(chat).filter((o) => o.cid === 'c1')).toHaveLength(1)
    // it goes in, and the log has it with its own spacing: the message keeps the id
    const item = src.agents.outbox.find((o) => o.cid === 'c1')!
    src.agents.outbox = src.agents.outbox.filter((o) => o !== item)
    ;(src.agents as unknown as { went(o: typeof item): void }).went(item)
    said('s1', 'Ship it\n now')
    const back = src.chats.chat(chat).messages.filter((m) => m.cid === 'c1')
    expect(back.map((m) => [m.id, m.waiting])).toEqual([['s1', undefined]])
    // the same words again, typed in the terminal, are not it: the id went to the first
    said('s2', 'Ship it now')
    expect(src.chats.chat(chat).messages.find((m) => m.id === 's2')?.cid).toBeUndefined()
  })
  test('one waiting on a menu fails once the agent is gone or herdr out of reach for a while', async () => {
    const agents = src.agents
    for (const [lost, error] of [
      [false, 'The agent stopped before it went in.'],
      [true, 'Lost herdr before it went in.'],
    ] as const) {
      agents.outbox = agents.outbox.filter((o) => o.key !== chat)
      const cid = `strand-${lost}`
      await as('blocked', () => src.act({ type: 'send', chat, text: 'Later', cid }))
      // the agent stops, or herdr goes away while the agent was working
      const reachable = agents.reachable
      agents.reachable = !lost
      try {
        await as(lost ? 'blocked' : 'gone', async () => {
          const t = Date.now()
          agents.checkStranded(t)
          // a moment gone (the agent starting again) is no reason yet
          agents.checkStranded(t + 10_000)
          expect(agents.waiting(chat).some((o) => o.cid === cid)).toBe(true)
          agents.checkStranded(t + agents.strandAfter)
        })
      } finally {
        agents.reachable = reachable
      }
      expect(agents.waiting(chat).some((o) => o.cid === cid)).toBe(false)
      expect(src.chats.chat(chat).failed).toContainEqual({ cid, error })
    }
    // back while it waited: it goes on waiting
    const reachable = agents.reachable
    agents.reachable = true
    await as('blocked', () => src.act({ type: 'send', chat, text: 'Later', cid: 'back' }))
    await as('gone', async () => agents.checkStranded(Date.now()))
    await as('blocked', async () => agents.checkStranded(Date.now() + agents.strandAfter))
    expect(agents.waiting(chat).some((o) => o.cid === 'back')).toBe(true)
    agents.reachable = reachable
    agents.outbox = agents.outbox.filter((o) => o.key !== chat)
  })
  test('sent again while the first is still on its way, it goes in once', async () => {
    const agents = src.agents
    agents.outbox = agents.outbox.filter((o) => o.key !== chat)
    const prompt = herdr.prompt
    const record = agents.records[chat]
    agents.records[chat] = { ...(record ?? {}), key: chat, name: 'held' } as (typeof agents.records)[string]
    let typed = 0
    let let_go!: () => void
    herdr.prompt = (async () => {
      typed++
      await new Promise<void>((r) => (let_go = r))
    }) as typeof prompt
    try {
      await as('idle', async () => {
        const first = src.act({ type: 'send', chat, text: 'Once', cid: 'twice' })
        await new Promise((r) => setTimeout(r, 10))
        const second = src.act({ type: 'send', chat, text: 'Once', cid: 'twice' })
        await new Promise((r) => setTimeout(r, 10))
        let_go()
        expect(await first).toMatchObject({ waiting: false })
        expect(await second).toMatchObject({ waiting: false })
        // and once it is in, a third goes nowhere either
        await src.act({ type: 'send', chat, text: 'Once', cid: 'twice' })
      })
    } finally {
      herdr.prompt = prompt
      if (record) agents.records[chat] = record
      else delete agents.records[chat]
    }
    expect(typed).toBe(1)
  })
  test('a busy agent takes it at once, mid-turn; only a menu, a starting agent or a CLI command mid-turn holds it', async () => {
    const agents = src.agents
    agents.outbox = agents.outbox.filter((o) => o.key !== chat)
    const prompt = herdr.prompt
    const record = agents.records[chat]
    agents.records[chat] = { ...(record ?? {}), key: chat, name: 'steer' } as (typeof agents.records)[string]
    const typed: string[] = []
    let menu = false
    herdr.prompt = (async (_name: string, text: string) => {
      // herdr itself refuses while a menu is up
      if (menu) throw Object.assign(new Error('blocked'), { code: 'agent_blocked' })
      typed.push(text)
    }) as typeof prompt
    try {
      expect(await as('working', () => src.act({ type: 'send', chat, text: 'Also the README', cid: 'w1' }))).toMatchObject({ waiting: false })
      expect(typed).toEqual(['Also the README'])
      // a reaction goes in at once too
      await as('working', () => agents.send(chat, '<reaction from="Elias" emoji="👍" to="x"/>', { reaction: { message: 'm', emoji: '👍' } }))
      expect(typed).toHaveLength(2)
      // a CLI command waits for the turn to end
      expect(await as('working', () => agents.send(chat, '/compact', { idle: true }))).toMatchObject({ waiting: true, waitsFor: 'turn' })
      await as('working', () => agents.deliver(chat))
      expect(typed).toHaveLength(2)
      await as('idle', () => agents.deliver(chat))
      expect(typed.at(-1)).toBe('/compact')
      // a menu came up just as it was typed: it waits for the menu instead of failing
      menu = true
      expect(await as('working', () => src.act({ type: 'send', chat, text: 'Pick nothing', cid: 'w2' }))).toMatchObject({ waiting: true, waitsFor: 'menu' })
      expect(agents.waiting(chat).map((o) => o.cid)).toEqual(['w2'])
      // still up: it stays, without failing
      await as('blocked', () => agents.deliver(chat))
      expect(agents.waiting(chat)).toHaveLength(1)
      expect(src.chats.chat(chat).failed?.some((f) => f.cid === 'w2')).toBeFalsy()
      // the menu is gone and the agent works on: it goes in
      menu = false
      await as('working', () => agents.deliver(chat))
      expect(typed.at(-1)).toBe('Pick nothing')
      expect(agents.waiting(chat)).toHaveLength(0)
      // starting up: it waits for the CLI to take input
      expect(await as('idle', async () => agents.holdOf(chat))).toBeUndefined()
      const info = agents.info
      agents.info = () => ({ interactive_ready: false }) as ReturnType<typeof info>
      try {
        expect(await as('idle', async () => agents.holdOf(chat))).toBe('start')
      } finally {
        agents.info = info
      }
    } finally {
      herdr.prompt = prompt
      if (record) agents.records[chat] = record
      else delete agents.records[chat]
      agents.outbox = agents.outbox.filter((o) => o.key !== chat)
    }
  })
  test("the first message to an agent just started waits until herdr takes input for it, then goes in once; no 'Not sent'", async () => {
    const agents = src.agents
    agents.outbox = agents.outbox.filter((o) => o.key !== chat)
    const prompt = herdr.prompt
    const status = agents.status
    const record = agents.records[chat]
    agents.records[chat] = { ...(record ?? {}), key: chat, name: 'fresh' } as (typeof agents.records)[string]
    // gone until it is started; then herdr lists it, but doesn't count it as its named agent yet
    let st = 'gone'
    let ready = false
    const typed: string[] = []
    agents.status = () => st as ReturnType<typeof status>
    herdr.prompt = (async (name: string, text: string) => {
      if (!ready) throw Object.assign(new Error(`agent ${name} is not an active named agent`), { code: 'agent_not_found' })
      typed.push(text)
    }) as typeof prompt
    try {
      const ensure = async () => void (st = 'idle')
      const r = await agents.send(chat, 'Hello there', { ensure, cid: 'first' })
      expect(r).toMatchObject({ waiting: true, waitsFor: 'start' })
      const mine = () => agents.waiting(chat).filter((o) => o.cid)
      expect(mine().map((o) => o.cid)).toEqual(['first'])
      // refreshes while herdr still refuses it: it keeps waiting, without failing
      await agents.deliver(chat)
      await agents.deliver(chat)
      expect(mine()).toHaveLength(1)
      expect(agents.holdOf(chat)).toBe('start')
      expect(src.chats.chat(chat).failed?.some((f) => f.cid === 'first')).toBeFalsy()
      // the agent is up: it goes in, once (news for this coordinator may go in too)
      ready = true
      await agents.deliver(chat)
      expect(typed.filter((x) => x === 'Hello there')).toHaveLength(1)
      expect(mine()).toHaveLength(0)
      expect(agents.holdOf(chat)).toBeUndefined()
      expect(src.chats.chat(chat).failed?.some((f) => f.cid === 'first')).toBeFalsy()
      // one that never comes up: after a while it fails, and Retry is offered
      ready = false
      const unreadyFor = agents.unreadyFor
      agents.unreadyFor = 0
      try {
        expect(await agents.send(chat, 'Anyone?', { cid: 'never' })).toMatchObject({ waiting: true, waitsFor: 'start' })
        // the try send started reads the screen first, so it may still be on its way: until it is over
        for (let i = 0; i < 100 && mine().length; i++) {
          await agents.deliver(chat)
          await new Promise((r) => setTimeout(r, 20))
        }
      } finally {
        agents.unreadyFor = unreadyFor
      }
      expect(mine()).toHaveLength(0)
      expect(src.chats.chat(chat).failed).toContainEqual({ cid: 'never', error: expect.stringContaining("The agent didn't take it") })
    } finally {
      herdr.prompt = prompt
      agents.status = status
      if (record) agents.records[chat] = record
      else delete agents.records[chat]
      agents.outbox = agents.outbox.filter((o) => o.key !== chat)
    }
  })
  test('its files carry the id too', async () => {
    const shot = await src.upload(chat, 'shot.png', new Uint8Array([1]))
    await as('blocked', () => src.act({ type: 'send', chat, text: '', attachments: [shot], cid: 'c2' }))
    expect(src.chats.chat(chat).messages.filter((m) => m.cid === 'c2').map((m) => m.file?.path)).toEqual([shot])
  })
})

describe('modes: regular and Autopilot', () => {
  test('a saved mode that is neither loads as regular and is dropped', () => {
    writeFileSync(join(process.env.SHEPHERDR_HOME!, 'modes.json'), JSON.stringify({ 'p:h:old': 'manual', 'p:h:co': 'copilot', 'p:h:auto': 'autopilot', 'p:h:reg': 'Regular' }))
    const places = new Places()
    expect([places.mode('p:h:old'), places.mode('p:h:co'), places.mode('p:h:auto'), places.mode('p:h:reg'), places.mode('p:h:new')]).toEqual(['regular', 'regular', 'autopilot', 'regular', 'regular'])
    places.save()
    expect(JSON.parse(readFileSync(join(process.env.SHEPHERDR_HOME!, 'modes.json'), 'utf8'))).toEqual({ 'p:h:auto': 'autopilot', 'p:h:reg': 'regular' })
  })
  test('shepherdr mode takes regular and autopilot, any case', async () => {
    expect((await src.cli(['mode', 'home', 'Autopilot'], {})).out.trim()).toBe('home: Autopilot on.')
    expect((await src.cli(['mode', 'home', 'regular'], {})).out.trim()).toBe('home: Autopilot off.')
    for (const m of ['copilot', 'manual', 'sleepy']) expect((await src.cli(['mode', 'home', m], {})).code).toBe(1)
  })
})

describe('news for coordinators', () => {
  test("every space starts regular; the main machine's home space hears every space's news, a task added only on Autopilot", async () => {
    const project = placeId.space('newsco')
    const home = placeId.home()
    const sent: string[] = []
    const send = src.agents.send
    src.agents.send = (async (k: string) => (k === project || k === home ? sent.push(k) : 0, { waiting: false })) as never
    const modes = { ...src.places.modes }
    const news = (kind: 'added' | 'ready', at = project) => (sent.length = 0, src.engine.emit('news', at, 'NC-1 is ready', kind, { task: 'NC-1', event: kind }), [...sent].sort())
    try {
      for (const id of [project, home]) delete src.places.modes[id]
      expect(news('ready')).toEqual([home, project].sort())
      expect(news('added')).toEqual([])
      src.places.modes[project] = 'regular'
      src.places.modes[home] = 'autopilot'
      expect(news('ready')).toEqual([home, project].sort())
      expect(news('added')).toEqual([home])
      // the home space's own news goes to it once
      expect(news('ready', home)).toEqual([home])
    } finally {
      src.agents.send = send
      src.places.modes = modes
    }
  })
  test("news the human sees as a card gets its coordinator's short reply after it; unseen news may get <no-reply>", async () => {
    const project = placeId.space('newsreply')
    const said = new Map<string, string>()
    const send = src.agents.send
    src.agents.send = (async (k: string, text: string) => (said.set(k, text), { waiting: false })) as never
    const modes = { ...src.places.modes }
    const engine = src.engine as unknown as { emit(e: string, ...a: unknown[]): void }
    try {
      for (const id of [project, placeId.home()]) delete src.places.modes[id]
      // a status's card goes up, then its news: the coordinator replies after the card, never quietly
      const t = { id: 'NR-1', project, posted: 'ready|' } as never
      engine.emit('card', t, 'ready', '')
      engine.emit('news', project, 'NR-1 is ready to merge', 'ready', { task: 'NR-1', event: 'ready' })
      expect(said.get(project)).toContain('A card in this chat already shows the human the news: reply after it in one or two short lines')
      expect(said.get(project)).toContain('Never <no-reply> to it.')
      // the main machine's home space sees no card: it may stay quiet, or alert what can't wait
      expect(said.get(placeId.home())).toContain('newsreply: NR-1 is ready to merge')
      expect(said.get(placeId.home())).toContain('alert the human with shepherdr alert')
      expect(said.get(placeId.home())).toContain('<no-reply>reason</no-reply>')
      // a second word on the same merge (what its cleanup kept) has no card of its own
      engine.emit('news', project, 'NR-1 was merged; kept notes.md', 'done', { task: 'NR-1', event: 'merged' })
      expect(said.get(project)).toContain('reply with only <no-reply>reason</no-reply>')
      // main went red: a one-line card in the project's chat, and the coordinator's reply after it
      engine.emit('news', project, 'main went red on GitHub at 1a2b3c4: Check failed (link).', 'red', { task: '', event: 'main-red' }, { red: 'main', text: 'went red: Check failed' })
      expect(src.chats.chat(project).messages.filter((m) => m.card?.red).map((m) => m.text)).toEqual(['main went red: Check failed'])
      expect(said.get(project)).toContain('Never <no-reply> to it.')
    } finally {
      src.agents.send = send
      src.places.modes = modes
    }
  })
})

describe("every machine's news reaches the main machine's home space", () => {
  test("another machine's needs-you, red main and health come over its worker's link in one line each; the space's coordinator has the whole", async () => {
    const { startApp } = await import('../server/app.ts')
    const { Emitter } = await import('../server/source.ts')
    // the other machine's worker: its app, its news as its LiveSource emits it
    const worker = Object.assign(new Emitter(), {
      kind: 'live',
      snapshot: () => ({ name: 'S', host: 'faraway', places: [{ id: 'p:faraway:shop', name: 'shop', machine: 'faraway', level: 1, status: 'need', mode: 'regular', chat: 'p:faraway:shop' }], machines: [{ id: 'm:faraway', machine: 'faraway', name: 'faraway', status: 'need' }], tasks: [{ id: 'FS-1', name: 'Colours', status: 'need', place: 'p:faraway:shop', age: '1m', state: '', on: '', created: '' }], automations: {}, asks: {}, settings: [], overrides: {}, user: 'u' }),
      chat: () => undefined,
      watch: () => {},
    })
    const app = startApp({ source: worker as never, dist: '/nonexistent', port: 0 })
    // news from before the main machine's link (its server restarting) waits for it
    worker.emit('news', { place: 'p:faraway:shop', line: 'FS-0 is ready to merge', kind: 'ready', about: { task: 'FS-0', event: 'ready' } })
    const home = placeId.home()
    const said: string[] = []
    const send = src.agents.send
    src.agents.send = (async (k: string, text: string) => (k === home && said.push(text), { waiting: false })) as never
    const alerts: { title: string; go: { place: string; task?: string }; task?: true }[] = []
    const off = src.on('alert', (a) => alerts.push(a))
    try {
      const l = src.peers!.adopt('faraway', `http://127.0.0.1:${app.port}`)
      await until(() => l.ws?.readyState === WebSocket.OPEN)
      await until(() => said.length > 0)
      expect(said.shift()).toContain('faraway · shop: FS-0 is ready to merge.')
      worker.emit('news', { place: 'p:faraway:shop', line: 'FS-1 needs you: Which colour?', kind: 'need', about: { task: 'FS-1', event: 'needs-you' } })
      await until(() => said.length > 0)
      expect(said[0]).toContain('<news task="FS-1" event="needs-you">faraway · shop: FS-1 needs you: Which colour?.')
      expect(said[0]).toContain('Its own coordinator has the whole of it.')
      expect(said[0]).toContain('alert the human with shepherdr alert')
      // main red: only its first line, never the log under it
      worker.emit('news', { place: 'p:faraway:shop', line: 'main went red on GitHub at 1a2b3c4: Check failed (link).\nIts log ends:\n> boom', kind: 'red', about: { task: '', event: 'main-red' } })
      worker.emit('news', { place: 'p:faraway:~', line: "herdr isn't answering (since 10:00), so its agents can't hear anything", kind: 'health', about: { kind: 'herdr', what: 'over' } })
      // a task added: only while the main machine's home space is on Autopilot
      worker.emit('news', { place: 'p:faraway:shop', line: 'FS-2 was added: More', kind: 'added', about: { task: 'FS-2', event: 'added' } })
      await until(() => said.length >= 3)
      await Bun.sleep(100)
      expect(said.length).toBe(3)
      expect(said[1]).toContain('faraway · shop: main went red on GitHub at 1a2b3c4: Check failed (link).')
      expect(said[1]).not.toContain('boom')
      expect(said[2]).toContain('<health kind="herdr" what="over">Machine faraway: herdr isn')
      // its alert about that task opens on it, and buzzes while the task waits on you
      expect((await src.cli(['alert', 'FS-1 waits on a colour', '--task', 'FS-1'], { chat: home })).out.trim()).toBe('Alert sent.')
      expect(alerts.at(-1)).toMatchObject({ title: 'FS-1 waits on a colour', go: { place: 'p:faraway:shop', task: 'FS-1' }, task: true })
    } finally {
      off()
      src.agents.send = send
      app.stop(true)
    }
  })
  test("a worker passes each piece of its machine's news on, for its link to carry to the main machine", () => {
    const got: { place: string; line: string; kind: string }[] = []
    const off = src.on('news', (n) => got.push(n))
    const send = src.agents.send
    src.agents.send = (async () => ({ waiting: false })) as never
    try {
      src.engine.emit('news', placeId.space('newsout'), 'NO-1 needs you: Signed out', 'need', { task: 'NO-1', event: 'needs-you' })
      expect(got).toEqual([{ place: placeId.space('newsout'), line: 'NO-1 needs you: Signed out', kind: 'need', about: { task: 'NO-1', event: 'needs-you' } } as never])
    } finally {
      off()
      src.agents.send = send
    }
  })
})

describe('Autopilot switched on or off', () => {
  test("posts one card in the place's chat and tells its coordinator quietly; no change, no card", async () => {
    const place = placeId.home(HOST)
    src.places.setMode(place, 'regular')
    // only this test's (shepherdr mode switched the home space before)
    const since = Date.now()
    const cards = () => src.chats.chat(place).messages.filter((m) => m.card?.mode && (m.at ?? 0) >= since)
    const sent: string[] = []
    const send = src.agents.send
    const status = src.agents.status
    src.agents.send = (async (_k: string, text: string) => (sent.push(text), { waiting: false })) as typeof send
    src.agents.status = (() => 'idle') as typeof status
    try {
      expect(await src.act({ type: 'mode', place, mode: 'regular' })).toEqual({})
      expect(cards()).toEqual([])
      expect(sent).toEqual([])
      expect((await src.cli(['mode', 'home', 'autopilot'], {})).code).toBe(0)
      expect(await src.act({ type: 'mode', place, mode: 'autopilot' })).toEqual({})
      expect((await src.cli(['mode', 'home', 'regular'], {})).code).toBe(0)
      expect(await src.act({ type: 'mode', place, mode: 'copilot' as never })).toEqual({ ok: false, error: 'No such mode.' })
    } finally {
      src.agents.send = send
      src.agents.status = status
    }
    expect(cards().map((m) => [m.card!.mode, m.text])).toEqual([
      ['autopilot', 'Autopilot on · works on its own'],
      ['autopilot', 'Autopilot off · does what you ask'],
    ])
    expect(sent).toHaveLength(2)
    expect(sent[0]).toContain('<shepherdr><mode mode="autopilot">Autopilot is now on: you also work on your own')
    expect(sent[1]).toContain('<shepherdr><mode mode="regular">Autopilot is now off: do what the human asks, nothing on your own, and no more heartbeats.')
    expect(sent[0]).toContain("don't announce it, reply with only <no-reply>reason</no-reply>")
  })
})

describe("Autopilot's heartbeat", () => {
  const min = 60_000
  test('on its time rule, only while its coordinator is idle; the news its event rules name wakes it at once, the rest waits for it', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-beat-'))
    execSync('git init -q -b main', { cwd: dir })
    const p = await src.places.add(dir, 'beat')
    const place = placeId.space(p.name)
    const sent: { k: string; text: string }[] = []
    const send = src.agents.send
    const status = src.agents.status
    const reachable = src.agents.reachable
    let st = 'working'
    src.agents.send = (async (k: string, text: string) => (sent.push({ k, text }), { waiting: false })) as typeof send
    src.agents.status = ((k: string) => (k === place ? st : 'gone')) as typeof status
    src.agents.reachable = true
    const beats = () => sent.filter((x) => x.k === place && x.text.includes('<heartbeat>'))
    // on a half hour: every 30 minutes runs on the clock's :00 and :30
    const t0 = Math.floor(Date.now() / (30 * min)) * 30 * min
    try {
      src.places.setMode(place, 'autopilot')
      src.heartbeats.switched(place, true, t0)
      src.heartbeat(t0 + 29 * min)
      expect(beats()).toEqual([])
      // due, but busy: it waits for the turn to end
      src.heartbeat(t0 + 31 * min)
      expect(beats()).toEqual([])
      // all news (the default) wakes the coordinator at once, merged news too: none waits for the heartbeat
      src.engine.emit('news', place, 'BE-1 was added: Fix it', 'added', { task: 'BE-1', event: 'added' })
      src.engine.emit('news', place, 'BE-2 needs you: a question', 'need', { task: 'BE-2', event: 'needs-you' })
      src.engine.emit('news', place, 'BE-3 is merged', 'done', { task: 'BE-3', event: 'merged' })
      expect(sent.filter((x) => x.k === place).map((x) => x.text)).toEqual([
        expect.stringContaining('<news task="BE-1" event="added">BE-1 was added'),
        expect.stringContaining('<news task="BE-2" event="needs-you">BE-2 needs you'),
        expect.stringContaining('<news task="BE-3" event="merged">BE-3 is merged'),
      ])
      // only what needs you wakes it: other news waits for the next heartbeat
      expect(src.settings.set('heartbeat.on', 'needs-you', { space: p.name }).ok).toBe(true)
      src.engine.emit('news', place, 'BE-4 is ready to merge', 'ready', { task: 'BE-4', event: 'ready' })
      expect(sent.filter((x) => x.k === place).some((x) => x.text.includes('BE-4'))).toBe(false)
      st = 'idle'
      src.heartbeat(t0 + 32 * min)
      expect(beats()).toHaveLength(1)
      const beat = beats()[0].text
      expect(beat).toStartWith('<shepherdr><heartbeat>Heartbeat. Go through your HEARTBEAT.md: ')
      expect(beat).toContain('Your HEARTBEAT.md:\n# Heartbeat')
      expect(beat).not.toContain('BE-1')
      expect(beat).toContain('News since the last heartbeat')
      expect(beat).toContain('- BE-4 is ready to merge')
      src.settings.set('heartbeat.on', null, { space: p.name })
      expect(beat).toContain('<no-reply>heartbeat: nothing to do</no-reply>')
      // the quiet marker the coordinator answers with shows nothing in the chat
      expect((await src.cli(['heartbeat'], { chat: place })).out).toMatch(/heartbeat: Every 30 minutes \(.+\); last .+; next .+ once idle\. Wakes at once on: all news\./)
      // read and changed with the command, from any machine: the checklist itself, never its path
      expect((await src.cli(['heartbeat'], { chat: place })).out).toContain('HEARTBEAT.md:\n# Heartbeat')
      expect((await src.cli(['heartbeat', 'set'], { chat: place })).out).toContain('on stdin')
      expect((await src.cli(['heartbeat', 'set', p.name], { stdin: '- Check the ROM builds\n' })).out).toContain('is saved')
      const shown = (await src.cli(['heartbeat', p.name], {})).out
      expect(shown).toContain('HEARTBEAT.md:\n- Check the ROM builds')
      expect(shown).not.toContain(heartbeatFile(place))
      const show = (await src.cli(['show'], { chat: place })).out
      expect(show).toContain('Heartbeat on Autopilot: Every 30 minutes (')
      expect(show).toContain("news that wakes it at once: all news, the rest waits for the heartbeat. Each brings the place's HEARTBEAT.md")
      // once given, the next is the rule's next run
      src.heartbeat(t0 + 40 * min)
      expect(beats()).toHaveLength(1)
      src.heartbeat(t0 + 62 * min)
      expect(beats()).toHaveLength(2)
      // an empty HEARTBEAT.md, edited in Settings: no heartbeat
      expect(await src.act({ type: 'setting', key: 'heartbeat.md', value: '', place })).toEqual({ ok: true })
      expect(src.snapshot().settings.find((x) => x.level === place && x.name === 'Autopilot')!.rows.find((r) => r.key === 'heartbeat.md')).toMatchObject({ value: '', doc: true })
      src.heartbeat(t0 + 93 * min)
      expect(beats()).toHaveLength(2)
      // off Autopilot: no heartbeat however long it waits
      src.places.setMode(place, 'regular')
      src.heartbeats.switched(place, false)
      await src.act({ type: 'setting', key: 'heartbeat.md', value: '- check things', place })
      src.heartbeat(t0 + 300 * min)
      expect(beats()).toHaveLength(2)
    } finally {
      src.agents.send = send
      src.agents.status = status
      src.agents.reachable = reachable
    }
  })
  test('the quiet marker shows nothing in the chat', async () => {
    const { agentLines } = await import('../server/live/chatlines.ts')
    expect(agentLines('<no-reply>heartbeat: nothing to do</no-reply>').text).toBe('')
  })
})

describe('automations add tasks with fresh agents', () => {
  test('on a time rule and on an event, each run a new task from its template; paused, run now, edited, removed', async () => {
    const { t, place } = await workedTask('autos')
    const name = src.places.space(place)!.name
    const tasks = () => src.engine.tasks.filter((x) => x.project === place && x.by.startsWith('the automation'))
    const add = await src.cli(['automation', 'add', 'Nightly', '--task', 'Bump deps', '--brief', 'Bump and test.', '--when', 'FREQ=MINUTELY;INTERVAL=5', '--agent', 'codex'], { chat: place })
    expect(add.out).toMatch(/^Automation “Nightly” is on: Every 5 minutes \(.+\); next .+ \[--when "DTSTART:20260101T000000 RRULE:FREQ=MINUTELY;INTERVAL=5"\]\. It adds the task “Bump deps” to autos, with a fresh agent/)
    // its first run is the rule's first after it was made: as if made six minutes ago
    src.automations.find(place, 'Nightly')!.since = Date.now() - 6 * 60_000
    ;(src as unknown as { runSchedules(): void }).runSchedules()
    expect(tasks().map((x) => [x.title, x.by, x.agent?.kind])).toEqual([['Bump deps', 'the automation “Nightly”', 'codex']])
    expect(tasks()[0].desc).toStartWith('Bump and test.')
    // the next run is five minutes after the last
    ;(src as unknown as { runSchedules(): void }).runSchedules()
    expect(tasks()).toHaveLength(1)
    expect(src.snapshot().automations[place][0]).toMatchObject({ name: 'Nightly', when: expect.stringMatching(/^Every 5 minutes \(/), what: 'Bump deps', on: true, next: expect.any(Array), runs: [{ task: tasks()[0].id }] })
    // an event: a task merged adds one; a task an automation added sets off no 'added' automation
    await src.cli(['automation', 'add', 'Notes', '--task', 'Write notes', '--on', 'merged'], { chat: place })
    await src.cli(['automation', 'add', 'Echo', '--task', 'Echo', '--on', 'added'], { chat: place })
    await src.cli(['merge', t.id], {})
    await until(() => tasks().some((x) => x.title === 'Write notes'))
    expect(tasks().filter((x) => x.title === 'Echo')).toHaveLength(0)
    // from the screen: paused, run now, removed; changed only by the command
    expect(await src.act({ type: 'automation', place, op: 'pause', name: 'Nightly' })).toEqual({ ok: true })
    expect(src.automations.dueNow(src.settings.zone(), Date.now() + 3_600_000).map((a) => a.name)).not.toContain('Nightly')
    expect(await src.act({ type: 'automation', place, op: 'run', name: 'Nightly' })).toEqual({ ok: true })
    expect(tasks().filter((x) => x.title === 'Bump deps')).toHaveLength(2)
    await src.cli(['automation', 'edit', 'Nightly', '--when', 'FREQ=DAILY;BYHOUR=3;BYMINUTE=0', '--task', 'Bump all deps', '--brief', ''], { chat: place })
    expect(src.automations.find(place, 'Nightly')).toMatchObject({ trigger: { time: 'DTSTART:20260101T000000 RRULE:FREQ=DAILY;BYHOUR=3;BYMINUTE=0' }, task: { title: 'Bump all deps', agent: { kind: 'codex' } }, on: false })
    expect(src.automations.find(place, 'Nightly')!.task.desc).toBeUndefined()
    expect((await src.cli(['automation', 'add', 'Bad', '--task', 'x', '--when', 'sometimes'], { chat: place })).out).toContain("Can't read the time rule")
    // the old flags are gone
    expect((await src.cli(['automation', 'add', 'Old', '--task', 'x', '--schedule', 'daily 9:00'], { chat: place })).out).toContain('takes no --schedule')
    // the screen adds or changes none
    expect(await src.act({ type: 'automation', place, op: 'save', name: 'Nightly' } as never)).toMatchObject({ ok: false })
    expect(await src.act({ type: 'automation', place, op: 'remove', name: 'Echo' })).toEqual({ ok: true })
    expect((await src.cli(['automation', 'list', '--space', name], {})).out).toMatch(/Nightly · Every day at 3:00 \(.+\) \[--when "DTSTART:20260101T000000 RRULE:FREQ=DAILY;BYHOUR=3;BYMINUTE=0"\] · adds “Bump all deps” \(codex\) · autos · paused · last ran/)
    // the old kinds are gone
    expect((await src.cli(['automation', 'add', 'Ask', '--ask', 'What waits?', '--when', 'FREQ=DAILY'], { chat: place })).out).toContain('takes no --ask')
  })
})

describe('adding a space from the sidebar', () => {
  const home = mkdtempSync(join(tmpdir(), 'shep-home-'))
  const realHome = process.env.HOME
  beforeAll(() => {
    process.env.HOME = home
  })
  afterAll(() => {
    process.env.HOME = realHome
  })

  test('on this machine: a new folder becomes a repo and a space, like shepherdr space add', async () => {
    const r = await src.addSpace({ machine: HOST, path: '~/dev/fresh', name: 'fresh-app', create: true })
    expect(r.place).toBe(placeId.space('fresh-app'))
    expect(src.places.byName('fresh-app')?.path).toEndWith('/dev/fresh')
    expect(src.snapshot().places.some((p) => p.id === r.place)).toBe(true)
    const f = await src.folders(HOST, '~/dev')
    expect(f.entries).toEqual([{ name: 'fresh', path: '~/dev/fresh', repo: true, added: true }])
  })
  test('a folder that is not a repo is not added without create; nothing outside home either', async () => {
    mkdirSync(join(home, 'plain'))
    await expect(src.addSpace({ machine: HOST, path: '~/plain' })).rejects.toThrow('not a git repository')
    await expect(src.addSpace({ machine: HOST, path: '/etc', create: true })).rejects.toThrow('Only folders in your home')
    await expect(src.addSpace({ machine: HOST, path: '~/plain', name: 'a/b', create: true })).rejects.toThrow()
  })
  test('another machine: folders and adding pass through to its copy; one unknown or out of reach says so', async () => {
    const got: { path: string; body?: unknown }[] = []
    const far = Bun.serve({
      port: 0,
      hostname: '127.0.0.1',
      async fetch(req) {
        const u = new URL(req.url)
        got.push({ path: u.pathname + u.search, body: req.method === 'POST' ? await req.json() : undefined })
        if (u.pathname === '/api/snapshot') return Response.json({ name: 'S', host: 'far', places: [{ id: 'p:far:~', name: 'home', machine: 'far', home: true, level: 1, status: 'idle', mode: 'manual', chat: 'p:far:~' }], machines: [{ id: 'm:far', machine: 'far', name: 'far', status: 'idle' }], tasks: [], automations: {}, asks: {}, settings: [], overrides: {}, user: 'u' })
        if (u.pathname === '/api/folders') return Response.json({ path: u.searchParams.get('path'), exists: true, dir: true, repo: null, entries: [] })
        if (u.pathname === '/api/spaces') return Response.json({ error: 'Another space on far has that name.' }, { status: 500 })
        return new Response('no', { status: 404 })
      },
    })
    try {
      src.peers!.adopt('far', `http://127.0.0.1:${far.port}`)
      for (let i = 0; i < 50 && !src.peers!.list().find((l) => l.rec.machine === 'far')?.reachable; i++) await new Promise((r) => setTimeout(r, 20))
      // an older helper there still says Manual: read as regular
      expect(src.snapshot().places.find((p) => p.id === 'p:far:~')?.mode).toBe('regular')
      expect(src.snapshot().machines.find((m) => m.id === 'm:far')?.status).toBe('idle')
      expect((await src.folders('far', '~/dev')).path).toBe('~/dev')
      expect(got.some((g) => g.path === `/api/folders?machine=far&path=${encodeURIComponent('~/dev')}`)).toBe(true)
      await expect(src.addSpace({ machine: 'far', path: '~/x', name: 'x' })).rejects.toThrow('Another space on far has that name.')
      expect(got.find((g) => g.path === '/api/spaces')?.body).toEqual({ machine: 'far', path: '~/x', name: 'x' })
      await expect(src.folders('nowhere', '~')).rejects.toThrow('No machine called nowhere')
    } finally {
      far.stop(true)
    }
  })
})

describe('removing a space', () => {
  test('the command and the sidebar forget it, its folder stays; not while one of its tasks works; added again, its tasks come back', async () => {
    const { t, dir, place } = await workedTask('forgotten')
    const statusOf = src.engine.statusOf.bind(src.engine)
    src.engine.statusOf = (x) => (x.id === t.id ? 'work' : statusOf(x))
    try {
      const busy = await src.cli(['space', 'remove', 'forgotten'], {})
      expect(busy.code).toBe(1)
      expect(busy.out).toContain(`${t.id} is still working: pause or finish it first.`)
      expect(await src.act({ type: 'space-remove', place })).toEqual({ ok: false, error: `${t.id} is still working: pause or finish it first.` })
    } finally {
      src.engine.statusOf = statusOf
    }
    expect(src.places.byName('forgotten')).toBeDefined()
    expect(await src.act({ type: 'space-remove', place })).toEqual({ ok: true })
    expect(src.places.byName('forgotten')).toBeUndefined()
    expect(existsSync(join(dir, 'README.md'))).toBe(true)
    expect(src.snapshot().places.some((p) => p.id === place)).toBe(false)
    expect(src.snapshot().tasks.some((x) => x.id === t.id)).toBe(false)
    expect(await src.act({ type: 'space-remove', place })).toEqual({ ok: false, error: 'That space is gone already.' })
    expect((await src.cli(['space', 'remove', 'forgotten'], {})).out).toContain('No space called forgotten')
    expect((await src.cli(['space', 'remove'], {})).out).toContain('shepherdr space remove <name>')
    // added again (project, from before spaces, still works): its task is back
    await src.cli(['project', 'add', dir, '--name', 'forgotten'], {})
    expect(src.snapshot().tasks.some((x) => x.id === t.id)).toBe(true)
    const r = await src.cli(['space', 'remove', 'forgotten'], {})
    expect(r.code).toBe(0)
    expect(r.out.trim()).toBe(`Removed forgotten from Shepherdr; ${dir} and its code stay on disk. shepherdr space add ${dir} --name forgotten brings it back with its open tasks and chat.`)
  })
  test("the home space can be renamed, never removed: by the sidebar or the command", async () => {
    const home = placeId.home()
    const says = `home is ${NAME}'s home space: it can be renamed, not removed.`
    expect(await src.act({ type: 'space-remove', place: home })).toEqual({ ok: false, error: says })
    const r = await src.cli(['space', 'remove', 'home'], {})
    expect(r.code).toBe(1)
    expect(r.out).toContain(says)
    expect(src.snapshot().places.find((p) => p.id === home)).toMatchObject({ home: true, name: 'home' })
  })
  test('shepherdr space add refuses a name another space has, or one that breaks the rules', async () => {
    const repo = () => {
      const dir = mkdtempSync(join(tmpdir(), 'shep-src-taken-'))
      execSync('git init -q -b main', { cwd: dir })
      return dir
    }
    await src.places.add(repo(), 'taken')
    const before = src.places.spaces.length
    for (const name of ['taken', 'home']) {
      const r = await src.cli(['space', 'add', repo(), '--name', name], {})
      expect(r.code).toBe(1)
      expect(r.out).toContain('Another space on this machine has that name.')
    }
    const bad = await src.cli(['space', 'add', repo(), '--name', 'My App'], {})
    expect(bad.code).toBe(1)
    expect(bad.out).toContain('Lowercase only.')
    expect(src.places.spaces.length).toBe(before)
  })
})

describe('the order of places', () => {
  test('A to Z until dragged; the order is kept on disk, and a new space slots in A to Z', async () => {
    const add = async (name: string) => {
      const dir = mkdtempSync(join(tmpdir(), 'shep-src-order-'))
      execSync('git init -q -b main', { cwd: dir })
      await src.places.add(dir, name)
    }
    await add('order-zed')
    await add('order-alpha')
    const mine = () => src.snapshot().places.filter((p) => p.name.startsWith('order-')).map((p) => p.name)
    expect(mine()).toEqual(['order-alpha', 'order-zed'])
    const machine = placeId.machine()
    const ids = src.snapshot().places.filter((p) => !p.home && p.machine === HOST).map((p) => p.id)
    const dragged = [placeId.space('order-zed'), ...ids.filter((id) => id !== placeId.space('order-zed'))]
    expect(await src.act({ type: 'order', under: machine, ids: dragged })).toEqual({ ok: true })
    expect(mine()).toEqual(['order-zed', 'order-alpha'])
    // the machine's home space is always first, then its spaces in their order
    const places = src.snapshot().places
    expect(places.find((p) => p.machine === HOST)?.id).toBe(placeId.home())
    expect(places.findIndex((p) => p.name === 'order-zed')).toBe(places.findIndex((p) => p.id === placeId.home()) + 1)
    expect(new Places().order[machine]).toEqual(dragged)
    await add('order-beta')
    expect(mine()).toEqual(['order-zed', 'order-alpha', 'order-beta'])
    expect(await src.act({ type: 'order', under: machine, ids: 'nope' } as never)).toEqual({ ok: false, error: 'No order to keep.' })
  })
})

describe('renaming places', () => {
  test("only the name that shows changes, kept on disk; its id, folder and task key stay; a space's name follows the rules", async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-src-rename-'))
    execSync('git init -q -b main', { cwd: dir })
    const p = await src.places.add(dir, 'rename-me')
    const id = placeId.space('rename-me')
    const home = placeId.home()
    const machine = placeId.machine()
    const named = (x: string) => src.snapshot().places.find((y) => y.id === x)?.name
    // a space's name, as Slack's channels: lowercase, no spaces
    expect(await src.act({ type: 'rename', id, name: '  My   App ' })).toEqual({ ok: false, error: 'Lowercase only.' })
    expect(await src.act({ type: 'rename', id, name: 'my app' })).toEqual({ ok: false, error: 'No spaces: use - or _.' })
    expect(await src.act({ type: 'rename', id, name: 'my-app' })).toEqual({ ok: true })
    expect(named(id)).toBe('my-app')
    expect(src.chatName(id)).toBe('my-app')
    expect([p.name, p.path, src.places.space(id)?.key]).toEqual(['rename-me', dir, p.key])
    expect(new Places().names[id]).toBe('my-app')
    // the command too
    expect((await src.cli(['space', 'rename', 'my-app', 'Bad Name'], {})).code).toBe(1)
    expect((await src.cli(['space', 'rename', 'my-app', 'good_name'], {})).out).toContain('my-app is called good_name now.')
    expect(named(id)).toBe('good_name')
    // a machine's name is free text
    expect(await src.act({ type: 'rename', id: machine, name: 'Home box' })).toEqual({ ok: true })
    expect(src.snapshot().machines.find((m) => m.id === machine)).toMatchObject({ name: 'Home box', machine: HOST })
    // and its emoji, as a Slack section's: one emoji, kept on disk; '' takes it away, none leaves it; a space takes none
    expect(await src.act({ type: 'rename', id: machine, name: 'Home box', emoji: '🏠' })).toEqual({ ok: true })
    expect(src.snapshot().machines.find((m) => m.id === machine)?.emoji).toBe('🏠')
    expect(new Places().emojis[machine]).toBe('🏠')
    expect(await src.act({ type: 'rename', id: machine, name: 'Home box' })).toEqual({ ok: true })
    expect(src.snapshot().machines.find((m) => m.id === machine)?.emoji).toBe('🏠')
    for (const bad of ['hi', '🏠🚀', 'x🏠'])
      expect(await src.act({ type: 'rename', id: machine, name: 'Home box', emoji: bad })).toEqual({ ok: false, error: 'Only a machine takes an emoji, one at a time.' })
    expect(await src.act({ type: 'rename', id, name: 'my-app', emoji: '🚀' })).toEqual({ ok: false, error: 'Only a machine takes an emoji, one at a time.' })
    expect(await src.act({ type: 'rename', id: machine, name: 'Home box', emoji: '👍🏽' })).toEqual({ ok: true })
    expect(await src.act({ type: 'rename', id: machine, name: 'Home box', emoji: '' })).toEqual({ ok: true })
    expect(src.snapshot().machines.find((m) => m.id === machine)?.emoji).toBeUndefined()
    expect(new Places().emojis).toEqual({})
    // the home space can be renamed; another space on the machine can't take its name
    expect(await src.act({ type: 'rename', id, name: 'home' })).toEqual({ ok: false, error: 'Another space on this machine has that name.' })
    expect(await src.act({ type: 'rename', id: home, name: 'base' })).toEqual({ ok: true })
    expect(named(home)).toBe('base')
    expect(await src.act({ type: 'rename', id: 'all', name: 'Everything' })).toEqual({ ok: false, error: 'Nothing to rename.' })
    expect(await src.act({ type: 'rename', id, name: 'x'.repeat(81) })).toEqual({ ok: false, error: 'A name fits 80 characters.' })
    // empty: its own name again
    expect(await src.act({ type: 'rename', id, name: '' })).toEqual({ ok: true })
    expect(await src.act({ type: 'rename', id: home, name: '' })).toEqual({ ok: true })
    expect(await src.act({ type: 'rename', id: machine, name: '' })).toEqual({ ok: true })
    expect(named(id)).toBe('rename-me')
    expect(named(home)).toBe('home')
    expect(new Places().names).toEqual({})
  })
})

describe('artifacts from the command', () => {
  const files = mkdtempSync(join(tmpdir(), 'shep-src-art-'))
  writeFileSync(join(files, 'board.html'), '<h1>Board</h1>')
  writeFileSync(join(files, 'data.json'), '{"n":2}')
  const ok = async () => ({ errors: [], warnings: [], skipped: 'no browser in the test' })

  test('made in a task chat: a card there, kept in its project; an update is a new version, data alone keeps the page', async () => {
    src.checker = ok
    const { t, place } = await workedTask('arts')
    const chat = `${t.id}/Agent`
    const r = await src.cli(['artifact', 'new', 'board.html', '--title', 'Weekly board'], { chat, cwd: files })
    expect(r.code).toBe(0)
    expect(r.out).toContain('No pictures: no browser in the test')
    const id = r.out.match(/artifact (\w+) “Weekly board” v1/)![1]
    expect(src.snapshot().artifacts.find((a) => a.id === id)).toMatchObject({ title: 'Weekly board', place, v: 1, by: `${t.id} agent`, keeper: `${t.id} agent` })
    expect(src.chats.chat(chat).messages.find((m) => m.artifact?.id === id)?.artifact).toEqual({ id, v: 1 })
    // its coordinator brings new numbers: v2, the same page, a card in the coordinator's chat
    const u = await src.cli(['artifact', 'update', id, '--data', 'data.json', '--note', 'fresh'], { chat: place, cwd: files })
    expect(u.out).toContain(`${id} “Weekly board” is v2 (same page, new data)`)
    expect(await src.artifact(id)).toEqual({ html: '<h1>Board</h1>', data: { n: 2 }, sent: undefined })
    expect(src.chats.chat(place).messages.find((m) => m.artifact?.id === id)?.artifact).toEqual({ id, v: 2 })
    expect((await src.cli(['artifact', 'show', id], {})).out).toContain(`v2  now  arts  (data only)  fresh`)
    expect((await src.cli(['artifact', 'list'], {})).out).toContain(`${id}  v2  Weekly board  · arts`)
  })

  test('its card is a message of its own after the words it came with; a reaction or reply to it names it, and reads back on it', async () => {
    src.checker = ok
    const { t } = await workedTask('artre')
    const chat = `${t.id}/Agent`
    const id = (await src.cli(['artifact', 'new', 'board.html', '--title', 'Weekly board'], { chat, cwd: files })).out.match(/artifact (\w+)/)![1]
    const ev = (src.chats as unknown as { events: Map<string, unknown[]> }).events
    const log = (e: object) => ev.set(chat, [...(ev.get(chat) ?? []), { at: Date.now(), ...e }])
    // the agent's words of that turn come after it, and the card shows under them
    log({ kind: 'agent', id: 'w1', text: 'Here is the board.' })
    const msgs = () => src.chats.chat(chat).messages
    const at = msgs().findIndex((m) => m.id === 'w1')
    expect(msgs()[at]).not.toHaveProperty('artifact')
    const card = msgs()[at + 1]
    expect(card).toMatchObject({ who: 'agent', artifact: { id, v: 1 } })
    const sent: string[] = []
    const send = src.agents.send
    src.agents.send = (async (_k: string, text: string) => (sent.push(text), { waiting: false })) as typeof send
    try {
      expect(await src.act({ type: 'react', chat, message: card.id, emoji: '👍' })).toEqual({})
      await src.act({ type: 'send', chat, text: 'Bars in blue', reply: card.id })
    } finally {
      src.agents.send = send
    }
    expect(sent).toEqual([
      `<reaction from="${src.user}" emoji="👍" title="Weekly board" artifact="${id}" version="1"/>`,
      `<reply from="${src.user}" to="Weekly board · v1" artifact="${id}">Bars in blue</reply>`,
    ])
    log({ kind: 'user', id: 'u1', text: sent[0] })
    log({ kind: 'user', id: 'u2', text: sent[1] })
    expect(msgs().find((m) => m.id === card.id)?.reactions).toEqual([{ emoji: '👍', by: 'me' }])
    expect(msgs().find((m) => m.id === 'w1')?.reactions).toBeUndefined()
    expect(msgs().find((m) => m.id === 'u2')).toMatchObject({ text: 'Bars in blue', reply: { id: card.id, who: 'agent', text: 'Weekly board · v1', artifact: true } })
    // taken back
    log({ kind: 'user', id: 'u3', text: sent[0].replace('<reaction ', '<reaction-removed ') })
    expect(msgs().find((m) => m.id === card.id)?.reactions ?? []).toEqual([])
  })

  test('a page whose scripts break or that reaches for the network is refused, with what broke', async () => {
    src.checker = ok
    const id = (await src.cli(['artifact', 'new', 'board.html', '--title', 'Fine'], { cwd: files })).out.match(/artifact (\w+)/)![1]
    src.checker = async () => ({ errors: ['script error: x is not defined', 'blocked: https://cdn.example/x.js (artifacts load nothing: put it inline)'], warnings: [], shots: { light: '/p.png', dark: '/p-dark.png' } })
    const bad = await src.cli(['artifact', 'update', id, 'board.html'], { cwd: files })
    expect(bad.code).toBe(1)
    expect(bad.out).toContain('Not updated')
    expect(bad.out).toContain('- script error: x is not defined')
    expect(bad.out).toContain('/p.png')
    expect(src.artifacts.get(id)!.versions.length).toBe(1)
    src.checker = ok
  })

  test('its keeper: the agent that made it while its task is open, the coordinator once that task is merged', async () => {
    src.checker = ok
    const { t, place } = await workedTask('kept')
    const id = (await src.cli(['artifact', 'new', 'board.html', '--title', 'Kept'], { chat: `${t.id}/Agent`, cwd: files })).out.match(/artifact (\w+)/)![1]
    expect(src.snapshot().artifacts.find((a) => a.id === id)!.keeper).toBe(`${t.id} agent`)
    await src.cli(['merge', t.id], {})
    expect(src.snapshot().artifacts.find((a) => a.id === id)!.keeper).toBe('kept')
  })

  test('render and preview are gone and say what replaced them', async () => {
    for (const c of ['render', 'preview']) {
      const r = await src.cli([c, 'board.html'], { cwd: files })
      expect(r.code).toBe(1)
      expect(r.out).toContain('shepherdr artifact new')
    }
  })
})

describe("another machine's artifacts", () => {
  test('listed, shown, pictured and acted on through the main machine, by their place', async () => {
    const got: { path: string; body?: unknown }[] = []
    const art = { id: 'farx', title: 'Far board', place: 'p:far2:proj', at: 1, v: 2, when: '1m', by: 'proj', keeper: 'proj', versions: [] }
    const far = Bun.serve({
      port: 0,
      hostname: '127.0.0.1',
      async fetch(req) {
        const u = new URL(req.url)
        got.push({ path: u.pathname + u.search, body: req.method === 'POST' ? await req.json() : undefined })
        if (u.pathname === '/api/snapshot')
          return Response.json({ name: 'S', host: 'far2', places: [{ id: 'p:far2:~', name: 'home', machine: 'far2', home: true, level: 1, status: 'idle', mode: 'manual', chat: 'p:far2:~' }], machines: [{ id: 'm:far2', machine: 'far2', name: 'far2', status: 'idle' }], tasks: [], automations: {}, artifacts: [art], asks: {}, settings: [], user: 'u' })
        if (u.pathname === '/api/artifacts/farx/raw') return Response.json({ html: `<p>far v${u.searchParams.get('v')}</p>`, data: { n: 1 } })
        if (u.pathname === '/api/artifacts/farx/thumb') return new Response('png', { headers: { 'content-type': 'image/png' } })
        if (u.pathname === '/api/action') return Response.json({ to: 'proj' })
        return new Response('no', { status: 404 })
      },
    })
    try {
      src.peers!.adopt('far2', `http://127.0.0.1:${far.port}`)
      for (let i = 0; i < 50 && !src.peers!.list().find((l) => l.rec.machine === 'far2')?.snap; i++) await new Promise((r) => setTimeout(r, 20))
      expect(src.snapshot().artifacts.find((a) => a.id === 'farx')).toMatchObject({ title: 'Far board', place: 'p:far2:proj' })
      expect(await src.artifact('farx', 2)).toEqual({ html: '<p>far v2</p>', data: { n: 1 } })
      expect(await (await src.artifactThumb('farx', true))!.text()).toBe('png')
      expect(got.some((g) => g.path === '/api/artifacts/farx/thumb?dark=1')).toBe(true)
      const a = { type: 'artifact-send' as const, place: 'p:far2:proj', artifact: 'farx', v: 2, text: 'Colour: Grey' }
      expect(await src.act(a)).toEqual({ to: 'proj' })
      expect(got.find((g) => g.path === '/api/action')?.body).toEqual(a)
      // one this machine doesn't know at all is nowhere
      expect(await src.artifact('nope')).toBeUndefined()
    } finally {
      far.stop(true)
    }
  })
})

describe("a task row's menu, and pin from the command", () => {
  test('pin and unpin: coordinators only, said plainly, shown in task list', async () => {
    const { t } = await workedTask('pins')
    expect((await src.cli(['task', 'pin', t.id], { chat: `${t.id}/Agent` })).code).toBe(1)
    expect((await src.cli(['task', 'pin', t.id], {})).out).toContain(`Pinned ${t.id}`)
    expect((await src.cli(['task', 'list'], {})).out).toContain('(pinned)')
    expect(JSON.parse((await src.cli(['task', 'list', '--json'], {})).out).find((x: { id: string }) => x.id === t.id).pinned).toBeNumber()
    await src.cli(['task', 'unpin', t.id], {})
    expect((await src.cli(['task', 'unpin', t.id], {})).out).toContain(`${t.id} isn't pinned.`)
    expect(await src.act({ type: 'pin', chat: `${t.id}/Agent`, on: true })).toMatchObject({ ok: true })
    expect(t.pinned).toBeNumber()
  })

  test('Mark done and Delete say what work stays unmerged, in words without paths, until forced', async () => {
    const { t } = await workedTask('rowmenu')
    expect(await src.act({ type: 'close', chat: `${t.id}/Agent` })).toEqual({ ok: false, held: [`work on ${t.branch} that main lacks`] })
    expect(await src.act({ type: 'delete', chat: `${t.id}/Agent` })).toMatchObject({ ok: false, held: [`work on ${t.branch} that main lacks`] })
    expect(src.engine.task(t.id)).toBe(t)
    expect(await src.act({ type: 'close', chat: `${t.id}/Agent`, force: true })).toMatchObject({ ok: true })
    expect(t.closed).toBeTruthy()
    // closed already: said, not swallowed
    expect(await src.act({ type: 'close', chat: `${t.id}/Agent` })).toMatchObject({ ok: false, error: expect.stringContaining('Done already (nothing to merge)') })
    expect(await src.act({ type: 'delete', chat: `${t.id}/Agent`, force: true })).toMatchObject({ ok: true })
    expect(src.engine.task(t.id)).toBeUndefined()
  })
})

describe('Autopilot from the command', () => {
  test('three settings and no limits: the heartbeat, the news that wakes it and its checklist; no budget anywhere', async () => {
    const { place } = await workedTask('piloting')
    src.places.setMode(place, 'autopilot')
    try {
      // the coordinator, on its own: Shepherdr adds what it asks; the machine's max tasks at once bounds the rest
      expect((await src.cli(['task', 'add', 'Fix the typo', '--desc', 'Fix it.'], { chat: place })).out).toContain('Added')
      expect((await src.cli(['task', 'add', 'Fix the link', '--desc', 'Fix it.'], { chat: place })).out).toContain('Added')
      // a checklist is a space's own (the home space's too), not a default Everywhere or the machine passes down
      for (const s of src.snapshot().settings.filter((x) => x.name === 'Autopilot')) expect(s.rows.map((r) => r.label)).toEqual(['Heartbeat', 'Wakes at once on', ...(s.level?.startsWith('p:') ? ['Checklist'] : [])])
      const show = (await src.cli(['show'], {})).out
      expect(show).toContain('heartbeat.when')
      expect(show).toContain('heartbeat.on')
      expect(show).toContain('time.zone')
      expect(show).not.toMatch(/tasksPerDay|wake\.|used today|budget/)
    } finally {
      src.places.setMode(place, 'regular')
    }
  })
})

describe('a turn the provider cut off, end to end', () => {
  /** a Claude Code log whose last turn the API cut off: one more line for each try that is cut off too */
  const cutLog = (dir: string) => {
    const f = join(dir, 'log.jsonl')
    let n = 0
    const cut = () => {
      const at = new Date().toISOString()
      const id = `00000000-0000-4000-8000-${String(++n).padStart(12, '0')}`
      const line = (o: object) => `${JSON.stringify(o)}\n`
      appendFileSync(f, line({ type: 'user', isSidechain: false, uuid: `u-${id}`, timestamp: at, sessionId: 's', message: { role: 'user', content: 'go on' } }))
      appendFileSync(f, line({ type: 'assistant', isSidechain: false, uuid: id, timestamp: at, error: 'server_error', isApiErrorMessage: true, apiErrorStatus: 529, message: { id: `m-${id}`, role: 'assistant', model: '<synthetic>', stop_reason: 'stop_sequence', content: [{ type: 'text', text: 'API Error: 529 Overloaded.' }] } }))
    }
    return { f, cut }
  }

  test("a coordinator's cut-off turn is tried again after each wait, then it says so in its chat, with an alert", async () => {
    const key = placeId.space('cutco')
    const dir = mkdtempSync(join(tmpdir(), 'shep-cut-'))
    const log = cutLog(dir)
    src.agents.records[key] = { key, name: 'shep-cutco-test', kind: 'claude', cwd: dir, workspace: 'cutco', logs: [log.f], started: 0 }
    Object.assign(src.retries, { waits: [30, 30, 30, 30] })
    const sent: string[] = []
    const send = src.agents.send
    src.agents.send = (async (k: string, text: string) => {
      // what other tests' news sends meanwhile aside
      if (k === key) sent.push(`${k} ${text}`)
      return { waiting: false, item: { id: 'x', key: k, text, at: Date.now() } }
    }) as never
    const alerts: { title: string; body: string }[] = []
    // other tests' tasks may still alert meanwhile: only this chat's are looked at
    const off = src.on('alert', (a) => void (a.tag === `cut:${key}` && alerts.push(a)))
    try {
      for (let i = 1; i <= 5; i++) {
        log.cut()
        src.agents.emit('status', key, 'idle')
        expect(src.retries.get(key)?.tries).toBe(i)
        if (i === 5) break
        // told to carry on once its wait is over; the line it is told is no message of yours: the count goes on
        await until(() => sent.length === i)
        src.agents.emit('sent', { id: `n${i}`, key, text: carryOn('Provider overloaded'), at: Date.now() })
      }
      expect(sent).toEqual(Array(4).fill(`${key} ${carryOn('Provider overloaded')}`))
      expect(src.retries.get(key)).toMatchObject({ over: true, reason: 'Provider overloaded, still after 5 tries' })
      expect(src.chats.log(key).posted.at(-1)?.card?.text).toBe('Cut off by its provider: provider overloaded, still after 5 tries. Write to carry on.')
      expect(alerts.at(-1)?.body).toStartWith('Provider overloaded, still after 5 tries.')
      // you write: it goes on, and the count starts again
      src.agents.emit('sent', { id: 'y', key, text: 'Go on please', at: Date.now() })
      expect(src.retries.get(key)).toBeUndefined()
    } finally {
      src.agents.send = send
      off()
      delete src.agents.records[key]
    }
  })

  test('a stopped task is never told to carry on, nor started again for it', async () => {
    const { t } = await workedTask('cutstop')
    const key = `${t.id}/Agent`
    const log = cutLog(mkdtempSync(join(tmpdir(), 'shep-cut-')))
    src.agents.records[key] = { key, name: 'shep-cutstop-test', kind: 'claude', cwd: t.worktree!, workspace: 'cutstop', logs: [log.f], started: 0 }
    Object.assign(src.retries, { waits: [30, 30, 30, 30] })
    const sent: string[] = []
    const send = src.agents.send
    src.agents.send = (async (k: string, text: string) => {
      if (k === key) sent.push(k)
      return { waiting: false, item: { id: 'x', key: k, text, at: Date.now() } }
    }) as never
    try {
      log.cut()
      src.agents.emit('status', key, 'idle')
      expect(src.retries.get(key)?.due).toBeGreaterThan(0)
      await src.engine.stopTask(t.id)
      // stopping it clears the wait; one seen again after that is let be when it is due
      expect(src.retries.get(key)).toBeUndefined()
      log.cut()
      src.agents.emit('status', key, 'idle')
      await Bun.sleep(80)
      expect(sent).toEqual([])
      expect(src.retries.get(key)).toBeUndefined()
    } finally {
      src.agents.send = send
    }
  })
})

describe("the home space's herdr space", () => {
  test('is home: the one the coordinators shared, still called Coordinators, is renamed in place, never a repo\'s, and no second one opens', async () => {
    const saved = { ...herdr }
    let spaces = [
      { workspace_id: 'w1', label: 'Coordinators', worktree: { checkout_path: '/r/Coordinators', is_linked_worktree: false, repo_key: '/r/Coordinators/.git', repo_name: 'Coordinators', repo_root: '/r/Coordinators' } },
      { workspace_id: 'w2', label: 'Coordinators' },
    ]
    const opened: string[] = []
    Object.assign(herdr, {
      workspaces: async () => spaces,
      renameWorkspace: async (id: string, label: string) => void (spaces = spaces.map((w) => (w.workspace_id === id ? { ...w, label } : w))),
      createWorkspace: async (label: string) => (opened.push(`new ${label}`), Promise.reject(new Error('herdr is not here'))),
      createTab: async (id: string) => (opened.push(id), Promise.reject(new Error('herdr is not here'))),
    })
    try {
      // its coordinator opens in the old space, renamed; next time it is found by its new name
      await (src as unknown as { coordinator(id: string): Promise<unknown> }).coordinator(placeId.home()).catch(() => undefined)
      expect(spaces.map((w) => w.label)).toEqual(['Coordinators', 'home'])
      expect((await src.agents.space('home', 'Coordinators'))?.workspace_id).toBe('w2')
      expect(opened).toEqual(['w2'])
      // with none left of either name, it is made under the new one
      spaces = []
      await (src as unknown as { coordinator(id: string): Promise<unknown> }).coordinator(placeId.home()).catch(() => undefined)
      expect(opened).toEqual(['w2', 'new home'])
    } finally {
      Object.assign(herdr, saved)
    }
  })
})

describe('machine health', () => {
  test("past a limit: a card in the home space's chat, news with the numbers for its coordinator, once; held tasks say why", async () => {
    const machine = placeId.home()
    const mon = src.health as unknown as { take: () => unknown; last?: unknown; watch: object }
    const take = mon.take
    const sent: { key: string; text: string }[] = []
    const send = src.agents.send
    src.agents.send = (async (key: string, text: string) => (sent.push({ key, text }), { waiting: false })) as typeof send
    src.places.setMode(machine, 'regular')
    const reach = src.agents.reachable
    mon.take = () => ({ at: 0, cores: 16, load: 2, memTotal: 100 * 2 ** 30, memUsed: 96 * 2 ** 30, disks: [{ path: '/tmp', pct: 40, freeGb: 60, dev: 1 }], gpus: [] })
    // browsers done tasks left open (never this machine's real ones): closed first, and counted in the news
    const browsers = src.browsers!
    const sweep = browsers.sweep
    let keep: ((t: string) => boolean) | undefined
    browsers.sweep = async (k) => ((keep = k), ['TG-4', 'TG-5'])
    try {
      src.agents.reachable = true
      src.healthTick()
      await until(() => sent.some((s) => s.key === machine && s.text.includes('<health ')))
      const card = src.chats.chat(machine).messages.find((m) => m.card?.health)
      expect(card?.card).toMatchObject({ text: 'memory is 96% used', health: { level: 'over', machine: HOST } })
      const news = sent.find((s) => s.key === machine && s.text.includes('<health '))!.text
      expect(news).toStartWith(`<shepherdr><health kind="memory" what="over">Machine ${HOST}: memory is over 90%: 96% in use.\nCPU 13% (load 2.0 on 16 cores) · Memory 96% (96 GB of 100 GB) · Disk /tmp 40% (60 GB free)\nTasks: `)
      expect(news).toContain('Using it most now:')
      expect(news).toContain('Shepherdr closed 2 shared browsers that finished tasks left open (TG-4, TG-5).')
      expect(keep!('TG-4')).toBe(false)
      expect(news).toContain('shepherdr task pause <id>')
      expect(news).toContain('Act on it if the human asked you to handle such things. If it can’t wait, alert the human with shepherdr alert and say why. A card in this chat already shows the human the news: reply after it in one or two short lines')
      // only the home space hears it: no coordinator of All machines any more
      expect(sent.filter((s) => s.text.includes('<health ')).map((s) => s.key)).toEqual([machine])
      expect(news).toContain('Never <no-reply> to it.')
      expect(news).not.toMatch(/recommend|do not act/i)
      // shown on the machine: red memory, amber is nothing else
      const h = src.snapshot().machines.find((m) => m.id === placeId.machine())!.health!
      expect(h.meters.find((m) => m.name === 'Memory')).toMatchObject({ pct: 96, level: 'over' })
      // told once
      const n = sent.length
      src.healthTick(Date.now() + 31_000)
      await Bun.sleep(1200)
      expect(sent.slice(n).filter((s) => s.text.includes('<health '))).toEqual([])
      // new tasks wait for the machine, and the command says so
      const dir = mkdtempSync(join(tmpdir(), 'shep-src-held-'))
      execSync('git init -q -b main && git -c user.email=t@t -c user.name=T commit -q --allow-empty -m init', { cwd: dir, shell: '/bin/sh' })
      await src.places.add(dir, 'held')
      const r = await src.cli(['task', 'add', 'Wait for room', '--desc', 'Add y.txt.', '--project', 'held'], { chat: placeId.space('held') })
      expect(r.out).toContain('held: the machine is busy')
      expect(r.out).toContain("starts by itself when there's room")
      expect((await src.cli(['show'], {})).out).toContain(`${HOST} now: CPU 13%`)
      // back under: told so
      mon.take = () => ({ at: 0, cores: 16, load: 2, memTotal: 100 * 2 ** 30, memUsed: 50 * 2 ** 30, disks: [{ path: '/tmp', pct: 40, freeGb: 60, dev: 1 }], gpus: [] })
      src.healthTick(Date.now() + 62_000)
      await until(() => sent.some((s) => s.key === machine && s.text.includes('memory is back under 90%')))
    } finally {
      browsers.sweep = sweep
      src.agents.send = send
      src.agents.reachable = reach
      src.places.setMode(machine, 'regular')
      mon.take = take
      mon.last = undefined
    }
  })

  test("herdr's socket refusing is said plainly", async () => {
    const e = await herdr.ping().catch((x: Error) => x)
    expect((e as Error).message).toBe("herdr isn't answering")
  })
})

describe('unread', () => {
  test("something new in a chat shows unread until read; a new message alerts as one (only Notify All lets it buzz), a task's card doesn't", async () => {
    const key = placeId.home()
    const alerts: import('../shared/types.ts').Alert[] = []
    const off = src.on('alert', (a) => void (a.message && alerts.push(a)))
    const read0 = src.snapshot().read!['*']
    expect(read0).toBeGreaterThan(0)
    src.chats.post(key, { id: 'unread-q', question: { id: 'ask:unread', title: 'Clean the disk now?', kind: 'Question', choices: [{ label: 'Yes' }, { label: 'Later' }], to: key } })
    const at = src.snapshot().last![key]
    expect(at).toBeGreaterThan(read0)
    expect(alerts.map((a) => [a.title, a.body, a.go.place])).toEqual([[src.chatName(key), 'Clean the disk now?', key]])
    // a card about a task: unread, but its task's own alert tells of it
    src.chats.post(key, { id: 'unread-card', card: { id: 'unread-card', task: 'X-1', status: 'done', text: 'X-1 is merged' } })
    expect(src.snapshot().last![key]).toBeGreaterThanOrEqual(at)
    expect(alerts).toHaveLength(1)
    // a task's chat: never unread, and no new-message alert (its own alerts say what it needs)
    src.chats.post('X-1/Agent', { id: 'unread-task', question: { id: 'ask:unread-task', title: 'Which port?', kind: 'Question', choices: [{ label: '80' }, { label: '443' }], to: 'X-1' } })
    expect(src.snapshot().last!['X-1/Agent']).toBeUndefined()
    expect(alerts).toHaveLength(1)
    off()
    // read on one device: every device's snapshot has it
    await src.act({ type: 'read', chats: { [key]: src.snapshot().last![key] } })
    expect(src.snapshot().read![key]).toBe(src.snapshot().last![key])
    // never back, and never the '*' mark
    await src.act({ type: 'read', chats: { [key]: 1, '*': 1 } })
    expect(src.snapshot().read![key]).toBe(src.snapshot().last![key])
    expect(src.snapshot().read!['*']).toBe(read0)
  })
})

describe('startup menus before the first message', () => {
  test('unparsed blockers preserve the screen and offer the terminal; changed cards cannot send keys', async () => {
    const key = 'startup-fixture'
    const originals = { info: src.agents.info, status: src.agents.status, visible: herdr.visible, refresh: src.agents.refresh, keys: src.agents.keys }
    let screen = 'A new CLI asks for a secret\nType your answer to continue'
    let sent = 0
    src.agents.info = (() => ({ pane_id: 'startup-pane', agent: 'claude', agent_status: 'blocked', cwd: '/tmp', tab_id: 'startup-tab', workspace_id: 'startup-space' })) as typeof src.agents.info
    src.agents.status = (() => 'blocked') as typeof src.agents.status
    herdr.visible = async () => screen
    src.agents.refresh = async () => {}
    src.agents.keys = async () => { sent++; screen = 'Ready for your message' }
    try {
      await src.chats.readMenu(key, true)
      const fallback = src.chats.chat(key).messages.find((m) => m.question)?.question
      expect(fallback?.screen).toBe(screen)
      expect(fallback?.terminal).toBe(true)
      expect(fallback?.noNote).toBe(true)
      expect(fallback?.choices).toEqual([])
      screen = 'Allow this tool? [y/n]'
      await src.chats.readMenu(key, true)
      expect(await src.act({ type: 'answer', chat: key, question: fallback!.id, choice: 0 })).toMatchObject({ ok: false })
      expect(sent).toBe(0)
      const q = src.chats.chat(key).messages.find((m) => m.question)?.question
      expect(await src.act({ type: 'answer', chat: key, question: q!.id, choice: -1 })).toMatchObject({ ok: false })
      expect(sent).toBe(0)
      // A checkbox redraw is still the same open prompt, not proof of a successful submission.
      screen = '2 new MCP servers found in this project\n❯ [✔] one\n  [✔] two\n     Enable selected\nSpace to select · Esc to reject all'
      await src.chats.readMenu(key, true)
      const checkboxes = src.chats.chat(key).messages.find((m) => m.question)?.question
      src.agents.status = (() => 'idle') as typeof src.agents.status
      expect(await src.agents.send(key, 'Wait until MCP consent closes')).toMatchObject({ waiting: true, waitsFor: 'menu' })
      src.agents.outbox = src.agents.outbox.filter((o) => o.key !== key)
      src.agents.keys = async () => { screen = screen.replace('[✔] one', '[ ] one') }
      expect(await src.act({ type: 'answer', chat: key, question: checkboxes!.id, choice: 1, choices: [1] })).toMatchObject({ ok: false, error: expect.stringContaining('still shows this prompt') })
      expect(src.chats.log(key).posted.some((m) => m.answered)).toBe(false)
    } finally {
      Object.assign(src.agents, { info: originals.info, status: originals.status, refresh: originals.refresh, keys: originals.keys })
      herdr.visible = originals.visible
      src.agents.outbox = src.agents.outbox.filter((o) => o.key !== key)
      src.chats.forget(key)
    }
  })
})

describe('visible launch controls', () => {
  test('coordinator and task metadata comes from the launch record, never new Settings; old actions cannot claim an unapplied change', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-controls-'))
    const p = await src.places.add(dir, 'launch-controls')
    const place = placeId.space(p.name)
    const t = src.engine.add({ project: p.name, title: 'Launch controls', by: 'test', later: true })
    const keys = [place, `${t.id}/Agent`]
    try {
      src.settings.set('coordinator.agent', 'Claude Code', { space: p.name })
      src.settings.set('task.agent', 'Claude Code', { space: p.name })
      for (const key of keys) {
        src.agents.records[key] = { key, name: `launch-${key}`, kind: 'codex', model: 'gpt-6-sol', effort: 'high', cwd: dir, workspace: 'launch', logs: [], started: 0 }
        expect(src.chat(key)?.agent).toEqual({ kind: 'codex', model: 'gpt-6-sol', modelLabel: 'GPT-6-Sol', effort: 'high' })
        const before = { ...src.agents.records[key] }
        expect(await src.act({ type: 'model', chat: key, model: 'gpt-6-luna' })).toMatchObject({ ok: false, error: expect.stringContaining('Restart') })
        expect(await src.act({ type: 'effort', chat: key, effort: 'low' })).toMatchObject({ ok: false })
        expect(src.agents.records[key]).toEqual(before)
        // Missing values are kept missing: no guessed model or effort, even when Settings specifies them.
        delete src.agents.records[key].model
        delete src.agents.records[key].effort
        expect(src.chat(key)?.agent).toEqual({ kind: 'codex' })
        const info = await src.act({ type: 'restart-info', chat: key }) as { info: { on: unknown; want: unknown } }
        expect(info.info.on).toEqual({ kind: 'codex' })
        expect(info.info.want).toMatchObject({ kind: 'claude' })
        const bad = await src.act({ type: 'restart', chat: key, kind: 'codex', model: 'no-longer-available', save: true })
        expect(bad).toMatchObject({ ok: false, error: expect.stringContaining('no available model') })
        expect(src.chat(key)?.agent).toEqual({ kind: 'codex' })
      }
    } finally {
      for (const key of keys) delete src.agents.records[key]
      src.settings.set('coordinator.agent', null, { space: p.name })
      src.settings.set('task.agent', null, { space: p.name })
    }
  })
})

test('native and npm agent CLI process identities', () => {
  expect(isAgentProcess('codex', ['/opt/bin/codex', '--resume'])).toBe(true)
  expect(isAgentProcess('pi', ['/usr/bin/node', '/opt/node_modules/@mariozechner/pi-coding-agent/dist/cli.js'])).toBe(true)
  expect(isAgentProcess('claude', ['/usr/bin/node', '/opt/node_modules/@anthropic-ai/claude-code/cli.js'])).toBe(true)
  expect(isAgentProcess('pi', ['/usr/bin/node', '/some/app.js', 'pi'])).toBe(false)
})

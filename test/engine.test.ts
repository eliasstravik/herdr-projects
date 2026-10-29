// The tasks engine on a real git repo, with the agents stubbed (engine-world.ts): a task starts its agent at once on
// its brief, and its status follows that agent alone (the list and the task's own chat always agree).
import { describe, expect, test } from 'bun:test'
import { existsSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { sh, repo, world, until, started, placeId, settingsBlock, writeJson, fakeHerdr } from './engine-world.ts'
const { fitStatus, reportOf, STALL_QUIET } = await import('../server/live/engine/engine.ts')

describe('recovered private routing failures', () => {
  const failure = 'Could not provision private agent command routing'
  async function failed(name: string) {
    const w = world()
    const ensure = w.agents.ensure
    w.agents.ensure = async (spec) => {
      await ensure(spec) // the same CLI survived the failed route provisioning
      throw new Error(failure)
    }
    const { t, key } = await started(w, repo(), name)
    await until(() => !!t.trouble)
    expect(w.engine.view(t)).toMatchObject({ status: 'need', outcome: 'failed', sub: 'Went wrong: Could not provision private' })
    return { w, t, key }
  }

  test('same agent recovers only after its private route is verified', async () => {
    const { w, t, key } = await failed('route-recovers')
    w.engine.report(t.id, 'Writing evidence', 80)
    expect(t.trouble).toBe(failure)
    w.routed[key] = true
    w.engine.agentStatus(key)
    w.engine.report(t.id, 'Still writing evidence', 80)
    expect(t.trouble).toBe(failure) // cached route + working status is not a fresh process check
    w.engine.routeRecovered(key) // route repair also notifies when status did not change
    expect(t.trouble).toBeUndefined()
    expect(w.engine.view(t)).toMatchObject({ status: 'work', sub: 'Working' })
    expect(w.engine.outcomeOf(t)).toBeUndefined()
    // Unrelated failures never disappear merely because the CLI is working.
    t.trouble = 'Could not make a worktree'
    w.engine.routeRecovered(key)
    expect(w.engine.outcomeOf(t)).toBe('failed')
  })

  test('a genuine retry failure remains failed', async () => {
    const { w, t, key } = await failed('route-retry-fails')
    w.agents.send = async () => { throw new Error(failure) }
    await expect(w.engine.tell(t.id, 'Try again')).rejects.toThrow(failure)
    w.engine.agentStatus(key)
    expect(t.trouble).toBe(failure)
    expect(w.engine.outcomeOf(t)).toBe('failed')
  })

  test('recovery preserves open questions and permission prompts', async () => {
    const { w, t, key } = await failed('route-question')
    w.chat.card = 'ask'
    w.chat.cardTitle = 'Which route?'
    expect(w.engine.view(t)).toMatchObject({ status: 'need', sub: 'Which route?', outcome: 'failed' })
    w.routed[key] = true
    w.engine.routeRecovered(key)
    expect(w.engine.view(t)).toMatchObject({ status: 'need', sub: 'Which route?' })
    expect(w.engine.outcomeOf(t)).toBeUndefined()
    w.chat.card = undefined
    w.chat.question = true
    expect(w.engine.view(t)).toMatchObject({ status: 'need', sub: 'Which one?' })
    w.chat.question = false
    w.agentIs(key, 'blocked')
    expect(w.engine.view(t)).toMatchObject({ status: 'need', sub: 'Asks permission' })
  })

  test('an old report and route cannot revive a stopped agent', async () => {
    const { w, t, key } = await failed('route-stopped')
    w.routed[key] = true
    w.agentIs(key, 'gone')
    w.engine.routeRecovered(key)
    w.engine.report(t.id, 'Writing evidence', 80)
    expect(w.engine.view(t)).toMatchObject({ status: 'need', outcome: 'failed' })
    expect(t.trouble).toBe(failure)
  })

  test('recovered work completes normally', async () => {
    const { w, t, key } = await failed('route-done')
    w.routed[key] = true
    w.engine.routeRecovered(key)
    w.engine.report(t.id, 'Writing evidence', 80)
    expect(t.trouble).toBeUndefined()
    expect(await w.engine.done(t.id, 'Investigation complete')).toBe('closed')
    expect(w.engine.view(t)).toMatchObject({ status: 'done', outcome: 'done', sub: 'Nothing to merge' })
  })
})

describe('a task is its brief and one agent', () => {
  test('adding a task starts its agent at once, in its own worktree from main, with the brief first', async () => {
    const w = world()
    const { p, t } = await started(w, repo(), 'starts')
    expect(t.id).toBe(`${p.key}-1`)
    expect(t.branch).toBe('work/1-say-hello')
    expect(existsSync(t.worktree!)).toBe(true)
    expect(sh('git branch --show-current', t.worktree!)).toBe('work/1-say-hello')
    const s = w.started[0]
    // the coordinator's words first, then the default way of working from Settings
    expect(s.first).toBe(`Say hello\n\nAdd hello.txt. Merge when done.\n\n${settingsBlock(t.id, { ways: w.settings.ways(), reviewer: w.settings.harness('reviewer'), agent: w.settings.harness('task'), by: w.settings.setBy() })}`)
    expect(s.first).toContain('Settings: (from shepherdr show; where the brief above says otherwise, it wins)')
    expect(t.desc).toStartWith('Add hello.txt. Merge when done.\n\nSettings:')
    expect(s.cwd).toBe(t.worktree!)
    // the prompt says where its skills are; how it works the task is theirs, not the prompt's
    expect(s.system).toContain('skill/shepherdr-agent/SKILL.md')
    expect(s.system).toContain(`task ${t.id}`)
    expect(w.engine.statusOf(t)).toBe('work')
    expect(w.news.some((n) => n.includes('was added'))).toBe(true)
  })

  test('starting, it shows To do with no second line until its agent is at work on the brief; no news or card meanwhile', async () => {
    const w = world(fakeHerdr(), { boot: 'idle' })
    const p = await w.places.add(repo(), 'boots')
    const t = w.engine.add({ project: p.name, title: 'Say hello', desc: 'Add hello.txt.', by: 'test' })
    const key = `${t.id}/Agent`
    // its worktree and space are still being made
    expect(w.engine.view(t)).toMatchObject({ status: 'todo', sub: '' })
    await until(() => w.started.some((s) => s.key === key))
    await Bun.sleep(20)
    // the CLI is up, its brief not in yet: still To do, even if herdr says it works (the CLI booting)
    expect(w.engine.view(t)).toMatchObject({ status: 'todo', sub: '' })
    w.agentIs(key, 'working')
    expect(w.engine.statusOf(t)).toBe('todo')
    // the brief goes in while it works: at work on it
    w.brief(key)
    expect(w.engine.view(t)).toMatchObject({ status: 'work', sub: 'Working' })
    expect(w.news.filter((n) => !n.includes('was added'))).toEqual([])
    expect(w.cards).toEqual([])
  })

  test('the brief goes in first and the agent gets to work after: Working then', async () => {
    const w = world(fakeHerdr(), { boot: 'idle' })
    const p = await w.places.add(repo(), 'boots2')
    const t = w.engine.add({ project: p.name, title: 'Say hello', desc: 'Add hello.txt.', by: 'test' })
    const key = `${t.id}/Agent`
    await until(() => w.started.some((s) => s.key === key))
    await Bun.sleep(20)
    w.brief(key)
    expect(w.engine.statusOf(t)).toBe('todo')
    w.agentIs(key, 'working')
    expect(w.engine.statusOf(t)).toBe('work')
    // its first turn ends: it waits on you as usual
    w.agentIs(key, 'idle')
    expect(w.engine.statusOf(t)).toBe('need')
  })

  test("the status follows the agent, and the list and the task's chat always show the same", async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'follows')
    const both = () => {
      const v = w.engine.view(t)
      expect(v.agent?.status).toBe(v.status)
      return v
    }
    expect(both().status).toBe('work')
    // working, its line is a standard word until its agent's status line takes over: never the brief, never its prose
    expect(both().sub).toBe('Working')
    w.agentIs(key, 'blocked')
    expect(both().status).toBe('need')
    expect(both().sub).toBe('Asks permission')
    // the agent got going again inside (a conflict it fixes, an answer it got): the list follows at once
    w.agentIs(key, 'working')
    expect(both().status).toBe('work')
    // its turn ended without reporting done: it waits for you
    w.agentIs(key, 'idle')
    expect(both().status).toBe('need')
    expect(both().sub).toBe('Which name do you want?')
    // it waits again with the same question in a later turn: that is news again
    w.agentIs(key, 'blocked')
    w.agentIs(key, 'working')
    const asked = w.news.filter((n) => n.includes('Asks permission')).length
    w.agentIs(key, 'blocked')
    expect(w.news.filter((n) => n.includes('Asks permission')).length).toBe(asked + 1)
    w.agentIs(key, 'working')
    // it reports done during a turn: still working until the turn ends, then ready
    w.agentIs(key, 'working')
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    await w.engine.done(t.id, 'Added hello.txt.')
    expect(both().status).toBe('work')
    w.agentIs(key, 'idle')
    expect(both().status).toBe('ready')
    expect(both().sub).toBe('Ready to merge')
    // writing to it again asks something of it: no longer done, and working even before it picks the message up
    w.engine.youWrote(key)
    expect(both().status).toBe('work')
    w.agentIs(key, 'working')
    expect(both().status).toBe('work')
  })

  test('coordinators hear once when it waits on you or is ready', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'news')
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    await w.engine.done(t.id, 'Did it.')
    w.agentIs(key, 'idle')
    await until(() => w.news.some((n) => n.includes('is ready to merge: Did it.')), 6000)
    w.agentIs(key, 'idle')
    await Bun.sleep(50)
    expect(w.news.filter((n) => n.includes('is ready')).length).toBe(1)
  })

  test("waiting on you shows a standard reason or the agent's own question, never a model's words: the same in the list, cards and news", async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'asks')
    w.chat.reply = 'The findings back up the design, so the plan stays as it is. I can start on the first step now, or wait while you read the notes. Which do you want?'
    w.agentIs(key, 'idle')
    await until(() => w.cards.length === 1)
    expect(w.engine.view(t).sub).toBe('Which do you want?')
    expect(w.cards[0].sub).toBe('Which do you want?')
    expect(w.news.at(-1)).toBe(`${t.id} needs you: Which do you want?`)
    expect(w.news.some((n) => n.includes('findings'))).toBe(false)
    // a long question is cut at a word to fit one line
    w.chat.reply = 'Should I rename every occurrence of the old setting across all the packages?'
    expect(w.engine.view(t).sub).toBe('Should I rename every occurrence')
    // words that ask nothing: the standard line, never the message
    w.chat.reply = 'A very long message that must never show up in the list of tasks or in the news.'
    expect(w.engine.view(t).sub).toBe('Waits for your reply')
  }, 10000)

  test('what its hooks report waits on you: a permission prompt, its own question tool, a plan, a lost login', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'hooked')
    const at = Date.now()
    w.status[key] = 'blocked'
    w.said[key] = { state: 'blocked', kind: 'permission', turnAt: at, at }
    expect(w.engine.statusOf(t)).toBe('need')
    expect(w.engine.view(t).sub).toBe('Asks permission')
    w.said[key] = { state: 'blocked', kind: 'question', ask: 'Which colour should the button be?', turnAt: at, at }
    expect(w.engine.view(t).sub).toBe('Which colour should the button be?')
    w.said[key] = { state: 'blocked', kind: 'question', turnAt: at, at }
    expect(w.engine.view(t).sub).toBe('Question for you')
    w.said[key] = { state: 'blocked', kind: 'plan', turnAt: at, at }
    expect(w.engine.view(t).sub).toBe('Plan to approve')
    w.status[key] = 'idle'
    w.said[key] = { state: 'idle', kind: 'auth', last: 'OAuth token has expired', turnAt: at, at }
    w.chat.reply = 'Done with the first part.'
    expect(w.engine.view(t).sub).toBe('Signed out')
  })

  test('a standard reason: a plan, a choice card, a question, a lost login', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'reasons')
    w.status[key] = 'idle'
    w.chat.card = 'plan'
    expect(w.engine.view(t).sub).toBe('Plan to approve')
    w.chat.card = 'ask'
    expect(w.engine.view(t).sub).toBe('Question for you')
    // a choice card says its own question
    w.chat.cardTitle = 'Rerun CI on main?'
    expect(w.engine.view(t).sub).toBe('Rerun CI on main?')
    w.chat.card = undefined
    w.chat.question = true
    w.status[key] = 'blocked'
    expect(w.engine.view(t).sub).toBe('Which one?')
    w.status[key] = 'idle'
    w.chat.question = false
    // a reply about logins is not a lost login
    for (const [r, sub] of [
      ['Fixed the bug where users got logged out after a refresh. Okay to merge?', 'Okay to merge?'],
      ['Now an invalid API key returns 401 instead of 500. Please run /login to try it?', 'Please run /login to try it?'],
      ['Users who are not logged in now see the sign-in page.', 'Waits for your reply'],
    ]) {
      w.chat.reply = r
      expect(w.engine.view(t).sub).toBe(sub)
    }
    w.chat.reply = 'API Error: 401 {"type":"error","error":{"type":"authentication_error","message":"OAuth token has expired."}} · Please run /login'
    expect(w.engine.view(t).sub).toBe('Signed out')
    w.agentIs(key, 'idle')
    await until(() => w.cards.length === 1, 6000)
    expect(w.news.at(-1)).toBe(`${t.id} needs you: Signed out`)
  }, 10000)

  test('a turn that ends while its own background work runs is still work: no news, no card', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'background')
    // it started its fresh-eyes reviewer in the background and ended its turn to wait for it
    w.chat.reply = 'Committed. A reviewer is checking it now.'
    w.bg[key] = 'agents'
    w.agentIs(key, 'done')
    expect(w.engine.statusOf(t)).toBe('work')
    expect(w.engine.view(t).sub).toBe('Working')
    await Bun.sleep(4300)
    expect(w.news.some((n) => n.includes('needs you'))).toBe(false)
    expect(w.cards.some((c) => c.status === 'need')).toBe(false)
    // shells or monitors alone are background work too
    w.bg[key] = 'shells'
    expect(w.engine.statusOf(t)).toBe('work')
    // a link is not a question
    w.chat.reply = 'The preview is up: https://box.ts.net/app?tab=2'
    expect(w.engine.statusOf(t)).toBe('work')
    // a question in its words alone is no fact: still at work while its background work runs
    w.chat.reply = 'The tests run in the background.\n\nWant me to merge once they pass?'
    expect(w.engine.statusOf(t)).toBe('work')
    // a choice card of its waits: that needs you, background or not
    w.chat.reply = 'Pick one.'
    w.chat.card = 'ask'
    expect(w.engine.statusOf(t)).toBe('need')
    w.chat.card = undefined
    // the reviewer finished and woke it; it fixed what it found and stopped with nothing left in the background
    w.agentIs(key, 'working')
    w.bg[key] = undefined
    w.chat.reply = 'Fixed both findings.'
    w.agentIs(key, 'done')
    await until(() => w.news.some((n) => n.includes('needs you')), 6000)
    expect(w.engine.statusOf(t)).toBe('need')
  }, 30_000)

  test('SHE-90: a turn that ends with only <no-reply> while its subagent runs is work, never needs you, and no news', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'paused-reviewer')
    // its reviewer's notice woke it ("waiting on its own background work") and it answered with a hidden reply only.
    // That its agent then reports a subagent in the background is tested on the real hooks and Agents
    // (hooks.test.ts, background.test.ts); this is the engine's half: such a turn is work, with no card and no news
    w.chat.reply = ''
    w.bg[key] = 'agents'
    w.agentIs(key, 'done')
    expect(w.engine.statusOf(t)).toBe('work')
    w.chat.reply = '<no-reply>Reviewer still running its checks; waiting for its result</no-reply>'
    expect(w.engine.statusOf(t)).toBe('work')
    expect(w.engine.view(t).sub).toBe('Working')
    await Bun.sleep(4300)
    expect(w.news.some((n) => n.includes('needs you'))).toBe(false)
    expect(w.cards.some((c) => c.status === 'need')).toBe(false)
  }, 30_000)

  test('SHE-193: a working task that goes quiet, or runs far past its time left, is stalled news: once per stall', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'stalls')
    w.agentIs(key, 'working')
    w.engine.report(t.id, 'Running the tests', 40, 10)
    const stalled = () => w.news.filter((n) => n.includes('may have stalled'))
    const later = (min: number) => Date.now() + min * 60_000
    w.engine.checkStalls(later(STALL_QUIET / 60_000 - 1))
    expect(stalled()).toEqual([])
    // quiet too long: what Shepherdr saw, once
    w.engine.checkStalls(later(STALL_QUIET / 60_000 + 1))
    expect(stalled().length).toBe(1)
    expect(stalled()[0]).toContain(`${t.id} may have stalled: its agent shows as working, but nothing has changed for 21m`)
    expect(stalled()[0]).toContain('Its last status: "Running the tests", 40% done, 21m ago')
    expect(stalled()[0]).toContain('herdr sees no agent CLI running in its pane')
    // and a one-line card in its project's chat
    expect(w.cards.filter((c) => c.status === 'work')).toEqual([{ task: t.id, status: 'work', sub: 'may be stuck: quiet for 21m' }])
    w.engine.checkStalls(later(STALL_QUIET / 60_000 + 30))
    expect(stalled().length).toBe(1)
    // it shows life again (its hooks moved): the next stall is news again
    w.said[key] = { state: 'working', turnAt: Date.now(), at: later(40) }
    w.engine.checkStalls(later(45))
    expect(stalled().length).toBe(1)
    w.engine.checkStalls(later(40 + STALL_QUIET / 60_000 + 1))
    expect(stalled().length).toBe(2)
    // never while it waits on you
    w.agentIs(key, 'idle')
    w.said[key] = undefined
    w.engine.checkStalls(later(500))
    expect(stalled().length).toBe(2)
  })

  test('SHE-193: busy all along but far past its own time left is stalled news too', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'overdue')
    w.agentIs(key, 'working')
    w.engine.report(t.id, 'Building', 50, 10)
    const stalled = () => w.news.filter((n) => n.includes('may have stalled'))
    // a sign of life every minute: never quiet
    const busy = (min: number) => ((w.said[key] = { state: 'working', turnAt: Date.now(), at: Date.now() + min * 60_000 }), w.engine.checkStalls(Date.now() + min * 60_000))
    busy(29)
    expect(stalled()).toEqual([])
    busy(31)
    expect(stalled()).toEqual([`${t.id} may have stalled: it has worked 31m since its agent said 10m were left. Its last status: "Building", 50% done, 31m ago; herdr sees no agent CLI running in its pane. Nudge it (shepherdr task tell ${t.id} "…"), stop it, or tell the human`])
    expect(w.cards.filter((c) => c.status === 'work').map((c) => c.sub)).toEqual(['may be stuck: 21m past its time left'])
    busy(60)
    expect(stalled().length).toBe(1)
  })

  test('a plan is a card in its chat; the answer goes back as a chat line', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'planned', 'Docs', 'Plan first and wait for my yes.')
    await w.engine.plan(t.id, ['One', 'Two'])
    const card = w.posted.find((x) => x.question?.kind === 'Plan')
    expect(card?.key).toBe(key)
    expect(card?.question?.steps).toEqual(['One', 'Two'])
    expect(w.engine.view(t).plan).toEqual(['One', 'Two'])
  })

  test('after a restart, what an open task shows is where it starts, not news', () => {
    writeJson('tasks.json', { next: { OLD: 2 }, tasks: [{ id: 'OLD-2', n: 2, project: 'p:x:old', title: 'Open', desc: '', by: 'x', created: 1, updated: 1, worktree: '/nowhere', stage: 'needs-you', round: 0, reviews: [], roles: ['Agent'] }] })
    const w = world()
    w.agentIs('OLD-2/Agent', 'blocked')
    expect(w.engine.statusOf(w.engine.task('OLD-2')!)).toBe('need')
    expect(w.news).toEqual([])
  })

  test('a task from before is read in: a finished one is merged', () => {
    writeJson('tasks.json', { next: { OLD: 1 }, tasks: [{ id: 'OLD-1', n: 1, project: 'p:x:old', title: 'Old', desc: '', by: 'x', created: 1, updated: 1, stage: 'done', round: 0, reviews: [], roles: ['Agent'] }] })
    const w = world()
    const t = w.engine.task('OLD-1')!
    expect(w.engine.statusOf(t)).toBe('done')
    expect('stage' in t).toBe(false)
    // no time it finished was kept then: its last update stands in
    expect(w.engine.view(t).doneAt).toBe(1)
    // its agent showing up again in herdr neither cleans it up again nor makes news; an old reviewer is put away
    w.status['OLD-1/Reviewer'] = 'idle'
    w.engine.agentStatus('OLD-1/Reviewer')
    expect(w.status['OLD-1/Reviewer']).toBe('gone')
    expect(t.cleaned).toBe(true)
    w.agentIs('OLD-1/Agent', 'idle')
    expect(w.records['OLD-1/Agent']).toBeUndefined()
    expect(w.news).toEqual([])
    // done before: no card for it
    expect(w.cards).toEqual([])
  })
})

describe("a task's CLI, model and effort", () => {
  test("Settings' for tasks, unless the coordinator chose others for this one; only on its first start", async () => {
    const w = world()
    w.settings.set('task.model', 'opus')
    const { t, key } = await started(w, repo(), 'harness-engine')
    // an alias goes in as the model it names now
    expect(w.started[0]).toMatchObject({ kind: 'claude', model: 'claude-opus-5-5', effort: undefined })
    expect(t.agent).toEqual({ kind: 'claude', model: 'claude-opus-5-5', effort: undefined })
    w.settings.set('task.model', null)
    const p = w.places.byName('harness-engine')!
    const t2 = w.engine.add({ project: p.name, title: 'On Codex', by: 'test', agent: { kind: 'codex', effort: 'low' } })
    await until(() => w.started.some((s) => s.key === `${t2.id}/Agent`))
    expect(w.started.find((s) => s.key === `${t2.id}/Agent`)).toMatchObject({ kind: 'codex', model: undefined, effort: 'low' })
    expect(w.engine.view(t2).on).toBe('Codex')
    // another CLI drops Settings' model, which names one of Claude Code's
    w.settings.set('task.model', 'claude-opus-5-5')
    const t3 = w.engine.add({ project: p.name, title: 'Codex too', by: 'test', agent: { kind: 'codex' } })
    expect(t3.agent).toEqual({ kind: 'codex', model: undefined, effort: undefined })
    const t4 = w.engine.add({ project: p.name, title: 'Claude', by: 'test', agent: { kind: 'claude', effort: 'max' } })
    expect(t4.agent).toEqual({ kind: 'claude', model: 'claude-opus-5-5', effort: 'max' })
    w.settings.set('task.model', null)
    await until(() => [t3, t4].every((x) => w.started.some((s) => s.key === `${x.id}/Agent`)))
    // started again later (its chat may have picked another model since): what its record holds counts
    w.agentIs(key, 'gone')
    await w.engine.resume(t.id)
    const again = w.started.filter((s) => s.key === key).at(-1)!
    expect('model' in again).toBe(false)
  })
})

describe("a task's own status line, % done and time left", () => {
  test('a status line longer than 40 characters is cut cleanly at a word', () => {
    expect(fitStatus('Fixing the two failing tests')).toBe('Fixing the two failing tests')
    expect(fitStatus('Fresh-eyes review running; awaiting plan approval for part 2')).toBe('Fresh-eyes review running; awaiting plan')
    // no dangling comma or "and" where it was cut
    expect(fitStatus('Updating file locations and running the checks')).toBe('Updating file locations and running')
    expect(fitStatus('Reading skill docs, searching old group names')).toBe('Reading skill docs, searching old group')
    expect(fitStatus('Moving the skills folder into the template and then')).toBe('Moving the skills folder')
    expect(fitStatus('Running checks for the whole workspace, and')).toBe('Running checks for the whole workspace')
    // one word too long for the line is cut where the line ends
    expect(fitStatus('x'.repeat(50))).toBe('x'.repeat(40))
    for (const t of ['a b c '.repeat(20), 'Rebasing onto main and rerunning all of the checks again']) expect(fitStatus(t).length).toBeLessThanOrEqual(40)
  })
  test('the minutes left count down while it works, stop while it waits on you, and nothing shows once its job is over', () => {
    const r = { msg: 'Fixing tests', progress: 40, left: 12, at: 0 }
    expect(reportOf(r, 'work', 0)).toEqual({ msg: 'Fixing tests', progress: 40, eta: 12, paused: undefined })
    expect(reportOf(r, 'work', 5 * 60_000)!.eta).toBe(7)
    expect(reportOf(r, 'work', 30 * 60_000)!.eta).toBe(0)
    expect(reportOf({ ...r, pausedAt: 5 * 60_000 }, 'need', 60 * 60_000)).toEqual({ msg: 'Fixing tests', progress: 40, eta: 7, paused: true })
    expect(reportOf({ ...r, left: undefined }, 'work', 0)!.eta).toBeUndefined()
    for (const s of ['ready', 'done', 'todo', 'queued'] as const) expect(reportOf(r, s, 0)).toBeUndefined()
    expect(reportOf(undefined, 'work')).toBeUndefined()
  })
})

describe('a turn the provider cut off', () => {
  type Retry = import('../server/live/retry.ts').Retry
  const cutOff = (w: ReturnType<typeof world>) => {
    const retries: Record<string, Retry | undefined> = {}
    w.engine.retry = (key) => retries[key]
    return retries
  }

  test('while it waits to try again it is still at work, saying why and when: no news, no card', async () => {
    const w = world()
    const retries = cutOff(w)
    const { t, key } = await started(w, repo(), 'retry-wait')
    retries[key] = { cut: 'e1', tries: 2, reason: 'Provider overloaded', words: 'Overloaded', due: Date.now() + 120_000 }
    w.agentIs(key, 'idle')
    await Bun.sleep(4100)
    expect(w.engine.view(t)).toMatchObject({ status: 'work', sub: 'Provider overloaded · retry in 2m' })
    expect(w.news.filter((n) => n.includes('needs you'))).toEqual([])
    // told to carry on: at work still while it takes that up
    retries[key] = { ...retries[key]!, due: undefined, nudged: Date.now() }
    expect(w.engine.statusOf(t)).toBe('work')
    // paused or stopped meanwhile: it is no longer told to, and shows so
    retries[key] = { ...retries[key]!, due: Date.now() + 60_000 }
    await w.engine.pause(t.id)
    expect(w.engine.mayRetry(key)).toBe(false)
    expect(w.engine.statusOf(t)).toBe('todo')
    w.engine.youWrote(key)
    expect(w.engine.mayRetry(key)).toBe(true)
    await w.engine.stopTask(t.id)
    expect(w.engine.mayRetry(key)).toBe(false)
  })

  test('reported done, a try still due does not make it Working', async () => {
    const w = world()
    const retries = cutOff(w)
    const { t, key } = await started(w, repo(), 'retry-ready')
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    await w.engine.done(t.id, 'Added hello.txt.')
    retries[key] = { cut: 'e1', tries: 1, reason: 'Provider overloaded', words: 'Overloaded', due: Date.now() + 60_000 }
    w.agentIs(key, 'idle')
    expect(w.engine.statusOf(t)).toBe('ready')
  })

  test('given up with no backup agent, it waits for you with the reason', async () => {
    const w = world()
    cutOff(w)
    w.settings.set('task.fallback', 'off')
    const { t, key } = await started(w, repo(), 'retry-over')
    w.engine.retry = () => ({ cut: 'e5', tries: 5, reason: 'Provider overloaded, still after 5 tries', words: 'Overloaded', over: true })
    w.agentIs(key, 'idle')
    await w.engine.cantGoOn(t.id, 'Provider overloaded, still after 5 tries')
    expect(w.engine.view(t)).toMatchObject({ status: 'need', sub: 'Provider overloaded, still after 5 tries' })
    expect(w.news).toContain(`${t.id} needs you: Provider overloaded, still after 5 tries`)
    expect(w.started.filter((s) => s.key === key)).toHaveLength(1)
    w.settings.set('task.fallback', null)
  })

  test('the backup agent takes over once, in the same worktree, with the brief and what was done so far', async () => {
    const w = world()
    cutOff(w)
    w.engine.installed = () => true
    const { t, key } = await started(w, repo(), 'retry-backup')
    sh('echo hi > hello.txt && git add -A && git commit -qm "Add hello.txt"', t.worktree!)
    writeFileSync(join(t.worktree!, 'more.txt'), 'more\n')
    w.chat.messages = [
      { who: 'you', text: 'Say hello\n\nAdd hello.txt. Merge when done.' },
      { who: 'agent', text: 'Added hello.txt; next the tests.' },
      { who: 'agent', card: {}, text: 'SHE-1 needs you' },
    ]
    w.agentIs(key, 'idle')
    await w.engine.cantGoOn(t.id, 'Out of allowance until 5pm')
    const s = w.started.filter((x) => x.key === key)
    expect(s).toHaveLength(2)
    expect(s[1]).toMatchObject({ kind: 'codex', model: 'default', cwd: t.worktree })
    expect(s[1].first).toStartWith('Say hello\n\nAdd hello.txt. Merge when done.')
    expect(s[1].first).toContain("Taking over: Claude Code worked on this task until now and can't go on (out of allowance until 5pm)")
    expect(s[1].first).toContain('Add hello.txt')
    expect(s[1].first).toContain('?? more.txt')
    expect(s[1].first).toContain('Claude Code: Added hello.txt; next the tests.')
    expect(s[1].first).not.toContain('needs you')
    expect(t.tookOver).toMatchObject({ from: 'claude', to: 'codex', why: 'out of allowance until 5pm' })
    expect(w.posted.some((p) => p.key === key && p.id.startsWith('takeover:'))).toBe(true)
    expect(w.engine.view(t)).toMatchObject({ status: 'work', on: 'Codex' })
    // the backup can't go on either: it waits for you
    w.agentIs(key, 'idle')
    w.engine.retry = () => ({ cut: 'e9', tries: 1, reason: 'Signed out', words: '401', over: true })
    await w.engine.cantGoOn(t.id, 'Signed out')
    expect(w.started.filter((x) => x.key === key)).toHaveLength(2)
    expect(w.engine.view(t)).toMatchObject({ status: 'need', sub: 'Signed out' })
  })
})

describe('restarting a task agent', () => {
  test('on its own CLI it resumes, interrupted first while it works, on the new model and effort, with a card', async () => {
    const w = world()
    w.engine.installed = () => true
    const { t, key } = await started(w, repo(), 'restart-same')
    w.agentIs(key, 'working')
    const on = await w.engine.restart(t.id, { kind: 'claude', model: 'sonnet', effort: 'low' })
    expect(on).toBe('Claude Code (Sonnet 5.5, low)')
    expect(w.stopped).toEqual([key])
    const s = w.started.filter((x) => x.key === key)
    expect(s).toHaveLength(2)
    expect(s[1]).toMatchObject({ kind: 'claude', model: 'claude-sonnet-5-5', effort: 'low', cwd: t.worktree })
    // a hidden line, not the brief again
    expect(s[1].first).toStartWith('<shepherdr><restart>')
    expect(s[1].first).toContain('restarted you on Claude Code (Sonnet 5.5, low), with your session as it was')
    expect(t.agent).toEqual({ kind: 'claude', model: 'claude-sonnet-5-5', effort: 'low' })
    expect(w.posted.find((p) => p.key === key && p.id.startsWith('restart:'))?.card).toMatchObject({ restart: true, text: 'Restarted on Claude Code (Sonnet 5.5, low)' })
  })

  test('on another CLI it takes over with the brief, the branch so far and the chat; the worktree and branch stay', async () => {
    const w = world()
    w.engine.installed = () => true
    const { t, key } = await started(w, repo(), 'restart-other')
    sh('echo hi > hello.txt && git add -A && git commit -qm "Add hello.txt"', t.worktree!)
    writeFileSync(join(t.worktree!, 'more.txt'), 'more\n')
    w.chat.messages = [
      { who: 'you', text: 'Say hello\n\nAdd hello.txt. Merge when done.' },
      { who: 'agent', text: 'Added hello.txt; next the tests.' },
    ]
    const head = sh('git rev-parse HEAD', t.worktree!)
    w.agentIs(key, 'idle')
    await w.engine.restart(t.id, { kind: 'codex' })
    const s = w.started.filter((x) => x.key === key)
    expect(s[1]).toMatchObject({ kind: 'codex', cwd: t.worktree })
    expect(s[1].first).toContain('Say hello\n\nAdd hello.txt. Merge when done.')
    expect(s[1].first).toContain('Claude Code ran it until now')
    expect(s[1].first).toContain('Add hello.txt')
    expect(s[1].first).toContain('?? more.txt')
    expect(s[1].first).toContain('Claude Code: Added hello.txt; next the tests.')
    expect(w.stopped).toEqual([])
    expect(sh('git rev-parse HEAD', t.worktree!)).toBe(head)
    expect(sh('git branch --show-current', t.worktree!)).toBe(t.branch!)
    expect(existsSync(join(t.worktree!, 'more.txt'))).toBe(true)
    expect(w.engine.view(t)).toMatchObject({ on: 'Codex' })
  })

  test('a Done task, or a CLI not installed here, is refused', async () => {
    const w = world()
    const { t } = await started(w, repo(), 'restart-refused')
    w.engine.installed = (k) => k === 'claude'
    await expect(w.engine.restart(t.id, { kind: 'codex' })).rejects.toThrow("Codex isn't installed")
    await w.engine.close(t.id)
    await expect(w.engine.restart(t.id, { kind: 'claude' })).rejects.toThrow('Done already')
  })
})

// To do, Queued, a task's place in herdr and archiving, on a real git repo (engine-world.ts).
import { describe, expect, test } from 'bun:test'
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import { join } from 'node:path'
import type { Task } from '../shared/types.ts'
import { sh, repo, world, until, started, placeId, writeJson, fakeHerdr } from './engine-world.ts'

describe('To do: its agent waits for a go', () => {
  const later = 'Add hi.txt. Don\'t start yet, wait for a go.'

  test('its agent starts at once on the brief; it shows To do once the agent says so, until it is told to go', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'saved', 'Say hi', later)
    expect(w.started.find((s) => s.key === key)?.first).toStartWith(`Say hi\n\n${later}\n\nSettings:`)
    expect(t.worktree).toBeTruthy()
    w.engine.todo(t.id)
    // To do at once, while the agent still ends the turn it said so in, and after
    expect(w.engine.statusOf(t)).toBe('todo')
    w.agentIs(key, 'idle')
    await Bun.sleep(50)
    expect(w.engine.statusOf(t)).toBe('todo')
    const v = w.engine.view(t)
    expect(v.agent?.chat).toBe(key)
    expect(v.agent?.status).toBe('todo')
    // a To do row is its title alone: no second line
    expect(v.sub).toBe('')
    expect(v.state).toMatch(/^To do · added /)
    // no card, no news that it waits on you
    expect(w.posted).toEqual([])
    expect(w.news.some((n) => n.includes('needs you'))).toBe(false)
    // the go is just a message to it: it is at work
    await w.engine.tell(t.id, 'Go ahead.')
    expect(w.sent.at(-1)).toEqual({ key, text: 'Go ahead.' })
    expect(w.engine.statusOf(t)).toBe('work')
  })

  test('talking to it leaves To do; only the agent puts it back, and one that stops otherwise needs you', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'talk', 'Say hi', later)
    w.engine.todo(t.id)
    w.agentIs(key, 'idle')
    // stuck on a question while it waits: that needs you
    w.agentIs(key, 'blocked')
    expect(w.engine.statusOf(t)).toBe('need')
    w.agentIs(key, 'idle')
    expect(w.engine.statusOf(t)).toBe('todo')
    // a change to the brief, in its chat
    w.engine.youWrote(key)
    expect(w.engine.statusOf(t)).toBe('work')
    w.agentIs(key, 'working')
    w.engine.todo(t.id)
    expect(w.engine.statusOf(t)).toBe('todo')
    // a go in its chat; its turn ends without it saying it waits again: it needs you, never To do
    await w.engine.tell(t.id, 'go')
    w.agentIs(key, 'idle')
    await until(() => w.engine.statusOf(t) === 'need', 12_000)
  }, 15_000)

  test('one saved for later with no agent, from before: its agent starts now and waits for a go', async () => {
    const p = await world().places.add(repo(), 'oldlater')
    const id = `${p.key}-1`
    const id2 = `${p.key}-2`
    const old = { project: placeId.space(p.name), by: 'x', created: 1, updated: 1, told: 'todo' }
    const says = "Later. Don't start yet, wait for a go."
    writeJson('tasks.json', { next: { [p.key]: 2 }, tasks: [{ ...old, id, n: 1, title: 'Saved', desc: 'For later.' }, { ...old, id: id2, n: 2, title: 'Says so', desc: says, after: id }] })
    const w = world()
    await until(() => w.started.some((s) => s.key === `${id2}/Agent`))
    expect(w.started.find((s) => s.key === `${id}/Agent`)?.first).toBe("Saved\n\nFor later.\n\nDon't start yet: wait for a go.")
    // its brief says so in its own words: the line goes in all the same; and what it once waited on is gone
    expect(w.started.find((s) => s.key === `${id2}/Agent`)?.first).toBe(`Says so\n\n${says}\n\nDon't start yet: wait for a go.`)
    expect('after' in w.engine.task(id2)!).toBe(false)
  })
})

describe('Queued after other tasks', () => {
  /** a first task, and a second queued after it (both started) */
  async function chain(name: string, after?: (first: string) => string[]) {
    const w = world()
    const dir = repo()
    const a = await started(w, dir, name, 'First', 'Add a.txt.')
    const b = w.engine.add({ project: a.p.name, title: 'Second', desc: 'Add b.txt.', by: 'test', after: after ? after(a.t.id) : [a.t.id] })
    const bkey = `${b.id}/Agent`
    await until(() => w.started.some((s) => s.key === bkey))
    return { w, dir, a, b, bkey }
  }
  const commit = (wt: string, file: string) => {
    writeFileSync(join(wt, file), `${file}\n`)
    sh(`git add -A && git commit -qm "add ${file}"`, wt)
  }

  test('its agent starts at once with the brief and the wait; it shows Queued, and goes by itself once that task is merged', async () => {
    const { w, a, b, bkey } = await chain('queued')
    expect(b.after).toEqual([a.t.id])
    const first = w.started.find((s) => s.key === bkey)!.first!
    // the brief, with the default way of working Shepherdr ends it with, then the wait
    expect(first.startsWith('Second\n\nAdd b.txt.\n\nSettings:')).toBe(true)
    expect(first).toContain('\n\nQueued: this task waits for')
    expect(first).toContain(a.t.id)
    // Queued while its agent reads the brief, and once its turn ends: never Needs you
    expect(w.engine.statusOf(b)).toBe('queued')
    w.agentIs(bkey, 'idle')
    await Bun.sleep(50)
    const v = w.engine.view(b)
    expect([v.status, v.agent?.status, v.sub]).toEqual(['queued', 'queued', `After ${a.t.id}`])
    expect(v.after).toEqual([a.t.id])
    expect(v.state).toMatch(/^Queued · /)
    expect(w.news.some((n) => n.startsWith(`${b.id} needs you`))).toBe(false)
    // the first is merged: the second's agent is told to go (hidden from its chat, a card says why it started)
    commit(a.t.worktree!, 'a.txt')
    await w.engine.done(a.t.id, 'Added a.txt.')
    expect(w.engine.statusOf(b)).toBe('queued')
    expect((await w.engine.merge(a.t.id, { self: true })).ok).toBe(true)
    const go = w.sent.find((s) => s.key === bkey)!
    expect(go.text).toBe(`<shepherdr><go>Go: ${a.t.id} is done, start now.\n${a.t.id} (First): Added a.txt.</go></shepherdr>`)
    expect(w.posted.some((x) => x.key === bkey && x.id.startsWith('go:'))).toBe(true)
    expect(b.after).toBeUndefined()
    expect(w.engine.statusOf(b)).toBe('work')
    // told once
    w.agentIs(a.key, 'idle')
    expect(w.sent.filter((s) => s.key === bkey).length).toBe(1)
  })

  test('done with nothing to merge (closed) counts as done; done with work to merge does not', async () => {
    const { w, a, b, bkey } = await chain('empty-done')
    commit(a.t.worktree!, 'a.txt')
    await w.engine.done(a.t.id, 'Added a.txt.')
    w.agentIs(a.key, 'idle')
    await until(() => w.engine.statusOf(a.t) === 'ready')
    expect(w.engine.statusOf(b)).toBe('queued')
    const r = await chain('research-done')
    await r.w.engine.done(r.a.t.id, 'Looked into it: nothing to change.')
    r.w.agentIs(r.a.key, 'idle')
    await until(() => r.w.sent.some((s) => s.key === r.bkey))
    expect(r.w.engine.statusOf(r.b)).toBe('work')
    void bkey
  }, 15_000)

  test('it waits for all of them, and its line names only those not done yet', async () => {
    const w = world()
    const dir = repo()
    const a = await started(w, dir, 'waits-all', 'First', 'Look into a.')
    const c = w.engine.add({ project: a.p.name, title: 'Other', desc: 'Look into c.', by: 'test' })
    const b = w.engine.add({ project: a.p.name, title: 'Third', desc: 'Then this.', by: 'test', after: [`${a.t.id.toLowerCase()}, ${c.id}`] })
    await until(() => w.started.some((s) => s.key === `${b.id}/Agent`) && w.started.some((s) => s.key === `${c.id}/Agent`))
    expect(b.after).toEqual([a.t.id, c.id])
    expect(w.engine.subOf(b)).toBe(`After ${a.t.id}, ${c.id}`)
    await w.engine.done(a.t.id, 'Nothing to change.')
    expect(w.engine.subOf(b)).toBe(`After ${c.id}`)
    expect(w.sent.some((s) => s.key === `${b.id}/Agent`)).toBe(false)
    await w.engine.done(c.id, 'Nothing here either.')
    await until(() => w.sent.some((s) => s.key === `${b.id}/Agent`))
    expect(w.sent.find((s) => s.key === `${b.id}/Agent`)!.text).toContain(`Go: ${a.t.id}, ${c.id} are done, start now.`)
  })

  test('a task it waits for is stopped or goes wrong: it needs you, with the reason; going again puts it back', async () => {
    const { w, a, b, bkey } = await chain('halts')
    w.agentIs(bkey, 'idle')
    await w.engine.stopTask(a.t.id)
    expect(w.engine.statusOf(b)).toBe('need')
    expect(w.engine.subOf(b)).toBe(`${a.t.id} was stopped`)
    expect(w.cards.some((c) => c.task === b.id && c.status === 'need' && c.sub === `${a.t.id} was stopped`)).toBe(true)
    expect(w.news).toContain(`${b.id} needs you: ${a.t.id} was stopped`)
    // the stopped task is written to again: the queued one waits again
    w.engine.youWrote(a.key)
    expect(w.engine.statusOf(b)).toBe('queued')
    a.t.trouble = 'could not make a worktree'
    w.agentIs(a.key, 'idle')
    expect(w.engine.subOf(b)).toBe(`${a.t.id} went wrong`)
    expect(w.sent.some((s) => s.key === bkey)).toBe(false)
  })

  test('a task it waits for is deleted: it needs you, saying so', async () => {
    const { w, a, b, bkey } = await chain('deleted')
    w.agentIs(bkey, 'idle')
    await w.engine.deleteTask(a.t.id)
    expect(w.engine.statusOf(b)).toBe('need')
    expect(w.engine.subOf(b)).toBe(`${a.t.id} is gone`)
  })

  test('a message reaches a queued task and leaves it queued: its agent clears the queue itself when it is a go', async () => {
    const { w, a, b, bkey } = await chain('early')
    w.agentIs(bkey, 'idle')
    w.engine.youWrote(bkey)
    expect(b.after).toEqual([a.t.id])
    expect(w.engine.statusOf(b)).toBe('queued')
    // the agent read a go in it
    expect(w.engine.queue(b.id, [], { self: true })).toEqual({ queued: [], went: true })
    w.agentIs(bkey, 'working')
    expect(w.engine.statusOf(b)).toBe('work')
    const told = w.sent.filter((s) => s.key === bkey).length
    await w.engine.done(a.t.id, 'Nothing to change.')
    expect(w.sent.filter((s) => s.key === bkey).length).toBe(told)
  })

  test('task queue: queues a task at work (its agent is told to hold on), changes what it waits for, and clears it', async () => {
    const w = world()
    const dir = repo()
    const a = await started(w, dir, 'requeue', 'First', 'Add a.txt.')
    const c = w.engine.add({ project: a.p.name, title: 'Other', desc: 'Add c.txt.', by: 'test' })
    const b = w.engine.add({ project: a.p.name, title: 'Second', desc: 'Add b.txt.', by: 'test' })
    const bkey = `${b.id}/Agent`
    await until(() => w.started.some((s) => s.key === bkey))
    expect(w.engine.queue(b.id, [a.t.id])).toEqual({ queued: [a.t.id] })
    expect(w.sent.at(-1)!.text).toContain(`Hold on: you are now queued after ${a.t.id}.`)
    expect(w.posted.some((x) => x.key === bkey && x.id.startsWith('queued:'))).toBe(true)
    expect(w.engine.statusOf(b)).toBe('queued')
    expect(w.engine.queue(b.id, [c.id])).toEqual({ queued: [c.id] })
    expect(w.sent.at(-1)!.text).toContain(`You now wait for ${c.id} instead.`)
    // its own agent queues itself: no message for it
    const n = w.sent.length
    w.engine.queue(b.id, [a.t.id, c.id], { self: true })
    expect(w.sent.length).toBe(n)
    expect(w.engine.queue(b.id, [])).toEqual({ queued: [], went: true })
    expect(w.sent.at(-1)!.text).toBe(`<shepherdr><go>Go: you no longer wait for ${a.t.id}, ${c.id}, start now.</go></shepherdr>`)
    expect(w.engine.statusOf(b)).toBe('work')
    // nothing to clear
    expect(w.engine.queue(b.id, [])).toEqual({ queued: [] })
  })

  test('refused: a task that is not there, itself, a circle; those done already are not waited for', async () => {
    const { w, a, b } = await chain('refused')
    expect(() => w.engine.queue(a.t.id, ['NOPE-9'])).toThrow('No task NOPE-9')
    expect(() => w.engine.queue(a.t.id, [a.t.id])).toThrow("can't wait for itself")
    expect(() => w.engine.queue(a.t.id, [b.id])).toThrow(`That would wait in a circle: ${a.t.id} → ${b.id} → ${a.t.id}`)
    expect(a.t.after).toBeUndefined()
    // an --after refused on add adds no task
    const before = w.engine.tasks.length
    expect(() => w.engine.add({ project: a.p.name, title: 'X', by: 'test', after: ['NOPE-9'] })).toThrow('No task NOPE-9')
    expect(w.engine.tasks.length).toBe(before)
    await w.engine.done(a.t.id, 'Nothing to change.')
    const d = w.engine.add({ project: a.p.name, title: 'After done', by: 'test', after: [a.t.id] })
    expect(d.after).toBeUndefined()
    expect(w.engine.statusOf(d)).toBe('todo')
  })

  test('waiting on you because one was stopped, a message leaves it to its agent to clear the queue', async () => {
    const { w, a, b, bkey } = await chain('halted-go')
    w.agentIs(bkey, 'idle')
    w.engine.youWrote(bkey, { go: false })
    expect(b.after).toEqual([a.t.id])
    await w.engine.stopTask(a.t.id)
    expect(w.engine.statusOf(b)).toBe('need')
    w.engine.youWrote(bkey)
    expect(b.after).toEqual([a.t.id])
    w.engine.queue(b.id, [], { self: true })
    expect(b.after).toBeUndefined()
  })

  test('after a restart: what it waits for is kept and checked again, and a go cut off is sent again', async () => {
    const p = await world().places.add(repo(), 'restart')
    const id = (n: number) => `${p.key}-${n}`
    const base = { project: placeId.space(p.name), by: 'x', created: 1, updated: 1 }
    writeJson('tasks.json', {
      next: { [p.key]: 4 },
      tasks: [
        { ...base, id: id(1), n: 1, title: 'Research', desc: '', reported: true, told: 'ready' },
        { ...base, id: id(2), n: 2, title: 'Then', desc: '', after: [id(1)], told: 'queued', worktree: '/nonexistent', branch: 'b', base: 'main' },
        // its go was sent, but the agent was never seen at work
        { ...base, id: id(3), n: 3, title: 'Went', desc: '', going: 'Go: X-1 is done, start now.', told: 'work', worktree: '/nonexistent', branch: 'b', base: 'main' },
      ],
    })
    const w = world()
    expect(w.engine.task(id(2))!.after).toEqual([id(1)])
    expect(w.engine.statusOf(w.engine.task(id(2))!)).toBe('queued')
    w.engine.resumeQueue()
    expect(w.engine.statusOf(w.engine.task(id(2))!)).toBe('queued')
    expect(w.sent.find((x) => x.key === `${id(3)}/Agent`)?.text).toBe('<shepherdr><go>Go: X-1 is done, start now.</go></shepherdr>')
    // seen at work: it is not sent again
    w.agentIs(`${id(3)}/Agent`, 'working')
    expect(w.engine.task(id(3))!.going).toBeUndefined()
  })

  test("another machine's task: named with its machine first, done when that machine says so", async () => {
    const w = world()
    const dir = repo()
    const p = await w.places.add(dir, 'peers')
    expect(() => w.engine.add({ project: p.name, title: 'X', by: 'test', after: ['vm-x:ELA-3'] })).toThrow('only the main machine')
    let theirs: Partial<Task> | 'away' | undefined = { id: 'ELA-3', name: 'Their API', status: 'work' }
    w.engine.peerTask = (machine, id) => (machine !== 'vm-x' ? 'unknown' : theirs === 'away' ? 'away' : theirs && id.toUpperCase() === 'ELA-3' ? (theirs as Task) : undefined)
    expect(() => w.engine.add({ project: p.name, title: 'X', by: 'test', after: ['vm-y:ELA-3'] })).toThrow('no machine called vm-y')
    expect(() => w.engine.add({ project: p.name, title: 'X', by: 'test', after: ['vm-x:ELA-9'] })).toThrow('vm-x has no task ELA-9')
    const t = w.engine.add({ project: p.name, title: 'Use their API', desc: 'Then use it.', by: 'test', after: ['VM-X:ela-3'] })
    await until(() => w.started.some((s) => s.key === `${t.id}/Agent`))
    expect(t.after).toEqual(['vm-x:ELA-3'])
    expect(w.engine.subOf(t)).toBe('After vm-x:ELA-3')
    // that machine can't be reached: it keeps waiting, and says so
    theirs = 'away'
    w.engine.recheck()
    expect(w.engine.statusOf(t)).toBe('queued')
    expect(w.engine.subOf(t)).toBe("After vm-x:ELA-3 · can't reach vm-x")
    theirs = { id: 'ELA-3', name: 'Their API', status: 'ready', outcome: 'done', said: 'The API is up.' }
    w.engine.recheck()
    expect(w.sent.find((s) => s.key === `${t.id}/Agent`)!.text).toBe('<shepherdr><go>Go: vm-x:ELA-3 is done, start now.\nvm-x:ELA-3 (Their API): The API is up.</go></shepherdr>')
  })
})

describe("a task's place in herdr", () => {
  const linked = (w: ReturnType<typeof world>, label: string) => w.herdr.spaces.find((x) => x.label === label && x.worktree?.is_linked_worktree)
  const branches = (dir: string) => sh('git branch --list "work/*" --format="%(refname:short)"', dir)

  test("its worktree is made through herdr, nested under the project's space, with .worktreeinclude's files copied in", async () => {
    const w = world()
    const dir = repo()
    writeFileSync(join(dir, '.gitignore'), '.env\nbuild/\n')
    writeFileSync(join(dir, '.worktreeinclude'), '# secrets\n.env\n')
    sh('git add -A && git commit -qm ignore', dir)
    writeFileSync(join(dir, '.env'), 'KEY=1\n')
    sh('mkdir build && touch build/big', dir)
    const { t, key } = await started(w, dir, 'nested')
    const ws = linked(w, `${t.id} Say hello`)!
    expect(ws.worktree).toEqual({ checkout_path: t.worktree!, is_linked_worktree: true, repo_root: dir })
    // herdr opened the repo's own space to nest it under, since none was open
    expect(w.herdr.spaces.some((x) => x.worktree?.checkout_path === dir && !x.worktree.is_linked_worktree)).toBe(true)
    // the agent runs in a tab of its own there (it has the agent's environment); the shell herdr opened is closed
    const s = w.started.find((x) => x.key === key)!
    expect(ws.panes).toEqual([s.pane!])
    expect(w.herdr.cwd.get(s.pane!)).toBe(t.worktree!)
    expect(sh('git branch --show-current', t.worktree!)).toBe('work/1-say-hello')
    expect(readFileSync(join(t.worktree!, '.env'), 'utf8')).toBe('KEY=1\n')
    expect(existsSync(join(t.worktree!, 'build'))).toBe(false)
  })

  test("after the merge herdr removes its worktree and space, its branch goes, and the repo space herdr opened closes", async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'tidy')
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    w.agentIs(key, 'idle')
    expect((await w.engine.merge(t.id)).ok).toBe(true)
    await until(() => !!t.cleaned)
    expect(existsSync(t.worktree!)).toBe(false)
    expect(branches(dir)).toBe('')
    expect(w.herdr.spaces).toEqual([])
    expect(t.kept).toBeUndefined()
    expect(w.engine.keptLeftovers()).toEqual([])
  })

  test('a repo space you were already using stays open', async () => {
    const herdr = fakeHerdr()
    const w = world(herdr)
    const dir = repo()
    const p = await w.places.add(dir, 'yours')
    // yours, open before any task, with an agent of yours in it
    herdr.spaces.push({ workspace_id: 'mine', label: 'yours', tab_count: 1, worktree: { checkout_path: dir, is_linked_worktree: false, repo_root: dir }, panes: ['mine:p1'] })
    herdr.agentPanes.add('mine:p1')
    const t = w.engine.add({ project: p.name, title: 'Say hello', by: 'test' })
    await until(() => w.started.some((s) => s.key === `${t.id}/Agent`))
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    w.agentIs(`${t.id}/Agent`, 'idle')
    expect((await w.engine.merge(t.id)).ok).toBe(true)
    await until(() => !!t.cleaned)
    expect(herdr.spaces.map((x) => x.workspace_id)).toEqual(['mine'])
  })

  test('unsaved or unmerged work is kept and said so; its space and agent close all the same', async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'keeps')
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    expect((await w.engine.merge(t.id, { self: true })).ok).toBe(true)
    // its agent goes on after the merge: a file it never commits, and a commit main never gets
    writeFileSync(join(t.worktree!, 'late.txt'), 'late\n')
    w.agentIs(key, 'idle')
    await until(() => !!t.cleaned)
    expect(existsSync(join(t.worktree!, 'late.txt'))).toBe(true)
    expect(t.kept).toEqual([`${t.worktree!.replace(homedir(), '~')} (uncommitted changes)`])
    expect(w.news.some((n) => n.startsWith(`${t.id} was merged; kept `) && n.endsWith('(uncommitted changes)'))).toBe(true)
    expect(w.engine.keptLeftovers()[0]).toStartWith(`${t.id} `)
    expect(linked(w, `${t.id} Say hello`)).toBeUndefined()
    expect(w.records[key].finished).toBe(true)
    sh('git add -A && git commit -qm late', t.worktree!)
    await w.engine.sweep()
    expect(t.kept![0]).toEndWith(`(work on ${t.branch} that main lacks)`)
    expect(branches(dir)).toBe(t.branch!)
  })

  test('something still running in its space puts it off; it is tried again later', async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'running')
    const ws = linked(w, `${t.id} Say hello`)!
    // a dev server you started in a pane of its own there
    const { root_pane } = await w.herdr.h.createTab(ws.workspace_id, 'Server', t.worktree!)
    w.herdr.busy.add(root_pane.pane_id)
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    w.agentIs(key, 'idle')
    expect((await w.engine.merge(t.id)).ok).toBe(true)
    expect(existsSync(t.worktree!)).toBe(true)
    expect(t.cleaned).toBeFalsy()
    w.herdr.busy.delete(root_pane.pane_id)
    await w.engine.sweep()
    expect(t.cleaned).toBe(true)
    expect(existsSync(t.worktree!)).toBe(false)
  })

  test('stopped with no work: agent, space, worktree and branch go; writing to it makes them again', async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'stopnone')
    const wt = t.worktree!
    expect(await w.engine.stopTask(t.id)).toEqual([])
    expect(existsSync(wt)).toBe(false)
    expect(branches(dir)).toBe('')
    expect(w.herdr.spaces.some((x) => x.label === `${t.id} Say hello`)).toBe(false)
    expect(w.records[key].finished).toBe(true)
    // resumed: the worktree is made again through herdr, nested as before, in the same folder (its agent's session
    // is kept by folder)
    await (w.engine as unknown as { launch(t: unknown): Promise<unknown> }).launch(t)
    expect(t.worktree).toBe(wt)
    expect(existsSync(t.worktree!)).toBe(true)
    expect(linked(w, `${t.id} Say hello`)).toBeTruthy()
  })

  test('stopped with work: its worktree and branch stay; resuming opens its space again through herdr', async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'stopwork')
    writeFileSync(join(t.worktree!, 'wip.txt'), 'wip\n')
    const kept = await w.engine.stopTask(t.id)
    expect(kept).toHaveLength(1)
    expect(kept[0]).toEndWith('(uncommitted changes)')
    expect(existsSync(join(t.worktree!, 'wip.txt'))).toBe(true)
    expect(w.herdr.spaces.some((x) => x.label === `${t.id} Say hello`)).toBe(false)
    w.status[key] = 'gone'
    await (w.engine as unknown as { launch(t: unknown): Promise<unknown> }).launch(t)
    const ws = linked(w, `${t.id} Say hello`)!
    expect(ws.worktree?.checkout_path).toBe(t.worktree!)
    expect(ws.panes).toEqual([w.started.at(-1)!.pane!])
  })

  test('deleted with no work: agent, space, worktree, branch, chat and record all go, and nothing is posted', async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'delnone')
    const wt = t.worktree!
    const news = w.news.length
    expect(await w.engine.deleteTask(t.id)).toEqual([])
    expect(existsSync(wt)).toBe(false)
    expect(branches(dir)).toBe('')
    expect(w.herdr.spaces).toEqual([])
    expect(w.engine.task(t.id)).toBeUndefined()
    expect(w.forgot).toEqual([`chat ${key}`, key])
    expect(w.news.length).toBe(news)
    // its number is never given out again
    expect(w.engine.add({ project: t.project, title: 'Next', by: 'test' }).id).not.toBe(t.id)
  })

  test('deleting refuses work main lacks, and changes nothing; --force deletes it all the same', async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'delwork')
    writeFileSync(join(t.worktree!, 'wip.txt'), 'wip\n')
    await expect(w.engine.deleteTask(t.id)).rejects.toThrow(/holds .*\(uncommitted changes\).*--force/)
    // what it holds comes with the refusal too, for the screen to ask
    expect(await w.engine.deleteTask(t.id).catch((e) => e.held)).toEqual([expect.stringContaining('(uncommitted changes)')])
    expect(w.engine.task(t.id)).toBe(t)
    expect(existsSync(join(t.worktree!, 'wip.txt'))).toBe(true)
    expect(w.status[key]).toBe('working')
    sh('git add -A && git commit -qm wip', t.worktree!)
    await expect(w.engine.deleteTask(t.id)).rejects.toThrow(`work on ${t.branch} that main lacks`)
    expect(await w.engine.deleteTask(t.id, { force: true })).toEqual([])
    expect(existsSync(t.worktree!)).toBe(false)
    expect(branches(dir)).toBe('')
    expect(w.herdr.spaces).toEqual([])
    expect(w.engine.task(t.id)).toBeUndefined()
  })

  test("a merged task can be deleted: its commits stay on main", async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'delmerged-engine')
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    w.agentIs(key, 'idle')
    expect((await w.engine.merge(t.id)).ok).toBe(true)
    await until(() => !!t.cleaned)
    expect(await w.engine.deleteTask(t.id)).toEqual([])
    expect(w.engine.task(t.id)).toBeUndefined()
    expect(sh('git log --format=%s -n 1', dir)).toBe(`${t.id}: Say hello`)
    expect(existsSync(join(dir, 'hello.txt'))).toBe(true)
  })

  test("the sweep puts away merged tasks' leftovers only: never a running task's, nor yours, nor while herdr is away", async () => {
    const dir = repo()
    const p = await world().places.add(dir, 'sweep')
    const old = mkdtempSync(join(tmpdir(), 'shep-old-'))
    // left by an older Shepherdr: squash-merged, marked cleaned, and main moved on over the same lines since
    const make = (n: number, file: string) => {
      const wt = join(old, `work-${n}`)
      sh(`git worktree add -q -b work/${n}-x ${wt} main`, dir)
      writeFileSync(join(wt, file), `${n}\n`)
      sh(`git add -A && git commit -qm "${n}"`, wt)
      return wt
    }
    const merged = make(1, 'a.txt')
    sh('git merge -q --squash work/1-x && git commit -qm "P-1: x"', dir)
    writeFileSync(join(dir, 'a.txt'), 'changed since\n')
    sh('git commit -qam since', dir)
    const dirty = make(2, 'b.txt')
    sh('git merge -q --squash work/2-x && git commit -qm "P-2: x"', dir)
    writeFileSync(join(dirty, 'notes.txt'), 'mine\n')
    const running = make(3, 'c.txt')
    const yours = make(4, 'd.txt')
    sh('git merge -q --squash work/4-x && git commit -qm "yours"', dir)
    const rec = (n: number, wt: string, more: object) => ({ id: `${p.key}-${n}`, n, project: placeId.space(p.name), title: 'x', desc: '', by: 'x', created: 1, updated: 1, branch: `work/${n}-x`, worktree: wt, base: 'main', ...more })
    writeJson('tasks.json', { next: { [p.key]: 3 }, tasks: [rec(1, merged, { merged: true, cleaned: true }), rec(2, dirty, { merged: true, cleaned: true }), rec(3, running, { told: 'work' })] })
    const w = world()
    w.herdr.down(true)
    await w.engine.sweep()
    expect(existsSync(merged)).toBe(true)
    w.herdr.down(false)
    await w.engine.sweep()
    expect(existsSync(merged)).toBe(false)
    expect(branches(dir).split('\n')).toEqual(['work/2-x', 'work/3-x', 'work/4-x'])
    expect(existsSync(join(dirty, 'notes.txt'))).toBe(true)
    expect(w.engine.task(`${p.key}-2`)!.kept![0]).toEndWith('(uncommitted changes)')
    expect(existsSync(running) && existsSync(yours)).toBe(true)
  })

  test("a squash merge main never had the tree of, with main moved on over the same lines since: it goes, unless work came after", async () => {
    const dir = repo()
    const p = await world().places.add(dir, 'squashed')
    const old = mkdtempSync(join(tmpdir(), 'shep-squashed-'))
    // each branch is squash-merged as it was, without a rebase on main (so no commit of main has its tree), and main
    // then changes the lines it added (so merging it into main now would conflict)
    const squashed = (n: number, subject: string, also?: string) => {
      const wt = join(old, `work-${n}`)
      sh(`git worktree add -q -b work/${n}-x ${wt} main`, dir)
      writeFileSync(join(wt, `${n}.txt`), `${n}\n`)
      if (also) writeFileSync(join(wt, also), `${n}\n`)
      sh(`git add -A && git commit -qm "${n}"`, wt)
      writeFileSync(join(dir, `main-${n}.txt`), 'meanwhile\n')
      sh('git add -A && git commit -qm meanwhile', dir)
      sh(`git merge -q --squash work/${n}-x && git commit -qm "${subject}"`, dir)
      const squash = sh('git rev-parse HEAD', dir)
      writeFileSync(join(dir, `${n}.txt`), 'changed since\n')
      sh('git commit -qam since', dir)
      return { wt, squash }
    }
    const byName = squashed(1, `${p.key}-1: x`)
    const recorded = squashed(2, 'squashed on GitHub')
    const after = squashed(3, `${p.key}-3: x`)
    writeFileSync(join(after.wt, 'more.txt'), 'more\n')
    sh('git add -A && git commit -qm "after the merge"', after.wt)
    // a commit after the merge that undoes part of it: merging it into its squash changes nothing, but it is work
    const undone = squashed(4, `${p.key}-4: x`, '4b.txt')
    sh('git rm -q 4b.txt && git commit -qm "undo part of it"', undone.wt)
    // its recorded squash is no longer on main (main was rewritten), and no commit has its subject
    const gone = squashed(5, 'not by name')
    sh(`git checkout -q -b elsewhere ${gone.squash}`, dir)
    sh('git checkout -q main && git reset -q --hard HEAD~2', dir)
    const rec = (n: number, wt: string, more: object = {}) => ({ id: `${p.key}-${n}`, n, project: placeId.space(p.name), title: 'x', desc: '', by: 'x', created: 1, updated: 1, branch: `work/${n}-x`, worktree: wt, base: 'main', merged: true, cleaned: true, ...more })
    writeJson('tasks.json', { next: { [p.key]: 3 }, tasks: [rec(1, byName.wt), rec(2, recorded.wt, { squash: recorded.squash }), rec(3, after.wt), rec(4, undone.wt), rec(5, gone.wt, { squash: gone.squash })] })
    const w = world()
    await w.engine.sweep()
    expect(existsSync(byName.wt) || existsSync(recorded.wt)).toBe(false)
    expect(branches(dir).split('\n')).toEqual(['work/3-x', 'work/4-x', 'work/5-x'])
    expect(existsSync(join(after.wt, 'more.txt'))).toBe(true)
    for (const n of [3, 4, 5]) expect(w.engine.task(`${p.key}-${n}`)!.kept![0]).toEndWith(`(work on work/${n}-x that main lacks)`)
  })

  test("it nests under the project's own space (its coordinator's), which stays open after", async () => {
    const herdr = fakeHerdr()
    const w = world(herdr)
    const dir = repo()
    const p = await w.places.add(dir, 'ownspace')
    // the project's space as Shepherdr opens it for its coordinator: a plain workspace in the repo
    herdr.spaces.push({ workspace_id: 'proj', label: p.name, tab_count: 1, panes: ['proj:p1'] })
    herdr.cwd.set('proj:p1', dir)
    herdr.agentPanes.add('proj:p1')
    const t = w.engine.add({ project: p.name, title: 'Say hello', by: 'test' })
    await until(() => w.started.some((s) => s.key === `${t.id}/Agent`))
    expect(herdr.spaces.map((x) => x.workspace_id)[0]).toBe('proj')
    expect(herdr.spaces.find((x) => x.workspace_id === 'proj')!.worktree).toEqual({ checkout_path: dir, is_linked_worktree: false, repo_root: dir })
    expect(herdr.spaces.filter((x) => !x.worktree?.is_linked_worktree)).toHaveLength(1)
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    w.agentIs(`${t.id}/Agent`, 'idle')
    expect((await w.engine.merge(t.id)).ok).toBe(true)
    await until(() => !!t.cleaned)
    expect(herdr.spaces.map((x) => x.workspace_id)).toEqual(['proj'])
  })

  test("a start herdr finished but whose answer was lost: its worktree is opened, not made twice", async () => {
    const herdr = fakeHerdr()
    const create = herdr.h.worktreeCreate
    herdr.h.worktreeCreate = (async (...a: Parameters<typeof create>) => {
      await create(...a)
      throw new Error('herdr worktree.create timed out')
    }) as typeof create
    const w = world(herdr)
    const { t } = await started(w, repo(), 'lost')
    expect(t.trouble).toBeUndefined()
    expect(linked(w, `${t.id} Say hello`)!.worktree!.checkout_path).toBe(t.worktree!)
  })

  test("the sweep leaves a merged task's agent alone while it works, and after you talk to it again", async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'sweepbusy')
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    expect((await w.engine.merge(t.id, { self: true })).ok).toBe(true)
    await w.engine.sweep()
    expect(existsSync(t.worktree!)).toBe(true)
    w.agentIs(key, 'idle')
    await until(() => !!t.cleaned)
    // resumed after its clean-up, in no worktree (a merged task's is never made again): left alone between replies
    await (w.engine as unknown as { launch(t: unknown): Promise<unknown> }).launch(t)
    expect(existsSync(t.worktree!)).toBe(false)
    w.status[key] = 'idle'
    const n = w.herdr.spaces.length
    await w.engine.sweep()
    expect(w.status[key]).toBe('idle')
    expect(w.herdr.spaces.length).toBe(n)
  })

  test("a task started before keeps its plain space: it is closed by its name, and git removes its worktree", async () => {
    const herdr = fakeHerdr()
    const dir = repo()
    const p = await world(herdr).places.add(dir, 'plain')
    const wt = join(mkdtempSync(join(tmpdir(), 'shep-plain-')), 'work-1')
    sh(`git worktree add -q -b work/1-x ${wt} main`, dir)
    writeFileSync(join(wt, 'x.txt'), 'x\n')
    sh('git add -A && git commit -qm x', wt)
    sh('git merge -q --squash work/1-x && git commit -qm "P-1: x"', dir)
    herdr.spaces.push({ workspace_id: 'old', label: `${p.key}-1 x`, tab_count: 1, panes: ['old:p1'] })
    herdr.cwd.set('old:p1', wt)
    writeJson('tasks.json', { next: { [p.key]: 1 }, tasks: [{ id: `${p.key}-1`, n: 1, project: placeId.space(p.name), title: 'x', desc: '', by: 'x', created: 1, updated: 1, branch: 'work/1-x', worktree: wt, base: 'main', merged: true }] })
    const w = world(herdr)
    await w.engine.sweep()
    expect(existsSync(wt)).toBe(false)
    expect(herdr.spaces).toEqual([])
    expect(w.engine.task(`${p.key}-1`)!.cleaned).toBe(true)
  })
})

describe('pausing', () => {
  test('a working task is interrupted and To do; its agent stays; writing to it goes on', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'paused')
    expect(w.engine.statusOf(t)).toBe('work')
    await w.engine.pause(t.id)
    expect(w.engine.statusOf(t)).toBe('todo')
    // the turn the interrupt ends is no question for you
    w.agentIs(key, 'idle')
    expect(w.engine.statusOf(t)).toBe('todo')
    expect(w.records[key].finished).toBeUndefined()
    expect(existsSync(t.worktree!)).toBe(true)
    await expect(w.engine.pause(t.id)).rejects.toThrow(/isn't working/)
    w.engine.youWrote(key)
    expect(t.todo).toBeUndefined()
  })

  test('paused while work it left in the background runs: woken by it, it shows Working again, not To do', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'pausebg')
    w.chat.reply = 'A reviewer is checking it now.'
    w.bg[key] = 'agents'
    w.agentIs(key, 'done')
    expect(w.engine.statusOf(t)).toBe('work')
    await w.engine.pause(t.id)
    expect(w.engine.statusOf(t)).toBe('todo')
    w.bg[key] = undefined
    w.agentIs(key, 'working')
    expect(w.engine.statusOf(t)).toBe('work')
    expect(t.todo).toBeUndefined()
  })

  test('an agent paused mid-turn that still reads as working is not taken as woken', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'pauselag')
    await w.engine.pause(t.id)
    // herdr has not seen the interrupt land yet
    w.agentIs(key, 'working')
    expect(w.engine.statusOf(t)).toBe('todo')
    w.agentIs(key, 'done')
    expect(w.engine.statusOf(t)).toBe('todo')
  })
})

describe('pinning', () => {
  test('a pin is kept across restarts; unpinning drops it; an archived task drops it and is not pinned again', async () => {
    const w = world()
    const { t } = await started(w, repo(), 'pinned')
    w.engine.pin(t.id, true)
    expect(t.pinned).toBeNumber()
    expect(w.engine.view(t).pinned).toBe(t.pinned!)
    // read back as after a restart
    expect(world().engine.task(t.id)!.pinned).toBe(t.pinned!)
    w.engine.pin(t.id, false)
    expect(world().engine.task(t.id)!.pinned).toBeUndefined()
    w.engine.pin(t.id, true)
    await w.engine.archive(t.id)
    expect(t.pinned).toBeUndefined()
    expect(() => w.engine.pin(t.id, true)).toThrow(/archived/)
    // brought back, it is not pinned again on its own
    w.engine.unarchive(t.id)
    expect(t.pinned).toBeUndefined()
  })

  test('a Done task that archives itself drops its pin', () => {
    const now = Date.now()
    writeJson('tasks.json', { next: { OLD: 1 }, tasks: [{ id: 'OLD-1', n: 1, project: 'p:x:old', title: 'Old', desc: '', by: 'x', created: 1, updated: 1, merged: true, doneAt: now - 30 * 86_400_000, pinned: 5 }] })
    const w = world()
    expect(w.engine.autoArchive(now)).toBe(1)
    expect(w.engine.task('OLD-1')!.pinned).toBeUndefined()
  })
})

describe('archiving', () => {
  test("a working task's agent stops first; it leaves every count, keeps its record, and nothing about it is news", async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'archwork')
    const wt = t.worktree!
    const before = { news: w.news.length, cards: w.cards.length }
    const r = await w.engine.archive(t.id)
    expect(r.stopped).toBe(true)
    expect(w.records[key].finished).toBe(true)
    // no work in it: its worktree went, as with task stop
    expect(existsSync(wt)).toBe(false)
    expect(t.archived).toBeNumber()
    w.engine.agentStatus(key)
    await Bun.sleep(50)
    expect(w.news.length).toBe(before.news)
    expect(w.cards.length).toBe(before.cards)
    expect(w.engine.count(placeId.space('archwork'), w.engine.statusOf(t))).toBe(0)
    const v = w.engine.view(t)
    expect(v.archived).toBe(t.archived!)
    expect(v.state).toStartWith('Archived')
    expect(w.engine.task(t.id)).toBe(t)
  })

  test('unarchive brings it back quietly; so does writing to it', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'archback')
    await w.engine.archive(t.id)
    const cards = w.cards.length
    w.engine.unarchive(t.id)
    expect(t.archived).toBeUndefined()
    expect(t.unarchived).toBeNumber()
    expect(w.cards.length).toBe(cards)
    await w.engine.archive(t.id)
    w.engine.youWrote(key)
    expect(t.archived).toBeUndefined()
    expect(w.engine.statusOf(t)).toBe('work')
  })

  test('Done tasks archive themselves after tasks.archiveAfter; off keeps them; a brought-back one waits again', () => {
    const w = world()
    const day = 86_400_000
    const now = Date.now()
    const done = (id: string, updated: number, more = {}) => ({ id, n: 1, project: 'p:x', title: id, desc: '', by: 't', created: 1, updated, merged: true, cleaned: true, ...more })
    const tasks = w.engine.tasks as unknown as ReturnType<typeof done>[]
    tasks.push(done('OLD-1', now - 8 * day), done('NEW-1', now - 2 * day), done('BACK-1', now - 30 * day, { unarchived: now - day }))
    tasks.push({ ...done('OPEN-1', now - 30 * day), merged: false } as never)
    // one whose agent you are talking to stays put
    tasks.push(done('BUSY-1', now - 30 * day))
    w.status['BUSY-1/Agent'] = 'working'
    // tasks earlier tests left in tasks.json are left out: only these five are looked at
    tasks.splice(0, tasks.length - 5)
    expect(w.engine.autoArchive(now)).toBe(1)
    expect(w.engine.task('OLD-1')!.archived).toBe(now)
    expect(w.engine.task('NEW-1')!.archived).toBeUndefined()
    expect(w.engine.task('BACK-1')!.archived).toBeUndefined()
    expect(w.engine.task('OPEN-1')!.archived).toBeUndefined()
    expect(w.engine.task('BUSY-1')!.archived).toBeUndefined()
    // writing to an archived Done task brings it back too
    w.engine.youWrote('OLD-1/Agent')
    expect(w.engine.task('OLD-1')!.archived).toBeUndefined()
    expect(w.settings.set('tasks.archiveAfter', '1 day').ok).toBe(true)
    expect(w.engine.autoArchive(now)).toBe(2)
    w.engine.unarchive('OLD-1')
    w.engine.unarchive('NEW-1')
    expect(w.settings.set('tasks.archiveAfter', 'off').ok).toBe(true)
    expect(w.engine.autoArchive(now + 99 * day)).toBe(0)
    expect(w.settings.set('tasks.archiveAfter', 'soon').ok).toBe(false)
    expect(w.settings.set('tasks.archiveAfter', '0 days').ok).toBe(false)
    expect(w.settings.set('tasks.archiveAfter', '10 days').ok).toBe(true)
    expect(w.settings.set('tasks.archiveAfter', '10').ok).toBe(true)
  })
})

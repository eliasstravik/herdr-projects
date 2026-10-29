// Room on the machine: new tasks, and tasks told to go, wait as Queued while the machine has its max tasks working or
// is too loaded, and start by themselves once there's room, oldest first (engine-world.ts).
import { describe, expect, test } from 'bun:test'
import { writeJson, placeId, repo, sh, started, until, world } from './engine-world.ts'

/** a fresh world with nothing from earlier tests, its machine's limits set */
function fresh(limits: Record<string, string>) {
  const w = world()
  w.engine.tasks.splice(0)
  for (const k of ['machine.maxTasks', 'machine.startLoad', 'machine.startMemory']) w.settings.set(k, null)
  for (const [k, v] of Object.entries(limits)) expect(w.settings.set(k, v).ok).toBe(true)
  return w
}

describe('room on the machine', () => {
  test('older waiting tasks keep their target across a settings change and two restarts', async () => {
    const w = fresh({ 'machine.maxTasks': '1' })
    const dir = repo()
    sh('git branch develop', dir)
    const p = await w.places.add(dir, 'legacy-base')
    writeJson('tasks.json', { next: { [p.key]: 1 }, tasks: [{ id: `${p.key}-1`, n: 1, project: placeId.space(p.name), title: 'Old queued task', desc: '', by: 'test', created: Date.now(), updated: Date.now(), told: 'queued', held: { why: 'slots', since: Date.now() } }] })
    const first = world()
    expect(first.engine.task(`${p.key}-1`)!.base).toBe('main')
    await first.places.setBase(p.name, 'develop')
    const second = world()
    expect(second.engine.task(`${p.key}-1`)!.base).toBe('main')
    expect(second.places.byName(p.name)!.base).toBe('develop')
  })

  test('space changes affect new tasks only, including tasks waiting for room; bad settings leave the old target', async () => {
    const w = fresh({ 'machine.maxTasks': '1' })
    const dir = repo()
    sh('git branch develop && git branch feature/topic', dir)
    const a = await started(w, dir, 'base-snapshot')
    w.brief(a.key)
    const b = w.engine.add({ project: a.p.name, title: 'Waiting', by: 'test' })
    expect(b.base).toBe('main')
    await w.places.setBase(a.p.name, 'develop')
    expect(a.t.base).toBe('main')
    expect(b.base).toBe('main')
    const c = w.engine.add({ project: a.p.name, title: 'New default', by: 'test' })
    const d = w.engine.add({ project: a.p.name, title: 'Override', base: 'feature/topic', by: 'test' })
    expect(c.base).toBe('develop')
    expect(d.base).toBe('feature/topic')
    await expect(w.places.setBase(a.p.name, 'missing')).rejects.toThrow('No local branch missing')
    await expect(w.places.setBase(a.p.name, '--all')).rejects.toThrow('Choose a branch name')
    expect(a.p.base).toBe('develop')
    expect(sh('git branch --show-current', dir)).toBe('main')
    w.agentIs(a.key, 'idle')
    await until(() => w.started.some((s) => s.key === `${b.id}/Agent`))
    expect(b.base).toBe('main')
    expect(w.started.find((s) => s.key === `${b.id}/Agent`)!.system).toContain('(from main)')
  })

  test('past the max working, a new task waits as Queued with no agent yet, and starts by itself once one stops working', async () => {
    const w = fresh({ 'machine.maxTasks': '1' })
    const a = await started(w, repo(), 'room-a', 'First', 'Add a.txt.')
    w.brief(a.key)
    expect(w.engine.working()).toBe(1)
    const b = w.engine.add({ project: a.p.name, title: 'Second', desc: 'Add b.txt.', by: 'test' })
    const c = w.engine.add({ project: a.p.name, title: 'Third', desc: 'Add c.txt.', by: 'test' })
    expect(w.engine.statusOf(b)).toBe('queued')
    expect(w.engine.subOf(b)).toBe('Waiting for a free slot')
    expect(w.engine.view(b).after).toBeUndefined()
    // no worktree or agent while it waits, and no news that it needs you
    expect(b.worktree).toBeUndefined()
    expect(w.started.some((s) => s.key === `${b.id}/Agent`)).toBe(false)
    expect(w.posted.some((x) => x.key === `${b.id}/Agent` && x.id.startsWith('held:'))).toBe(true)
    expect(w.news.some((n) => n.includes(`${b.id} needs you`))).toBe(false)
    // the first one waits on you now: only working tasks count, so the oldest held one starts, on its brief
    w.agentIs(a.key, 'idle')
    await until(() => w.started.some((s) => s.key === `${b.id}/Agent`))
    expect(w.started.find((s) => s.key === `${b.id}/Agent`)!.first).toStartWith('Second\n\nAdd b.txt.')
    expect(w.posted.some((x) => x.key === `${b.id}/Agent` && x.id.startsWith('go:'))).toBe(true)
    // the next waits for b's slot
    expect(w.engine.statusOf(c)).toBe('queued')
    expect(c.worktree).toBeUndefined()
  })

  test('a task for later reads its brief at once: only work waits for room', async () => {
    const w = fresh({ 'machine.maxTasks': '1' })
    const a = await started(w, repo(), 'room-later', 'First', 'Add a.txt.')
    w.brief(a.key)
    const b = w.engine.add({ project: a.p.name, title: 'Later', desc: 'Add b.txt.', by: 'test', later: true })
    expect(b.held).toBeUndefined()
    await until(() => w.started.some((s) => s.key === `${b.id}/Agent`))
  })

  test("a queued task's go waits for room; a coordinator's go to a To do task waits too, yours goes in at once", async () => {
    const w = fresh({ 'machine.maxTasks': '1' })
    const a = await started(w, repo(), 'room-go', 'First', 'Add a.txt.')
    w.brief(a.key)
    const b = w.engine.add({ project: a.p.name, title: 'After', desc: 'Add b.txt.', by: 'test', after: [a.t.id] })
    const bkey = `${b.id}/Agent`
    await until(() => w.started.some((s) => s.key === bkey))
    w.agentIs(bkey, 'idle')
    // a later for-later task, already started and waiting for its go
    const t = w.engine.add({ project: a.p.name, title: 'Saved', desc: 'Add t.txt.', by: 'test', later: true })
    const tkey = `${t.id}/Agent`
    await until(() => w.started.some((s) => s.key === tkey))
    w.engine.todo(t.id)
    w.agentIs(tkey, 'idle')
    // what b waits for is cleared, but a holds the one slot: b's go waits
    expect(w.engine.queue(b.id, [])).toEqual({ queued: [], went: true })
    expect(b.after).toBeUndefined()
    expect(w.engine.statusOf(b)).toBe('queued')
    expect(w.engine.subOf(b)).toBe('Waiting for a free slot')
    expect(w.sent.some((s) => s.key === bkey && s.text.includes('Go:'))).toBe(false)
    // the coordinator's go to the To do one waits behind it, and says so
    const r = await w.engine.tell(t.id, 'Go ahead.', { gate: true })
    expect(r.held).toBe('slots')
    expect(w.sent.some((s) => s.key === tkey)).toBe(false)
    expect(w.engine.statusOf(t)).toBe('queued')
    // a's turn ends: b, the oldest, gets its go; t still waits
    w.agentIs(a.key, 'idle')
    await until(() => w.sent.some((s) => s.key === bkey && s.text.includes(`Go: you no longer wait for ${a.t.id}, start now.`)), 12_000)
    expect(w.engine.statusOf(t)).toBe('queued')
    // yours goes in at once, past the limit
    await w.engine.tell(t.id, 'Go on, please.')
    expect(w.sent.at(-1)).toEqual({ key: tkey, text: 'Go on, please.' })
    expect(t.held).toBeUndefined()
    expect(w.engine.statusOf(t)).toBe('work')
  }, 20_000)

  test('a busy machine holds new tasks ("Waiting for the machine") and lets them go one at a time, 30 seconds apart', async () => {
    const w = fresh({})
    let m: { busy?: 'cpu' | 'memory'; herdr: boolean } = { busy: 'cpu', herdr: true }
    w.engine.machine = () => m
    const p = await w.places.add(repo(), 'room-load')
    const a = w.engine.add({ project: p.name, title: 'One', desc: 'Add a.txt.', by: 'test' })
    const b = w.engine.add({ project: p.name, title: 'Two', desc: 'Add b.txt.', by: 'test' })
    expect(w.engine.subOf(a)).toBe('Waiting for the machine')
    expect(w.engine.heldTasks().map((t) => t.id)).toEqual([a.id, b.id])
    // herdr not answering holds them too
    m = { herdr: false }
    w.engine.release()
    expect(a.worktree).toBeUndefined()
    m = { herdr: true }
    const now = Date.now()
    w.engine.release(now)
    await until(() => w.started.some((s) => s.key === `${a.id}/Agent`))
    expect(w.engine.statusOf(b)).toBe('queued')
    w.engine.release(now + 10_000)
    expect(b.held).toBeTruthy()
    w.engine.release(now + 31_000)
    await until(() => w.started.some((s) => s.key === `${b.id}/Agent`))
  })
})

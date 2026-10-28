// A home space's tasks (its machine's) by status or by space; the main machine's home space's (every machine's) also by
// machine, or by machine and then space; any other space's own list is by status alone.
import { describe, expect, test } from 'bun:test'
import { machines, places, tasks } from '../server/demo/data.ts'
import { DONE_SHOWN, GROUPS, groupChoices, groupFor, groupTasks as grouped, shownGroup } from '../src/groups.ts'
import type { Machine, Place, Task } from '../shared/types.ts'

const ps = places()
const ms = machines()
/** a space's tasks in their groups, under the demo's machines (or those given) */
const groupTasks = (pl: Place[], list: Task[], by: Parameters<typeof grouped>[3], level?: number, m: Machine[] = ms) => grouped(pl, m, list, by, level)
const ts = tasks()
const shape = (by: Parameters<typeof grouped>[3], list: Task[] = ts, pl: Place[] = ps, level = 1) =>
  groupTasks(pl, list, by, level).map((g) => [...(by === 'machine-project' ? [g.machine?.name] : []), g.label, g.tasks.map((t) => t.id)])
const names = (g: { names: { machine: boolean; project: boolean } }) => `${g.names.machine ? 'machine ' : ''}${g.names.project ? 'project' : ''}`.trim()
const vmStr = ts.filter((t) => t.place.startsWith('p:vm-str:'))

describe('grouping tasks', () => {
  test('by status: Needs you, Ready, Merging, Working, Queued, To do, Done, empty groups left out but To do', () => {
    const gs = groupTasks(ps, ts, 'status')
    expect(gs.map((g) => g.label)).toEqual(['Needs you', 'Ready', 'Merging', 'Working', 'Queued', 'To do', 'Done'])
    for (const g of gs) expect(new Set(g.tasks.map((t) => t.status)).size).toBe(1)
    expect(groupTasks(ps, ts.filter((t) => t.status === 'need'), 'status').map((g) => [g.label, g.tasks.length])).toEqual([
      ['Needs you', ts.filter((t) => t.status === 'need' && !t.archived).length],
      ['To do', 0],
    ])
    expect(groupTasks(ps, [], 'status').map((g) => g.label)).toEqual(['To do'])
    // by project there is no To do to keep
    expect(groupTasks(ps, [], 'project')).toEqual([])
  })

  test('by project: one group per project in the sidebar order, its tasks Needs you first', () => {
    expect(shape('project', vmStr)).toEqual([
      ['gtm', ['GTM-14', 'GTM-16', 'GTM-15', 'GTM-18', 'GTM-17', 'GTM-12']],
      ['herdr-projects', ['HP-201', 'HP-202', 'HP-203', 'HP-204']],
    ])
  })

  test('All machines by status: across every machine, not split by machine again; rows name machine and project', () => {
    const gs = groupTasks(ps, ts, 'status', 0)
    expect(shape('status', ts, ps, 0)).toEqual([
      ['Needs you', ['GTM-14', 'GTM-16', 'HP-201', 'LV-3', 'LV-4']],
      ['Ready', ['GTM-15']],
      ['Merging', ['GTM-18']],
      ['Working', ['HP-202']],
      ['Queued', ['HP-203']],
      ['To do', ['GTM-17', 'HP-204']],
      ['Done', ['GTM-12']],
    ])
    expect(gs.every((g) => !g.machine && names(g) === 'machine project')).toBe(true)
    // a machine's rows name their project only; a project's, nothing
    expect(names(groupTasks(ps, vmStr, 'status', 1)[0])).toBe('project')
    expect(names(groupTasks(ps, vmStr, 'status', 2)[0])).toBe('')
  })

  test('All machines by machine: one group per machine in the sidebar order, its rows naming their project', () => {
    const gs = groupTasks(ps, ts, 'machine', 0)
    expect(shape('machine', ts, ps, 0)).toEqual([
      ['vm-str', ['GTM-14', 'GTM-16', 'HP-201', 'GTM-15', 'GTM-18', 'HP-202', 'HP-203', 'GTM-17', 'HP-204', 'GTM-12']],
      ['mac', ['LV-3', 'LV-4']],
    ])
    expect(gs.map((g) => [g.kind, names(g)])).toEqual([['machine', 'project'], ['machine', 'project']])
    // a machine shown by a name of its own ("Mac mini" for the key mac) says that name
    const named = ms.map((m) => (m.id === 'm:mac' ? { ...m, name: 'Mac mini' } : m))
    expect(groupTasks(ps, ts, 'machine', 0, named).at(-1)!.label).toBe('Mac mini')
  })

  test('All machines by project: one group per project, its machine quiet after its name; Machine › Project under each machine', () => {
    const gs = groupTasks(ps, ts, 'project', 0)
    expect(gs.map((g) => [g.label, g.note, names(g)])).toEqual([
      ['gtm', 'vm-str', ''],
      ['herdr-projects', 'vm-str', ''],
      ['lilla-varlden', 'mac', ''],
    ])
    expect(shape('machine-project', ts, ps, 0)).toEqual([
      ['vm-str', 'gtm', ['GTM-14', 'GTM-16', 'GTM-15', 'GTM-18', 'GTM-17', 'GTM-12']],
      ['vm-str', 'herdr-projects', ['HP-201', 'HP-202', 'HP-203', 'HP-204']],
      ['mac', 'lilla-varlden', ['LV-3', 'LV-4']],
    ])
    // each group's key is its own, machine by machine
    const keys = groupTasks(ps, ts, 'machine-project', 0).map((g) => g.key)
    expect(new Set(keys).size).toBe(keys.length)
    // nothing at all: nothing, not an empty To do per machine
    expect(groupTasks(ps, [], 'machine-project', 0)).toEqual([])
    expect(groupTasks(ps, [], 'machine', 0)).toEqual([])
  })

  test('Machine › Space: a space of the same name on two machines is two groups, each under its machine', () => {
    const more: Place[] = [...ps, { id: 'p:mac:gtm', name: 'gtm', machine: 'mac', level: 2, status: 'idle', mode: 'regular', chat: 'p:mac:gtm' }]
    const extra = [...ts, { ...ts[0], id: 'MAC-1', place: 'p:mac:gtm' }]
    expect(shape('machine-project', extra, more, 0).map(([m, l]) => `${m} ${l}`)).toEqual(['vm-str gtm', 'vm-str herdr-projects', 'mac lilla-varlden', 'mac gtm'])
  })

  test('a list of more machines groups by status, machine, space or machine › space; of one machine by status or space; of one space by status', () => {
    expect(groupChoices(0)).toEqual(['status', 'machine', 'project', 'machine-project'])
    expect(groupChoices(1)).toEqual(['status', 'project'])
    expect(groupChoices(2)).toEqual(['status'])
    const g = { all: 'machine-project', machine: 'project' } as const
    expect(groupFor(g, 0)).toBe('machine-project')
    expect(groupFor(g, 1)).toBe('project')
    expect(groupFor(g, 2)).toBe('status')
    // a choice a machine's list doesn't have: by status
    expect(groupFor({ all: 'status', machine: 'machine' }, 1)).toBe('status')
  })

  test('Done tasks show newest finished first, by status and by project; one without a time goes last', () => {
    const d = (id: string, doneAt?: number) => ({ ...ts[0], id, status: 'done' as const, doneAt })
    const list = [d('D-old', 1), ts[0], d('D-none'), d('D-new', 3), d('D-mid', 2)]
    expect(groupTasks(ps, list, 'status').find((g) => g.key === 'done')!.tasks.map((t) => t.id)).toEqual(['D-new', 'D-mid', 'D-old', 'D-none'])
    expect(shape('project', list)).toEqual([['gtm', [ts[0].id, 'D-new', 'D-mid', 'D-old', 'D-none']]])
    expect(shape('machine-project', list, ps, 0)).toEqual([['vm-str', 'gtm', [ts[0].id, 'D-new', 'D-mid', 'D-old', 'D-none']]])
  })

  test('a group shows its first Done tasks until Show all, and says how many Done it has', () => {
    const done = Array.from({ length: DONE_SHOWN + 3 }, (_, i) => ({ ...ts[0], id: `D-${i}`, status: 'done' as const, doneAt: i }))
    const g = groupTasks(ps, [ts[0], ...done], 'project')[0]
    const cut = shownGroup(g, false)
    expect([cut.tasks.length, cut.count, cut.more, cut.less]).toEqual([1 + DONE_SHOWN, 1 + DONE_SHOWN + 3, DONE_SHOWN + 3, false])
    // the newest first, and Show all keeps that order
    expect(cut.tasks.slice(1).map((t) => t.doneAt)).toEqual([7, 6, 5, 4, 3])
    const all = shownGroup(g, true)
    // all showing: Show less cuts them short again
    expect([all.tasks.length, all.more, all.less]).toEqual([1 + DONE_SHOWN + 3, 0, true])
    expect(all.tasks.slice(1).map((t) => t.doneAt)).toEqual([7, 6, 5, 4, 3, 2, 1, 0])
    // with only a few Done there is nothing to show or hide
    const few = groupTasks(ps, [ts[0], ...done.slice(0, DONE_SHOWN)], 'project')[0]
    expect([shownGroup(few, true).less, shownGroup(few, false).more]).toEqual([false, 0])
  })
})

describe('archived tasks', () => {
  test('are in no group, by status or project, on a machine or All machines', () => {
    const archived = ts.filter((t) => t.archived).map((t) => t.id)
    expect(archived.length).toBeGreaterThan(0)
    for (const by of ['status', 'machine', 'project', 'machine-project'] as const) for (const level of [0, 1]) for (const g of groupTasks(ps, ts, by, level)) for (const t of g.tasks) expect(archived).not.toContain(t.id)
  })
})

describe('pinned tasks', () => {
  const pin = (ids: string[]) => ts.map((t) => (ids.includes(t.id) ? { ...t, pinned: 1 } : t))

  test('show first, in Pinned, by status order, and in no other group; by any grouping', () => {
    const list = pin(['GTM-12', 'HP-201', 'LV-3'])
    for (const [by, level] of [['status', 1], ['project', 1], ['status', 0], ['machine', 0], ['project', 0], ['machine-project', 0]] as const) {
      const [top, ...rest] = groupTasks(ps, list, by, level)
      expect(top.label).toBe('Pinned')
      // the same order the rest go in: Needs you first, Done last
      const rank = top.tasks.map((t) => GROUPS.findIndex(([st]) => st === t.status))
      expect(rank).toEqual([...rank].sort((a, b) => a - b))
      expect(top.tasks.map((t) => t.id).sort()).toEqual(['GTM-12', 'HP-201', 'LV-3'])
      for (const g of rest) for (const t of g.tasks) expect(t.pinned).toBeUndefined()
      expect(rest.reduce((n, g) => n + g.tasks.length, top.tasks.length)).toBe(groupTasks(ps, ts, by, level).reduce((n, g) => n + g.tasks.length, 0))
    }
    // Pinned says nothing of where: its rows do, as much as the view doesn't
    expect(names(groupTasks(ps, list, 'project', 1)[0])).toBe('project')
    expect(names(groupTasks(ps, list, 'machine-project', 0)[0])).toBe('machine project')
  })

  test('no Pinned group when nothing is pinned, and none for an archived one', () => {
    for (const by of ['status', 'project'] as const) expect(groupTasks(ps, ts, by, 0).map((g) => g.key)).not.toContain('pinned')
    const archived = ts.find((t) => t.archived)!.id
    expect(groupTasks(ps, pin([archived]), 'status').map((g) => g.key)).not.toContain('pinned')
  })

  test('Pinned shows all its Done tasks, never cut short', () => {
    const done = Array.from({ length: DONE_SHOWN + 3 }, (_, i) => ({ ...ts[0], id: `D-${i}`, status: 'done' as const, doneAt: i, archived: undefined, pinned: 1 }))
    const g = groupTasks(ps, done, 'status')[0]
    expect(shownGroup(g, false)).toMatchObject({ more: 0, count: done.length })
    expect(shownGroup(g, false).tasks.length).toBe(done.length)
  })
})

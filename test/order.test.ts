// The sidebar's order: A to Z, the order the human dragged, where a new space slots in, the home space first until
// dragged, and the sections spaces move into.
import { describe, expect, test } from 'bun:test'
import { deleteSection, moveSpace, newSection, ordered, PINNED, sectionProblem, setOrder, sortPlaces, TOP, type PlaceOrder, type SectionDefs } from '../shared/order.ts'
import type { Machine, Place } from '../shared/types.ts'

const names = (xs: string[], saved?: string[]) => ordered(xs.map((name) => ({ name })), saved, (x) => x.name, (x) => x.name).map((x) => x.name)

describe('ordering places', () => {
  test('A to Z by default, any case, numbers as numbers', () => {
    expect(names(['shepherdr', 'shepherdr-ttt-2', 'gtm'])).toEqual(['gtm', 'shepherdr', 'shepherdr-ttt-2'])
    expect(names(['b', 'A', 'app10', 'app9'])).toEqual(['A', 'app9', 'app10', 'b'])
  })

  test('the saved order wins; one it lacks goes after the last one before it A to Z; one gone is skipped', () => {
    expect(names(['gtm', 'shepherdr', 'shepherdr-ttt-2'], ['shepherdr', 'gtm', 'shepherdr-ttt-2'])).toEqual(['shepherdr', 'gtm', 'shepherdr-ttt-2'])
    // a favourite on top stays on top
    expect(names(['gtm', 'shepherdr', 'shepherdr-ttt-2', 'herdr'], ['shepherdr', 'gtm', 'shepherdr-ttt-2'])).toEqual(['shepherdr', 'gtm', 'herdr', 'shepherdr-ttt-2'])
    expect(names(['gtm', 'zed', 'alpha'], ['zed', 'gtm'])).toEqual(['alpha', 'zed', 'gtm'])
    expect(names(['gtm'], ['old', 'gtm'])).toEqual(['gtm'])
  })

  test('each machine in its order, its home space first until it is dragged, then its spaces in theirs', () => {
    const sorted = sortPlaces(places, machines, {})
    expect(sorted.machines.map((x) => x.id)).toEqual(['m:mac', 'm:vm'])
    expect(sorted.places.map((x) => x.id)).toEqual(['p:mac:~', 'p:mac:x', 'p:vm:~', 'p:vm:a', 'p:vm:b'])
    // an order saved before the home space could move (it lacks it): home first still
    expect(sortPlaces(places, machines, { 'm:vm': ['p:vm:b', 'p:vm:a'] }).places.map((x) => x.id)).toEqual(['p:mac:~', 'p:mac:x', 'p:vm:~', 'p:vm:b', 'p:vm:a'])
    const dragged = sortPlaces(places, machines, { all: ['m:vm', 'm:mac'], 'm:vm': ['p:vm:b', 'p:vm:~', 'p:vm:a'] })
    expect(dragged.machines.map((x) => x.id)).toEqual(['m:vm', 'm:mac'])
    expect(dragged.places.map((x) => x.id)).toEqual(['p:vm:b', 'p:vm:~', 'p:vm:a', 'p:mac:~', 'p:mac:x'])
  })
})

const m = (name: string): Machine => ({ id: `m:${name}`, machine: name, name, status: 'idle' })
const p = (machine: string, name: string, home = false): Place => ({ id: `p:${machine}:${home ? '~' : name}`, name, machine, ...(home ? { home } : {}), level: home ? 1 : 2, status: 'idle', mode: 'regular', chat: `p:${machine}:${name}` })
const places = [p('vm', 'b'), p('vm', 'a'), p('vm', 'home', true), p('mac', 'x'), p('mac', 'home', true)]
const machines = [m('vm'), m('mac')]

describe('sections', () => {
  test('Pinned is always one, first at the top; one made goes last among its peers; a space moves in and back where it was', () => {
    const order: PlaceOrder = {}
    const defs: SectionDefs = {}
    const top = newSection(defs, order, { name: 'Clients', emoji: '💼' }, machines)
    const inner = newSection(defs, order, { name: 'Labs', machine: 'vm' }, machines)
    expect(sortPlaces(places, machines, order, defs).sections.map((s) => [s.id, s.name, s.machine ?? '', s.spaces])).toEqual([
      [PINNED, 'Pinned', '', []],
      [top, 'Clients', '', []],
      [inner, 'Labs', 'vm', []],
    ])
    // any machine's space goes in one at the top; only its own machine's in one inside it
    expect(moveSpace(order, defs, places, machines, 'p:mac:x', top)).toBeUndefined()
    expect(moveSpace(order, defs, places, machines, 'p:vm:b', top)).toBeUndefined()
    expect(moveSpace(order, defs, places, machines, 'p:mac:~', inner)).toBe('That section holds only its machine’s spaces.')
    expect(moveSpace(order, defs, places, machines, 'p:vm:a', inner)).toBeUndefined()
    expect(moveSpace(order, defs, places, machines, 'p:vm:~', PINNED)).toBeUndefined()
    // before another there
    expect(moveSpace(order, defs, places, machines, 'p:mac:~', top, 'p:vm:b')).toBeUndefined()
    let sorted = sortPlaces(places, machines, order, defs)
    expect(sorted.sections.map((s) => s.spaces)).toEqual([['p:vm:~'], ['p:mac:x', 'p:mac:~', 'p:vm:b'], ['p:vm:a']])
    // as the sidebar shows them: the top ones, then each machine's own, then its sections'
    expect(sorted.places.map((x) => x.id)).toEqual(['p:vm:~', 'p:mac:x', 'p:mac:~', 'p:vm:b', 'p:vm:a'])
    // one move: out of one section into another
    expect(moveSpace(order, defs, places, machines, 'p:vm:b', PINNED)).toBeUndefined()
    expect(sortPlaces(places, machines, order, defs).sections.map((s) => s.spaces)).toEqual([['p:vm:~', 'p:vm:b'], ['p:mac:x', 'p:mac:~'], ['p:vm:a']])
    // back under its machine, where it was; never another machine
    expect(moveSpace(order, defs, places, machines, 'p:vm:b', 'm:mac')).toBe('A space stays on its own machine.')
    expect(moveSpace(order, defs, places, machines, 'p:vm:b', 'm:vm')).toBeUndefined()
    // a section deleted: its spaces back under their machines
    deleteSection(defs, order, top)
    sorted = sortPlaces(places, machines, order, defs)
    expect(sorted.sections.map((s) => s.id)).toEqual([PINNED, inner])
    expect(sorted.places.map((x) => x.id)).toEqual(['p:vm:~', 'p:mac:~', 'p:mac:x', 'p:vm:b', 'p:vm:a'])
  })

  test('the sections at the top drag into an order, Pinned among them; names are their own among their peers', () => {
    const order: PlaceOrder = {}
    const defs: SectionDefs = {}
    const a = newSection(defs, order, { name: 'A' }, machines)
    order[TOP] = [a, PINNED]
    expect(sortPlaces(places, machines, order, defs).sections.map((s) => s.id)).toEqual([a, PINNED])
    expect(sectionProblem(defs, 'a')).toBe('Another section is called a.')
    expect(sectionProblem(defs, 'pinned')).toBe('Pinned is there already.')
    expect(sectionProblem(defs, 'A', 'vm')).toBeUndefined()
    expect(sectionProblem(defs, '')).toBe('A section needs a name.')
  })

  test("a machine's spaces dragged as they show keep those in sections where they were", () => {
    const order: PlaceOrder = { 'm:vm': ['p:vm:~', 'p:vm:a', 'p:vm:b'] }
    // a sits in a section: the human drags b over home
    setOrder(order, 'm:vm', ['p:vm:b', 'p:vm:~'])
    expect(order['m:vm']).toEqual(['p:vm:b', 'p:vm:~', 'p:vm:a'])
    setOrder(order, 'all', ['m:vm'])
    expect(order.all).toEqual(['m:vm'])
  })
})

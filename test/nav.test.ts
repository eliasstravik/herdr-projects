// Back inside the app: which changes of screen are steps in the history, which replace one, and which go back.
import { describe, expect, test } from 'bun:test'
import { decide, stepKey, stepsTo, urlOf, type Entry, type Screen } from '../src/nav.ts'

const at = (s: Partial<Screen> = {}): Screen => ({
  place: 'all',
  task: null,
  tab: 'chat',
  sheet: null,
  viewer: null,
  settings: null,
  search: false,
  drawer: false,
  photo: null,
  adding: false,
  archived: null,
  ...s,
})

/** the history entry for a screen, with the step before it */
const entry = (s: Screen, prev?: Screen): Entry => ({ shep: 1, key: stepKey(s), prev: prev ? stepKey(prev) : null, screen: s })

describe('steps in the history', () => {
  test('a task, a place, the drawer and every layer over them are steps', () => {
    const place = at({ place: 'p:gtm' })
    expect(decide(entry(place), at({ place: 'p:gtm', task: 'GTM-1' }))).toBe('push')
    expect(decide(entry(place), at({ place: 'p:other' }))).toBe('push')
    for (const layer of [{ drawer: true }, { settings: { section: 0 } }, { search: true }, { viewer: { artifact: 'a1' } }, { photo: { chat: 'c', msg: 'm' } }, { adding: true }])
      expect(decide(entry(place), at({ place: 'p:gtm', ...layer }))).toBe('push')
    expect(decide(entry(at({ task: 'T' })), at({ task: 'T', sheet: 'files' }))).toBe('push')
  })

  test('closing what the step before showed goes back to it, so Back never lands on what you closed', () => {
    const place = at()
    const task = at({ task: 'GTM-1' })
    expect(decide(entry(task, place), place)).toBe('back')
    expect(decide(entry(at({ drawer: true }), place), place)).toBe('back')
    expect(decide(entry(at({ task: 'T', sheet: 'diff' }), at({ task: 'T' })), at({ task: 'T' }))).toBe('back')
  })

  test('task, then place, then the places list it was picked from', () => {
    // the drawer open, a place picked from it, a task opened there
    const list = at({ drawer: true })
    const place = at({ place: 'p:gtm' })
    expect(decide(entry(list, at()), place)).toBe('push')
    const task = at({ place: 'p:gtm', task: 'GTM-1' })
    expect(decide(entry(place, list), task)).toBe('push')
    // Back from the task lands on the place, whose step before is the list
    expect(decide(entry(task, place), place)).toBe('back')
  })

  test("a section opened from a phone's list of them is a step, and Back from it goes to the list", () => {
    const list = at({ settings: { section: null } })
    const section = at({ settings: { section: 2 } })
    expect(decide(entry(at()), list)).toBe('push')
    expect(decide(entry(list, at()), section)).toBe('push')
    expect(decide(entry(section, list), list)).toBe('back')
  })

  test('switching tabs, a settings section, a version or the picture shown replaces the step', () => {
    expect(decide(entry(at()), at({ tab: 'tasks' }))).toBe('replace')
    expect(decide(entry(at({ settings: { section: 0 } })), at({ settings: { section: 2 } }))).toBe('replace')
    expect(decide(entry(at({ viewer: { artifact: 'a1' } })), at({ viewer: { artifact: 'a1', v: 2 } }))).toBe('replace')
    expect(decide(entry(at({ photo: { chat: 'c', msg: '1' } })), at({ photo: { chat: 'c', msg: '2' } }))).toBe('replace')
  })

  test('a layer closed, or swapped for another, over the same place replaces its step rather than adding one', () => {
    // search opened from the drawer takes the drawer's step: Back from it is the place, not the drawer again
    expect(decide(entry(at({ drawer: true }), at()), at({ search: true }))).toBe('replace')
    // a layer closed whose step before isn't the bare place (an old entry): no new step for it
    expect(decide(entry(at({ search: true }), at({ drawer: true })), at())).toBe('replace')
  })

  test('nothing written yet: the first screen replaces the entry the app opened on', () => {
    expect(decide(null, at())).toBe('replace')
  })
})

describe('a link from an alert', () => {
  test('a task gets its place first, so Back from it goes to the place', () => {
    const steps = stepsTo(at({ place: 'p:gtm', task: 'GTM-1' }))
    expect(steps.map((s) => [s.place, s.task])).toEqual([
      ['p:gtm', null],
      ['p:gtm', 'GTM-1'],
    ])
  })

  test('a sheet over a task: the place, the task, then the sheet', () => {
    const steps = stepsTo(at({ place: 'p:gtm', task: 'GTM-1', sheet: 'diff' }))
    expect(steps.map((s) => [s.task, s.sheet])).toEqual([
      [null, null],
      ['GTM-1', null],
      ['GTM-1', 'diff'],
    ])
  })

  test('an artifact over a place: the place, then the artifact; a place alone is one step', () => {
    expect(stepsTo(at({ viewer: { artifact: 'a6' } })).map((s) => s.viewer?.artifact ?? null)).toEqual([null, 'a6'])
    expect(stepsTo(at({ place: 'p:gtm', tab: 'tasks' }))).toEqual([at({ place: 'p:gtm', tab: 'tasks' })])
  })
})

describe('the address', () => {
  test('place, task, the phone tab and what is open over them', () => {
    expect(urlOf(at({ place: 'p:gtm', tab: 'tasks' }), true, '/')).toBe('/?place=p%3Agtm&tab=tasks')
    expect(urlOf(at({ place: 'p:gtm', tab: 'tasks' }), false, '/')).toBe('/?place=p%3Agtm')
    expect(urlOf(at({ task: 'T', sheet: 'files' }), true, '/')).toBe('/?place=all&task=T&item=files')
    expect(urlOf(at({ viewer: { artifact: 'a1' } }), true, '/')).toBe('/?place=all&artifact=a1')
  })
})

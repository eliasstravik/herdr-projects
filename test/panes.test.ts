// Split view's panes: opened beside the focused one up to the room there is, closed, made the only one, a place
// picked into the focused one; a lone place's chat is the middle as always.
import { describe, expect, test } from 'bun:test'
import { closed, normal, NO_PANES, only, opened, replaced, type Pane } from '../src/panes.ts'

const home: Pane = { kind: 'place', id: 'home' }
const gtm: Pane = { kind: 'place', id: 'gtm' }
const task: Pane = { kind: 'task', id: 'GTM-14' }
const art: Pane = { kind: 'artifact', id: 'a7' }

describe('split view panes', () => {
  test('the first split puts what the middle shows first and the new one beside it, focused', () => {
    expect(opened(NO_PANES, home, gtm, 3)).toEqual({ panes: [home, gtm], pane: 1, sizes: [1, 1] })
    // beside the focused one, not at the end
    const two = { panes: [home, gtm], pane: 0, sizes: [2, 1] }
    expect(opened(two, home, task, 3)).toEqual({ panes: [home, task, gtm], pane: 1, sizes: [1, 1, 1] })
  })

  test('one already open is focused, not opened twice', () => {
    const two = { panes: [home, gtm], pane: 1, sizes: [1, 1] }
    expect(opened(two, home, home, 3)).toEqual({ ...two, pane: 0 })
  })

  test('no room for another: it takes the place of the one beside the focused one', () => {
    const two = { panes: [home, gtm], pane: 0, sizes: [3, 1] }
    expect(opened(two, home, art, 2)).toEqual({ panes: [home, art], pane: 1, sizes: [1, 1] })
    expect(opened({ ...two, pane: 1 }, home, art, 2)).toEqual({ panes: [art, gtm], pane: 0, sizes: [1, 1] })
    // room for one only: it is the only one
    expect(opened(NO_PANES, home, task, 1)).toEqual({ panes: [task], pane: 0, sizes: [1] })
  })

  test('closed, its neighbour takes the focus; a lone place left is the middle as always', () => {
    const three = { panes: [home, gtm, task], pane: 1, sizes: [1, 2, 1] }
    expect(closed(three, 1)).toEqual({ panes: [home, task], pane: 1, sizes: [1, 1] })
    expect(closed(three, 0)).toEqual({ panes: [gtm, task], pane: 0, sizes: [2, 1] })
    expect(closed({ panes: [home, task], pane: 1, sizes: [1, 1] }, 1)).toEqual(NO_PANES)
    // a lone task stays a pane: closing it brings the place's chat back
    expect(closed({ panes: [home, task], pane: 1, sizes: [1, 1] }, 0)).toEqual({ panes: [task], pane: 0, sizes: [1] })
    expect(closed({ panes: [task], pane: 0, sizes: [1] }, 0)).toEqual(NO_PANES)
  })

  test('the only one', () => {
    const three = { panes: [home, gtm, task], pane: 0, sizes: [1, 1, 1] }
    expect(only(three, 2)).toEqual({ panes: [task], pane: 0, sizes: [1] })
    expect(only(three, 1)).toEqual(NO_PANES)
  })

  test('a place picked opens in the focused pane, or focuses the pane that has it', () => {
    const two = { panes: [home, task], pane: 1, sizes: [1, 1] }
    expect(replaced(two, 1, gtm)).toEqual({ panes: [home, gtm], pane: 1, sizes: [1, 1] })
    expect(replaced(two, 1, home)).toEqual({ ...two, pane: 0 })
  })

  test('sizes that no longer fit are shared evenly', () => {
    expect(normal({ panes: [home, gtm], pane: 5, sizes: [1] })).toEqual({ panes: [home, gtm], pane: 1, sizes: [1, 1] })
  })
})

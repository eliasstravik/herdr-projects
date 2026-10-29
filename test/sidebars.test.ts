// The wide screen's sidebar widths: within their spans, the chat keeping its room, places snapping shut.
import { describe, expect, test } from 'bun:test'
import { BASE, canDock, CHAT_MIN, CHAT_READ, dragLeft, dragPanel, dragRight, fit, FOCUS_BELOW, LEFT, PANEL, PANEL_SNAP, RIGHT, showPanel, SNAP, toggleFocus } from '../src/sidebars.ts'

describe('sidebar widths', () => {
  test('a wide window draws them as set; hidden places takes no room', () => {
    expect(fit(BASE, 1440, true)).toEqual({ left: LEFT.base, right: 400, panel: 0 })
    expect(fit({ ...BASE, open: false }, 1440, true)).toEqual({ left: 0, right: 400, panel: 0 })
    // a tablet: places is a drawer
    expect(fit(BASE, 900, false)).toEqual({ left: 0, right: 400, panel: 0 })
  })

  test('left at its default, tasks grow a little on a very wide window; set by hand, they keep their width', () => {
    expect(fit(BASE, 1920, true).right).toBe(400)
    expect(fit(BASE, 2560, true).right).toBe(480)
    expect(fit(BASE, 5120, true).right).toBe(480)
    expect(fit({ ...BASE, right: 420 }, 2560, true).right).toBe(420)
  })

  test('a narrow window shrinks tasks before the chat goes under its room, never under their minimum', () => {
    const { left, right } = fit({ ...BASE, right: 700 }, 1100, true)
    expect(left).toBe(0)
    expect(1100 - left - right).toBe(CHAT_READ)
    expect(fit({ ...BASE, right: 700 }, 640, false).right).toBe(RIGHT.min)
  })

  test('tablet and small desktop docking follows requested widths, without changing saved choices', () => {
    for (const width of [820, 1024, 1100]) {
      expect(canDock(BASE, width)).toBe(false)
      const drawn = fit(BASE, width, true)
      expect(drawn.left).toBe(0)
      expect(width - drawn.right).toBeGreaterThanOrEqual(CHAT_READ)
    }
    for (const width of [1180, 1440, 1920]) expect(canDock(BASE, width)).toBe(true)
    // Widened lists or tools need more room, even on a desktop. Narrowing the window keeps saved widths intact.
    const saved = { ...BASE, right: 700 }
    expect(canDock(saved, 1440)).toBe(true)
    expect(canDock(saved, 1180)).toBe(false)
    expect(fit(saved, 1180, true).left).toBe(0)
    expect(fit(saved, 1440, true).right).toBe(700)
    expect(saved).toEqual({ ...BASE, right: 700 })
    expect(canDock({ ...BASE, panelOpen: true }, 1600, true)).toBe(false)
    expect(canDock({ ...BASE, panelOpen: true }, 1920, true)).toBe(true)
    expect(fit({ ...BASE, open: false }, 2560, true).left).toBe(0)
    expect(fit(BASE, 2560, true, true).panel).toBe(0)
  })

  test('dragging stays within the spans; places snaps shut below SNAP and keeps its width to open at', () => {
    const wide = { ...BASE, left: 320 }
    expect(dragLeft(wide, 1000, 2000).left).toBe(LEFT.max)
    expect(dragLeft(wide, SNAP + 10, 2000)).toEqual({ ...wide, left: LEFT.min, open: true })
    expect(dragLeft(wide, SNAP - 1, 2000)).toEqual({ ...wide, open: false })
    // dragged back out in the same drag, it opens again
    expect(dragLeft({ ...wide, open: false }, 280, 2000)).toEqual({ ...wide, left: 280, open: true })
    expect(dragRight(BASE, 10, 2000, true).right).toBe(RIGHT.min)
    expect(dragRight(BASE, 900, 1200, true).right).toBe(1200 - LEFT.base - CHAT_READ)
  })

  test('tasks drag as wide as the window leaves the chat its room, and keep that width through a resize', () => {
    // a big screen: no fixed most, only the chat's room (and places' width, or none while hidden)
    expect(dragRight(BASE, 5000, 5120, true).right).toBe(5120 - LEFT.base - CHAT_READ)
    expect(dragRight({ ...BASE, open: false }, 5000, 5120, true).right).toBe(5120 - CHAT_READ)
    // a narrower window draws them narrower; wide again, they come back to the width they were dragged to
    const wide = { ...BASE, right: 4000 }
    expect(fit(wide, 1440, true).right).toBe(1440 - CHAT_READ)
    expect(fit(wide, 5120, true).right).toBe(4000)
  })

  test("an open task's side panel takes its width; tasks give way first, then on a narrow window places is drawn shut", () => {
    const open = { ...BASE, panelOpen: true }
    // closed, or no task open: no room taken
    expect(fit(BASE, 1920, true, true).panel).toBe(0)
    expect(fit(open, 1920, true, false).panel).toBe(0)
    expect(fit(open, 1920, true, true)).toEqual({ left: LEFT.base, right: 400, panel: PANEL.base })
    // the chat keeps its room: the panel shrinks to fit, never under its minimum
    const mid = fit({ ...open, panel: 900 }, 1600, true, true)
    expect(1600 - mid.left - mid.right - mid.panel).toBe(CHAT_READ)
    const tight = fit(open, 1200, true, true)
    expect(tight.left).toBe(0)
    expect(tight.panel).toBeGreaterThanOrEqual(PANEL.min)
  })

  test('dragging the panel stays within its span and snaps it shut below PANEL_SNAP, keeping its width', () => {
    const open = { ...BASE, panelOpen: true, panel: 600 }
    // no fixed most: as wide as tasks at their least and the chat's room leave it
    expect(dragPanel(open, 5000, 4000, true).panel).toBe(4000 - LEFT.base - RIGHT.min - CHAT_READ)
    expect(dragPanel(open, PANEL_SNAP + 1, 2000, true)).toEqual({ ...open, panel: PANEL.min })
    expect(dragPanel(open, PANEL_SNAP - 1, 2000, true)).toEqual({ ...open, panelOpen: false })
    // dragged wide, tasks give way down to their minimum
    const wide = dragPanel(open, 1200, 1700, true)
    expect(wide.panel).toBe(1700 - RIGHT.min - CHAT_READ)
    expect(fit(wide, 1700, true, true).right).toBe(RIGHT.min)
  })

  test('focus gives an open task the window: places and the chat take nothing, its chat the rest beside the panel', () => {
    const focus = { ...BASE, focus: true, panelOpen: true }
    expect(fit(focus, 1440, true, true, true)).toEqual({ left: 0, right: 1440 - PANEL.base, panel: PANEL.base })
    // no task open: focus waits
    expect(fit(focus, 1440, true, false, false)).toEqual(fit({ ...focus, focus: false }, 1440, true, false, false))
    // a tablet: the task's chat takes it all
    expect(fit({ ...BASE, focus: true }, 900, false, false, true)).toEqual({ left: 0, right: 900, panel: 0 })
    // widened, the panel takes all but the chat's least, in focus whether or not it was on
    expect(fit({ ...BASE, panelOpen: true, wide: true }, 1440, true, true, true)).toEqual({ left: 0, right: CHAT_MIN, panel: 1440 - CHAT_MIN })
  })

  test('opening the panel on a narrow window turns focus on, and closing it turns that focus off again', () => {
    const narrow = showPanel(BASE, true, FOCUS_BELOW - 1)
    expect(narrow).toMatchObject({ panelOpen: true, focus: true, focusAuto: true })
    expect(showPanel(narrow, false, FOCUS_BELOW - 1)).toMatchObject({ panelOpen: false, focus: false, focusAuto: false })
    expect(showPanel(BASE, true, FOCUS_BELOW)).toMatchObject({ panelOpen: true, focus: false })
    // focus turned on by hand stays when the panel closes; a widened panel opens at its own width next time
    const own = toggleFocus({ ...BASE, panelOpen: true, wide: true }, true)
    expect(own).toMatchObject({ focus: false, wide: false })
    const on = toggleFocus(BASE, true)
    expect(showPanel({ ...on, panelOpen: true, wide: true }, false, 1000)).toMatchObject({ focus: true, wide: false })
  })

  test('in focus the panel drags up to all but the least of the chat; a widened one dragged keeps its width, in focus', () => {
    const focus = { ...BASE, focus: true, panelOpen: true }
    expect(dragPanel(focus, 1300, 1440, true, true).panel).toBe(1440 - RIGHT.min)
    expect(dragPanel(focus, 5000, 5120, true, true).panel).toBe(5120 - RIGHT.min)
    const dragged = dragPanel({ ...BASE, panelOpen: true, wide: true }, 900, 1440, true, true)
    expect(dragged).toMatchObject({ wide: false, focus: true, panel: 900 })
  })
})

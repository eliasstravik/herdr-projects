// Coordinators: one agent per place. Each does what the human asks, and Autopilot is a switch on top of it:
// then it also works on its own.
// The switch is at the top of the coordinator chat's ⋯ menu, over search and the terminal. While Autopilot is on, a small
// red pill at the header's right, beside that menu (on a phone after the subtitle, under the name), says so, and a tap
// on it opens the menu; with it off the header says nothing of it.
import { useState } from 'react'
import { act, definePlugin, has, setUI, useSnap, useUI } from '@client/kernel.ts'
import { ic, MenuItem, MenuSwitch, ModeIcon } from '@client/ui.tsx'
import type { Place } from '@shared/types.ts'

/** Autopilot on, as a red pill with a dot: at the header's right, or after the subtitle on a phone; a tap opens the ⋯
 *  menu, where it is switched off. Nothing when off */
function AutopilotPill({ place }: { place: Place }) {
  const menu = useUI((u) => u.menu)
  return place.mode === 'autopilot' ? (
    <button
      type="button"
      data-mode="autopilot"
      aria-label="Autopilot is on. Change it"
      aria-haspopup="menu"
      aria-expanded={menu}
      className="mode-pill hit press"
      onClick={() => setUI({ menu: !menu, plus: null, pick: null })}
    >
      <span>Autopilot</span>
    </button>
  ) : null
}

/** Autopilot on or off at the top of the ⋯ menu */
function AutopilotSwitch({ place }: { place: Place }) {
  const on = place.mode === 'autopilot'
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const state = useSnap()?.settings.find((section) => section.name === 'Autopilot' && section.level === place.id)?.rows.find((row) => row.label === 'State')?.value
  const change = async (keepWorkers = false) => {
    if (busy) return
    setBusy(true); setError('')
    try { const result = await act({ type: 'mode', place: place.id, mode: on ? 'regular' : 'autopilot', keepWorkers }); setError(result.error ?? '') }
    catch (e) { setError((e as Error).message) }
    finally { setBusy(false) }
  }
  if (!on && !has('autopilot')) return null
  return (
    <>
    <MenuSwitch
      name="Autopilot"
      icon={<ModeIcon className={ic} />}
      on={on}
      note={busy ? 'Changing coordinator…' : state && state !== 'On' && state !== 'Off' ? state : on ? 'Works on its own' : 'Off: does what you ask'}
      onClick={() => void change()}
    />
    {on ? <MenuItem onClick={() => void change(true)}>Turn off; let workers finish</MenuItem> : null}
    {error ? <p role="alert" className="max-w-72 px-3 py-2 text-sm text-pretty text-need [overflow-wrap:anywhere]">{error}</p> : null}
    </>
  )
}

export default definePlugin({
  name: 'coordinators',
  setup(reg) {
    reg.placeMark.push({ order: 10, View: AutopilotPill })
    reg.placeMenu.push({ order: 10, View: AutopilotSwitch })
  },
})

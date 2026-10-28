// Keyboard: on a desktop everything works without a mouse. ? opens the list of keys in Settings; ⌘K (Ctrl+K off a Mac) searches and jumps.
import { useMemo, useState } from 'react'
import {
  agentChat,
  closeArtifact,
  definePlugin,
  getState,
  goPlace,
  panelHere,
  has,
  keysChat,
  listView,
  matches,
  openTask,
  orderedTasks,
  placeOf,
  shownPlaces,
  setUI,
  taskOf,
  toggleFocus,
  togglePlaces,
  toggleWide,
  useSnap,
  type UI,
} from '@client/kernel.ts'
import { toggleDictation } from '@plugins/chat/client.tsx'
import clsx from 'clsx'
import { ALT, CTRL, Dot, ic, Kbd, KeyboardIcon, MOD, PlaceIcon, rowClass, SearchIcon, SHIFT, TaskDot } from '@client/ui.tsx'
import { rowLook, sectionOf } from '@client/Settings.tsx'
import { machineName, type Snapshot, type Report } from '@shared/types.ts'
import { Sheet, SheetClose } from '@client/Stack.tsx'

export const KEYS: [string[], string][] = [
  [['↑', '↓'], 'Move in tasks (or J, K)'],
  [['Enter'], 'Open'],
  [['Esc'], 'Back'],
  [['[', ']'], 'Previous or next place'],
  [[ALT, '↑', '↓'], 'Place above or below in Places, from the message box too'],
  [[ALT, SHIFT, '↑', '↓'], 'Move a focused machine or space up or down'],
  [[MOD, 'K'], 'Search and jump'],
  [[MOD, 'B'], 'Show or hide Places'],
  [[MOD, 'Click'], 'Open a space, task or artifact in split view'],
  [['F'], "Focus on a task: hide its coordinator's chat, or show it again"],
  [[SHIFT, 'F'], 'Widen the Task panel, or narrow it again'],
  [['/'], 'Write a message'],
  [['F'], 'Filter the tasks or artifacts (Esc clears it)'],
  [['T'], 'Talk to type: Enter puts it in the box, Esc drops it'],
  [['V'], 'Voice call (V again ends it)'],
  [['M'], 'Mute or unmute the call'],
  [['E'], 'React to the last message'],
  [[CTRL, '`'], "Terminal and back: a task's in its Task panel"],
  [[','], 'Settings'],
  [['N'], 'New space'],
  [['A'], 'Attach a file'],
  [['Tab'], 'Move through buttons, form choices and Send'],
  [['?'], 'These shortcuts'],
]

const SHORTCUTS = 'Keyboard shortcuts'

/** Settings' Keyboard shortcuts page (a desktop's): each key beside what it does, in Settings' own rows */
function Shortcuts({ phone }: { phone: boolean }) {
  return (
    <dl className="w-full">
      {KEYS.map(([k, l]) => (
        <div key={l} className={rowLook(phone)}>
          <dt className="min-w-0">{l}</dt>
          <dd className="shrink-0">
            <Kbd keys={k} />
          </dd>
        </div>
      ))}
    </dl>
  )
}

/* ---------- ⌘K ---------- */

/** a place's hit has its level (its icon, as in the sidebar); a task's its status dot */
type Hit = { id: string; tag?: string; label: string; sub: string; level?: number; status: Parameters<typeof Dot>[0]['s']; w?: Report; go: () => void }

/** an archived task opens with its list's Archived view behind it, where it is listed (Esc goes back there) */
function openArchived(id: string) {
  openTask(id)
  const { ui, snap } = getState()
  setUI({ archived: snap ? listView(ui, snap).key : ui.place })
}

function hits(snap: Snapshot, q: string): Hit[] {
  const n = q.trim().toLowerCase()
  const all: Hit[] = [
    ...shownPlaces(snap).map((p) => ({ id: `p${p.id}`, label: p.name, sub: `${p.home ? 'Home space' : 'Space'} on ${machineName(snap.machines, p.machine)}`, level: 2, status: p.status, go: () => goPlace(p.id) })),
    ...snap.tasks.filter((t) => !t.archived).map((t) => ({ id: `t${t.id}`, tag: t.id, label: t.name, sub: [placeOf(snap, t.place).name, t.report && !t.report.paused ? t.report.msg : t.sub].filter(Boolean).join(' · '), status: t.status, w: t.report, go: () => openTask(t.id) })),
  ]
  // archived tasks are found by what you type, after the rest; never in the list before you type
  if (!n) return all.slice(0, 12)
  all.push(...snap.tasks.filter((t) => t.archived).map((t) => ({ id: `t${t.id}`, tag: t.id, label: t.name, sub: ['Archived', placeOf(snap, t.place).name, t.sub].filter(Boolean).join(' · '), status: (t.status === 'done' ? 'done' : 'idle') as Hit['status'], go: () => openArchived(t.id) })))
  return all.filter((h) => matches(n, h.tag, h.label, h.sub)).slice(0, 20)
}

function Search({ onClose }: { onClose: () => void }) {
  const snap = useSnap()!
  const [q, setQ] = useState('')
  const [cur, setCur] = useState(0)
  const list = useMemo(() => hits(snap, q), [snap, q])
  const pick = (h?: Hit) => {
    if (!h) return
    onClose()
    h.go()
  }
  return (
    <Sheet label="Search or jump to" onClose={onClose} panel="max-w-[560px]">
        <div className="flex shrink-0 items-center gap-2 border-b border-line pr-2 pl-4">
          <SearchIcon className={`${ic} text-fg3`} />
          <input
            autoFocus
            name="search"
            role="combobox"
            aria-expanded="true"
            aria-controls="search-list"
            aria-activedescendant={list[cur] ? `search-${cur}` : undefined}
            aria-label="Search or jump to"
            placeholder="Search or jump to"
            value={q}
            onChange={(e) => {
              setQ(e.target.value)
              setCur(0)
            }}
            onKeyDown={(e) => {
              if (e.key === 'ArrowDown') {
                e.preventDefault()
                setCur(Math.min(list.length - 1, cur + 1))
              } else if (e.key === 'ArrowUp') {
                e.preventDefault()
                setCur(Math.max(0, cur - 1))
              } else if (e.key === 'Enter') pick(list[cur])
              else if (e.key === 'Escape') onClose()
            }}
            className="h-[52px] min-w-0 flex-1 bg-transparent outline-none placeholder:text-fg3 phone:text-[1rem]"
          />
          <SheetClose onClose={onClose} />
        </div>
        <ul id="search-list" role="listbox" aria-label="Places and tasks" className="max-h-[60vh] min-h-0 overflow-y-auto overscroll-contain p-1.5 phone:max-h-none phone:flex-1 phone:pb-[calc(env(safe-area-inset-bottom)+6px)]">
          {list.map((h, i) => (
            <li
              key={h.id}
              id={`search-${i}`}
              role="option"
              aria-selected={i === cur}
              onMouseMove={() => setCur(i)}
              onClick={() => pick(h)}
              // a list row's look, the one the arrow keys are on picked out
              className={clsx(rowClass(i === cur), 'min-h-10 gap-2.5 py-1.5')}
            >
              {h.level !== undefined ? <PlaceIcon level={h.level} className={clsx(ic, h.status === 'off' ? 'text-fg3' : i === cur ? 'text-fg' : 'text-fg2')} /> : <TaskDot s={h.status} w={h.w} />}
              <span className="min-w-0 flex-1 truncate">
                {/* a task's id is grey, as in the task list */}
                {h.tag ? <span className="text-fg3">{h.tag} </span> : null}
                {h.label}
                <span className="block truncate text-xs/4 text-fg2 phone:text-sm/5">{h.sub}</span>
              </span>
            </li>
          ))}
          {!list.length ? <li className="px-2.5 py-2 text-fg2">Nothing found.</li> : null}
        </ul>
    </Sheet>
  )
}

/* ---------- keys ---------- */

const typing = (e: KeyboardEvent) => {
  const el = e.target as HTMLElement
  return el?.closest?.('input, textarea, select, [contenteditable=true]')
}

function back(ui: UI, snap: Snapshot): Partial<UI> | null {
  if (ui.adding) return { adding: false }
  if (ui.drawer) return { drawer: false }
  if (ui.settings) return { settings: null }
  if (ui.plus) return { plus: null }
  if (ui.menu) return { menu: false }
  if (ui.photo) return { photo: null }
  if (ui.viewer) return (closeArtifact(), {})
  if (ui.pick) return { pick: null }
  if (ui.focus) return { focus: null }
  if (ui.task) return { cur: Math.max(0, orderedTasks(snap, listView(ui, snap)).findIndex((t) => t.id === ui.task)), task: null }
  if (ui.archived) return { archived: null, cur: 0 }
  return null
}

export default definePlugin({
  name: 'keyboard',
  setup(reg) {
    // a desktop's, and a tablet's once a key was pressed on it (an iPad with a keyboard)
    reg.settingsPages.push({ name: SHORTCUTS, about: 'Everything works without a mouse.', Icon: KeyboardIcon, when: (ui) => ui.device === 'desktop' || (ui.device === 'tablet' && ui.kb), View: Shortcuts })
    reg.views.search = Search
    reg.keys.push({
      order: 10,
      handle: ({ e, ui, snap }) => {
        if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
          e.preventDefault()
          setUI({ search: !ui.search, focus: null })
          return true
        }
        // a key pressed is a keyboard's: a tablet's (an iPad with one) gets the keys a desktop has
        if (ui.search) {
          // Esc closes it wherever the focus is in it (its x, its list), not only from the box
          if (e.key !== 'Escape') return false
          e.preventDefault()
          setUI({ search: false })
          return true
        }
        // works while typing too, like ⌘K: the message box has no bold
        if ((e.metaKey || e.ctrlKey) && !e.altKey && !e.shiftKey && e.key.toLowerCase() === 'b') {
          e.preventDefault()
          // a tablet's places are in its drawer
          if (ui.device === 'desktop') togglePlaces()
          else setUI({ drawer: !ui.drawer })
          return true
        }
        if (typing(e)) return false
        if (e.metaKey || e.ctrlKey || e.altKey) return false
        const k = e.key
        const go = (p: Partial<UI>) => {
          e.preventDefault()
          setUI({ ...p, kb: true })
          return true
        }
        // the Add a project dialog keeps the keys to itself
        if (ui.adding) return k === 'Escape' ? go({ adding: false }) : false
        const task = taskOf(snap, ui.task)
        const list = task ? null : orderedTasks(snap, listView(ui, snap))
        if (k === '?') {
          // Settings at its Keyboard shortcuts page, and shut again from there
          const section = sectionOf(SHORTCUTS, snap.settings, { ...ui, kb: true })
          if (section < 0) return false
          return go({ settings: ui.settings?.section === section ? null : { section } })
        }
        if (k === 'Escape') {
          const b = back(ui, snap)
          return b ? go(b) : false
        }
        // an artifact full screen keeps the keys: the chat behind it is out of reach
        if (ui.viewer) return false
        if ((k === 'ArrowDown' || k === 'j') && list) return go({ cur: Math.min(list.length - 1, ui.cur + 1) })
        if ((k === 'ArrowUp' || k === 'k') && list) return go({ cur: Math.max(0, ui.cur - 1) })
        if (k === 'Enter' && list && list[ui.cur] && !(e.target as HTMLElement)?.closest?.('button, a, [role=button]')) return go({ task: list[ui.cur].id, cur: 0 })
        if (k === '[' || k === ']') {
          const places = shownPlaces(snap)
          const i = places.findIndex((p) => p.id === ui.place)
          const n = places.length
          return go({ place: places[(i + (k === ']' ? 1 : n - 1)) % n].id, task: null, cur: 0 })
        }
        if (k === 'n' && has('places')) return go({ adding: true })
        if (k === 'f' && task && ui.device !== 'phone') {
          e.preventDefault()
          toggleFocus()
          return true
        }
        if (k === 'F' && panelHere(ui, snap)) {
          e.preventDefault()
          toggleWide()
          return true
        }
        if (k === ',') return go({ settings: ui.settings ? null : { section: ui.device === 'phone' ? null : 0 } })
        // split, the focused pane's
        const chatKey = keysChat(ui, snap)
        if (k === '/') return go({ focus: chatKey })
        if (k === 'f' && !task) {
          const box = document.querySelector<HTMLInputElement>('input[name=filter]')
          if (!box) return false
          e.preventDefault()
          box.focus()
          return true
        }
        if (k === 't') {
          e.preventDefault()
          toggleDictation(chatKey)
          return true
        }
        if (k === 'a') {
          e.preventDefault()
          setUI({ plus: chatKey, kb: true })
          setTimeout(() => document.querySelector<HTMLInputElement>(`input[name=file][data-chat="${CSS.escape(chatKey)}"]`)?.click(), 0)
          return true
        }
        if (k === 'e') {
          const msgs = getState().chats[chatKey]?.messages ?? []
          const m = [...msgs].reverse().find((x) => x.who === 'agent')
          return m ? go({ pick: ui.pick === m.id ? null : m.id }) : false
        }
        return false
      },
    })
  },
})

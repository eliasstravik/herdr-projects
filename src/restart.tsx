// Restart picks for either chat role, used once or saved for its space. Opened from the + or ⋯ menu.
import clsx from 'clsx'
import { ChevronUpDownIcon } from '@heroicons/react/20/solid'
import { useEffect, useState, useSyncExternalStore, type ReactNode } from 'react'
import type { Place, RestartCli, RestartInfo } from '@shared/types.ts'
import { act, setUI, taskOf, useSnap, useStore } from './kernel.ts'
import { Dialog } from './rowmenu.tsx'
import { Pick, shown, Switch, TextValue } from './Settings.tsx'
import { EffortIcon, ic, MenuItem, ModelIcon, ModeIcon, Primary, RestartIcon } from './ui.tsx'

/* ---------- the sheet ---------- */

type Opening = { chat: string; name: string; current?: boolean }
let opening: Opening | null = null
const listeners = new Set<() => void>()
const subscribe = (f: () => void) => (listeners.add(f), () => void listeners.delete(f))

/** open the restart sheet for a coordinator (chat: its place id) or a task's agent (ID/Agent); name: what it is called */
export function openRestart(chat: string, name: string, current = false) {
  opening = { chat, name, current }
  listeners.forEach((f) => f())
}

const close = () => {
  opening = null
  listeners.forEach((f) => f())
}

/** the sheet, while one is open: each anew, its picks fresh */
export function RestartSheet() {
  const o = useSyncExternalStore(subscribe, () => opening)
  return o ? <Sheet key={o.chat} chat={o.chat} name={o.name} current={o.current} /> : null
}

/** "Restart agent" in a ⋯ menu */
export function RestartItem({ chat, name, onPick }: { chat: string; name: string; onPick?: () => void }) {
  return (
    <MenuItem
      icon={<RestartIcon className={ic} />}
      onClick={() => {
        onPick?.()
        openRestart(chat, name)
      }}
    >
      Restart agent…
    </MenuItem>
  )
}

const DEFAULT = 'default'

/** a CLI's model choices by name, and the levels the one picked takes */
const modelsOf = (c?: RestartCli) => c?.models ?? []
const levelsOf = (c: RestartCli | undefined, model?: string) => (model ? modelsOf(c).find((m) => m.id === model)?.efforts : undefined) ?? c?.efforts ?? []

function Sheet({ chat, name, current }: Opening) {
  const [info, setInfo] = useState<RestartInfo | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [kind, setKind] = useState('')
  const [model, setModel] = useState('')
  const [effort, setEffort] = useState('')
  const [save, setSave] = useState(false)
  useEffect(() => {
    act({ type: 'restart-info', chat })
      .then((r: { ok?: boolean; error?: string; info?: RestartInfo; waiting?: boolean }) => {
        if (!r.info) return setError(r.waiting ? 'Its machine is out of reach: restart it once it is back.' : r.error || 'It could not be restarted.')
        setInfo(r.info)
        const pick = current ? r.info.on : r.info.want
        setKind(pick.kind)
        setModel(pick.model ?? '')
        setEffort(pick.effort ?? '')
      })
      .catch((e: Error) => setError(e.message))
  }, [chat, current])
  const cli = info?.clis.find((c) => c.kind === kind)
  const levels = levelsOf(cli, model)
  const unavailable = !!(model && cli?.models?.length && !cli.models.some((m) => m.id === model))
  const pickKind = (k: string) => {
    const c = info?.clis.find((x) => x.kind === k)
    setKind(k)
    // a model of the CLI it was on means nothing to another; a level stays where the new one takes it
    setModel(k === info?.want.kind ? (info.want.model ?? '') : '')
    if (!levelsOf(c, k === info?.want.kind ? info.want.model : undefined).includes(effort)) setEffort('')
  }
  const pickModel = (id: string) => {
    setModel(id)
    if (!levelsOf(cli, id).includes(effort)) setEffort('')
  }
  const go = async () => {
    setBusy(true)
    setError(null)
    const r = await act({ type: 'restart', chat, kind, model, effort, save }).catch((e: Error) => ({ ok: false, error: e.message }))
    setBusy(false)
    if (r.ok === false) return setError(r.error || 'It could not be restarted.')
    close()
  }
  const task = chat.includes('/')
  const other = !!info && kind !== info.on.kind
  const changed = !!info && (kind !== info.want.kind || model !== (info.want.model ?? '') || effort !== (info.want.effort ?? ''))
  const label = (id: string) => (id ? (modelsOf(cli).find((m) => m.id === id)?.label ?? id) : DEFAULT)
  return (
    <Dialog
      title={task ? `Restart ${name} agent?` : 'Restart coordinator?'}
      onCancel={close}
      buttons={
        <Primary autoFocus disabled={!info || busy || unavailable} onClick={go}>
          {busy ? 'Restarting…' : 'Restart'}
        </Primary>
      }
    >
      {info ? (
        <div data-restart className="flex flex-col gap-4">
          <div className="flex flex-col gap-2.5 text-fg">
            <Line label="Agent">
              <Pick label="Agent" value={cli?.name ?? kind} options={info.clis.map((c) => c.name)} onPick={(n) => pickKind(info.clis.find((c) => c.name === n)!.kind)} />
            </Line>
            <Line label="Model">
              {cli?.models ? (
                cli.models.length ? (
                  <Pick label="Model" value={label(model)} options={[DEFAULT, ...cli.models.map((m) => m.label)]} onPick={(l) => pickModel(cli.models!.find((m) => m.label === l)?.id ?? '')} />
                ) : (
                  <TextValue label="Model" value={model || DEFAULT} onEdit={() => undefined} onSave={(v) => pickModel(v.trim() === DEFAULT ? '' : v.trim())} />
                )
              ) : <span className="text-fg3">Default</span>}
            </Line>
            {cli?.models && levels.length ? (
              <Line label="Effort">
                <Pick label="Effort" value={effort || DEFAULT} options={[DEFAULT, ...levels]} ordered onPick={(v) => setEffort(v === DEFAULT ? '' : v)} />
              </Line>
            ) : <Line label="Effort"><span className="text-fg3">Default</span></Line>}
            {unavailable ? <p className="text-need-fg">{model} is unavailable. Choose Default or an available model before restarting.</p> : null}
            {changed || save ? (
              <Line label={saveWords(info)}>
                <Switch label={saveWords(info)} on={save} onChange={setSave} />
              </Line>
            ) : null}
          </div>
          {other ? <p className="text-sm text-fg3">{cli?.name ?? shown(kind)} starts a new session with this chat{task ? ', the brief and branch' : ''}.</p> : null}
          {error ? <p className="text-need-fg">{error}</p> : null}
        </div>
      ) : (
        <p className={clsx(error && 'text-need-fg')}>{error ?? 'Reading what it runs on…'}</p>
      )}
    </Dialog>
  )
}

/** "Save as nas's setting", for its role's agents there from now on */
const saveWords = (i: RestartInfo) => `Use for future ${i.role === 'task' ? 'task agents' : 'coordinators'} ${i.space ? `in ${i.space === '~' ? 'the home space' : i.space}` : 'on this machine'}`

function Line({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex min-h-9 items-center justify-between gap-3">
      <span className="min-w-0">{label}</span>
      {children}
    </div>
  )
}

/** Kept for plugin imports. The + menu replaces the old Settings mismatch banner. */
export function RestartHint(_props: { chatKey: string; cli: string }) {
  return null
}

/* ---------- in the ⋯ menus ---------- */

/** a task header's ⋯ (agentMenu): its agent, while the task is open */
export function TaskRestartItem({ chatKey }: { chatKey: string }) {
  const id = chatKey.split('/')[0]
  const snap = useSnap()
  const t = snap && taskOf(snap, id)
  if (!t || t.status === 'done' || t.archived) return null
  return <RestartItem chat={chatKey} name={id} onPick={() => setUI({ taskMenu: false })} />
}

/** a coordinator chat's top-right menu (placeMenu) */
export function PlaceRestartItem({ place }: { place: Place }) {
  return <RestartItem chat={place.chat} name={place.name} onPick={() => setUI({ menu: false })} />
}

/** Compact launch controls inside the composer + menu, shared by both chat roles. */
export function AgentControls({ chatKey, cli }: { chatKey: string; cli: string }) {
  const agent = useStore((s) => s.chats[chatKey]?.agent)
  const model = agent?.model && agent.model !== 'default' ? (agent.modelLabel ?? agent.model) : 'Default / unknown'
  const effort = agent?.effort && agent.effort !== 'default' ? (agent.effort === 'xhigh' ? 'Extra high' : agent.effort.charAt(0).toUpperCase() + agent.effort.slice(1)) : 'Default / unknown'
  const open = () => {
    setUI({ plus: null })
    openRestart(chatKey, chatKey.includes('/') ? chatKey.split('/')[0] : 'this space', true)
  }
  return <div data-agent-controls>
    {([['Agent', agent ? cli : `${cli} · unknown`, ModeIcon], ['Model', model, ModelIcon], ['Effort', effort, EffortIcon]] as const).map(([label, value, Icon]) =>
      <MenuItem key={label} icon={<Icon className={ic} />} aria-label={`${label}: ${value}. Select and restart`} title={`${label}: ${value}. Select and restart`} onClick={open}>
        <span className="flex min-w-0 items-center justify-between gap-4">
          <span>{label}</span>
          <span className="flex min-w-0 items-center gap-1 text-fg2"><span className="truncate">{value}</span><ChevronUpDownIcon className={`${ic} shrink-0`} /></span>
        </span>
      </MenuItem>)}
  </div>
}

// Restarting an agent from the screen: a space's coordinator, or a task's agent. The sheet (from a ⋯ menu) shows what
// it runs on now and what it restarts with, Settings' CLI, model and effort for its role, changed for this restart only
// or saved as its space's. The hint above a chat's box says once Settings for its role say other than what its agent
// runs on, and restarts it on them in one tap.
import clsx from 'clsx'
import { useEffect, useState, useSyncExternalStore, type ReactNode } from 'react'
import type { Place, RestartCli, RestartInfo } from '@shared/types.ts'
import { act, setUI, taskOf, useSnap, useStore } from './kernel.ts'
import { Dialog } from './rowmenu.tsx'
import { Pick, shown, Switch, TextValue } from './Settings.tsx'
import { ic, MenuItem, Primary, RestartIcon } from './ui.tsx'

/* ---------- the sheet ---------- */

type Opening = { chat: string; name: string }
let opening: Opening | null = null
const listeners = new Set<() => void>()
const subscribe = (f: () => void) => (listeners.add(f), () => void listeners.delete(f))

/** open the restart sheet for a coordinator (chat: its place id) or a task's agent (ID/Agent); name: what it is called */
export function openRestart(chat: string, name: string) {
  opening = { chat, name }
  listeners.forEach((f) => f())
}

const close = () => {
  opening = null
  listeners.forEach((f) => f())
}

/** the sheet, while one is open: each anew, its picks fresh */
export function RestartSheet() {
  const o = useSyncExternalStore(subscribe, () => opening)
  return o ? <Sheet key={o.chat} chat={o.chat} name={o.name} /> : null
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

function Sheet({ chat, name }: Opening) {
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
        setKind(r.info.want.kind)
        setModel(r.info.want.model ?? '')
        setEffort(r.info.want.effort ?? '')
      })
      .catch((e: Error) => setError(e.message))
  }, [chat])
  const cli = info?.clis.find((c) => c.kind === kind)
  const levels = levelsOf(cli, model)
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
  const label = (id: string) => (id ? (modelsOf(cli).find((m) => m.id === id)?.label ?? id) : DEFAULT)
  return (
    <Dialog
      title={task ? `Restart ${name}’s agent?` : `Restart ${name}’s coordinator?`}
      onCancel={close}
      buttons={
        <Primary autoFocus disabled={!info || busy} onClick={go}>
          {busy ? 'Restarting…' : 'Restart'}
        </Primary>
      }
    >
      {info ? (
        <div data-restart className="flex flex-col gap-4">
          <p>
            Runs on {info.onWords} now.{info.working ? ' It is working: it stops cleanly first.' : ''}
          </p>
          <div className="flex flex-col gap-2.5 text-fg">
            <Line label="Agent">
              <Pick label="Agent" value={cli?.name ?? kind} options={info.clis.map((c) => c.name)} onPick={(n) => pickKind(info.clis.find((c) => c.name === n)!.kind)} />
            </Line>
            {cli?.models ? (
              <Line label="Model">
                {cli.models.length ? (
                  <Pick label="Model" value={label(model)} options={[DEFAULT, ...cli.models.map((m) => m.label)]} onPick={(l) => pickModel(cli.models!.find((m) => m.label === l)?.id ?? '')} />
                ) : (
                  <TextValue label="Model" value={model || DEFAULT} onEdit={() => undefined} onSave={(v) => pickModel(v.trim() === DEFAULT ? '' : v.trim())} />
                )}
              </Line>
            ) : null}
            {cli?.models && levels.length ? (
              <Line label="Effort">
                <Pick label="Effort" value={effort || DEFAULT} options={[DEFAULT, ...levels]} ordered onPick={(v) => setEffort(v === DEFAULT ? '' : v)} />
              </Line>
            ) : null}
            <Line label={saveWords(info)}>
              <Switch label={saveWords(info)} on={save} onChange={setSave} />
            </Line>
          </div>
          <p className="text-[0.8125rem] text-fg3 phone:text-[0.875rem]">
            {other
              ? `${cli?.name ?? shown(kind)} takes over with the chat so far${task ? ', the brief and the work on its branch' : ''}.`
              : 'It resumes its session.'}{' '}
            {task ? 'Its worktree and branch stay as they are.' : 'Its tasks keep going.'} What you send meanwhile waits for it.
          </p>
          {error ? <p className="text-need-fg">{error}</p> : null}
        </div>
      ) : (
        <p className={clsx(error && 'text-need-fg')}>{error ?? 'Reading what it runs on…'}</p>
      )}
    </Dialog>
  )
}

/** "Save as nas's setting", for its role's agents there from now on */
const saveWords = (i: RestartInfo) => `Save as ${!i.space || i.space === '~' ? 'the home space' : i.space}’s setting`

function Line({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex min-h-9 items-center justify-between gap-3">
      <span className="truncate">{label}</span>
      {children}
    </div>
  )
}

/* ---------- the hint above a chat's box ---------- */

/** "Runs on Claude Code · Settings now say Codex · Restart": Restart starts it again on Settings at once */
export function RestartHint({ chatKey }: { chatKey: string; cli: string }) {
  const hint = useStore((s) => s.chats[chatKey]?.restart)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => setError(null), [hint?.want])
  if (!hint) return null
  const go = async () => {
    setBusy(true)
    const r = await act({ type: 'restart', chat: chatKey }).catch((e: Error) => ({ ok: false, error: e.message }))
    setBusy(false)
    setError(r.ok === false ? r.error || 'It could not be restarted.' : null)
  }
  return (
    <div data-restart-hint className="flex justify-center pb-1.5">
      <div className="inline-flex max-w-full min-w-0 items-center gap-1 rounded-full bg-bubble py-1 pr-1 pl-2.5 text-[0.8125rem]/5 text-fg2 phone:text-[0.875rem]/5">
        {/* narrow, what it runs on gives way first: what Settings now say stays whole */}
        {error ? (
          <span className="truncate">{error}</span>
        ) : (
          <>
            <span className="min-w-8 truncate">Runs on {hint.on} </span>
            <span className="max-w-[60%] shrink-0 truncate">· Settings now say {hint.want}</span>
          </>
        )}
        <button type="button" disabled={busy} onClick={go} className="press-dim shrink-0 rounded-full px-1.5 font-medium text-fg enabled:hover:bg-hover disabled:opacity-40">
          {busy ? 'Restarting…' : error ? 'Try again' : 'Restart'}
        </button>
      </div>
    </div>
  )
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

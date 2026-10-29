import { useEffect, useMemo, useState } from 'react'
import { getState, goChat, goPlace, keysChat, listView, openArtifact, openTask, placeOf, setUI, useSnap } from '@client/kernel.ts'
import { Sheet, SheetClose } from '@client/Stack.tsx'
import { ic, SearchIcon } from '@client/ui.tsx'
import type { Chat } from '@shared/types.ts'
import { jumpTo } from '@plugins/chat/reply.ts'
import { searchChats, searchResults, type SearchHit, type SearchKind, type SearchScope } from './search-data.ts'
import { loadSearchChats } from './search-load.ts'

const kinds: SearchKind[] = ['Spaces', 'Tasks', 'Messages', 'Artifacts']

/** Navigate by stable identity, never a display name or row number. Wait for the destination chat to mount. */
function openHit(hit: SearchHit, q: string) {
  if (hit.artifact) { goPlace(hit.place); openArtifact(hit.artifact); return }
  if (hit.message && hit.chat) {
    goChat(hit.chat)
    setUI({ find: { chat: hit.chat, q, id: hit.message } })
    const started = Date.now()
    const seek = () => {
      if (getState().ui.find?.id !== hit.message) return
      if (jumpTo(hit.chat!, hit.message!)) return
      if (Date.now() - started < 8000) setTimeout(seek, 100)
    }
    requestAnimationFrame(seek)
    return
  }
  if (hit.task) {
    openTask(hit.task)
    const { ui, snap } = getState()
    if (snap?.tasks.find(t => t.id === hit.task)?.archived) setUI({ archived: listView(ui, snap).key })
  } else goPlace(hit.place)
}

export function Search({ onClose }: { onClose: () => void }) {
  const snap = useSnap()!
  const [origin] = useState(() => {
    const ui = getState().ui
    const p = placeOf(snap, ui.place)
    return { place: p, chat: keysChat(ui, snap), task: ui.task }
  })
  const [scope, setScope] = useState<SearchScope>(() => origin.task ? { kind: 'chat', id: origin.chat } : origin.place.level === 2 ? { kind: 'space', id: origin.place.id } : origin.place.level === 1 ? { kind: 'machine', id: origin.place.machine } : { kind: 'all' })
  const [kind, setKind] = useState<SearchKind | null>(null)
  const [q, setQ] = useState('')
  const [settled, setSettled] = useState('')
  const [cur, setCur] = useState(0)
  const [limit, setLimit] = useState(8)
  const [retry, setRetry] = useState(0)
  const [chats, setChats] = useState<Record<string, Chat>>({})
  const [coverage, setCoverage] = useState({ done: 0, missing: [] as string[], finished: false })
  const sources = useMemo(() => searchChats(snap, scope), [snap, scope])
  const sourceKeys = JSON.stringify(sources.map(c => c.key))
  useEffect(() => {
    const timer = setTimeout(() => setSettled(q), 120)
    return () => clearTimeout(timer)
  }, [q])
  useEffect(() => {
    setChats({})
    setCoverage({ done: 0, missing: [], finished: false })
    return loadSearchChats(JSON.parse(sourceKeys), chat => setChats(old => ({ ...old, [chat.key]: chat })), (done, missing, finished) => setCoverage({ done, missing, finished }))
  }, [sourceKeys, retry])
  const results = useMemo(() => searchResults(snap, scope, settled, chats), [snap, scope, settled, chats])
  const groups = kinds.filter(k => !kind || kind === k).map(k => ({ kind: k, all: results.filter(h => h.kind === k) }))
  const list = groups.flatMap(g => g.all.slice(0, limit))
  const current = list[Math.min(cur, Math.max(0, list.length - 1))]
  useEffect(() => { document.getElementById(`search-hit-${current?.key}`)?.scrollIntoView({ block: 'nearest' }) }, [current?.key])
  const pick = (hit?: SearchHit) => { if (hit) { onClose(); openHit(hit, settled) } }
  const changeScope = (s: SearchScope) => { setScope(s); setCur(0); setLimit(8) }
  const chip = 'min-h-11 shrink-0 rounded-full border px-3 text-sm aria-pressed:border-accent aria-pressed:text-accent'
  const scopeOptions: { label: string; scope: SearchScope }[] = [
    { label: 'This chat', scope: { kind: 'chat', id: origin.chat } },
    { label: `Space: ${origin.place.name}`, scope: { kind: 'space', id: origin.place.id } },
    { label: `Machine: ${snap.machines.find(m => m.machine === origin.place.machine)?.name ?? origin.place.machine}`, scope: { kind: 'machine', id: origin.place.machine } },
    { label: 'Everywhere', scope: { kind: 'all' } },
  ]
  return <Sheet label="Search or jump to" onClose={onClose} panel="max-w-[640px]">
    <div className="flex shrink-0 items-center gap-2 border-b border-line pr-2 pl-4">
      <SearchIcon className={`${ic} text-fg3`} />
      <input autoFocus name="search" role="combobox" aria-expanded="true" aria-controls="search-list" aria-activedescendant={current ? `search-hit-${current.key}` : undefined} aria-label="Search or jump to" placeholder="Search spaces, tasks, messages, artifacts" value={q}
        onChange={e => { setQ(e.target.value); setCur(0); setLimit(8) }}
        onKeyDown={e => {
          if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); setCur(Math.max(0, Math.min(list.length - 1, cur + (e.key === 'ArrowDown' ? 1 : -1)))) }
          else if (e.key === 'Enter' && q === settled) pick(current)
          else if (e.key === 'Escape') onClose()
        }} className="h-[52px] min-w-0 flex-1 bg-transparent outline-none placeholder:text-fg3 phone:text-[1rem]" />
      <SheetClose onClose={onClose} />
    </div>
    <div className="shrink-0 space-y-2 border-b border-line p-3">
      <div role="group" aria-label="Search scope" className="flex gap-2 overflow-x-auto pb-1">{scopeOptions.map(o => <button key={o.scope.kind} className={chip} aria-pressed={JSON.stringify(o.scope) === JSON.stringify(scope)} onClick={() => changeScope(o.scope)}>{o.label}</button>)}</div>
      <div role="group" aria-label="Result types" className="flex gap-2 overflow-x-auto pb-1">{[null, ...kinds].map(k => <button key={k ?? 'all'} className={chip} aria-pressed={kind === k} onClick={() => { setKind(k); setCur(0); setLimit(8) }}>{k ?? 'All types'}</button>)}</div>
      <p role="status" className="text-xs text-fg2">{!q.trim() ? 'Recent items · ' : ''}{q !== settled ? 'Filtering… · ' : ''}{coverage.finished ? `Read ${coverage.done} of ${sources.length} chats` : `Loading messages… ${coverage.done} of ${sources.length} chats`}</p>
      {coverage.missing.length > 0 ? <div className="text-xs text-fg2">{coverage.missing.length} chats unavailable. Results may be incomplete. <button className="min-h-11 underline" onClick={() => setRetry(n => n + 1)}>Retry</button></div> : null}
      {snap.machines.some(m => m.status === 'off') ? <p className="text-xs text-fg2">Unavailable machines: {snap.machines.filter(m => m.status === 'off').map(m => m.name).join(', ')}. Saved items may be out of date.</p> : null}
    </div>
    <div id="search-list" role="listbox" aria-label="Search results" aria-busy={!coverage.finished || q !== settled} className="max-h-[60vh] min-h-0 overflow-y-auto overscroll-contain p-2 phone:max-h-none phone:flex-1 phone:pb-[calc(env(safe-area-inset-bottom)+8px)]">
      {groups.map(group => <div role="group" aria-label={group.kind} key={group.kind}>
        <h3 className="px-3 pt-3 pb-1 text-xs font-medium text-fg2">{group.kind} · {group.all.length}</h3>
        {group.all.slice(0, limit).map(hit => <div role="option" tabIndex={0} aria-selected={hit.key === current?.key} key={hit.key} id={`search-hit-${hit.key}`} onClick={() => pick(hit)} onKeyDown={e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); pick(hit) } }} className="min-h-14 cursor-pointer rounded-lg px-3 py-2 hover:bg-well focus:outline-accent aria-selected:bg-well">
          <p className="line-clamp-2 break-words text-sm">{hit.label}</p><p className="truncate text-xs text-fg2">{hit.context}{hit.at > 0 ? ` · ${new Date(hit.at).toLocaleDateString()}` : ''}</p>
        </div>)}
        {!group.all.length ? <p className="px-3 py-2 text-sm text-fg3">{group.kind === 'Messages' && !coverage.finished ? 'Still loading…' : 'No matches'}</p> : null}
        {group.all.length > limit ? <button className="min-h-11 px-3 text-sm text-accent" onClick={() => setLimit(n => n + 20)}>Show more {group.kind.toLowerCase()} ({group.all.length - limit})</button> : null}
      </div>)}
    </div>
  </Sheet>
}

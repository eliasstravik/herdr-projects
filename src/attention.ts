import type { Snapshot, Task } from '@shared/types.ts'

/** Unanswered requests in original chats, independent of forwarded copies in chats. Ready is separate. */
export function attentionCounts(snap: Snapshot, places: string[]) {
  const ids = new Set(places)
  const tasks = snap.tasks.filter((t) => ids.has(t.place) && !t.archived)
  const requests = new Set<string>()
  const add = (chat: string, fallback: string[]) => {
    for (const id of snap.attentionRequests?.[chat] ?? fallback) requests.add(`${chat}\0${id}`)
  }
  for (const t of tasks.filter((t) => t.status === 'need')) add(t.agent?.chat ?? `${t.id}/Agent`, ['attention'])
  for (const p of snap.places.filter((p) => ids.has(p.id))) add(p.chat, snap.asks[p.id] ? [snap.asks[p.id]] : [])
  return { need: requests.size, ready: new Set(tasks.filter((t) => t.status === 'ready').map((t) => t.id)).size }
}

export const actionable = (t: Task) => t.status === 'need' || t.status === 'ready'

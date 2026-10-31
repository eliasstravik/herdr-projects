import type { Snapshot, Task } from '@shared/types.ts'

/** What waits on you at these places: one need per card, the same list the Needs you view shows. Ready is separate:
 *  a task whose merge waits on you is a need, not also Ready. */
export function attentionCounts(snap: Snapshot, places: string[]) {
  const ids = new Set(places)
  const needs = snap.needs.filter((n) => ids.has(n.place))
  const waiting = new Set(needs.map((n) => n.task))
  return { need: needs.length, ready: snap.tasks.filter((t) => ids.has(t.place) && !t.archived && t.status === 'ready' && !waiting.has(t.id)).length }
}

export const actionable = (t: Task) => t.status === 'need' || t.status === 'ready'

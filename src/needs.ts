// The Needs you view's list as it stands on a screen: oldest first when it opens, and from then on nothing under the
// thumb moves. A new need only ever joins the end. One that stops waiting (answered here or elsewhere) keeps its place
// until no open need sits below it, so the next card never slides up into a tap that was meant for this one.
import type { Need } from '@shared/types.ts'

export interface HeldNeed {
  need: Need
  /** it no longer waits: only its place is kept */
  gone?: boolean
}

const key = (n: Need) => `${n.chat}\0${n.id}`

/** the list after a new snapshot: needs is the server's, oldest first */
export function holdNeeds(held: HeldNeed[], needs: Need[]): HeldNeed[] {
  const now = new Map(needs.map((n) => [key(n), n]))
  const known = new Set(held.map((h) => key(h.need)))
  const kept = held.map((h): HeldNeed => {
    const need = now.get(key(h.need))
    return need ? { need } : { need: h.need, gone: true }
  })
  const list: HeldNeed[] = [...kept, ...needs.filter((n) => !known.has(key(n))).map((need) => ({ need }))]
  while (list.at(-1)?.gone) list.pop()
  return list
}

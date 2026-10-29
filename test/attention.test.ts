import { expect, test } from 'bun:test'
import { attentionCounts } from '../src/attention.ts'
import { DemoWorld } from '../server/demo/world.ts'

test('attention counts requests once, Ready separately, and only within its named scope', () => {
  const snap = new DemoWorld().snapshot()
  delete snap.attentionRequests // Older peers remain compatible.
  const all = snap.places.map((p) => p.id)
  const need = snap.tasks.filter((t) => t.status === 'need' && !t.archived)
  const ready = snap.tasks.filter((t) => t.status === 'ready' && !t.archived)
  expect(attentionCounts(snap, all)).toEqual({ need: need.length + Object.keys(snap.asks).length, ready: ready.length })
  // Copies of a task request must not make it count twice; archived work never counts.
  snap.tasks.push({ ...need[0] }, { ...need[0], id: 'ARCHIVED', archived: 1 })
  expect(attentionCounts(snap, [...all, ...all]).need).toBe(need.length + Object.keys(snap.asks).length)
  const p = ready[0].place
  expect(attentionCounts(snap, [p]).ready).toBe(ready.filter((t) => t.place === p).length)
  expect(attentionCounts(snap, [])).toEqual({ need: 0, ready: 0 })
})

test('multiple questions count separately, copies once, answered requests disappear', () => {
  const snap = new DemoWorld().snapshot()
  const need = snap.tasks.find((t) => t.status === 'need')!
  const p = snap.places.find((p) => p.id === need.place)!
  const key = need.agent?.chat ?? `${need.id}/Agent`
  snap.tasks = [need, { ...need, id: 'READY', status: 'ready' }]
  snap.asks = { [p.id]: 'coordinator-1' }
  snap.attentionRequests = { [key]: ['question-1', 'question-2', 'question-1'], [p.chat]: ['coordinator-1', 'coordinator-2'] }
  expect(attentionCounts(snap, [p.id])).toEqual({ need: 4, ready: 1 })
  snap.attentionRequests[key] = ['question-2']
  expect(attentionCounts(snap, [p.id])).toEqual({ need: 3, ready: 1 })
})

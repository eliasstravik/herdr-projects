// The service worker shows an alert only when no window of this app is on screen, one per task, sets the app icon's
// count, lets alerts about what no longer waits go, opens the app on the card, and answers it from a button where
// alerts have them.
import { describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import type { PushPayload } from '../shared/alerts.ts'

const code = readFileSync(join(import.meta.dir, '..', 'public', 'sw.js'), 'utf8')
const scope = 'https://box.tail.ts.net/shepherdr/'

type Win = { url: string; visibilityState: string; focus?: () => Promise<unknown>; postMessage?: (m: unknown) => void; navigate?: (u: string) => Promise<unknown> }
type Shown = { title: string; tag: string; data: { url: string; waits: boolean; answer?: unknown }; actions?: unknown[]; renotify?: boolean; close: () => void; closed?: boolean }

/** sw.js against fake windows and a fake device: maxActions 0 is an iPhone (no buttons), no badge a Linux desktop */
function worker(opts: { windows?: Win[]; maxActions?: number; badge?: boolean; answer?: { ok: boolean }; waiting?: { count: number; tags: string[] } } = {}) {
  const on: Record<string, (e: unknown) => void> = {}
  const shown: Shown[] = []
  const badges: (number | 'clear')[] = []
  const posts: { url: string; body: unknown }[] = []
  const opened: string[] = []
  const nav = opts.badge === false ? {} : { setAppBadge: async (n: number) => void badges.push(n), clearAppBadge: async () => void badges.push('clear') }
  const self = {
    addEventListener: (ev: string, fn: (e: unknown) => void) => (on[ev] = fn),
    skipWaiting() {},
    navigator: nav,
    clients: { claim: async () => undefined, matchAll: async () => opts.windows ?? [], openWindow: async (u: string) => void opened.push(u) },
    registration: {
      scope,
      showNotification: async (title: string, o: Omit<Shown, 'title' | 'close'>) => {
        // a newer alert with the same tag takes the old one's place
        const i = shown.findIndex((n) => n.tag === o.tag && !n.closed)
        const n: Shown = { title, ...o, close: () => (n.closed = true) }
        if (i >= 0) shown.splice(i, 1, n)
        else shown.push(n)
      },
      getNotifications: async () => shown.filter((n) => !n.closed),
    },
  }
  const fetch = async (u: URL, init?: { body?: string }) => {
    if (init?.body) posts.push({ url: String(u), body: JSON.parse(init.body) })
    const body = String(u).endsWith('api/waiting') ? (opts.waiting ?? { count: 0, tags: [] }) : (opts.answer ?? { ok: true })
    return { ok: true, json: async () => body }
  }
  new Function('self', 'Notification', 'fetch', code)(self, { maxActions: opts.maxActions ?? 0 }, fetch)
  const run = async (ev: string, e: object) => {
    let done: Promise<unknown> = Promise.resolve()
    on[ev]({ ...e, waitUntil: (p: Promise<unknown>) => (done = p) })
    await done
  }
  return {
    shown: () => shown.filter((n) => !n.closed),
    badges,
    posts,
    opened,
    push: (a: Partial<PushPayload>) => run('push', { data: { json: () => ({ id: 'a1', title: 'X-1 needs you', body: '', ...a }) } }),
    click: (n: Shown, action = '') => run('notificationclick', { notification: n, action }),
  }
}

const titles = async (windows: Win[]) => {
  const w = worker({ windows })
  await w.push({})
  return w.shown().map((n) => n.title)
}

describe('service worker', () => {
  test('the app on screen: no notification', async () => {
    expect(await titles([{ url: `${scope}?place=all`, visibilityState: 'visible' }])).toEqual([])
  })
  test('the app closed or in the background: a notification', async () => {
    expect(await titles([])).toEqual(['X-1 needs you'])
    expect(await titles([{ url: scope, visibilityState: 'hidden' }])).toEqual(['X-1 needs you'])
  })
  test('another app on the same host on screen: still a notification', async () => {
    expect(await titles([{ url: 'https://box.tail.ts.net/other-app/', visibilityState: 'visible' }])).toEqual(['X-1 needs you'])
    expect(await titles([{ url: 'https://box.tail.ts.net/shepherdr-old/', visibilityState: 'visible' }])).toEqual(['X-1 needs you'])
  })
})

describe('one alert per task', () => {
  test("a task's newer alert takes its old one's place, and buzzes again", async () => {
    const w = worker()
    await w.push({ id: 'a1', tag: 'SHE-63', waits: true, title: 'SHE-63 · Plan to approve', open: ['SHE-63'] })
    await w.push({ id: 'a2', tag: 'SHE-63', waits: true, title: 'SHE-63 · Waits for your reply', open: ['SHE-63'] })
    expect(w.shown().map((n) => n.title)).toEqual(['SHE-63 · Waits for your reply'])
    expect(w.shown()[0].renotify).toBe(true)
  })
  test('alerts about what no longer waits go with the next one; plain alerts stay', async () => {
    const w = worker()
    await w.push({ id: 'a1', tag: 'SHE-1', waits: true, title: 'SHE-1 · Ready to merge', open: ['SHE-1'] })
    await w.push({ id: 'c1', tag: 'c1', title: 'All machines: Main is red', open: ['SHE-1'] })
    await w.push({ id: 'a2', tag: 'SHE-2', waits: true, title: 'SHE-2 · Went wrong', open: ['SHE-2'] })
    expect(w.shown().map((n) => n.title)).toEqual(['All machines: Main is red', 'SHE-2 · Went wrong'])
  })
  test('with the app on screen, the old alerts still go', async () => {
    const windows: Win[] = []
    const w = worker({ windows })
    await w.push({ id: 'a1', tag: 'SHE-1', waits: true, title: 'SHE-1', open: ['SHE-1'] })
    windows.push({ url: scope, visibilityState: 'visible' })
    await w.push({ id: 'a2', tag: 'SHE-2', waits: true, title: 'SHE-2', open: ['SHE-2'] })
    expect(w.shown()).toEqual([])
  })
})

describe("the app icon's count", () => {
  test('every alert sets it; none waiting clears it', async () => {
    const w = worker()
    await w.push({ badge: 3 })
    await w.push({ badge: 0 })
    expect(w.badges).toEqual([3, 'clear'])
  })
  test('set even while the app is on screen', async () => {
    const w = worker({ windows: [{ url: scope, visibilityState: 'visible' }] })
    await w.push({ badge: 2 })
    expect(w.badges).toEqual([2])
    expect(w.shown()).toEqual([])
  })
  test('no badge on this device (Linux): the alert still shows', async () => {
    const w = worker({ badge: false })
    await w.push({ badge: 2 })
    expect(w.shown().length).toBe(1)
  })
})

describe('a tap', () => {
  test('with the app closed, it opens on the card', async () => {
    const w = worker()
    await w.push({ url: './?place=p&task=SHE-63&focus=plan%3A1' })
    await w.click(w.shown()[0])
    expect(w.opened).toEqual([`${scope}?place=p&task=SHE-63&focus=plan%3A1`])
    expect(w.shown()).toEqual([])
  })
  test('with the app open in the background, it goes to the card without reloading', async () => {
    const msgs: unknown[] = []
    const win: Win = { url: scope, visibilityState: 'hidden', focus: async () => win, postMessage: (m) => void msgs.push(m), navigate: async () => win }
    const w = worker({ windows: [win] })
    await w.push({ url: './?place=all&focus=ask%3A1' })
    await w.click(w.shown()[0])
    expect(msgs).toEqual([{ type: 'open', url: `${scope}?place=all&focus=ask%3A1` }])
  })
})

describe('buttons', () => {
  const card = { tag: 'SHE-63', waits: true as const, answer: { chat: 'SHE-63/Agent', question: 'ask:1' }, actions: [{ action: '0', title: 'Rerun it' }, { action: '1', title: 'Skip it' }] }
  test('an iPhone has none: the alert shows without them', async () => {
    const w = worker({ maxActions: 0 })
    await w.push(card)
    expect(w.shown()[0].actions).toBeUndefined()
  })
  test("Android and desktop Chrome: the card's choices", async () => {
    const w = worker({ maxActions: 2 })
    await w.push(card)
    expect(w.shown()[0].actions).toEqual(card.actions)
  })
  test('a button answers the card, then the count follows', async () => {
    const w = worker({ maxActions: 2, waiting: { count: 1, tags: ['SHE-9'] } })
    await w.push(card)
    await w.click(w.shown()[0], '1')
    expect(w.posts).toEqual([{ url: `${scope}api/action`, body: { type: 'answer', chat: 'SHE-63/Agent', question: 'ask:1', choice: 1, fromAlert: true } }])
    expect(w.badges.at(-1)).toBe(1)
    expect(w.opened).toEqual([])
  })
  test("an answer that doesn't go in opens the app on the card instead", async () => {
    const w = worker({ maxActions: 2, answer: { ok: false } })
    await w.push({ ...card, url: './?place=p&task=SHE-63&focus=ask%3A1' })
    await w.click(w.shown()[0], '0')
    expect(w.opened).toEqual([`${scope}?place=p&task=SHE-63&focus=ask%3A1`])
  })
})

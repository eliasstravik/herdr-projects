// The chat as a phone draws it, shared by the layout-*.test.ts files: the demo world with a few extra messages, served
// from the screen built once (web.ts) and opened in a real browser. One of each per file, so the files run in
// parallel; skipped where no browser is installed.
import { afterAll } from 'bun:test'
import type { Browser, Locator, Page } from 'playwright-core'
import { startApp } from '../server/app.ts'
import { DemoWorld } from '../server/demo/world.ts'
import type { Source } from '../server/source.ts'
import type { Action, Message } from '../shared/types.ts'
import { launch, launchWebkit } from './chromium.ts'
import { builtScreen } from './web.ts'

const all = (by: 'me' | 'them') => ['👍', '❤️', '😂', '🙌', '👀', '✅'].map((emoji) => ({ emoji, by }))

/** what just opened has finished springing or sliding in (the endless ones, like typing dots, aside) */
export const settled = (page: Page) =>
  page.evaluate(() => Promise.all(document.getAnimations().filter((a) => a.effect?.getTiming().iterations !== Infinity).map((a) => a.finished.catch(() => {}))))
/** what read() gives once ok() takes it, or its last within the time, for an expect to say what it was: a width that
 *  slides, a value the server sends back */
export async function until<T>(read: () => Promise<T>, ok: (v: T) => boolean, ms = 5000): Promise<T> {
  const end = Date.now() + ms
  let v = await read()
  while (!ok(v) && Date.now() < end) {
    await Bun.sleep(50)
    v = await read()
  }
  return v
}
/** it stops moving before a click: a frame in the chat takes its page's height a moment after it loads, moving all
 *  above it, after the two still frames a click waits for. So it is scrolled to first (lazy frames near it start
 *  loading), every frame on screen has loaded its own page (not the blank one a lazy frame starts with), and then its
 *  box stays put a while */
export async function still(loc: Locator, ms = 300) {
  await loc.scrollIntoViewIfNeeded()
  const page = loc.page()
  const end = Date.now() + 10_000
  for (const el of await page.locator('iframe').elementHandles()) {
    const box = await el.boundingBox()
    const size = page.viewportSize()!
    if (!box || box.y + box.height < 0 || box.y > size.height) continue
    const frame = await el.contentFrame()
    while (frame && /^about:/.test(frame.url()) && Date.now() < end) await Bun.sleep(50)
    await frame?.waitForLoadState('load')
  }
  let was = ''
  for (;;) {
    const now = JSON.stringify(await loc.boundingBox())
    if (now === was) return
    if (Date.now() > end) throw new Error(`still moving: ${now}`)
    was = now
    await Bun.sleep(ms)
  }
}
export const pills = (id: string) => `[data-msg="${id}"] button[aria-pressed][data-keep-box]`
export type Box = { left: number; right: number; top: number; bottom: number }

/** the lists show everything at the main home space (no chips there): every machine's tasks and artifacts, as before
 *  chips; another place picked, its own again. Each new page, before the screen loads. */
function keepEverything() {
  try {
    localStorage.setItem('shepherdr.lists', JSON.stringify({ pinned: false, chips: [], at: 'p:vm-str:~' }))
  } catch {
    /* about:blank: nothing to keep */
  }
}
const everything = (page: Page) => page.addInitScript(keepEverything)

/** a browser, and the demo world served to it, closed once the file's tests are done. everything: its pages' lists
 *  show everything at the main home space (tests of grouping every machine's tasks); webkit: Safari's engine, not
 *  Chromium (an iPhone's tests) */
export async function layout({ everything: wide = false, webkit = false } = {}) {
  const browser: Browser | null = await (webkit ? launchWebkit() : launch())
  if (browser && wide) {
    const newPage = browser.newPage.bind(browser)
    const newContext = browser.newContext.bind(browser)
    browser.newPage = async (o) => {
      const page = await newPage(o)
      await everything(page)
      return page
    }
    browser.newContext = async (o) => {
      const ctx = await newContext(o)
      await ctx.addInitScript(keepEverything)
      return ctx
    }
  }
  /** what you send: held on its way while a test wants it (the agent's log is slow), refused while sends fail */
  const sending = { hold: null as Promise<void> | null, fail: false }
  if (!browser) return { browser, sending, world: undefined as unknown as DemoWorld, server: undefined as unknown as ReturnType<typeof startApp>, url: () => '' }
  const dist = await builtScreen()
  const world = new DemoWorld()
  // one clock reading: pictures sent together share their time, and a millisecond ticking between two readings would
  // split them
  const now = Date.now()
  const short: Message[] = [
    { id: 'code-agent', who: 'agent', text: 'Run `bun test` first, then see [the docs](https://example.com/docs):\n\n```sh\nbun run check\n```', at: now - 10 },
    { id: 'tall-agent', who: 'agent', text: Array.from({ length: 40 }, (_, i) => `Line ${i + 1} of a reply far taller than the screen.`).join('\n\n'), at: now - 3 },
    { id: 'long-you', who: 'you', text: 'Can you look at the flaky test on main and tell me whether it is the network mock or the clock?', at: now - 2, reactions: [{ emoji: '👍', by: 'them' }] },
    { id: 'long-agent', who: 'agent', text: 'It is the clock. The test waits 50 ms for a timer the mock never advances, so a slow runner times out.', at: now - 1, reactions: [{ emoji: '❤️', by: 'both' }] },
    { id: 'short-you', who: 'you', text: 'Go', at: now, reactions: all('them') },
    { id: 'short-agent', who: 'agent', text: 'OK', at: now + 1, reactions: all('me') },
    // a picture this browser can't show: a file tile that keeps its reaction
    { id: 'heic-you', who: 'you', file: { name: 'photo.heic', path: '~/inbox/x-photo.heic' }, at: now - 3, reactions: [{ emoji: '👍', by: 'them' }] },
    // two of them sent together: small tiles in an album
    { id: 'heic-a', who: 'you', file: { name: 'a.heic', path: '~/inbox/x-a.heic' }, at: now - 4 },
    { id: 'heic-b', who: 'you', file: { name: 'b.heic', path: '~/inbox/x-b.heic' }, at: now - 4 },
  ]
  world.chats['p:vm-str:~'].messages.push(...short)
  const act = world.act.bind(world)
  world.act = async (a: Action) => {
    if (a.type === 'send') {
      if (sending.fail) throw new Error('herdr is not answering')
      await sending.hold
    }
    return act(a)
  }
  const server = startApp({ source: world as unknown as Source, dist, port: 0 })
  afterAll(async () => {
    await browser.close()
    server.stop(true)
  })
  return { browser, sending, world, server, url: () => `http://127.0.0.1:${server.port}/?place=vm-str/home` }
}

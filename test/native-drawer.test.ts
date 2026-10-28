// The places drawer and sheets follow a finger on a phone, and nothing scrolls sideways. Opened in a real browser
// (native.ts); skipped where none is installed.
import { describe, expect, test } from 'bun:test'
import { GTM, drawerOpen, native, settled } from './native.ts'

const { browser, at, phone } = await native()

describe.skipIf(!browser)('swipes (phone)', () => {
  test('from the left edge the drawer follows the finger out; a swipe left puts it back', async () => {
    const { ctx, page, swipe } = await phone()
    await page.goto(at('place=vm-str/home'))
    await page.waitForSelector('[data-msg]')
    // let go halfway, slowly: it goes back
    await swipe(4, 400, 120, 400, 400)
    await settled(page)
    expect(await drawerOpen(page)).toBe(false)
    // a fling: it opens, and that is a step
    await swipe(4, 400, 260, 400, 120)
    await settled(page)
    expect(await drawerOpen(page)).toBe(true)
    expect(await page.evaluate(() => (history.state as { screen: { drawer: boolean } }).screen.drawer)).toBe(true)
    await swipe(300, 500, 20, 500, 150)
    await settled(page)
    expect(await drawerOpen(page)).toBe(false)
    expect(await page.evaluate(() => (history.state as { screen: { drawer: boolean } }).screen.drawer)).toBe(false)
    await ctx.close()
  }, 30_000)

  test('the drawer stays under the finger, even as a thumb arcs up from the edge', async () => {
    const { ctx, page, drag } = await phone()
    // a chat long enough to scroll: a finger drifting up would scroll it, and the browser would take the swipe away
    await page.goto(at(GTM))
    await page.waitForSelector('[data-msg]')
    const list = page.locator('[data-messages]').locator('xpath=ancestor::*[contains(@class,"overflow-y-auto")][1]')
    const scrolls = () => list.evaluate((n) => n.scrollTop)
    expect(await list.evaluate((n) => n.scrollHeight > n.clientHeight)).toBe(true)
    await list.evaluate((n) => (n.scrollTop = n.scrollHeight / 2))
    const top = await scrolls()
    // from 6px in, up a little more than rightwards (about 50° off): the browser would take it as a scroll
    const pts = Array.from({ length: 27 }, (_, i) => [6 + i * 8, 640 - i * 9] as [number, number])
    const { at: right, up } = await drag(pts)
    // its edge has moved as far as the finger, from the first step on, and the chat under it stayed put
    right.forEach((r, i) => expect(Math.abs(r - (pts[i + 1][0] - 6))).toBeLessThan(2))
    expect(await scrolls()).toBe(top)
    await up()
    await settled(page)
    expect(await drawerOpen(page)).toBe(true)
    await ctx.close()
  }, 30_000)

  test('a short flick opens the drawer, and a short flick back shuts it; a slow short drag springs back', async () => {
    const { ctx, page, flick, swipe } = await phone()
    await page.goto(at('place=vm-str/home'))
    await page.waitForSelector('[data-msg]')
    await swipe(6, 400, 90, 400, 600)
    await settled(page)
    expect(await drawerOpen(page)).toBe(false)
    // a flick that stopped before letting go is no flick
    await flick(6, 400, 86, 400, 48, 150)
    await settled(page)
    expect(await drawerOpen(page)).toBe(false)
    await flick(6, 400, 66, 400, 80)
    await settled(page)
    expect(await drawerOpen(page)).toBe(true)
    await flick(250, 400, 200, 400, 64)
    await settled(page)
    expect(await drawerOpen(page)).toBe(false)
    // across the chat too: from a stretch of it that is no message, as a slide right on a message is a reply (the
    // home space's chat is mostly messages and artifacts)
    const y = await page.evaluate(() => {
      for (let y = 120; y < 700; y += 5) {
        const e = document.elementFromPoint(150, y)
        if (e && e.closest('[data-messages]') !== e && !e.closest('[data-msg], iframe, [data-chrome], [data-composer], nav')) return y
      }
      return 400
    })
    await flick(150, y, 200, y, 64)
    await settled(page)
    expect(await drawerOpen(page)).toBe(true)
    await ctx.close()
  }, 30_000)

  test('a sheet drags down to close', async () => {
    const { ctx, page, swipe } = await phone()
    await page.goto(at('place=vm-str/home'))
    await page.getByRole('button', { name: 'Search', exact: true }).click()
    const sheet = page.getByRole('dialog', { name: 'Search or jump to' })
    await sheet.waitFor()
    await settled(page)
    const box = (await sheet.boundingBox())!
    // it leaves the top of the app showing, dimmed, as an iOS sheet does
    expect(box.y).toBeGreaterThan(0)
    // from its grabber (the box you type in keeps its finger)
    await swipe(box.x + box.width / 2, box.y + 8, box.x + box.width / 2, box.y + 500, 200)
    await sheet.waitFor({ state: 'detached' })
    await ctx.close()
  }, 30_000)

  test('nothing scrolls the page sideways, and the chrome selects no text', async () => {
    const { ctx, page } = await phone()
    await page.goto(at(`${GTM}&task=GTM-14`))
    await page.getByRole('button', { name: 'Back', exact: true }).waitFor()
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
    expect(await page.evaluate(() => getComputedStyle(document.documentElement).overscrollBehaviorY)).toBe('none')
    // the header and buttons are chrome; what you type, and what the chat shows, select
    expect(await page.$eval('[data-chrome]', (r) => getComputedStyle(r).userSelect)).toBe('none')
    expect(await page.$eval('button', (r) => getComputedStyle(r).userSelect)).toBe('none')
    expect(await page.$eval('textarea', (t) => getComputedStyle(t).userSelect)).not.toBe('none')
    // what the chat shows outside a message (a question card's words) selects; a message selects after Select text
    const shown = await page.$$eval('[data-messages] *', (els) =>
      els.filter((e) => e.childElementCount === 0 && e.textContent?.trim() && !e.closest('[data-anchor], button, label')).map((e) => getComputedStyle(e).userSelect),
    )
    expect(shown.length).toBeGreaterThan(0)
    expect(shown.every((u) => u !== 'none')).toBe(true)
    await ctx.close()
  }, 30_000)
})

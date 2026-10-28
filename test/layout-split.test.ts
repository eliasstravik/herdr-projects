// Split view on a desktop: spaces, tasks and artifacts open side by side from their menus or a ⌘/Ctrl-click, the
// focused pane leads the sidebar and the keys, dividers resize, panes close or become the only one, and it all comes
// back on reload; a phone shows one at a time. In a real browser (layout.ts).
import { describe, expect, test } from 'bun:test'
import { layout } from './layout.ts'

const { browser, server } = await layout()
const at = (q: string) => `http://127.0.0.1:${server!.port}/?${q}`

describe.skipIf(!browser)('split view', () => {
  test('a desktop: open, focus, keys, resize, reload, close and the only one', async () => {
    // room for three panes with some to share out between them
    const page = await browser!.newPage({ viewport: { width: 1920, height: 900 } })
    await page.goto(at('place=vm-str/home'))
    await page.getByRole('textbox', { name: 'Message home…' }).waitFor()
    const panes = page.locator('[data-pane]')
    const current = () => page.locator('#places [aria-current]').innerText()
    expect(await panes.count()).toBe(0)

    // a space's ⋯ → Open in split view: beside the chat, focused, the sidebar on it
    const gtm = page.locator('#places li[data-project="p:vm-str:gtm"]')
    await gtm.hover()
    await gtm.getByRole('button', { name: 'More for gtm' }).click()
    await page.getByRole('menuitem', { name: 'Open in split view' }).click()
    await page.getByRole('textbox', { name: 'Message gtm…' }).waitFor()
    expect(await panes.count()).toBe(2)
    expect(await page.locator('[data-pane="1"][data-focused]').count()).toBe(1)
    expect(await current()).toBe('gtm')

    // a task Ctrl-clicked in the list: a third pane with its own chat and box
    await page.getByRole('region', { name: 'Tasks', exact: true }).getByRole('button', { name: /GTM-14/ }).first().click({ modifiers: ['Control'] })
    await page.getByRole('textbox', { name: 'Message GTM-14…' }).waitFor()
    expect(await panes.count()).toBe(3)

    // a click in the first pane gives it the focus: the sidebar and / follow it
    await page.locator('[data-pane="0"] [data-messages]').click({ position: { x: 5, y: 5 } })
    expect(await page.locator('[data-pane="0"][data-focused]').count()).toBe(1)
    expect(await current()).toBe('home')
    await page.keyboard.press('/')
    await expect(page.evaluate(() => document.activeElement?.getAttribute('aria-label'))).resolves.toBe('Message home…')
    await page.keyboard.press('Escape')

    // the divider between the first two shares out their width
    const w0 = (await panes.nth(0).boundingBox())!.width
    const sep = (await page.getByRole('separator', { name: 'Resize panes' }).first().boundingBox())!
    await page.mouse.move(sep.x + sep.width / 2, sep.y + 300)
    await page.mouse.down()
    await page.mouse.move(sep.x + sep.width / 2 + 100, sep.y + 300, { steps: 6 })
    await page.mouse.up()
    expect((await panes.nth(0).boundingBox())!.width).toBeGreaterThan(w0 + 60)

    // all of it back on reload
    await page.reload()
    await page.getByRole('textbox', { name: 'Message GTM-14…' }).waitFor()
    expect(await panes.count()).toBe(3)
    expect((await panes.nth(0).boundingBox())!.width).toBeGreaterThan(w0 + 60)
    expect(await current()).toBe('home')

    // a task pane's Diff opens the task on the right with its side panel
    await panes.nth(2).getByRole('button', { name: 'More', exact: true }).click()
    await page.getByRole('menuitem', { name: 'Diff' }).click()
    await page.locator('#task-panel').waitFor()
    expect(await panes.count()).toBe(2)
    expect(await page.getByRole('textbox', { name: 'Message GTM-14…' }).count()).toBe(1)
    await page.getByRole('button', { name: 'Back', exact: true }).click()
    await page.locator('#task-panel').waitFor({ state: 'detached' })

    // a link to a space opens it in the focused pane
    await page.goto(at('place=vm-str/herdr-projects'))
    await page.getByRole('textbox', { name: 'Message herdr-projects…' }).waitFor()
    expect(await panes.count()).toBe(2)
    await page.goto(at('place=vm-str/home'))
    await page.getByRole('textbox', { name: 'Message home…' }).waitFor()

    // a third beside home, the last one closed: two left; the first made the only one: the middle as always
    const proj = page.locator('#places li[data-project="p:vm-str:herdr-projects"]')
    await proj.hover()
    await proj.getByRole('button', { name: 'More for herdr-projects' }).click()
    await page.getByRole('menuitem', { name: 'Open in split view' }).click()
    await panes.nth(2).waitFor()
    await panes.nth(2).getByRole('button', { name: 'Close pane' }).click()
    expect(await panes.count()).toBe(2)
    await panes.nth(0).getByRole('button', { name: 'Make it the only one' }).click()
    expect(await panes.count()).toBe(0)
    await page.getByRole('textbox', { name: 'Message home…' }).waitFor()
    expect(await page.getByRole('textbox', { name: 'Message gtm…' }).count()).toBe(0)
    await page.close()
  }, 30_000)

  test('an artifact from its card, and a phone with panes kept shows one at a time', async () => {
    const ctx = await browser!.newContext({ viewport: { width: 1600, height: 900 } })
    const page = await ctx.newPage()
    await page.goto(at('place=vm-str/home'))
    const card = page.locator('[data-card]').filter({ hasText: 'This week across machines' })
    await card.getByRole('button', { name: 'Open in split view' }).click()
    const pane = page.locator('[data-pane="1"]')
    await pane.locator('iframe[title="This week across machines"]').waitFor()
    expect((await pane.locator('iframe').boundingBox())!.height).toBeGreaterThan(600)

    // the same device at phone width: no panes, the chat as always
    await page.setViewportSize({ width: 390, height: 844 })
    await page.locator('[data-pane]').first().waitFor({ state: 'detached' })
    await page.getByRole('textbox', { name: /^Message home/ }).waitFor()
    await ctx.close()
  }, 30_000)
})

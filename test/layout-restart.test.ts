// Restarting an agent, in a real browser (layout.ts): the hint above a chat's box once Settings say another CLI, the
// restart sheet from a ⋯ menu, and the card its chat shows after.
import { describe, expect, test } from 'bun:test'
import { layout } from './layout.ts'

const { browser, world, server } = await layout({ everything: true })

describe.skipIf(!browser)('restarting an agent', () => {
  test('the hint says what it runs on and what Settings say; one tap restarts it, a card says on what (phone and desktop)', async () => {
    const p = world.snapshot().places.find((x) => !x.home && world.chats[x.chat]?.restart)!
    for (const phone of [true, false]) {
      world.chats[p.chat].restart = { on: 'Claude Code', want: 'Codex' }
      world.chats[p.chat].cli = 'Claude Code'
      const width = phone ? 390 : 1280
      const page = await browser!.newPage(phone ? { viewport: { width, height: 844 }, hasTouch: true, isMobile: true } : { viewport: { width, height: 800 } })
      await page.goto(`http://127.0.0.1:${server!.port}/?place=${encodeURIComponent(p.id)}`)
      const hint = page.locator('[data-restart-hint]')
      await hint.waitFor()
      expect(await hint.textContent()).toBe('Runs on Claude Code · Settings now say CodexRestart')
      const box = (await hint.locator('> div').boundingBox())!
      expect(box.x).toBeGreaterThanOrEqual(0)
      expect(box.x + box.width).toBeLessThanOrEqual(width)
      await hint.getByRole('button', { name: 'Restart' }).click()
      await hint.waitFor({ state: 'detached' })
      const card = page.locator('[data-restart-card]').last()
      await card.waitFor()
      expect(await card.textContent()).toBe('Restarted on Codex')
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
      await page.close()
    }
  }, 30_000)

  test("a task's ⋯ opens the sheet: Settings' CLI picked, another CLI offers its models; Restart posts the card", async () => {
    const page = await browser!.newPage({ viewport: { width: 1440, height: 900 } })
    await page.goto(`http://127.0.0.1:${server!.port}/?place=${encodeURIComponent('p:vm-str:gtm')}`)
    const row = page.locator('section[aria-label="Tasks"] li', { hasText: 'GTM-17' })
    await row.hover()
    await row.getByRole('button', { name: 'More for this task' }).click()
    await page.getByRole('menuitem', { name: 'Restart agent…' }).click()
    const sheet = page.getByRole('alertdialog', { name: 'Restart GTM-17’s agent?' })
    await sheet.waitFor()
    await sheet.locator('[data-restart]').waitFor()
    expect(await sheet.textContent()).toContain('It resumes its session.')
    expect(await sheet.textContent()).toContain('Its worktree and branch stay as they are.')
    await sheet.getByRole('button', { name: /^Agent: / }).click()
    await page.getByRole('menuitemradio', { name: 'Codex' }).click()
    expect(await sheet.textContent()).toContain('Codex takes over with the chat so far, the brief and the work on its branch.')
    await sheet.getByRole('button', { name: /^Model: / }).click()
    await page.getByRole('menuitemradio', { name: 'GPT-6-Sol', exact: true }).click()
    await sheet.getByRole('button', { name: /^Effort: / }).click()
    await page.getByRole('menuitemradio', { name: 'High', exact: true }).click()
    await sheet.getByRole('button', { name: 'Restart', exact: true }).click()
    await sheet.waitFor({ state: 'detached' })
    expect(world.chats['GTM-17/Agent'].messages.at(-1)?.card).toMatchObject({ restart: true, text: 'Restarted on Codex (GPT-6-Sol, high)' })
    await page.close()
  }, 30_000)

  test('the sheet fits a phone, and Cancel leaves the agent as it was', async () => {
    const p = world.snapshot().places.find((x) => !x.home)!
    const ctx = await browser!.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true })
    const page = await ctx.newPage()
    await page.goto(`http://127.0.0.1:${server!.port}/?place=${encodeURIComponent(p.id)}`)
    await page.getByRole('button', { name: /^Menu/ }).click()
    await page.getByRole('menuitem', { name: 'Restart agent…' }).click()
    const sheet = page.getByRole('alertdialog', { name: `Restart ${p.name}’s coordinator?` })
    await sheet.locator('[data-restart]').waitFor()
    expect(await sheet.textContent()).toContain('Its tasks keep going.')
    const box = (await sheet.boundingBox())!
    expect(box.x).toBeGreaterThanOrEqual(0)
    expect(box.x + box.width).toBeLessThanOrEqual(390)
    const cards = world.chats[p.chat].messages.filter((m) => m.card?.restart).length
    await sheet.getByRole('button', { name: 'Cancel' }).click()
    await sheet.waitFor({ state: 'detached' })
    expect(world.chats[p.chat].messages.filter((m) => m.card?.restart).length).toBe(cards)
    await ctx.close()
  }, 30_000)
})

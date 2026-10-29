// Persistent launch controls for both chat roles; only GitHub runs these browser tests.
import { describe, expect, test } from 'bun:test'
import { layout } from './layout.ts'

const { browser, world, server } = await layout({ everything: true })

describe.skipIf(!browser)('persistent agent controls', () => {
  test('both chats expose all three controls at phone, tablet and desktop widths; picks wait for Restart and cancel changes nothing', async () => {
    const p = world.snapshot().places.find((x) => !x.home)!
    const key = 'GTM-17/Agent'
    for (const width of [320, 768, 1440]) {
      for (const task of [false, true]) {
        const chat = task ? key : p.chat
        world.chats[chat].agent = { kind: 'codex', model: 'gpt-6-sol', effort: 'high' }
        world.chats[chat].cli = 'Codex'
        const before = JSON.stringify(world.chats[chat].agent)
        const page = await browser!.newPage({ viewport: { width, height: 900 } })
        await page.goto(`http://127.0.0.1:${server!.port}/?place=${encodeURIComponent(p.id)}${task ? '&task=GTM-17' : ''}`)
        const controls = page.locator('[data-agent-controls]').last()
        await controls.waitFor()
        expect(await controls.getByRole('button').count()).toBe(3)
        expect(await controls.textContent()).toContain('gpt-6-sol')
        await controls.getByRole('button', { name: /^Model:/ }).click()
        const sheet = page.getByRole('alertdialog')
        await sheet.locator('[data-restart]').waitFor()
        expect(await sheet.textContent()).toContain('Applies to this chat after Restart')
        expect(await sheet.getByRole('switch').getAttribute('aria-checked')).toBe('false')
        await sheet.getByRole('button', { name: /^Agent: / }).click()
        await page.getByRole('menuitemradio', { name: 'Claude Code', exact: true }).click()
        // A model from Codex cannot leak into Claude; the new CLI starts from Default.
        expect(await sheet.getByRole('button', { name: 'Model: Default', exact: true }).count()).toBe(1)
        expect(JSON.stringify(world.chats[chat].agent)).toBe(before)
        await sheet.getByRole('button', { name: 'Cancel', exact: true }).click()
        expect(JSON.stringify(world.chats[chat].agent)).toBe(before)
        expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
        await page.close()
      }
    }
  }, 30_000)

  test('a retired launch model stays visible, blocks Restart, and can be replaced with Default', async () => {
    const key = 'GTM-17/Agent'
    world.chats[key].agent = { kind: 'codex', model: 'retired-model' }
    world.chats[key].cli = 'Codex'
    const page = await browser!.newPage({ viewport: { width: 390, height: 844 } })
    await page.goto(`http://127.0.0.1:${server!.port}/?place=all&task=GTM-17`)
    await page.locator('[data-agent-controls]').last().getByRole('button', { name: /^Model:/ }).click()
    const sheet = page.getByRole('alertdialog')
    await sheet.getByText('retired-model is unavailable.', { exact: false }).waitFor()
    expect(await sheet.getByRole('button', { name: 'Restart', exact: true }).isDisabled()).toBe(true)
    await sheet.getByRole('button', { name: 'Model: retired-model', exact: true }).click()
    await page.getByRole('menuitemradio', { name: 'Default', exact: true }).click()
    expect(await sheet.getByRole('button', { name: 'Restart', exact: true }).isEnabled()).toBe(true)
    await sheet.getByRole('button', { name: 'Restart', exact: true }).click()
    await sheet.waitFor({ state: 'detached' })
    expect(world.chats[key].agent).toEqual({ kind: 'codex', model: '', effort: '' })
    expect(world.chats[key].messages.at(-1)?.card?.text).toBe('Restarted on Codex')
    await page.close()
  }, 30_000)
})

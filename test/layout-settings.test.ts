// Notifications and the content's scope picker: UI regression coverage for SHE-263.
import { describe, expect, test } from 'bun:test'
import { layout, settled } from './layout.ts'

const { browser, world, url } = await layout()

describe.skipIf(!browser)('notification settings', () => {
  test('a space edit and reset leave the machine, Everywhere and another home unchanged', async () => {
    const page = await browser!.newPage({ viewport: { width: 1280, height: 900 } })
    const before = world.sections
    try {
      await page.goto(url())
      await page.getByRole('button', { name: 'Settings', exact: true }).click()
      const settings = page.getByRole('region', { name: 'Settings' })
      await settings.getByRole('button', { name: 'Notifications', exact: true }).click()
      const rule = settings.getByRole('group', { name: 'Notify', exact: true })
      const value = (level: string) => world.sections.find((s) => s.level === level && s.name === 'Notify')!.rows[0].value
      const others = ['all', 'm:vm-str', 'p:mac:~'].map(value)
      await rule.getByRole('button', { name: 'Off', exact: true }).click()
      await settings.getByRole('button', { name: 'Reset to Needs me', exact: true }).waitFor()
      expect(value('p:vm-str:~')).toBe('Off')
      expect(['all', 'm:vm-str', 'p:mac:~'].map(value)).toEqual(others)
      expect(await settings.textContent()).toContain('Set here')
      expect(await settings.textContent()).toContain('Inherits Needs me · Everywhere')
      await settings.getByRole('button', { name: 'Add this device', exact: true }).waitFor()
      await settings.getByRole('button', { name: 'Reset to Needs me', exact: true }).click()
      await settings.getByRole('button', { name: 'Reset to Needs me', exact: true }).waitFor({ state: 'detached' })
      expect(value('p:vm-str:~')).toBe('Needs me')
      expect(['all', 'm:vm-str', 'p:mac:~'].map(value)).toEqual(others)
      await settings.getByRole('button', { name: 'Plugins', exact: true }).click()
      expect(await settings.getByRole('button', { name: /^Editing:/ }).count()).toBe(0)
      expect(await settings.textContent()).toContain('Editing:Everywhere')
      await settings.getByRole('button', { name: 'Voice', exact: true }).click()
      expect(await settings.getByRole('button', { name: 'Add this device', exact: true }).count()).toBe(0)
      await settings.getByRole('button', { name: 'How tasks work', exact: true }).click()
      const wording = settings.locator('details').first()
      expect(await wording.getAttribute('open')).toBeNull()
      await wording.locator('summary').click()
      expect(await wording.innerText()).toContain('Adds:')
      expect(await settings.textContent()).toContain('Every new brief ends with')
    } finally {
      world.stores['vm-str'].set('notify', null, { space: '~' })
      world.sections = before
      world.emit('snapshot')
      await page.close()
    }
  })

  test('twenty machines and duplicate homes can be searched and picked on a phone', async () => {
    const before = { machines: world.machines, places: world.places, sections: world.sections }
    const template = world.places.find((p) => p.id === 'p:vm-str:~')!
    const machines = Array.from({ length: 20 }, (_, i) => `machine-${i + 1}`)
    world.machines = [...world.machines, ...machines.map((m) => ({ ...world.machines[0], id: `m:${m}`, machine: m, name: m }))]
    world.places = [...world.places, ...machines.map((m) => ({ ...template, id: `p:${m}:~`, machine: m, name: 'home' }))]
    world.sections = [...world.sections, ...machines.flatMap((m) => before.sections.filter((s) => s.level === 'm:vm-str' || s.level === template.id).map((s) => ({ ...s, machine: m, level: s.level!.replace('vm-str', m) })))]
    world.emit('snapshot')
    const page = await browser!.newPage({ viewport: { width: 375, height: 812 }, hasTouch: true, isMobile: true })
    try {
      await page.goto(url())
      await page.getByRole('button', { name: 'Places', exact: true }).click()
      await page.getByRole('button', { name: 'Settings', exact: true }).click()
      await page.getByRole('button', { name: 'Editing: home · vm-str', exact: true }).click()
      await page.getByRole('textbox', { name: 'Find scope or value' }).fill('home · machine-20')
      const menu = page.getByRole('menu', { name: 'Choices' })
      expect(await menu.getByRole('menuitemradio').count()).toBe(1)
      await menu.getByRole('menuitemradio', { name: 'home · machine-20', exact: true }).click()
      await page.getByRole('button', { name: 'Editing: home · machine-20', exact: true }).waitFor()
      await page.getByRole('button', { name: 'Notifications', exact: true }).click()
      await settled(page)
      const region = page.getByRole('region', { name: 'Notifications', exact: true })
      await region.getByRole('button', { name: 'Editing: home · machine-20', exact: true }).waitFor()
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(375)
    } finally {
      Object.assign(world, before)
      world.emit('snapshot')
      await page.close()
    }
  })
})

describe.skipIf(!browser)('chat agent choices', () => {
  test('Agents explains the chat limit and offers only readable agents', async () => {
    const page = await browser!.newPage({ viewport: { width: 375, height: 812 }, hasTouch: true, isMobile: true })
    try {
      await page.goto(url())
      await page.getByRole('button', { name: 'Places', exact: true }).click()
      await page.getByRole('button', { name: 'Settings', exact: true }).click()
      const settings = page.getByRole('region', { name: 'Settings' })
      await settings.getByRole('button', { name: 'Agents', exact: true }).click()
      expect(await settings.textContent()).toContain('other installed CLIs cannot show replies here')
      expect(await settings.textContent()).toContain('Headless review is separate')
      for (const group of ['Tasks', 'Coordinators', 'Backup']) {
        await settings.getByRole('heading', { name: group, exact: true }).locator('..').getByRole('button', { name: /^Agent:/ }).click()
        const choices = await page.getByRole('menuitemradio').allTextContents()
        expect(choices.map((v) => v.trim())).toEqual(group === 'Backup' ? ['Off', 'Claude Code', 'Codex', 'Pi'] : ['Claude Code', 'Codex', 'Pi'])
        await page.keyboard.press('Escape')
      }
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(375)
    } finally { await page.close() }
  })
})

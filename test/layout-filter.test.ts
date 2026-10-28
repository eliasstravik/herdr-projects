// Filtering a place's Tasks and Artifacts lists as you type, in a real browser (layout.ts).
import { describe, expect, test } from 'bun:test'
import { layout } from './layout.ts'

const { browser, server } = await layout({ everything: true })

const ids = (rows: import('playwright-core').Locator) => rows.evaluateAll((li) => li.map((l) => l.textContent?.match(/[A-Z]+-\d+/)?.[0]))

describe.skipIf(!browser)('filtering the tasks', () => {
  test('by id, title or status line, Done and Archived ones too; Esc clears it, then leaves it; Enter opens the first; F gets to it', async () => {
    const page = await browser!.newPage({ viewport: { width: 1440, height: 900 } })
    await page.goto(`http://127.0.0.1:${server!.port}/?place=vm-str/home`)
    const tasks = page.locator('section[aria-label="Tasks"]')
    const box = tasks.getByRole('searchbox', { name: 'Filter tasks' })
    await box.waitFor()
    // F, from anywhere but a box, puts you in it
    await page.locator('body').click({ position: { x: 600, y: 500 } })
    await page.keyboard.press('f')
    expect(await box.evaluate((b) => b === document.activeElement)).toBe(true)
    // the id and the title: archived ones after the rest, under Archived; the Archived link goes
    await page.keyboard.type('gtm')
    await tasks.getByText('Archived', { exact: true }).waitFor()
    expect(await ids(tasks.locator('li'))).toEqual(['GTM-14', 'GTM-16', 'GTM-15', 'GTM-18', 'GTM-17', 'GTM-12', 'GTM-11', 'GTM-9'])
    // every word, in any order and case: the status line counts too
    await box.fill('mute TRACING')
    await tasks.locator('li').first().getByText('HP-202').waitFor()
    expect(await ids(tasks.locator('li'))).toEqual(['HP-202'])
    // nothing found, said calmly
    await box.fill('zzz')
    await tasks.getByText('No tasks match “zzz”.').waitFor()
    // Esc clears it and the whole list is back; again, it leaves the box
    await page.keyboard.press('Escape')
    expect(await box.inputValue()).toBe('')
    await page.keyboard.press('Escape')
    expect(await box.evaluate((b) => b === document.activeElement)).toBe(false)
    // Enter opens the first one left
    await box.fill('webhook signature')
    await page.keyboard.press('Enter')
    await page.getByText('Merge GTM-15?').waitFor()
    await page.close()
  }, 30_000)

  test('a phone: the box is at the top of Tasks, finger-sized, its text 16px so it does not zoom; its x clears it', async () => {
    const page = await browser!.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true })
    await page.goto(`http://127.0.0.1:${server!.port}/?place=vm-str/home&tab=tasks`)
    const box = page.getByRole('searchbox', { name: 'Filter tasks' })
    await box.waitFor()
    expect(await box.evaluate((b) => getComputedStyle(b).fontSize)).toBe('16px')
    expect((await box.evaluate((b) => b.parentElement!.getBoundingClientRect().height))).toBeGreaterThanOrEqual(44)
    await box.tap()
    await box.fill('slack')
    expect(await ids(page.locator('li'))).toEqual(['GTM-12'])
    await page.getByRole('button', { name: 'Clear' }).tap()
    expect(await box.inputValue()).toBe('')
    await page.getByText('HP-202', { exact: true }).waitFor()
    await page.close()
  }, 30_000)
})

describe.skipIf(!browser)('filtering the artifacts', () => {
  test('by title or the task that made it; nothing found is said calmly', async () => {
    const page = await browser!.newPage({ viewport: { width: 1440, height: 900 } })
    await page.goto(`http://127.0.0.1:${server!.port}/?place=vm-str/home`)
    await page.getByRole('button', { name: 'Artifacts', exact: true }).click()
    const list = page.locator('section[aria-label="Artifacts"]')
    const box = list.getByRole('searchbox', { name: 'Filter artifacts' })
    await box.fill('deck')
    await list.getByText('Onboarding deck').waitFor()
    expect(await list.locator('li').count()).toBe(1)
    await box.fill('hp-201')
    await list.getByText('Board with the fixed count').waitFor()
    expect(await list.locator('li').count()).toBe(1)
    await box.fill('zzz')
    await list.getByText('No artifacts match “zzz”.').waitFor()
    // a project lists its own only, and the Tasks tab comes back
    await page.goto(`http://127.0.0.1:${server!.port}/?place=p%3Avm-str%3Agtm`)
    await page.getByRole('button', { name: 'Artifacts', exact: true }).click()
    await list.getByText('Onboarding deck').waitFor()
    expect(await list.locator('li').count()).toBe(1)
    await page.getByRole('button', { name: /^Tasks/ }).click()
    await page.locator('section[aria-label="Tasks"]').waitFor()
    await page.close()
  }, 30_000)
})

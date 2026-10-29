// The UI pass (SHE-81): Settings, appearance and the rest, in a real browser (layout.ts).
import { describe, expect, test } from 'bun:test'
import { layout, settled } from './layout.ts'
import type { Locator } from 'playwright-core'
import type { Action } from '../shared/types.ts'

const until = async (f: () => boolean, ms = 5000) => {
  for (const t0 = Date.now(); !f(); await Bun.sleep(20)) if (Date.now() - t0 > ms) throw new Error('timed out')
}

const { browser, server, world, url } = await layout()

describe.skipIf(!browser)('the UI pass (SHE-81)', () => {
  const at = (q: string) => `http://127.0.0.1:${server!.port}/?${q}`

  test('an unreadable startup prompt keeps its screen and offers the terminal without a dead Send button', async () => {
    const chat = world.chats['p:vm-str:~']
    chat.messages.push({ id: 'startup-fallback', who: 'agent', at: Date.now(), question: { id: 'menu:fallback', title: 'The terminal is waiting for you', screen: 'A CLI asks for your input', terminal: true, noNote: true, kind: 'Form', choices: [], to: 'home' } })
    const page = await browser!.newPage({ viewport: { width: 390, height: 844 } })
    try {
      await page.goto(url())
      const card = page.locator('[data-whole="startup-fallback"] [data-card]')
      await card.waitFor()
      expect(await card.locator('pre').textContent()).toBe('A CLI asks for your input')
      expect(await card.getByRole('button', { name: 'Open terminal', exact: true }).count()).toBe(1)
      expect(await card.getByRole('button', { name: 'Send', exact: true }).count()).toBe(0)
      expect(await card.getByRole('textbox', { name: 'Note' }).count()).toBe(0)
    } finally {
      chat.messages = chat.messages.filter((m) => m.id !== 'startup-fallback')
      await page.close()
    }
  }, 30_000)

  test("a needs-you card answers its task's question right in the space's chat, and says where once answered", async () => {
    const chat = world.chats['p:vm-str:~']
    const now = Date.now()
    const q = { id: 'ask:copy', title: 'Which colour?', kind: 'Form' as const, choices: [{ label: 'Blue' }, { label: 'Green' }], to: 'LV-3 agent' }
    chat.messages.push(
      { id: 'ask-done', who: 'agent', at: now - 2, card: { id: 'ask-done', task: 'LV-3', status: 'need', text: 'needs you: Question for you', ask: { chat: 'LV-3/Agent', answered: { where: 'space' }, questions: [{ ...q, id: 'ask:old', sent: 0, elsewhere: { label: 'Answered here ·', chat: 'LV-3/Agent', name: 'LV-3 chat' } }] } } },
      { id: 'ask-reply', who: 'agent', at: now - 1, card: { id: 'ask-reply', task: 'LV-3', status: 'need', text: 'needs you: Waits for your reply', ask: { chat: 'LV-3/Agent', open: true, reply: { id: 'lv3-q', text: 'Keep the old API around?' } } } },
      { id: 'ask-open', who: 'agent', at: now, card: { id: 'ask-open', task: 'LV-3', status: 'need', text: 'needs you: Question for you', ask: { chat: 'LV-3/Agent', open: true, questions: [q] } } },
    )
    const sent: Action[] = []
    const act = world.act
    world.act = async (a: Action) => (a.type === 'answer' || (a.type === 'send' && a.chat === 'LV-3/Agent') ? (sent.push(a), { ok: true }) : act(a))
    const page = await browser!.newPage({ viewport: { width: 390, height: 844 } })
    try {
      await page.goto(url())
      const card = page.locator('[data-whole="ask-open"] [data-card]')
      await card.waitFor()
      expect(await card.locator('[data-card-source]').textContent()).toBe('LV-3 · Upload page for files')
      await card.getByRole('button', { name: 'Green' }).click()
      await card.getByRole('textbox', { name: 'Note' }).fill('the darker one')
      await card.getByRole('button', { name: 'Send', exact: true }).click()
      await until(() => sent.length > 0)
      expect(sent[0]).toMatchObject({ type: 'answer', chat: 'LV-3/Agent', question: 'ask:copy', choice: 1, note: 'the darker one', space: 'p:vm-str:~' })
      // a plain question: its words and a reply box
      const reply = page.locator('[data-whole="ask-reply"] [data-card]')
      expect(await reply.locator('[data-ask-text]').textContent()).toBe('Keep the old API around?')
      await reply.getByRole('textbox', { name: 'Reply to LV-3' }).fill('Yes, one more release')
      await reply.getByRole('button', { name: 'Send', exact: true }).click()
      await until(() => sent.length > 1)
      expect(sent[1]).toMatchObject({ type: 'send', chat: 'LV-3/Agent', text: 'Yes, one more release', reply: 'lv3-q', space: 'p:vm-str:~' })
      // answered: where, a link to the task's chat, and no Send
      const done = page.locator('[data-whole="ask-done"] [data-card]')
      expect(await done.textContent()).toContain('Answered here ·LV-3 chat')
      expect(await done.getByRole('button', { name: 'Send', exact: true }).count()).toBe(0)
      // Every copy keeps its source inside the card, including plain replies and answered cards.
      for (const c of [card, reply, done]) {
        expect(await c.locator('[data-card-source]').textContent()).toBe('LV-3 · Upload page for files')
        expect((await c.boundingBox())!.width).toBeLessThanOrEqual(390)
      }
    } finally {
      world.act = act
      chat.messages = chat.messages.filter((m) => !m.id.startsWith('ask-'))
      await page.close()
    }
  }, 30_000)

  for (const width of [390, 1440])
    test(`forwarded permission cards keep their source and answer routing (${width}px)`, async () => {
      const task = world.tasks.find((t) => t.id === 'LV-3')!
      const oldName = task.name
      task.name = 'Upload page for files with resumable transfers and a readable progress indicator on every device'
      const chat = world.chats['p:vm-str:~']
      const q = { id: 'source:permission', title: 'Allow writing the upload route?', kind: 'Question' as const, choices: [{ label: 'Allow once' }, { label: 'Deny' }], to: 'LV-3 agent' }
      chat.messages.push(
        { id: 'source-permission', who: 'agent', at: Date.now(), card: { id: 'source-permission', task: 'LV-3', status: 'need', text: 'needs you: Asks permission', ask: { chat: 'LV-3/Agent', open: true, questions: [q] } } },
        { id: 'source-missing', who: 'agent', at: Date.now() + 1, card: { id: 'source-missing', task: 'MISSING-1', status: 'need', text: 'needs you', ask: { chat: 'MISSING-1/Agent', questions: [{ ...q, id: 'source:missing' }] } } },
      )
      const sent: Action[] = []
      const act = world.act
      world.act = async (a: Action) => a.type === 'answer' ? (sent.push(a), { ok: true }) : act(a)
      const page = await browser!.newPage({ viewport: { width, height: 900 } })
      try {
        await page.goto(url())
        const card = page.locator('[data-whole="source-permission"] [data-card]')
        await card.waitFor()
        const source = card.locator('[data-card-source]')
        expect(await source.textContent()).toBe(`LV-3 · ${task.name}`)
        expect(await card.locator('[data-card-title]').textContent()).toBe(q.title)
        const size = await source.evaluate((el) => ({ width: el.clientWidth, content: el.scrollWidth, height: el.clientHeight, contentHeight: el.scrollHeight }))
        expect(size.content).toBeLessThanOrEqual(size.width)
        expect(size.contentHeight).toBeLessThanOrEqual(size.height)
        expect((await card.boundingBox())!.width).toBeLessThanOrEqual(Math.min(width, 440))
        expect(await card.getByRole('button', { name: 'Send', exact: true }).isDisabled()).toBe(true)
        await card.getByRole('button', { name: 'Allow once', exact: true }).click()
        await card.getByRole('button', { name: 'Send', exact: true }).click()
        await until(() => sent.length > 0)
        expect(sent[0]).toMatchObject({ type: 'answer', chat: 'LV-3/Agent', question: q.id, choice: 0, space: 'p:vm-str:~' })
        // A later snapshot (answered in the task chat) preserves the source on the forwarded copy.
        const copy = chat.messages.find((m) => m.id === 'source-permission')!.card!.ask!
        copy.open = false
        copy.questions = [{ ...q, sent: 0, elsewhere: { label: 'Answered in', chat: 'LV-3/Agent', name: 'LV-3 chat' } }]
        await page.reload()
        await card.waitFor()
        expect(await source.textContent()).toBe(`LV-3 · ${task.name}`)
        expect(await card.textContent()).toContain('Answered inLV-3 chat')
        expect(await card.getByRole('button', { name: 'Send', exact: true }).count()).toBe(0)
        expect(await page.locator('[data-whole="source-missing"] [data-card-source]').textContent()).toBe('MISSING-1')
      } finally {
        task.name = oldName
        world.act = act
        chat.messages = chat.messages.filter((m) => !m.id.startsWith('source-'))
        await page.close()
      }
    }, 30_000)

  test('a choice card starts with nothing picked: Send waits for a pick or a typed answer', async () => {
    const page = await browser!.newPage({ viewport: { width: 1440, height: 900 } })
    await page.goto(at('place=p:mac:lilla-varlden&task=LV-3'))
    const card = page.locator('[data-card]').filter({ hasText: 'Which upload limit?' })
    await card.waitFor()
    expect(await card.locator('[data-card-source]').count()).toBe(0)
    expect(await card.locator('button[aria-pressed=true]').count()).toBe(0)
    const send = card.getByRole('button', { name: 'Send', exact: true })
    expect(await send.isDisabled()).toBe(true)
    await card.getByRole('textbox', { name: 'Other answer' }).fill('1 GB')
    expect(await send.isDisabled()).toBe(false)
    await card.getByRole('textbox', { name: 'Other answer' }).fill('')
    expect(await send.isDisabled()).toBe(true)
    await card.getByRole('button', { name: /^50 MB per file/ }).click()
    expect(await send.isDisabled()).toBe(false)
    await page.close()
  }, 30_000)

  for (const [name, viewport] of [
    ['desktop', { width: 1440, height: 900 }],
    ['phone', { width: 390, height: 844 }],
  ] as const)
    test(`a long question wraps on its card (${name}), the ? on its first line, and each choice's description in grey under it`, async () => {
      const title = "Should the rebuilt workflow also read the founders' own LinkedIn profiles and recent posts, or only the company page as before?"
      world.chats['p:vm-str:~'].messages.push({
        id: 'long-q',
        who: 'agent',
        at: Date.now(),
        question: { id: 'menu:long-q', title, kind: 'Form', to: 'home', asks: 'Claude Code asks', choices: [{ label: 'Company page only', description: 'Same as before: just the company page' }, { label: 'Company page and founders', description: "Also read each founder's profile" }] },
      })
      const page = await browser!.newPage({ viewport, hasTouch: name === 'phone' })
      await page.goto(url())
      const card = page.locator('[data-whole="long-q"] [data-card]')
      await card.waitFor()
      const head = card.locator('[data-card-title]')
      // all of it, over more than one line
      expect(await head.textContent()).toBe(title)
      const t = await head.evaluate((el) => ({ lines: el.getClientRects().length, h: el.getBoundingClientRect().height, lh: parseFloat(getComputedStyle(el).lineHeight), top: el.getBoundingClientRect().top, cut: el.scrollWidth > el.clientWidth }))
      expect(t.h).toBeGreaterThan(t.lh * 1.5)
      expect(t.cut).toBe(false)
      // the ? sits in the middle of the first line
      const icon = (await head.evaluate((el) => el.previousElementSibling!.getBoundingClientRect().toJSON())) as { top: number; height: number }
      expect(Math.abs(icon.top + icon.height / 2 - (t.top + t.lh / 2))).toBeLessThan(1.5)
      // the label on its line, the description in grey below it
      const choice = card.getByRole('button', { name: /^Company page only/ })
      const [label, desc] = await choice.evaluate((b) => [...b.querySelectorAll('span > span')].map((s) => ({ bottom: s.getBoundingClientRect().bottom, top: s.getBoundingClientRect().top, color: getComputedStyle(s).color })))
      expect(desc.top).toBeGreaterThanOrEqual(label.bottom - 1)
      expect(desc.color).not.toBe(label.color)
      world.chats['p:vm-str:~'].messages = world.chats['p:vm-str:~'].messages.filter((m) => m.id !== 'long-q')
      await page.close()
    }, 30_000)

  for (const [name, viewport] of [
    ['desktop', { width: 1440, height: 900 }],
    ['tablet', { width: 1024, height: 768 }],
  ] as const)
    test(`nothing pops up a tooltip on hover: no title on any button, row or icon (${name})`, async () => {
      const page = await browser!.newPage({ viewport, hasTouch: name === 'tablet' })
      // an artifact frame's title only names it to screen readers
      const titled = () => page.evaluate(() => [...document.querySelectorAll('[title]:not(iframe), svg title')].map((e) => e.outerHTML.slice(0, 120)))
      await page.goto(at('place=p:mac:lilla-varlden&task=LV-3'))
      await page.locator('[data-card]').first().waitFor()
      expect(await titled()).toEqual([])
      await page.goto(at('place=vm-str/home&tab=tasks'))
      await page.locator('[data-msg]').first().waitFor()
      await page.getByRole('button', { name: 'Menu', exact: true }).click()
      await page.getByRole('menuitemcheckbox', { name: 'Autopilot' }).waitFor()
      expect(await titled()).toEqual([])
      await page.close()
    }, 30_000)

  test('V and M while typing in search or an artifact type letters; they start no call', async () => {
    const page = await browser!.newPage({ viewport: { width: 1440, height: 900 } })
    await page.goto(at('place=vm-str/home'))
    await page.getByRole('button', { name: /^Search or jump to/ }).click()
    const box = page.getByRole('combobox', { name: 'Search or jump to' })
    await box.waitFor()
    await page.keyboard.type('vm')
    expect(await box.inputValue()).toBe('vm')
    expect(await page.getByRole('button', { name: 'End call' }).count()).toBe(0)
    await page.keyboard.press('Escape')
    await page.goto(at('place=vm-str/home&artifact=a1'))
    const ask = page.getByRole('dialog').getByRole('textbox')
    await ask.click()
    await page.keyboard.type('v')
    expect(await ask.inputValue()).toBe('v')
    expect(await page.getByRole('button', { name: 'End call' }).count()).toBe(0)
    await page.close()
  }, 30_000)

  test("a tablet's first key adds Keyboard shortcuts to Settings, and the page shown stays in view", async () => {
    const page = await browser!.newPage({ viewport: { width: 834, height: 1112 }, hasTouch: true })
    await page.goto(at('place=vm-str/home'))
    await page.getByRole('button', { name: 'Places' }).click()
    await page.getByRole('button', { name: 'Settings', exact: true }).click()
    const nav = page.getByRole('navigation', { name: 'Sections' })
    expect(await nav.getByRole('button', { name: 'Keyboard shortcuts' }).count()).toBe(0)
    await nav.getByRole('button', { name: 'Plugins' }).click()
    await page.keyboard.press('ArrowDown')
    await nav.getByRole('button', { name: 'Keyboard shortcuts' }).waitFor()
    expect(await nav.locator('[aria-current=page]').textContent()).toBe('Plugins')
    // and ? opens it
    await page.keyboard.press('?')
    expect(await nav.locator('[aria-current=page]').textContent()).toBe('Keyboard shortcuts')
    await page.close()
  }, 30_000)

  test("a phone's Settings lists its sections in groups, as iOS's does; one opens over the list and Back goes back to it", async () => {
    const page = await browser!.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true })
    await page.goto(at('place=vm-str/home'))
    await page.getByRole('button', { name: 'Places' }).click()
    await page.getByRole('button', { name: 'Settings', exact: true }).click()
    const settings = page.getByRole('region', { name: 'Settings' })
    const list = settings.getByRole('navigation', { name: 'Sections' })
    await list.waitFor()
    // (once it has slid in)
    await page.waitForTimeout(600)
    // no strip of tabs that scrolls sideways: every section is a row of the list, whole
    for (const name of ['How tasks work', 'Agents', 'Appearance System', 'Machines']) {
      const r = (await list.getByRole('button', { name, exact: true }).boundingBox())!
      expect(r.x).toBeGreaterThanOrEqual(0)
      expect(r.x + r.width).toBeLessThanOrEqual(390.5)
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390)
    // opened from a space, it sets that space's defaults; the picker switches to a machine's, another space's or Everywhere
    await list.getByRole('button', { name: 'Defaults for: home · vm-str' }).click()
    await page.getByRole('menuitemradio', { name: 'gtm · vm-str' }).click()
    await list.getByRole('button', { name: 'Defaults for: gtm · vm-str' }).waitFor()
    // a space has no Machine page: a machine's capacity is its own
    expect(await list.getByRole('button', { name: 'Machine', exact: true }).count()).toBe(0)
    await list.getByRole('button', { name: 'Agents' }).click()
    const agents = settings.getByRole('region', { name: 'Agents' })
    await agents.waitFor()
    // where an inherited value comes from, and a reset for one set here
    expect(await agents.textContent()).toContain('From vm-str')
    await agents.getByRole('button', { name: 'Reset to Claude Code' }).waitFor()
    await agents.getByRole('button', { name: 'Back', exact: true }).click()
    await agents.waitFor({ state: 'detached' })
    await list.getByRole('button', { name: 'Defaults for: gtm · vm-str' }).click()
    await page.getByRole('menuitemradio', { name: 'Everywhere' }).click()
    await list.getByRole('button', { name: 'How tasks work' }).click()
    const section = settings.getByRole('region', { name: 'How tasks work' })
    await section.waitFor()
    // the keys (brief.plan) are for agents, not on the screen; the brief's end reads as words, not code
    expect(await section.textContent()).not.toContain('brief.plan')
    expect(await section.locator('pre').count()).toBe(0)
    await section.getByRole('button', { name: 'Back' }).click()
    await section.waitFor({ state: 'detached' })
    await list.getByRole('button', { name: 'Agents' }).waitFor()
    // the system's Back goes from a section to the list, then out of Settings
    await list.getByRole('button', { name: 'Tasks', exact: true }).click()
    await settings.getByRole('region', { name: 'Tasks' }).waitFor()
    await page.goBack()
    await settings.getByRole('region', { name: 'Tasks' }).waitFor({ state: 'detached' })
    await page.goBack()
    await settings.waitFor({ state: 'detached' })
    await page.close()
  }, 30_000)

  test("Settings' place picker is the app's own menu, themed: the sidebar's tree, scrolling when long, by keyboard too (SHE-205)", async () => {
    // eight more spaces on vm-str, each with its defaults: more places than the menu shows at once
    const was = { places: world.places, sections: world.sections }
    const gtm = world.places.find((p) => p.id === 'p:vm-str:gtm')!
    const more = Array.from({ length: 8 }, (_, i) => `zz-${i + 1}`)
    world.places = [...world.places, ...more.map((name) => ({ ...gtm, id: `p:vm-str:${name}`, name, chat: `chat:${name}` }))]
    world.sections = [...world.sections, ...more.flatMap((name) => was.sections.filter((x) => x.level === gtm.id).map((x) => ({ ...x, level: `p:vm-str:${name}` })))]
    world.emit('snapshot')
    const ctx = await browser!.newContext({ viewport: { width: 1280, height: 800 }, colorScheme: 'dark' })
    const page = await ctx.newPage()
    try {
      await page.goto(at(`place=${encodeURIComponent(gtm.id)}`))
      await page.getByRole('button', { name: 'Settings', exact: true }).click()
      const settings = page.getByRole('region', { name: 'Settings' })
      const pick = settings.getByRole('button', { name: 'Defaults for: gtm · vm-str' })
      await pick.click()
      const menu = page.getByRole('menu', { name: 'Choices' })
      await menu.waitFor()
      // none of the device's own pickers, which Chrome on Linux draws white
      expect(await settings.locator('select').count()).toBe(0)
      expect(await menu.evaluate((el) => getComputedStyle(el).backgroundColor)).not.toBe('rgb(255, 255, 255)')
      // Everywhere, then each machine with its spaces under it, its home space first, in the sidebar's order
      const rows = menu.getByRole('menuitemradio')
      const names = (await rows.allTextContents()).map((t) => t.trim())
      expect(names.slice(0, 8)).toEqual(['Everywhere', 'mac', 'home', 'lilla-varlden', 'vm-str', 'home', 'gtm', 'herdr-projects'])
      expect(names.slice(-8)).toEqual(more)
      // each name after its place's icon, as the sidebar's (SHE-207)
      const textX = (name: string) => menu.getByRole('menuitemradio', { name, exact: true }).locator('span span span').evaluate((el) => el.getBoundingClientRect().left)
      const [allX, machineX, projectX] = [await textX('Everywhere'), await textX('vm-str'), await textX('gtm · vm-str')]
      expect(machineX).toBeGreaterThan(allX)
      expect(projectX).toBeGreaterThan(machineX)
      expect(await menu.getByRole('menuitemradio', { name: 'gtm · vm-str' }).getAttribute('aria-checked')).toBe('true')
      // too many to show whole: it scrolls, the menu stays on the screen
      const box = (await menu.boundingBox())!
      expect(box.y + box.height).toBeLessThanOrEqual(800)
      expect(await menu.locator('> div').evaluate((el) => el.scrollHeight > el.clientHeight)).toBe(true)
      // the keyboard walks it, the row it reaches scrolled into view, and Enter picks
      await page.keyboard.press('End')
      const last = menu.getByRole('menuitemradio', { name: 'zz-8 · vm-str' })
      expect(await last.evaluate((el) => el === document.activeElement)).toBe(true)
      const inView = async () => {
        const [r, m] = [(await last.boundingBox())!, (await menu.boundingBox())!]
        return r.y >= m.y && r.y + r.height <= m.y + m.height
      }
      expect(await inView()).toBe(true)
      await page.keyboard.press('Home')
      await page.keyboard.press('ArrowDown')
      await page.keyboard.press('Enter')
      await menu.waitFor({ state: 'detached' })
      await settings.getByRole('button', { name: 'Defaults for: mac' }).waitFor()
      // Esc closes it, the focus back on the picker
      await settings.getByRole('button', { name: 'Defaults for: mac' }).click()
      await menu.waitFor()
      await page.keyboard.press('Escape')
      await menu.waitFor({ state: 'detached' })
      expect(await settings.getByRole('button', { name: 'Defaults for: mac' }).evaluate((el) => el === document.activeElement)).toBe(true)
      // and so does a click outside it, leaving the level as it was
      await settings.getByRole('button', { name: 'Defaults for: mac' }).click()
      await menu.waitFor()
      await page.mouse.click(900, 700)
      await menu.waitFor({ state: 'detached' })
      await settings.getByRole('button', { name: 'Defaults for: mac' }).waitFor()
      // a phone on its side: the menu fits the room it has, and scrolls
      const phone = await browser!.newPage({ viewport: { width: 600, height: 360 }, hasTouch: true, isMobile: true })
      await phone.goto(at(`place=${encodeURIComponent(gtm.id)}`))
      await phone.getByRole('button', { name: 'Places' }).click()
      await phone.getByRole('button', { name: 'Settings', exact: true }).click()
      await phone.getByRole('button', { name: 'Defaults for: gtm · vm-str' }).click()
      const small = phone.getByRole('menu', { name: 'Choices' })
      await small.waitFor()
      await settled(phone)
      const b = (await small.boundingBox())!
      expect(b.y).toBeGreaterThanOrEqual(0)
      expect(b.y + b.height).toBeLessThanOrEqual(360)
      expect(await small.getByRole('menuitemradio', { name: 'gtm · vm-str' }).isVisible()).toBe(true)
      await phone.close()
    } finally {
      await ctx.close()
      Object.assign(world, was)
      world.emit('snapshot')
    }
  }, 30_000)

  test("Settings → Autopilot: its three settings, the heartbeat and its news in words with presets, and the place's own HEARTBEAT.md, edited and saved (phone)", async () => {
    const page = await browser!.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true })
    await page.goto(at(`place=${encodeURIComponent('p:vm-str:gtm')}`))
    await page.getByRole('button', { name: 'Places' }).click()
    await page.getByRole('button', { name: 'Settings', exact: true }).click()
    const settings = page.getByRole('region', { name: 'Settings' })
    const list = settings.getByRole('navigation', { name: 'Sections' })
    await list.getByRole('button', { name: 'Defaults for: gtm · vm-str' }).waitFor()
    await list.getByRole('button', { name: 'Autopilot' }).click()
    const pilot = settings.getByRole('region', { name: 'Autopilot' })
    await pilot.waitFor()
    const text = (await pilot.textContent())!
    // the rule in words with its next runs, never as written; the news that wakes it at once
    for (const words of ['Heartbeat', 'Every 30 minutes (Stockholm)', 'Next: ', 'Wakes at once on', 'All news', 'Ask the coordinator for anything else', 'Checklist', 'HEARTBEAT.md', 'Write it in the checklist']) expect(text).toContain(words)
    for (const words of ['FREQ=', 'RRULE', 'Quiet hours', 'Heartbeat every', 'Max new tasks']) expect(text).not.toContain(words)
    // a preset: picked, it is the rule in words at once, set here with a Reset back to the default
    const presets = pilot.getByRole('group', { name: 'Heartbeat' })
    expect(await presets.getByRole('button').allInnerTexts()).toEqual(['15 min', '30 min', '1 hour', 'Workdays'])
    expect(await presets.getByRole('button', { name: '30 min' }).getAttribute('aria-pressed')).toBe('true')
    await presets.getByRole('button', { name: 'Workdays' }).click()
    await pilot.getByText('Every 30 minutes, 8:00-18:00, Monday to Friday (Stockholm)').waitFor()
    expect(await presets.getByRole('button', { name: 'Workdays' }).getAttribute('aria-pressed')).toBe('true')
    await pilot.getByRole('button', { name: "Reset to Everywhere's" }).click()
    await pilot.getByText('Every 30 minutes (Stockholm)').waitFor()
    // the news: all, or none that waits for the heartbeat
    const news = pilot.getByRole('group', { name: 'Wakes at once on' })
    // in words, not the button of the same name
    const words = (w: string) => pilot.getByRole('paragraph').filter({ hasText: new RegExp(`^${w}$`) })
    await news.getByRole('button', { name: 'None: it waits' }).click()
    await words('No news').waitFor()
    await news.getByRole('button', { name: 'All news' }).click()
    await words('All news').waitFor()
    const doc = pilot.getByRole('textbox', { name: 'Checklist' })
    expect(await doc.inputValue()).toContain('Tasks stuck or stalled')
    // Save shows only once it changed
    expect(await pilot.getByRole('button', { name: 'Save' }).count()).toBe(0)
    await doc.fill('- Keep the live link working\n- Ship the onboarding flow')
    await pilot.getByRole('button', { name: 'Save' }).click()
    await pilot.getByRole('button', { name: 'Save' }).waitFor({ state: 'detached' })
    expect(await doc.inputValue()).toBe('- Keep the live link working\n- Ship the onboarding flow')
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390)
    await page.close()
  }, 30_000)

  test('each setting has the control its value needs, and every page fits a 375px phone, light and dark (SHE-206)', async () => {
    for (const colorScheme of ['light', 'dark'] as const) {
      const ctx = await browser!.newContext({ viewport: { width: 375, height: 812 }, hasTouch: true, isMobile: true, colorScheme })
      const page = await ctx.newPage()
      await page.goto(at(`place=${encodeURIComponent('p:vm-str:~')}`))
      await page.getByRole('button', { name: 'Places' }).click()
      await page.getByRole('button', { name: 'Settings', exact: true }).click()
      const settings = page.getByRole('region', { name: 'Settings' })
      const list = settings.getByRole('navigation', { name: 'Sections' })
      // a machine's own defaults: picked, as a machine has no space of its own to open them from
      await list.getByRole('button', { name: 'Defaults for: home · vm-str' }).click()
      await page.getByRole('menuitemradio', { name: 'vm-str', exact: true }).click()
      await list.getByRole('button', { name: 'Defaults for: vm-str' }).waitFor()
      const open = async (name: string) => {
        await list.getByRole('button', { name, exact: true }).click()
        const region = settings.getByRole('region', { name })
        await region.waitFor()
        await settled(page)
        expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(375)
        // every control inside the screen's edges, and every value whole (none cut short with …)
        for (const box of await region.locator('dd > *').all()) {
          const r = (await box.boundingBox())!
          expect(r.x + r.width).toBeLessThanOrEqual(375)
        }
        expect(await region.locator('dd .truncate').evaluateAll((els) => els.filter((e) => e.scrollWidth > e.clientWidth).map((e) => e.textContent))).toEqual([])
        return region
      }
      const back = async (region: Locator) => {
        await region.getByRole('button', { name: 'Back', exact: true }).click()
        await region.waitFor({ state: 'detached' })
      }
      // on or off: a switch
      const machine = await open('Machine')
      expect(await machine.getByRole('switch', { name: 'herdr stops answering' }).getAttribute('aria-checked')).toBe('true')
      expect(await machine.getByRole('button', { name: 'More: Max tasks working at once' }).count()).toBe(1)
      await back(machine)
      const tasks = await open('How tasks work')
      expect(await tasks.getByRole('switch', { name: 'Review' }).getAttribute('aria-checked')).toBe('true')
      await back(tasks)
      // a few short choices: segments, the one picked pressed
      const notify = await open('Notify')
      const seg = notify.getByRole('group', { name: 'Notify' })
      expect(await seg.getByRole('button', { name: 'Needs me' }).getAttribute('aria-pressed')).toBe('true')
      await back(notify)
      // one with no other choice is only its words; an action is a button
      const alerts = await open('Alerts and voice')
      expect(await alerts.getByRole('button', { name: /ElevenLabs/ }).count()).toBe(0)
      await alerts.getByRole('button', { name: 'Add this device' }).waitFor()
      await back(alerts)
      for (const name of ['Agents', 'Tasks', 'Autopilot', 'Time zone']) await back(await open(name))
      await ctx.close()
    }
  }, 60_000)

  test('Appearance: System, Light or Dark, kept on this device, on the page before its first paint, the bars and artifacts too', async () => {
    const ctx = await browser!.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true, colorScheme: 'light' })
    const page = await ctx.newPage()
    await page.goto(at('place=vm-str/home'))
    const look = () =>
      page.evaluate(() => ({
        theme: document.documentElement.dataset.theme,
        bar: document.querySelector<HTMLMetaElement>('meta[name=theme-color]')!.content,
        bg: getComputedStyle(document.body).backgroundColor,
      }))
    expect((await look()).theme).toBe('light')
    await page.getByRole('button', { name: 'Places' }).click()
    await page.getByRole('button', { name: 'Settings', exact: true }).click()
    await page.getByRole('button', { name: 'Appearance System' }).click()
    await page.getByRole('radiogroup', { name: 'Appearance' }).getByText('Dark', { exact: true }).click()
    expect(await page.getByRole('radio', { name: 'Dark', exact: true }).isChecked()).toBe(true)
    expect(await look()).toMatchObject({ theme: 'dark', bar: '#0b0b0d' })
    // Tailwind's dark variant follows the pick, not the device's own mode
    const card = await page.getByRole('radiogroup', { name: 'Appearance' }).evaluate((el) => getComputedStyle(el).backgroundColor)
    expect(card).toBe('rgb(28, 28, 32)')
    // the device turning dark or light doesn't change a pick of its own
    await page.emulateMedia({ colorScheme: 'light' })
    expect((await look()).theme).toBe('dark')
    // a reload paints dark from the start: the page's first script sets it, before the app's code
    const seen: string[] = []
    await page.exposeFunction('seenTheme', (t: string) => seen.push(t))
    await page.addInitScript(() => document.addEventListener('readystatechange', () => document.readyState === 'interactive' && (window as unknown as { seenTheme: (t: string) => void }).seenTheme(document.documentElement.dataset.theme ?? '')))
    await page.reload()
    await page.getByRole('button', { name: 'Places' }).waitFor()
    expect(seen[0]).toBe('dark')
    expect((await look()).bar).toBe('#0b0b0d')
    // an artifact opens in the app's dark, and is told when it changes
    const frame = page.frameLocator('iframe').first()
    expect(await frame.locator('html').getAttribute('class')).toContain('shepherdr-dark')
    await page.evaluate(() => localStorage.setItem('shepherdr.appearance', 'light'))
    await page.reload()
    expect(await look()).toMatchObject({ theme: 'light', bar: '#ffffff' })
    // System follows the device, as it changes
    await page.evaluate(() => localStorage.removeItem('shepherdr.appearance'))
    await page.reload()
    await page.emulateMedia({ colorScheme: 'dark' })
    await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark')
    expect(await page.locator('iframe').first().evaluate((f: HTMLIFrameElement) => f.src)).toContain('&theme=')
    await ctx.close()
  }, 30_000)
})

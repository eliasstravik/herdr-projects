// Artifacts: one HTML file and its data, kept per place, a new version on every update, checked before anyone sees it.
import { describe, expect, test } from 'bun:test'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

process.env.SHEPHERDR_HOME ??= mkdtempSync(join(tmpdir(), 'shep-art-'))
// one process runs every test file: the home is whichever file set it first
const { HOME } = await import('../server/store.ts')

const { Artifacts, browserChecker, inlineImages, KEEP_VERSIONS, MAX_HTML, titleCase } = await import('../server/live/artifacts.ts')
const { wrapPage } = await import('../server/frame.ts')
const { Automations } = await import('../server/live/automations.ts')

const dir = mkdtempSync(join(tmpdir(), 'shep-art-files-'))
const file = (name: string, body: string | Buffer) => {
  const f = join(dir, name)
  writeFileSync(f, body)
  return f
}

describe('the store', () => {
  test('an update is a new version of the same one; data alone keeps the page', () => {
    const s = new Artifacts()
    const a = s.create({ title: 'Weekly board', place: 'p:vm:gtm', by: 'GTM-3 agent', chat: 'GTM-3/Agent', html: '<h1>one</h1>', data: '{"n":1}' })
    expect(a.id).toMatch(/^[a-z2-9]{4}$/)
    expect(s.latest(a)).toMatchObject({ n: 1, html: 1, data: 1, by: 'GTM-3 agent' })
    const v2 = s.update(a, { by: 'gtm', chat: 'p:vm:gtm', data: '{"n":2}', note: 'fresh numbers' })
    expect(v2).toMatchObject({ n: 2, html: 1, data: 2, note: 'fresh numbers' })
    expect(s.html(a)).toBe('<h1>one</h1>')
    expect(s.data(a)).toEqual({ n: 2 })
    expect(s.data(a, 1)).toEqual({ n: 1 })
    s.update(a, { by: 'gtm', html: '<h1>two</h1>' })
    expect(s.html(a)).toBe('<h1>two</h1>')
    expect(s.data(a)).toEqual({ n: 2 })
    expect(s.html(a, 2)).toBe('<h1>one</h1>')
    expect(() => s.update(a, { by: 'x' })).toThrow('Nothing to update')
    // kept across a restart
    expect(new Artifacts().get(a.id.toUpperCase())?.versions.length).toBe(3)
  })
  test('a title starts with a capital, and so does what follows a task id', () => {
    expect(titleCase('SHE-90: evidence')).toBe('SHE-90: Evidence')
    expect(titleCase('SHE-83: artifacts, full width')).toBe('SHE-83: Artifacts, full width')
    expect(titleCase(' weekly board ')).toBe('Weekly board')
    expect(titleCase('GTM-3: UI/UX findings')).toBe('GTM-3: UI/UX findings')
    expect(titleCase('SHE-90:evidence')).toBe('SHE-90:Evidence')
    expect(titleCase('vm-str:SHE-12: evidence')).toBe('vm-str:SHE-12: Evidence')
    expect(titleCase('éclair menu')).toBe('Éclair menu')
    // lowercase on purpose: code, files, commands, names like iOS
    expect(titleCase('SHE-12: iOS keyboard')).toBe('SHE-12: iOS keyboard')
    expect(titleCase('package.json diff')).toBe('package.json diff')
    expect(titleCase('`shepherdr done` output')).toBe('`shepherdr done` output')
    const s = new Artifacts()
    const a = s.create({ title: 'SHE-1: evidence', place: 'all', by: 'x', html: '<p>1</p>' })
    expect(a.title).toBe('SHE-1: Evidence')
    s.update(a, { by: 'x', title: 'SHE-1: evidence, round two' })
    expect(a.title).toBe('SHE-1: Evidence, round two')
    // titles kept from before are fixed on load
    a.title = 'SHE-1: old one'
    s.save()
    expect(new Artifacts().get(a.id)?.title).toBe('SHE-1: Old one')
  })
  test('what its page sent stays with the page it was sent from', () => {
    const s = new Artifacts()
    const a = s.create({ title: 'Form', place: 'all', by: 'All machines', html: '<form></form>' })
    s.sent(a, 1, 'Colour: Accent')
    s.update(a, { by: 'x', data: '{}' })
    expect(s.sentFor(a)).toBe('Colour: Accent')
    s.update(a, { by: 'x', html: '<form id=b></form>' })
    expect(s.sentFor(a)).toBeUndefined()
  })
  test('the last 20 versions stay, and the files an old one still shows', () => {
    const s = new Artifacts()
    const a = s.create({ title: 'Many', place: 'all', by: 'x', html: '<p>first</p>' })
    for (let i = 0; i < KEEP_VERSIONS + 4; i++) s.update(a, { by: 'x', data: `{"i":${i}}` })
    expect(a.versions.length).toBe(KEEP_VERSIONS)
    expect(a.versions[0].n).toBe(6)
    // every kept version shows the first page, so its file stays; the oldest data files go
    expect(s.html(a)).toBe('<p>first</p>')
    expect(existsSync(s.file(a.id, 2, 'json'))).toBe(false)
    expect(existsSync(s.file(a.id, 24, 'json'))).toBe(true)
  })
  test('deleted is gone', () => {
    const s = new Artifacts()
    const a = s.create({ title: 'Gone', place: 'all', by: 'x', html: '<p>x</p>' })
    expect(s.remove(a.id)).toBe(true)
    expect(s.get(a.id)).toBeUndefined()
    expect(existsSync(join(HOME, 'artifacts', a.id))).toBe(false)
  })
  test('the pages from before become artifacts with their own ids, and their chat cards follow', () => {
    mkdirSync(join(HOME, 'pages'), { recursive: true })
    writeFileSync(join(HOME, 'pages', 'f0b8dc4e-8e7.html'), '<p>old</p>')
    writeFileSync(join(HOME, 'pages.json'), JSON.stringify({ 'f0b8dc4e-8e7': { id: 'f0b8dc4e-8e7', at: 1, title: 'Bottom bar', kind: 'Preview', by: 'SHE-6 agent', chat: 'SHE-6/Agent', sent: 'ok' } }))
    const s = new Artifacts()
    const moved: string[] = []
    s.migratePages(
      (chat) => (chat === 'SHE-6/Agent' ? 'p:vm:shepherdr' : chat),
      (chat, id) => moved.push(`${chat} ${id}`),
    )
    const a = s.get('f0b8dc4e-8e7')!
    expect(a).toMatchObject({ title: 'Bottom bar', place: 'p:vm:shepherdr', sent: { html: 1, text: 'ok' } })
    expect(s.html(a)).toBe('<p>old</p>')
    expect(moved).toEqual(['SHE-6/Agent f0b8dc4e-8e7'])
    expect(existsSync(join(HOME, 'pages.json'))).toBe(false)
    expect(existsSync(join(HOME, 'pages'))).toBe(false)
  })
})

describe('what an agent hands over', () => {
  test('a picture by its path goes in as data; a file that only claims to be one does not', () => {
    const png = file('dot.png', Buffer.from('89504e470d0a1a0a0000000d49484452', 'hex'))
    const fake = file('secret.png', 'API_KEY=hunter2')
    const r = inlineImages(`<img src="${png}"><div style="background:url(${png})"></div><img src='${fake}'><img src="${dir}/none.png">`)
    expect(r.html).toContain('src="data:image/png;base64,iVBORw0KGgo')
    expect(r.html).toContain('url(data:image/png;base64,')
    expect(r.html).toContain(fake)
    expect(r.missing).toEqual([fake, `${dir}/none.png`])
  })
  test('HTML over 2 MB, data that is not JSON, and a missing file are refused with the reason', () => {
    expect(() => Artifacts.read(file('big.html', 'x'.repeat(MAX_HTML + 1)), undefined, dir)).toThrow('over 2 MB')
    expect(() => Artifacts.read(file('ok.html', '<p>ok</p>'), file('bad.json', '{nope'), dir)).toThrow('is not JSON')
    expect(() => Artifacts.read('nothere.html', undefined, dir)).toThrow('No file at')
    file('good.json', '{"a":1}')
    expect(Artifacts.read('ok.html', 'good.json', dir)).toEqual({ html: '<p>ok</p>', data: '{"a":1}', missing: [] })
  })
})

describe('the check', () => {
  test("script errors and what it reached for break it; console errors and media that doesn't fit are only said", async () => {
    const seen: string[][] = []
    const check = browserChecker(
      async (args) => {
        seen.push(args)
        for (const a of args) if (a.startsWith('screenshot ')) writeFileSync(a.slice('screenshot '.length), 'png')
        const kept = ['script error: Uncaught ReferenceError: x is not defined (line 3)', 'blocked: https://cdn.example/chart.js (artifacts load nothing: put it inline)', 'console: careful', 'media: img 1 (800×400 px) is wider than the page (390 px)']
        return { ok: true, out: `✓ Done\n${JSON.stringify(JSON.stringify(kept))}`, err: '' }
      },
      (html, data) => wrapPage(html, { check: true, phone: true, data }),
    )
    const r = await check('<p>hi</p>', { n: 1 })
    expect(r.errors).toEqual(['script error: Uncaught ReferenceError: x is not defined (line 3)', 'blocked: https://cdn.example/chart.js (artifacts load nothing: put it inline)'])
    expect(r.warnings).toEqual(['careful'])
    // media that doesn't fit is said too, not refused
    expect(r.media).toEqual(['img 1 (800×400 px) is wider than the page (390 px)'])
    expect(r.shots?.light).toEndWith('.png')
    expect(r.shots?.dark).toEndWith('-dark.png')
    // at phone width, light and dark
    expect(seen[0]).toContain('set viewport 390 844')
    expect(seen[0]).toContain('set media dark')
  })
  test('two at once take turns in the one browser, so neither gets the other one\'s pictures or errors', async () => {
    let busy = false
    let overlapped = false
    const check = browserChecker(
      async (args) => {
        if (busy) overlapped = true
        busy = true
        await Bun.sleep(20)
        const page = args.find((a) => a.startsWith('open file://'))!.slice('open file://'.length)
        for (const a of args) if (a.startsWith('screenshot ')) writeFileSync(a.slice('screenshot '.length), 'png')
        busy = false
        const bad = readFileSync(page, 'utf8').includes('RED')
        return { ok: true, out: JSON.stringify(JSON.stringify(bad ? ['script error: red'] : [])), err: '' }
      },
      (h) => h,
    )
    const [red, blue] = await Promise.all([check('<p>RED</p>'), check('<p>BLUE</p>')])
    expect(overlapped).toBe(false)
    expect(red.errors).toEqual(['script error: red'])
    expect(blue.errors).toEqual([])
    expect(red.shots!.light).not.toBe(blue.shots!.light)
  })
  test('no browser here: it says so and does not refuse', async () => {
    const check = browserChecker(async () => ({ ok: false, out: '', err: 'agent-browser: not found' }), (h) => h)
    const r = await check('<p>hi</p>')
    expect(r.errors).toEqual([])
    expect(r.skipped).toContain('not found')
  })
  test('the page it opens has the frame policy and the probe first', () => {
    const html = wrapPage('<p>x</p>', { check: true, data: { a: 1 } })
    expect(html.indexOf('Content-Security-Policy')).toBeLessThan(html.indexOf('__shepherdrCheck'))
    expect(html.indexOf('__shepherdrCheck')).toBeLessThan(html.indexOf('window.shepherdr ='))
    expect(html).toContain('data = {"a":1}')
    expect(html).not.toContain('@font-face')
  })
})

describe('the frame', () => {
  test("it opens in the app's own light or dark, and the device's mode when the app sends none (the check)", () => {
    const dark = wrapPage('<p>x</p>', { phone: true, theme: 'dark' })
    expect(dark).toStartWith('<!doctype html><html class="phone shepherdr-dark"><head>')
    expect(dark).toContain(':root.shepherdr-dark{color-scheme:dark;--bg:#0b0b0d')
    // the device's dark, unless the app said light; and the app's word again when it changes
    expect(dark).toContain('@media(prefers-color-scheme:dark){:root:not(.shepherdr-light){--bg:#0b0b0d')
    expect(dark).toContain("d.shepherdr === 'theme'")
    expect(wrapPage('<p>x</p>', { theme: 'light' })).toStartWith('<!doctype html><html class="shepherdr-light">')
    expect(wrapPage('<p>x</p>')).toStartWith('<!doctype html><html><head>')
  })
  test("a page's own <html> keeps its classes; one without a head gets one", () => {
    expect(wrapPage('<html class="deck"><body>x</body></html>', { big: true })).toContain('<html class="full deck">')
    const bare = wrapPage('<html><body>x</body></html>')
    expect(bare).toMatch(/^<!doctype html><html><head><meta charset="utf-8">/)
    expect(wrapPage('<p>x</p>', { phone: true })).toStartWith('<!doctype html><html class="phone"><head>')
  })
  test('<header> is not <head>: a page that starts with one still gets a doctype, its classes and the policy in its head', () => {
    const w = wrapPage('<header><h1>Brief</h1></header><p>x</p>', { big: true, phone: true, check: true })
    expect(w).toStartWith('<!doctype html><html class="full phone"><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy"')
    expect(w).toContain('<body><header><h1>Brief</h1></header>')
    // a head without an <html>, and an <html> without a doctype
    expect(wrapPage('<head><title>t</title></head><body>x</body>', { big: true })).toMatch(/^<!doctype html><html class="full"><head><meta charset="utf-8">/)
    expect(wrapPage('<html lang="en"><head></head><body>x</body></html>', { big: true })).toMatch(/^<!doctype html><html lang="en" class="full"><head><meta charset/)
  })
  test('data with $ patterns goes in as it is', () => {
    expect(wrapPage('<p>x</p>', { data: { s: "$' $& $1" } })).toContain(`data = {"s":"$' $& $1"}`)
  })
})


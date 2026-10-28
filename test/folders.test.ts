// Adding a project from the sidebar: only folders in home are listed or made, however the path is spelled.
import { describe, expect, test } from 'bun:test'
import { mkdirSync, mkdtempSync, realpathSync, symlinkSync, utimesSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { listFolders, makeRepo, resolveInHome } from '../server/live/folders.ts'
import { nameProblem, slugify } from '../shared/types.ts'
import { git } from '../server/live/engine/git.ts'

const outside = realpathSync(mkdtempSync(join(tmpdir(), 'shep-out-')))
const h = realpathSync(mkdtempSync(join(tmpdir(), 'shep-home-')))
mkdirSync(join(h, 'dev', 'old', '.git'), { recursive: true })
mkdirSync(join(h, 'dev', 'new', '.git'), { recursive: true })
mkdirSync(join(h, 'dev', 'added', '.git'), { recursive: true })
mkdirSync(join(h, 'dev', 'scratch'), { recursive: true })
mkdirSync(join(h, '.config'))
writeFileSync(join(h, 'dev', 'notes.txt'), 'not a folder')
writeFileSync(join(h, 'dev', 'new', '.git', 'HEAD'), 'ref: refs/heads/main\n')
// worked in: new now, added a day ago, old a year ago
for (const [r, ago] of [['new', 0], ['added', 86_400], ['old', 365 * 86_400]] as const) {
  const t = Date.now() / 1000 - ago
  utimesSync(join(h, 'dev', r, '.git'), t, t)
  if (r === 'new') utimesSync(join(h, 'dev', r, '.git', 'HEAD'), t, t)
}
symlinkSync(outside, join(h, 'out'))
symlinkSync(join(outside, 'nowhere'), join(h, 'dangling'))

describe('where a path leads', () => {
  test('~, relative and absolute paths in home are fine', () => {
    expect(resolveInHome('~', h)).toEqual({ abs: h, exists: true })
    expect(resolveInHome('~/dev/old', h)).toEqual({ abs: join(h, 'dev', 'old'), exists: true })
    expect(resolveInHome('dev', h).abs).toBe(join(h, 'dev'))
    expect(resolveInHome(join(h, 'dev'), h).abs).toBe(join(h, 'dev'))
    expect(resolveInHome('~/dev/brand-new/app', h)).toEqual({ abs: join(h, 'dev', 'brand-new', 'app'), exists: false })
  })
  test('no spelling leads out of home: .., absolute paths, links out, links to nowhere', () => {
    for (const p of ['/etc', '~/..', '~/dev/../../', '../x', `${h}-other`, '~/out', '~/out/new', '~/dangling/new', 'a\0b'])
      expect(() => resolveInHome(p, h)).toThrow()
  })
})

describe('listing folders', () => {
  test('folders only, repos first and the latest worked in on top; hidden ones are there for a typed dot', async () => {
    const f = await listFolders('~/dev', [join(h, 'dev', 'added')], h)
    expect(f.path).toBe('~/dev')
    expect(f.entries.map((e) => e.name)).toEqual(['new', 'added', 'old', 'scratch'])
    expect(f.entries.find((e) => e.name === 'scratch')).toEqual({ name: 'scratch', path: '~/dev/scratch', repo: false })
    expect(f.entries.find((e) => e.name === 'added')?.added).toBe(true)
    expect(f.entries.some((e) => e.name === 'notes.txt')).toBe(false)
    const home = await listFolders('~', [], h)
    expect(home.entries.some((e) => e.name === '.config')).toBe(true)
    // a link out of home is not shown
    expect(home.entries.some((e) => e.name === 'out')).toBe(false)
  })
  test('at home, the likely repos come first, leaving out ones already added', async () => {
    const f = await listFolders('~', [join(h, 'dev', 'added')], h)
    expect(f.likely?.[0]?.path).toBe('~/dev/new')
    expect(f.likely?.some((e) => e.name === 'added')).toBe(false)
    expect(f.likely?.some((e) => e.name === 'scratch')).toBe(false)
  })
  test('a path not made yet says so', async () => {
    const f = await listFolders('~/dev/nothing', [], h)
    expect(f).toMatchObject({ exists: false, dir: false, repo: null, entries: [] })
  })
  test('outside home is refused', async () => {
    await expect(listFolders('/etc', [], h)).rejects.toThrow('Only folders in your home')
  })
})

describe('making a new project', () => {
  test('a new folder gets git on main and a first commit', async () => {
    const at = await makeRepo('~/dev/fresh/app', h)
    expect(at).toBe(join(h, 'dev', 'fresh', 'app'))
    expect((await git(at, 'branch', '--show-current')).out).toBe('main')
    expect((await git(at, 'log', '--format=%s')).out).toBe('First commit')
    const f = await listFolders('~/dev/fresh/app', [], h)
    expect(f.repo).toBe('~/dev/fresh/app')
  })
  test('an existing folder keeps its files, uncommitted', async () => {
    writeFileSync(join(h, 'dev', 'scratch', 'idea.md'), 'hi')
    const at = await makeRepo('~/dev/scratch', h)
    expect((await git(at, 'status', '--porcelain')).out).toContain('idea.md')
  })
  test('not home itself, not a file, not outside', async () => {
    await expect(makeRepo('~', h)).rejects.toThrow()
    await expect(makeRepo('~/dev/notes.txt', h)).rejects.toThrow('is a file')
    await expect(makeRepo('~/out/x', h)).rejects.toThrow('Only folders in your home')
  })
  test("a space's name follows Slack's channel rules: lowercase, numbers, - and _, no spaces, at most 80, its own", () => {
    expect(slugify('My Project')).toBe('my-project')
    expect(slugify('  Café  au lait!! ')).toBe('cafe-au-lait')
    expect(slugify('x'.repeat(90))).toHaveLength(80)
    expect(nameProblem('my-project_2')).toBeUndefined()
    expect(nameProblem('')).toBe('Give it a name.')
    expect(nameProblem('My-project')).toBe('Lowercase only.')
    expect(nameProblem('my project')).toBe('No spaces: use - or _.')
    expect(nameProblem('a/b')).toBe('Only letters, numbers, - and _.')
    expect(nameProblem('x'.repeat(81))).toBe('At most 80 characters.')
    expect(nameProblem('x'.repeat(80))).toBeUndefined()
    expect(nameProblem('gtm', ['home', 'gtm'])).toBe('Another space on this machine has that name.')
  })
})

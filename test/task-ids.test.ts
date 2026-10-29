import { describe, expect, test } from 'bun:test'
import { mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { TaskIds, MAX_TASK_NUMBER, prefixProblem, taskIdsSection } from '../server/live/task-ids.ts'
import type { Space } from '../server/live/places.ts'

const space = (name: string, key: string): Space => ({ name, key, path: `/repos/${name}`, github: null, base: 'main', added: 1 })
const dir = () => mkdtempSync(join(tmpdir(), 'shep-task-ids-'))
const id = (name: string) => `p:machine:${name}`
const empty = () => ({ next: {}, tasks: [] })

describe('task ID reservations', () => {
  test('GAME-100 then GAME-101; old IDs unchanged; prefix switches keep numbers forward', () => {
    const old = { id: 'OLD-7', n: 7, project: id('game') }
    const ids = new TaskIds([space('game', 'OLD')], { next: { OLD: 7 }, tasks: [old] }, id, dir())
    try {
      ids.set(id('game'), 'taskIds.prefix', 'GAME')
      ids.set(id('game'), 'taskIds.next', '100')
      expect(ids.allocate(id('game')).id).toBe('GAME-100')
      expect(ids.allocate(id('game')).id).toBe('GAME-101')
      expect(old.id).toBe('OLD-7')
      expect(() => ids.set(id('game'), 'taskIds.next', '101')).toThrow('102 or higher')
      ids.set(id('game'), 'taskIds.prefix', 'OLD')
      expect(ids.allocate(id('game')).id).toBe('OLD-102')
    } finally { ids.close() }
  })

  test('deleted IDs, abandoned reservations and skipped numbers survive restart and stale engines', () => {
    const home = dir()
    const stale = { next: { WEB: 250 }, tasks: [] }
    const a = new TaskIds([space('web', 'WEB')], stale, id, home)
    a.set(id('web'), 'taskIds.next', '300')
    expect(a.allocate(id('web')).id).toBe('WEB-300') // reserved, even if task persistence subsequently fails
    const b = new TaskIds([space('web', 'WEB')], stale, id, home)
    try {
      expect(b.allocate(id('web')).id).toBe('WEB-301')
      a.set(id('web'), 'taskIds.prefix', 'SITE')
      expect(b.allocate(id('web')).id).toBe('SITE-302')
      expect(() => b.set(id('web'), 'taskIds.next', '250')).toThrow()
    } finally { a.close(); b.close() }
    const spaces = [space('web', 'WEB')]
    const restarted = new TaskIds(spaces, stale, id, home)
    try {
      expect(spaces[0].key).toBe('SITE')
      expect(restarted.allocate(id('web')).id).toBe('SITE-303')
    } finally { restarted.close() }
  })

  test('active and historical prefixes stay reserved; unknown legacy owners fail closed', () => {
    const ids = new TaskIds([space('a', 'A'), space('b', 'B')], { next: { GONE: 9 }, tasks: [] }, id, dir())
    try {
      for (const prefix of ['A', 'GONE']) expect(() => ids.set(id('b'), 'taskIds.prefix', prefix)).toThrow('reserved')
      ids.set(id('a'), 'taskIds.prefix', 'NEW')
      expect(() => ids.set(id('b'), 'taskIds.prefix', 'A')).toThrow('reserved')
      expect(ids.keys()).toContain('A')
      expect(ids.keys()).toContain('GONE')
      expect(() => ids.ensure(id('c'), 'A')).toThrow('reserved')
    } finally { ids.close() }
  })

  test('validation rejects malformed prefixes, unsafe numbers and backwards settings', () => {
    const ids = new TaskIds([space('a', 'A')], empty(), id, dir())
    try {
      for (const p of ['', 'lower', 'A-B', '1ABC', '../X', 'A'.repeat(17), ' A']) expect(prefixProblem(p)).toBeDefined()
      for (const p of ['GAME', 'API2', 'A']) expect(prefixProblem(p)).toBeUndefined()
      for (const n of ['', '0', '-1', '1.5', '1e2', '001', 'Infinity', String(Number.MAX_SAFE_INTEGER)]) expect(() => ids.set(id('a'), 'taskIds.next', n)).toThrow()
      ids.set(id('a'), 'taskIds.next', String(MAX_TASK_NUMBER))
      expect(ids.allocate(id('a')).n).toBe(MAX_TASK_NUMBER)
      expect(() => ids.allocate(id('a'))).toThrow('exhausted')
    } finally { ids.close() }
  })

  test('separate machines can use identical prefixes independently', () => {
    const a = new TaskIds([space('a', 'GAME')], empty(), id, dir())
    const b = new TaskIds([space('b', 'GAME')], empty(), (s) => `p:remote:${s}`, dir())
    try {
      expect(a.allocate(id('a')).id).toBe('GAME-1')
      expect(b.allocate('p:remote:b').id).toBe('GAME-1')
    } finally { a.close(); b.close() }
  })

  test('simultaneous processes reserve different numbers on the owning machine', async () => {
    const home = dir()
    const ids = new TaskIds([space('a', 'A')], empty(), id, home)
    ids.close()
    const module = join(import.meta.dir, '../server/live/task-ids.ts')
    const script = `import { TaskIds } from ${JSON.stringify(module)}; const ids = new TaskIds([], {next:{},tasks:[]}, x=>x, process.argv[1]); console.log(ids.allocate('p:machine:a').id); ids.close();`
    const results = await Promise.all(Array.from({ length: 6 }, async () => {
      const proc = Bun.spawn([process.execPath, '-e', script, home], { stdout: 'pipe', stderr: 'pipe' })
      const out = await new Response(proc.stdout).text()
      const err = await new Response(proc.stderr).text()
      expect(await proc.exited, err).toBe(0)
      return out.trim()
    }))
    expect(new Set(results).size).toBe(6)
    expect(results.sort()).toEqual(['A-1', 'A-2', 'A-3', 'A-4', 'A-5', 'A-6'])
  })

  test('space page has only two editable rows and no inherited reset', () => {
    const page = taskIdsSection('p:machine:a', 'machine', 'GAME', 100)
    expect(page.level).toBe('p:machine:a')
    expect(page.global).toBeUndefined()
    expect(page.rows.map((r) => r.key)).toEqual(['taskIds.prefix', 'taskIds.next'])
    expect(page.rows.every((r) => r.text && !r.inherits)).toBe(true)
  })
})

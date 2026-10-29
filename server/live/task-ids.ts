// Each machine owns its ID namespace. SQLite commits reservations before a task starts, so concurrent requests,
// restarts and deleted tasks cannot reuse an ID. Remote settings and creation are routed to that machine.
import { Database } from 'bun:sqlite'
import { mkdirSync } from 'node:fs'
import { join } from 'node:path'
import type { SettingSection } from '../../shared/types.ts'
import { HOME } from '../store.ts'
import type { Space } from './places.ts'

export const MAX_TASK_NUMBER = Number.MAX_SAFE_INTEGER - 1
export function prefixProblem(value: string) {
  return /^[A-Z][A-Z0-9]{0,15}$/.test(value) ? undefined : 'Use 1–16 capital letters or digits, starting with a letter.'
}

export function taskIdsSection(level: string, machine: string, prefix: string, next: number): SettingSection {
  return {
    name: 'Task IDs', level, machine,
    about: 'For new tasks in this space. Existing IDs stay the same.',
    rows: [
      { label: 'Prefix', key: 'taskIds.prefix', value: prefix, text: true, hint: 'Capital letters and digits, starting with a letter. Unique on this machine.' },
      { label: 'Next number', key: 'taskIds.next', value: String(next), text: true, hint: `Next task: ${prefix}-${next}. Numbers can only move forward.` },
    ],
  }
}

type Sequence = { space: string; prefix: string; last: number }
export class TaskIds {
  private db: Database
  constructor(spaces: Space[], legacy: { next: Record<string, number>; tasks: { id: string; n: number; project: string }[] }, idOf: (name: string) => string, dir = HOME) {
    mkdirSync(dir, { recursive: true, mode: 0o700 })
    this.db = new Database(join(dir, 'task-ids.sqlite'), { create: true })
    this.db.exec('PRAGMA busy_timeout=5000; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;')
    this.db.exec('CREATE TABLE IF NOT EXISTS prefixes (prefix TEXT PRIMARY KEY, space TEXT NOT NULL); CREATE TABLE IF NOT EXISTS sequences (space TEXT PRIMARY KEY, prefix TEXT NOT NULL, last INTEGER NOT NULL);')
    this.db.transaction(() => {
      // Import tombstones too: tasks.json.next remembers numbers even after tasks were deleted. Unknown owners
      // stay reserved. Never change an existing reservation when another engine has an older JSON snapshot.
      for (const prefix of new Set([...Object.keys(legacy.next), ...spaces.map((s) => s.key), ...legacy.tasks.map((t) => t.id.replace(/-\d+$/, ''))])) {
        const owners = new Set([...spaces.filter((s) => s.key === prefix).map((s) => idOf(s.name)), ...legacy.tasks.filter((t) => t.id.startsWith(`${prefix}-`)).map((t) => t.project)])
        const owner = owners.size === 1 ? [...owners][0] : ''
        this.db.query('INSERT OR IGNORE INTO prefixes VALUES (?, ?)').run(prefix, owner)
      }
      for (const s of spaces) {
        const space = idOf(s.name)
        const last = Math.max(legacy.next[s.key] ?? 0, ...legacy.tasks.filter((t) => t.project === space).map((t) => t.n), 0)
        this.db.query('INSERT OR IGNORE INTO sequences VALUES (?, ?, ?)').run(space, s.key, last)
        // SQLite is authoritative after migration, including a crash before spaces.json was mirrored.
        s.key = this.get(space).prefix
      }
    }).immediate()
  }

  keys() { return (this.db.query('SELECT prefix FROM prefixes').all() as { prefix: string }[]).map((x) => x.prefix) }
  get(space: string): Sequence {
    const row = this.db.query('SELECT * FROM sequences WHERE space = ?').get(space) as Sequence | null
    if (!row) throw new Error('Select a repository space in Settings.')
    return row
  }
  /** New spaces appear after construction. Their automatic prefix avoids all historical reservations. */
  ensure(space: string, prefix: string) {
    this.db.transaction(() => {
      if (this.db.query('SELECT 1 FROM sequences WHERE space = ?').get(space)) return
      const owner = this.db.query('SELECT space FROM prefixes WHERE prefix = ?').get(prefix) as { space: string } | null
      if (owner && owner.space !== space) throw new Error('That prefix is reserved on this machine.')
      this.db.query('INSERT OR IGNORE INTO prefixes VALUES (?, ?)').run(prefix, space)
      this.db.query('INSERT INTO sequences VALUES (?, ?, 0)').run(space, prefix)
    }).immediate()
  }
  set(space: string, key: string, value: string | null) {
    this.db.transaction(() => {
      const row = this.get(space)
      if (key === 'taskIds.prefix') {
        const prefix = value ?? ''
        const bad = prefixProblem(prefix)
        if (bad) throw new Error(bad)
        const owner = this.db.query('SELECT space FROM prefixes WHERE prefix = ?').get(prefix) as { space: string } | null
        if (owner && owner.space !== space) throw new Error('That prefix is already used or reserved on this machine.')
        this.db.query('INSERT OR IGNORE INTO prefixes VALUES (?, ?)').run(prefix, space)
        this.db.query('UPDATE sequences SET prefix = ? WHERE space = ?').run(prefix, space)
      } else if (key === 'taskIds.next') {
        const next = Number(value)
        if (!value || !/^[1-9]\d*$/.test(value) || !Number.isSafeInteger(next) || next > MAX_TASK_NUMBER) throw new Error(`Use a whole number from 1 to ${MAX_TASK_NUMBER}.`)
        if (next <= row.last) throw new Error(`Use ${row.last + 1} or higher. Earlier numbers are reserved.`)
        this.db.query('UPDATE sequences SET last = ? WHERE space = ?').run(next - 1, space)
      } else throw new Error('No such task ID setting.')
    }).immediate()
    return this.get(space)
  }
  allocate(space: string) {
    return this.db.transaction(() => {
      const row = this.get(space)
      const n = row.last + 1
      if (!Number.isSafeInteger(n) || n > MAX_TASK_NUMBER) throw new Error('Task numbers are exhausted for this space.')
      const owner = this.db.query('SELECT space FROM prefixes WHERE prefix = ?').get(row.prefix) as { space: string } | null
      if (owner?.space !== space) throw new Error('This prefix has conflicting history. Choose another in Settings → Task IDs.')
      this.db.query('UPDATE sequences SET last = ? WHERE space = ?').run(n, space)
      return { id: `${row.prefix}-${n}`, n, prefix: row.prefix }
    }).immediate()
  }
  close() { this.db.close() }
}

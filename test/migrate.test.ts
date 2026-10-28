// Once, on the first start after spaces came (server/live/migrate.ts): what was kept under All machines' and each
// machine's coordinator moves to the machine's home space. Run as Shepherdr starts, in a process of its own on a folder
// of records from before (SHEPHERDR_HOME), by Places as the live source makes it.
import { describe, expect, test } from 'bun:test'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, utimesSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const root = join(import.meta.dir, '..')
const HOME_ID = 'p:main:~'

/** Places made as Shepherdr starts on machine "main", with that folder of records: what it says to close */
function start(home: string): { retire: string[] } {
  const code = `const { Places } = await import(${JSON.stringify(join(root, 'server/live/places.ts'))}); const p = new Places(); console.log(JSON.stringify({ retire: p.retired, spaces: p.spaces.length }))`
  const r = spawnSync(process.execPath, ['-e', code], { encoding: 'utf8', env: { ...process.env, SHEPHERDR_HOME: home, SHEPHERDR_MACHINE: 'main', NODE_ENV: 'test' } })
  if (r.status !== 0) throw new Error(r.stderr)
  return { retire: JSON.parse(r.stdout).retire }
}

describe('moving to spaces, once', () => {
  test("All machines' and the machine's coordinators: their chats, modes, marks, heartbeat, pins and artifacts move to the home space", () => {
    const home = mkdtempSync(join(tmpdir(), 'shep-migrate-'))
    const put = (name: string, v: unknown) => {
      mkdirSync(join(home, name, '..'), { recursive: true })
      writeFileSync(join(home, name), typeof v === 'string' ? v : JSON.stringify(v))
    }
    const read = (name: string) => JSON.parse(readFileSync(join(home, name), 'utf8'))
    // the coordinators' session logs: the machine's older than All machines'
    const logs = mkdtempSync(join(tmpdir(), 'shep-migrate-logs-'))
    const log = (name: string, at: number) => {
      const f = join(logs, name)
      writeFileSync(f, '{}\n')
      utimesSync(f, at, at)
      return f
    }
    const allLog = log('all.jsonl', 2000)
    const mLog = log('machine.jsonl', 1000)
    const farLog = log('far.jsonl', 1500)

    const project = { name: 'web', path: '/x/web', key: 'WEB', github: null, base: 'main', added: 1 }
    put('projects.json', [project])
    // the herdr spaces opened for repos were kept in spaces.json before
    put('spaces.json', { '/x/web': 'ws-1' })
    put('modes.json', { all: 'autopilot', 'm:main': 'regular', 'm:far': 'autopilot', 'p:main:web': 'regular' })
    put('agents.json', {
      all: { key: 'all', name: 'shep-all', kind: 'claude', cwd: '/c/all', workspace: 'Coordinators', logs: [allLog], started: 1 },
      'm:main': { key: 'm:main', name: 'shep-machine', kind: 'codex', cwd: '/c/machine', workspace: 'Coordinators', logs: [mLog], started: 2 },
      'm:far': { key: 'm:far', name: 'shep-machine-far', kind: 'claude', cwd: '/c/far', workspace: 'Coordinators', logs: [farLog], started: 3 },
      'p:main:web': { key: 'p:main:web', name: 'shep-web', kind: 'claude', cwd: '/x/web', workspace: 'web', logs: [], started: 4 },
    })
    put('outbox.json', [{ key: 'all', text: 'queued' }, { key: 'p:main:web', text: 'stays' }])
    put('chatlog/all.json', { posted: [{ id: 'a1', at: 30 }, { id: 'a2', at: 10 }], answers: { 'ask:a': 1 } })
    put('chatlog/m_main.json', { posted: [{ id: 'm1', at: 20 }], answers: { 'ask:m': 0 } })
    put('unread.json', { last: { all: 50, 'm:main': 70, 'p:main:web': 5 }, read: { '*': 1, all: 40, 'm:main': 30 } })
    put('heartbeat/all.md', '- Anything waiting across machines')
    put('heartbeat/m-main.md', '- Disk and memory')
    put('heartbeats.json', { all: { last: 5 } })
    put('artifacts.json', {
      a1: { place: 'all', versions: [{ chat: 'm:main' }, { chat: 'X-1/Agent' }] },
      a2: { place: 'p:main:web', versions: [{ chat: 'p:main:web' }] },
    })
    put('order.json', { pinned: ['m:main', 'all', 'p:main:web'], 'm:main': ['p:main:web'] })
    put('names.json', { 'p:main:web': 'My App', 'm:main': 'Mac mini' })
    put('settings.json', { everywhere: { 'brief.review': 'off' }, machine: {}, projects: { web: { 'task.agent': 'pi' } } })

    // the coordinators' agents are closed once herdr shows them
    expect(start(home).retire.sort()).toEqual(['shep-all', 'shep-machine', 'shep-machine-far'])

    // projects are spaces, as they were; the herdr spaces opened for repos keep a file of their own
    expect(existsSync(join(home, 'projects.json'))).toBe(false)
    expect(read('spaces.json')).toEqual([project])
    expect(read('repo-spaces.json')).toEqual({ '/x/web': 'ws-1' })
    // on Autopilot if either was; another machine's goes to its own home space
    expect(read('modes.json')).toEqual({ 'p:main:web': 'regular', [HOME_ID]: 'autopilot', 'p:far:~': 'autopilot' })
    // one record of the home space's coordinator: the machine's session resumed, both chats' logs, oldest first
    const recs = read('agents.json')
    expect(Object.keys(recs).sort()).toEqual(['p:far:~', HOME_ID, 'p:main:web'].sort())
    expect(recs[HOME_ID]).toMatchObject({ key: HOME_ID, name: 'shepherdr-home', kind: 'codex', cwd: '/c/machine', logs: [mLog, allLog] })
    expect(recs['p:far:~']).toMatchObject({ key: 'p:far:~', name: 'shepherdr-home', logs: [farLog] })
    expect(recs['p:main:web'].name).toBe('shep-web')
    expect(read('outbox.json')).toEqual([{ key: HOME_ID, text: 'queued' }, { key: 'p:main:web', text: 'stays' }])
    // what Shepherdr posted in both chats, by time, in one
    expect(existsSync(join(home, 'chatlog', 'all.json'))).toBe(false)
    expect(existsSync(join(home, 'chatlog', 'm_main.json'))).toBe(false)
    const chat = read('chatlog/p_main_.json')
    expect(chat.posted.map((x: { id: string }) => x.id)).toEqual(['a2', 'm1', 'a1'])
    expect(chat.answers).toEqual({ 'ask:a': 1, 'ask:m': 0 })
    // unread marks: the latest of each
    expect(read('unread.json')).toEqual({ last: { 'p:main:web': 5, [HOME_ID]: 70 }, read: { '*': 1, [HOME_ID]: 40 } })
    // both checklists, side by side
    expect(existsSync(join(home, 'heartbeat', 'all.md'))).toBe(false)
    expect(existsSync(join(home, 'heartbeat', 'm-main.md'))).toBe(false)
    const beat = readFileSync(join(home, 'heartbeat', 'p-main-.md'), 'utf8')
    expect(beat).toContain('- Anything waiting across machines')
    expect(beat).toContain('- Disk and memory')
    expect(read('heartbeats.json')).toEqual({ [HOME_ID]: { last: 5 } })
    // artifacts live in the home space, their versions' chats too; a task's stays
    const arts = read('artifacts.json')
    expect(arts.a1).toEqual({ place: HOME_ID, versions: [{ chat: HOME_ID }, { chat: 'X-1/Agent' }] })
    expect(arts.a2).toEqual({ place: 'p:main:web', versions: [{ chat: 'p:main:web' }] })
    // pinned: the home space once; the machine's order of its spaces stays under its heading
    expect(read('order.json')).toEqual({ pinned: [HOME_ID, 'p:main:web'], 'm:main': ['p:main:web'] })
    // a space's name follows the space rules; a machine's stays as it was
    expect(read('names.json')).toEqual({ 'p:main:web': 'my-app', 'm:main': 'Mac mini' })
    // settings' project level is the space level
    expect(read('settings.json')).toEqual({ everywhere: { 'brief.review': 'off' }, machine: {}, spaces: { web: { 'task.agent': 'pi' } } })

    // the next start: nothing moves again
    const before = Object.fromEntries(['modes.json', 'agents.json', 'unread.json', 'order.json', 'names.json', 'settings.json', 'spaces.json'].map((f) => [f, read(f)]))
    put('modes.json', { ...before['modes.json'], all: 'regular' })
    expect(start(home)).toEqual({ retire: [] })
    expect(read('modes.json')).toEqual({ ...before['modes.json'], all: 'regular' })
    for (const f of ['agents.json', 'unread.json', 'order.json', 'names.json', 'settings.json', 'spaces.json']) expect(read(f)).toEqual(before[f])
  })

  test('a fresh install has nothing to move: no spaces yet, nothing to close', () => {
    const home = mkdtempSync(join(tmpdir(), 'shep-migrate-new-'))
    expect(start(home)).toEqual({ retire: [] })
    expect(JSON.parse(readFileSync(join(home, 'spaces.json'), 'utf8'))).toEqual([])
    expect(start(home)).toEqual({ retire: [] })
  })
})

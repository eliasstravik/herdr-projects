// Once, on the first start after spaces came: machine coordinators and All machines' are gone, each machine's home
// space does their job. What was kept under their ids moves to that home space: the coordinator's chat (its agent's
// record with its logs, and what Shepherdr posted in it), its mode, unread marks, heartbeat, pins and artifacts. All
// machines' goes to this machine's home space too (the main machine's: it keeps the overview), merged with the
// machine's. projects.json becomes spaces.json, settings' project level its space level, and the names the human gave
// follow the space name rules. Every machine's own folder of records (a worker's too) runs it for itself.
import { existsSync, readdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { slugify } from '../../shared/types.ts'
import { HOME, readJson, writeJson } from '../store.ts'

/** what a coordinator's id from before is now: All machines' and a machine's go to a home space */
export function movedId(id: string, home: (machine?: string) => string): string {
  if (id === 'all') return home()
  const m = id.match(/^m:(.+)$/)
  return m ? home(m[1]) : id
}

const old = (id: string) => id === 'all' || id.startsWith('m:')
const safe = (key: string) => key.replace(/[^a-zA-Z0-9_.-]+/g, '_')

interface Rec {
  key: string
  name: string
  logs: string[]
  [k: string]: unknown
}

/** the herdr agents of the coordinators that are gone, to close once herdr answers; nothing when it ran before */
export function migrateToSpaces(home: (machine?: string) => string, homeAgent: string): { retire: string[] } {
  // spaces.json was the herdr spaces opened for repos (engine/worktrees.ts) before: a map, not the list of spaces
  const was = readJson<unknown>('spaces.json', null)
  if (Array.isArray(was)) return { retire: [] }
  if (was && typeof was === 'object') writeJson('repo-spaces.json', { ...(was as Record<string, string>), ...readJson<Record<string, string>>('repo-spaces.json', {}) })
  const to = (id: string) => movedId(id, home)
  const retire: string[] = []

  // modes: on Autopilot if either was
  const modes = readJson<Record<string, string>>('modes.json', {})
  for (const [id, m] of Object.entries(modes))
    if (old(id)) {
      delete modes[id]
      if (modes[to(id)] !== 'autopilot') modes[to(id)] = m
    }
  writeJson('modes.json', modes)

  // the coordinators' agents: their sessions' logs make the chat, oldest first; the machine's session is the one
  // resumed (All machines' only where there was no machine's)
  const recs = readJson<Record<string, Rec>>('agents.json', {})
  const mtime = (f: string) => {
    try {
      return statSync(f).mtimeMs
    } catch {
      return 0
    }
  }
  for (const id of Object.keys(recs)) {
    if (!old(id)) continue
    const r = recs[id]
    delete recs[id]
    retire.push(r.name)
    const into = to(id)
    const had = recs[into]
    if (!had) recs[into] = { ...r, key: into, name: homeAgent }
    else {
      const keep = id === 'all' ? had : r
      recs[into] = { ...keep, key: into, name: homeAgent, logs: [...new Set([...had.logs, ...r.logs])].sort((a, b) => mtime(a) - mtime(b)) }
    }
  }
  writeJson('agents.json', recs)
  for (const f of ['outbox.json', 'typed.json']) {
    const items = readJson<{ key: string }[]>(f, [])
    if (items.some((x) => old(x.key))) writeJson(f, items.map((x) => (old(x.key) ? { ...x, key: to(x.key) } : x)))
  }
  const retries = readJson<Record<string, unknown>>('retries.json', {})
  if (Object.keys(retries).some(old)) writeJson('retries.json', Object.fromEntries(Object.entries(retries).filter(([k]) => !old(k))))

  // what Shepherdr posted in their chats, merged by time
  const dir = join(HOME, 'chatlog')
  const files = existsSync(dir) ? readdirSync(dir) : []
  for (const f of files) {
    const id = f === 'all.json' ? 'all' : f.startsWith('m_') ? `m:${f.slice(2, -5)}` : null
    if (!id) continue
    const from = readJson<{ posted: { at: number }[]; answers: Record<string, unknown> }>(join('chatlog', f), { posted: [], answers: {} })
    const name = join('chatlog', `${safe(to(id))}.json`)
    const into = readJson<typeof from>(name, { posted: [], answers: {} })
    writeJson(name, { ...into, ...from, posted: [...into.posted, ...from.posted].sort((a, b) => a.at - b.at), answers: { ...into.answers, ...from.answers } })
    rmSync(join(dir, f), { force: true })
  }

  // unread marks: the latest of each
  const unread = readJson<{ last: Record<string, number>; read: Record<string, number> } | null>('unread.json', null)
  if (unread) {
    for (const marks of [unread.last, unread.read])
      for (const [id, at] of Object.entries(marks))
        if (old(id)) {
          delete marks[id]
          marks[to(id)] = Math.max(marks[to(id)] ?? 0, at)
        }
    writeJson('unread.json', unread)
  }

  // heartbeats: the checklists side by side where both had one
  const beats = join(HOME, 'heartbeat')
  for (const f of existsSync(beats) ? readdirSync(beats) : []) {
    const id = f === 'all.md' ? 'all' : f.startsWith('m-') ? `m:${f.slice(2, -3)}` : null
    if (!id) continue
    const into = join(beats, `${to(id).replace(/[^\w.-]+/g, '-')}.md`)
    const text = readFileSync(join(beats, f), 'utf8')
    if (existsSync(into)) writeFileSync(into, `${readFileSync(into, 'utf8').trimEnd()}\n\n${text}`)
    else renameSync(join(beats, f), into)
    rmSync(join(beats, f), { force: true })
  }
  const last = readJson<Record<string, { last?: number }>>('heartbeats.json', {})
  if (Object.keys(last).some(old)) writeJson('heartbeats.json', Object.fromEntries(Object.entries(last).map(([k, v]) => [to(k), v])))

  // artifacts that lived in their places, and the chats their versions came from
  const arts = readJson<Record<string, { place: string; versions: { chat?: string }[] }>>('artifacts.json', {})
  for (const a of Object.values(arts)) {
    a.place = to(a.place)
    for (const v of a.versions) if (v.chat && old(v.chat)) v.chat = to(v.chat)
  }
  writeJson('artifacts.json', arts)

  // the sidebar: pinned machines are their home spaces now; names follow the space rules (a machine's stays as it was)
  const order = readJson<Record<string, string[]>>('order.json', {})
  if (order.pinned) {
    order.pinned = [...new Set(order.pinned.map(to))]
    writeJson('order.json', order)
  }
  const names = readJson<Record<string, string>>('names.json', {})
  for (const [id, n] of Object.entries(names)) if (id.startsWith('p:')) names[id] = slugify(n) || n
  writeJson('names.json', names)

  // settings: the project level is the space level
  const settings = readJson<Record<string, unknown>>('settings.json', {})
  if (settings.projects) {
    settings.spaces = settings.projects
    delete settings.projects
    writeJson('settings.json', settings)
  }
  // the spaces, every project as it was, last: spaces.json says it ran, so a start cut short before this runs it again
  // (each step above finds nothing left to move the second time)
  writeJson('spaces.json', readJson('projects.json', []))
  rmSync(join(HOME, 'projects.json'), { force: true })
  return { retire }
}

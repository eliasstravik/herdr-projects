// Shepherdr installed by herdr in place of herdr-projects tidies the old one away once: a dated copy first, then its
// skill and sidebar rows out, its programs stopped; ~/.herdr-projects is never changed, and a linked checkout never
// takes over.
import { describe, expect, test } from 'bun:test'
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { takeOver, withoutSidebar, type Sys } from '../scripts/take-over.ts'

const KEY = `# >>> herdr-projects key (added by the herdr-projects plugin; \`herdr-projects remove-extras\` removes it)
[[keys.command]]
key = "prefix+a"
type = "plugin_action"
command = "herdr-projects.open"
description = "work board"
# <<< herdr-projects key
`
const SIDEBAR = `# >>> herdr-projects sidebar (added by the herdr-projects plugin; \`herdr-projects remove-extras\` removes it)
[ui.sidebar.spaces]
rows = [["state_icon", "workspace"], [{ token = "$summary", dim = true }, "$progress"]]
# <<< herdr-projects sidebar
`
const OWN = '# mine\n[ui]\ntoast = false\n'

function world() {
  const home = mkdtempSync(join(tmpdir(), 'shep-take-over-'))
  const cfg = join(home, '.config')
  const plugins = join(cfg, 'herdr', 'plugins')
  const root = join(plugins, 'github-installations', 'herdr-projects-1', '1-0', 'checkout')
  mkdirSync(root, { recursive: true })
  mkdirSync(join(plugins, 'config', 'herdr-projects'), { recursive: true })
  const env = { HOME: home, XDG_CONFIG_HOME: cfg, HERDR_PLUGIN_ID: 'herdr-projects', HERDR_PLUGIN_CONFIG_DIR: join(plugins, 'config', 'herdr-projects') }
  const old = join(home, '.herdr-projects', 'data')
  mkdirSync(old, { recursive: true })
  writeFileSync(join(old, 'keeper.json'), JSON.stringify({ pid: 4242 }))
  writeFileSync(join(old, 'extras.json'), JSON.stringify({ skillPaths: [join(home, '.claude', 'skills', 'herdr-projects'), '/elsewhere/skills/herdr-projects'] }))
  for (const d of ['.claude', '.codex']) {
    mkdirSync(join(home, d, 'skills', 'herdr-projects'), { recursive: true })
    writeFileSync(join(home, d, 'skills', 'herdr-projects', 'SKILL.md'), '---\nname: herdr-projects\n---\nold how-to\n')
  }
  // a folder of that name that isn't herdr-projects' own skill
  mkdirSync(join(home, '.pi', 'agent', 'skills', 'herdr-projects'), { recursive: true })
  writeFileSync(join(home, '.pi', 'agent', 'skills', 'herdr-projects', 'notes.txt'), 'mine')
  mkdirSync(join(cfg, 'herdr'), { recursive: true })
  writeFileSync(join(cfg, 'herdr', 'config.toml'), `${OWN}\n${KEY}\n${SIDEBAR}`)
  const killed: number[] = []
  const alive = new Map<number, string>([
    [4242, `${root}/bin/herdr-projects-linux-x86_64 keeper`],
    [4243, `${root}/bin/herdr-projects-linux-x86_64 board`],
    [4244, '/home/me/dev/herdr-projects/target/release/herdr-projects keeper'],
    [4245, 'vim notes'],
  ])
  const ts: string[][] = []
  const sys: Sys = {
    args: (pid) => alive.get(pid) ?? null,
    list: () => [...alive].map(([pid, args]) => ({ pid, args })),
    kill: (pid) => {
      killed.push(pid)
      alive.delete(pid)
    },
    tailscale: (a) => (ts.push(a), a[0] === 'serve' && a[1] === 'status' ? '{}' : ''),
    reload: () => {},
    sleep: () => {},
  }
  const snapshot = () => readdirSync(join(home, '.herdr-projects'), { recursive: true }).map((f) => `${f}`).sort()
  return { home, root, env: { ...env, SHEPHERDR_HOME: join(home, '.shepherdr') }, sys, killed, ts, snapshot, config: join(cfg, 'herdr', 'config.toml') }
}

const NOW = Date.UTC(2026, 9, 28)

describe('taking over herdr-projects', () => {
  test('copies first, then takes out its skill and sidebar rows and stops its programs; its folder stays as it was', () => {
    const w = world()
    const before = w.snapshot()
    const rec = takeOver(w.root, { env: w.env, home: w.home, now: NOW, sys: w.sys })!
    const backup = join(w.home, '.herdr-projects-backup-2026-10-28')
    expect(rec.backup).toBe(backup)
    // the copies
    expect(readFileSync(join(backup, 'config.toml'), 'utf8')).toContain('herdr-projects sidebar')
    expect(readFileSync(join(backup, 'skills', '1-claude', 'SKILL.md'), 'utf8')).toContain('old how-to')
    expect(readFileSync(join(backup, 'skills', '2-codex', 'SKILL.md'), 'utf8')).toContain('old how-to')
    expect(readFileSync(join(backup, 'README.txt'), 'utf8')).toContain('~/.herdr-projects was left exactly as it was')
    // the tidying: its own skill folders only, the sidebar block only (its key now opens Shepherdr)
    expect(existsSync(join(w.home, '.claude', 'skills', 'herdr-projects'))).toBe(false)
    expect(existsSync(join(w.home, '.codex', 'skills', 'herdr-projects'))).toBe(false)
    expect(existsSync(join(w.home, '.pi', 'agent', 'skills', 'herdr-projects', 'notes.txt'))).toBe(true)
    expect(readFileSync(w.config, 'utf8')).toBe(`${OWN}\n${KEY}\n`)
    // its programs from herdr's plugin folder; not a developer's build, nor anything else
    expect(w.killed.sort()).toEqual([4242, 4243])
    expect(w.snapshot()).toEqual(before)
    expect(rec.text).toContain('stopped its keeper and board')
    expect(rec.text).toContain('~/.herdr-projects-backup-2026-10-28')
    // once only
    expect(takeOver(w.root, { env: w.env, home: w.home, now: NOW, sys: w.sys })).toBeNull()
  })

  test('a checkout linked by hand, or Shepherdr under its own id, never takes over', () => {
    const w = world()
    const linked = join(w.home, 'dev', 'shepherdr')
    mkdirSync(linked, { recursive: true })
    expect(takeOver(linked, { env: w.env, home: w.home, now: NOW, sys: w.sys })).toBeNull()
    expect(takeOver(w.root, { env: { ...w.env, HERDR_PLUGIN_ID: 'shepherdr' }, home: w.home, now: NOW, sys: w.sys })).toBeNull()
    expect(takeOver(w.root, { env: { ...w.env, HERDR_PLUGIN_ID: undefined }, home: w.home, now: NOW, sys: w.sys })).toBeNull()
    expect(existsSync(join(w.home, '.claude', 'skills', 'herdr-projects'))).toBe(true)
    expect(w.killed).toEqual([])
  })

  test('no herdr-projects here: nothing to do', () => {
    const w = world()
    const home = mkdtempSync(join(tmpdir(), 'shep-take-over-empty-'))
    expect(takeOver(w.root, { env: w.env, home, now: NOW, sys: w.sys })).toBeNull()
  })

  test('a second backup the same day gets its own folder', () => {
    const w = world()
    mkdirSync(join(w.home, '.herdr-projects-backup-2026-10-28'))
    expect(takeOver(w.root, { env: w.env, home: w.home, now: NOW, sys: w.sys })!.backup).toBe(join(w.home, '.herdr-projects-backup-2026-10-28-2'))
  })

  test('the web UI link its page server took over points back at the web UI', () => {
    const w = world()
    const data = join(w.home, '.herdr-projects', 'data')
    writeFileSync(join(data, 'extras.json'), JSON.stringify({ webUiLinks: [{ https: 8443, host: 'h', was: 7317, now: 'http://127.0.0.1:41203' }] }))
    const status = JSON.stringify({ Web: { 'h:8443': { Handlers: { '/': { Proxy: 'http://127.0.0.1:41203' } } } } })
    const sys: Sys = { ...w.sys, tailscale: (a) => (w.ts.push(a), a[1] === 'status' ? status : '') }
    takeOver(w.root, { env: w.env, home: w.home, now: NOW, sys })
    expect(w.ts).toContainEqual(['serve', '--bg', '--https=8443', 'http://127.0.0.1:7317'])
  })
})

describe('the sidebar block', () => {
  test('goes, with everything else as it was', () => {
    expect(withoutSidebar(`${OWN}\n${SIDEBAR}\n${KEY}`)).toBe(`${OWN}\n${KEY}`)
    expect(withoutSidebar(OWN)).toBe(OWN)
  })
})

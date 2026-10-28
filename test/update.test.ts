// Shepherdr updates itself when its checkout moves: each commit becomes a frozen release, built and checked; the server
// restarts on it only when no voice call is on; a failed release leaves the running one.
import { describe, expect, test } from 'bun:test'
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import type { UpdateStatus } from '../shared/types.ts'
import { codeOf, Supervisor } from '../scripts/supervisor.ts'
import { head, refDirs, releasesOf, update } from '../scripts/update.ts'

/** a stand-in Shepherdr checkout: its build writes the screen saying which version it is */
function fixture() {
  const repo = mkdtempSync(join(tmpdir(), 'shep-update-'))
  const git = (...a: string[]) => Bun.spawnSync(['git', '-C', repo, '-c', 'user.name=t', '-c', 'user.email=t@t', ...a], { stderr: 'pipe' })
  git('init', '-q', '-b', 'main')
  const commit = (version: string, o: { build?: string; server?: string; deps?: string } = {}) => {
    const build = o.build ?? 'mkdir -p dist && cp VERSION dist/index.html'
    writeFileSync(join(repo, 'package.json'), JSON.stringify({ name: 'x', private: true, description: o.deps ?? '', scripts: { build } }))
    writeFileSync(join(repo, 'VERSION'), version)
    mkdirSync(join(repo, 'server'), { recursive: true })
    writeFileSync(join(repo, 'server', 'main.ts'), o.server ?? `console.log(${JSON.stringify(version)})\n`)
    git('add', '-A')
    git('commit', '-qm', version)
    return Bun.spawnSync(['git', '-C', repo, 'rev-parse', 'HEAD']).stdout.toString().trim()
  }
  const screen = (release?: string) => readFileSync(join(releasesOf(repo), release!, 'dist', 'index.html'), 'utf8')
  return { repo, commit, screen }
}

describe('a release', () => {
  const { repo, commit, screen } = fixture()
  const first = commit('v1')

  test('nothing new: nothing happens', async () => {
    expect(await head(repo)).toBe(first)
    expect(await update(repo, { built: first })).toBeNull()
  })

  test('it watches the checkout, its branches and its packed refs', async () => {
    const dirs = await refDirs(repo)
    expect(dirs).toContain(join(repo, '.git'))
    expect(dirs).toContain(join(repo, '.git', 'refs', 'heads'))
  })

  let built = first
  let release: string | undefined
  test('a merged commit becomes a release of its own, built and checked', async () => {
    const second = commit('v2')
    const r = await update(repo, { built })
    expect(r).toMatchObject({ state: 'ok', commit: second, subject: 'v2', built: second })
    expect(screen(r!.release)).toBe('v2')
    built = second
    release = r!.release
  })

  test('a merge while it builds leaves the release on the commit it started from', async () => {
    const third = commit('v3', { build: `mkdir -p dist && cp VERSION dist/index.html && git -C ${repo} -c user.name=t -c user.email=t@t commit -q --allow-empty -m v4` })
    const r = await update(repo, { built })
    expect(r).toMatchObject({ state: 'ok', commit: third, subject: 'v3' })
    expect(screen(r!.release)).toBe('v3')
    expect(await head(repo)).not.toBe(third)
    // the next look makes a release of the commit that came in
    expect((await update(repo, { built: third }))?.subject).toBe('v4')
    built = (await head(repo))!
  })

  let broken = ''
  test('a failed build keeps the running release and says why, plainly', async () => {
    broken = commit('v5', { build: "printf '\\033[31merror during build:\\033[0m\\nsrc/App.tsx: unexpected token\\n    at build (vite.mjs:1:1)\\n' && exit 1" })
    const r = await update(repo, { built })
    expect(r?.state).toBe('failed')
    expect(r?.built).toBe(built)
    expect(r?.error).toContain('the build failed')
    expect(r?.error).toContain('error during build:\nsrc/App.tsx: unexpected token')
    expect(r?.error).not.toContain('\x1b')
    expect(r?.error).not.toContain('vite.mjs')
    expect(r?.release).toBeUndefined()
    expect(screen(release)).toBe('v2')
  })

  test('a commit that failed is not tried again on every change, unless forced', async () => {
    expect(await update(repo, { built, skip: broken })).toBeNull()
    expect((await update(repo, { built, skip: broken, force: true }))?.state).toBe('failed')
  })

  test("server code that doesn't load fails the update too", async () => {
    commit('v6', { server: "import './nowhere.ts'\n" })
    const r = await update(repo, { built })
    expect(r?.state).toBe('failed')
    expect(r?.error).toContain("server code doesn't load")
  })

  test('dependencies are installed once per lockfile, and again when it changes', async () => {
    const deps = () => readdirSync(releasesOf(repo)).filter((n) => n.startsWith('deps-')).length
    const before = deps()
    commit('v7')
    expect((await update(repo, { built }))?.state).toBe('ok')
    expect(deps()).toBe(before)
    commit('v8', { deps: 'another lockfile' })
    expect((await update(repo, { built }))?.state).toBe('ok')
    expect(deps()).toBe(before + 1)
  })
})

describe('the supervisor', () => {
  test('knows when its own code changed: its file or one it imports, not types only or files it never runs', () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-code-'))
    const w = (f: string, text: string) => writeFileSync(join(dir, f), text)
    w('main.ts', "import { a } from './a.ts'\nimport type { T } from './types.ts'\nconst b = await import('./b.ts')\n")
    w('a.ts', "export { c } from './c.ts'\nexport const a = 1\n")
    w('b.ts', 'export const b = 1\n')
    w('c.ts', 'export const c = 1\n')
    w('types.ts', 'export type T = 1\n')
    w('other.ts', 'export const o = 1\n')
    const was = codeOf(join(dir, 'main.ts'))
    w('types.ts', 'export type T = 2\n')
    w('other.ts', 'export const o = 2\n')
    expect(codeOf(join(dir, 'main.ts'))).toBe(was)
    for (const f of ['c.ts', 'b.ts', 'main.ts']) {
      const before = codeOf(join(dir, 'main.ts'))
      w(f, `${readFileSync(join(dir, f), 'utf8')}// changed\n`)
      expect(codeOf(join(dir, 'main.ts'))).not.toBe(before)
    }
  })

  /** a supervisor on a fixture; `calls` are the voice calls the server says are on, each time it is asked */
  function setup(calls: number[] = []) {
    const f = fixture()
    const log: string[] = []
    const saved: UpdateStatus[] = []
    let t = 1_000_000
    let onWait = async () => {}
    const sup = new Supervisor({
      root: f.repo,
      status: null,
      save: (s) => void saved.push(s),
      drain: async (on) => {
        log.push(on ? 'stop calls' : 'calls again')
        return on ? (calls.shift() ?? 0) : 0
      },
      restart: () => void log.push('restart'),
      sleep: async (ms) => {
        t += ms
        if (ms === 5000) await onWait()
      },
      log: () => undefined,
      now: () => t,
    })
    const running = () => readFileSync(join(sup.dir(), 'dist', 'index.html'), 'utf8')
    return { ...f, sup, log, saved, running, waiting: (fn: () => Promise<void>) => (onWait = fn), tick: (ms: number) => (t += ms) }
  }

  test('the first start makes a release of the checkout and restarts on it', async () => {
    const s = setup()
    s.commit('v1')
    await s.sup.check()
    expect(s.log).toEqual(['stop calls', 'restart'])
    expect(s.running()).toBe('v1')
    expect(s.saved.at(-1)?.state).toBe('ok')
  })

  test('it waits for a voice call to end, letting calls start meanwhile, then stops new calls and restarts at once', async () => {
    const s = setup([1, 1, 0])
    s.commit('v1')
    await s.sup.check()
    // the last word before the restart stops new calls: none can start between that look and the restart
    expect(s.log).toEqual(['stop calls', 'calls again', 'stop calls', 'calls again', 'stop calls', 'restart'])
  })

  test('a merge during the wait for a call: it restarts once, on the newer commit', async () => {
    const s = setup([1])
    s.commit('v1')
    s.waiting(async () => {
      s.commit('v2')
      await s.sup.check()
    })
    await s.sup.check()
    expect(s.log.filter((l) => l === 'restart').length).toBe(1)
    expect(s.running()).toBe('v2')
    expect(s.saved.at(-1)?.subject).toBe('v2')
  })

  test("a broken merge during the wait for a call doesn't touch what runs", async () => {
    const s = setup([0, 1])
    s.commit('v1')
    await s.sup.check()
    const running = s.sup.dir()
    s.commit('v2')
    s.waiting(async () => {
      s.commit('v3', { server: "import './nowhere.ts'\n" })
      await s.sup.check()
    })
    await s.sup.check()
    expect(s.log.filter((l) => l === 'restart').length).toBe(1)
    expect(s.sup.dir()).toBe(running)
    expect(s.running()).toBe('v1')
    expect(s.saved.at(-1)).toMatchObject({ state: 'failed', subject: 'v3' })
  })

  test('a new release that stops as it starts is taken back', async () => {
    const s = setup()
    s.commit('v1')
    await s.sup.check()
    const good = s.sup.dir()
    s.tick(60_000)
    s.commit('v2', { server: 'process.exit(1)\n' })
    await s.sup.check()
    expect(s.sup.dir()).not.toBe(good)
    s.sup.stopped(1)
    expect(s.sup.dir()).toBe(good)
    expect(s.saved.at(-1)).toMatchObject({ state: 'failed', subject: 'v2' })
    expect(s.saved.at(-1)?.error).toContain('stopped as it started')
    // and it isn't tried again until something new comes
    await s.sup.check()
    expect(s.sup.dir()).toBe(good)
  })

  test('a server stopping later, or on purpose, is just started again', async () => {
    const s = setup()
    s.commit('v1')
    await s.sup.check()
    const dir = s.sup.dir()
    s.sup.stopped(0)
    s.tick(60_000)
    s.sup.stopped(1)
    expect(s.sup.dir()).toBe(dir)
    expect(s.saved.at(-1)?.state).toBe('ok')
  })
})

describe("the human's own plugins", () => {
  // the fixture's build fails on any plugin of the human's own that says BROKEN, naming its file, as vite does
  const build = 'if grep -rl BROKEN user-plugins; then echo "error: can\'t build that"; exit 1; fi; mkdir -p dist && cp VERSION dist/index.html && ls user-plugins > dist/plugins.txt'
  const { repo, commit } = fixture()
  const home = mkdtempSync(join(tmpdir(), 'shep-update-home-'))
  const plugin = (name: string, text: string) => {
    mkdirSync(join(home, 'plugins', name), { recursive: true })
    writeFileSync(join(home, 'plugins', name, 'client.tsx'), text)
  }
  const at = (release: string | undefined, ...p: string[]) => join(releasesOf(repo), release!, ...p)
  const staged = (release?: string) => JSON.parse(readFileSync(at(release, 'user-plugins.json'), 'utf8'))

  let first: Omit<UpdateStatus, 'at'> | null = null
  test('a release takes them in from outside the repo, with their off list', async () => {
    plugin('hello', 'hello v1')
    writeFileSync(join(home, 'plugins.json'), JSON.stringify({ off: ['voice'] }))
    commit('v1', { build })
    first = await update(repo, { built: null, home })
    expect(first?.state).toBe('ok')
    expect(readFileSync(at(first?.release, 'user-plugins', 'hello', 'client.tsx'), 'utf8')).toBe('hello v1')
    expect(staged(first?.release)).toEqual({ off: ['voice'], broken: {} })
  })

  test("one that doesn't build is set aside: its last good version, or else left out; the update goes on", async () => {
    plugin('hello', 'hello v2 BROKEN')
    plugin('bad', 'BROKEN')
    const r = await update(repo, { built: first!.built, release: first!.release, force: true, home })
    expect(r?.state).toBe('ok')
    expect(readFileSync(at(r?.release, 'user-plugins', 'hello', 'client.tsx'), 'utf8')).toBe('hello v1')
    expect(readFileSync(at(r?.release, 'dist', 'plugins.txt'), 'utf8').trim()).toBe('hello')
    const { broken } = staged(r?.release)
    expect(broken.hello).toMatchObject({ kept: true })
    expect(broken.hello.error).toBe('~/.shepherdr/plugins/hello/client.tsx')
    expect(broken.bad.kept).toBeUndefined()
    expect(broken.bad.error).toBe('~/.shepherdr/plugins/bad/client.tsx')
    first = r
  })

  test('an update to a new commit keeps them, and a fixed one comes back', async () => {
    plugin('hello', 'hello v3')
    rmSync(join(home, 'plugins', 'bad'), { recursive: true })
    commit('v2', { build })
    const r = await update(repo, { built: first!.built, release: first!.release, home })
    expect(r?.state).toBe('ok')
    expect(readFileSync(at(r?.release, 'user-plugins', 'hello', 'client.tsx'), 'utf8')).toBe('hello v3')
    expect(staged(r?.release).broken).toEqual({})
  })

  test('the base code failing still fails the update', async () => {
    commit('v3', { build: 'echo "error: base is broken"; exit 1' })
    const r = await update(repo, { built: first!.built, home })
    expect(r?.state).toBe('failed')
    expect(r?.error).toContain('base is broken')
  })
})

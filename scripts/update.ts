// Shepherdr updating itself: when its own checkout moves to a new commit (a task merged into main), it makes a release
// of that commit (.releases/<commit>-<time>): a frozen copy, installed if the lockfile changed, built and checked. The
// server runs from the release, never from the checkout, so a later merge can't change what runs. Any step that
// fails leaves the running release as it was. The supervisor restarts the server on a good release.
import { spawn } from 'node:child_process'
import { createHash } from 'node:crypto'
import { copyFileSync, cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, readlinkSync, renameSync, rmSync, symlinkSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import type { UpdateStatus } from '../shared/types.ts'
import { readStaged, stagedPlugins, stageUserPlugins, writeStaged } from './build-plugins.ts'

export interface Run {
  ok: boolean
  out: string
}

export function run(cmd: string[], cwd: string, env: Record<string, string> = {}): Promise<Run> {
  return new Promise((resolve) => {
    const p = spawn(cmd[0], cmd.slice(1), { cwd, env: { ...process.env, ...env }, stdio: ['ignore', 'pipe', 'pipe'] })
    let out = ''
    p.stdout.on('data', (d) => (out += d))
    p.stderr.on('data', (d) => (out += d))
    p.on('error', (e) => resolve({ ok: false, out: e.message }))
    p.on('close', (code) => resolve({ ok: code === 0, out }))
  })
}

const git = (root: string, ...args: string[]) => run(['git', ...args], root)

/** the commit the checkout is on, or null outside git */
export async function head(root: string): Promise<string | null> {
  const r = await git(root, 'rev-parse', 'HEAD')
  return r.ok ? r.out.trim() : null
}

/** the folders to watch to see the checkout move: its HEAD, and the branches and packed refs it may point at */
export async function refDirs(root: string): Promise<string[]> {
  const r = await git(root, 'rev-parse', '--absolute-git-dir', '--git-common-dir')
  if (!r.ok) return []
  const [dir, common] = r.out.trim().split('\n')
  const abs = common.startsWith('/') ? common : join(root, common)
  return [...new Set([dir, abs, join(abs, 'refs', 'heads')])].filter((d) => existsSync(d))
}

/** a dozen lines of a failed step, from its first error, enough to see why: no colours, no stack traces */
function tail(out: string) {
  const lines = out
    .replace(/\x1b\[[0-9;]*m/g, '')
    .split('\n')
    // bun run echoes the script it runs ("$ vite build"): skip it
    .filter((l) => l.trim() && !/^\s+at /.test(l) && !l.startsWith('$ '))
  const at = lines.findIndex((l) => /error/i.test(l))
  // only the closing "script exited" line says error: the lines before it say why
  return (at >= 0 && at < lines.length - 2 ? lines.slice(at, at + 12) : lines.slice(-12)).join('\n')
}

/** where releases live, beside the checkout (ignored by git) */
export const releasesOf = (root: string) => join(root, '.releases')

/**
 * Make a release of the commit the checkout is on: a frozen copy of that commit (with the checkout's plugin list and
 * the human's own plugins as they are now) in its own folder, installed, built and checked there. The checkout can
 * move on meanwhile, or while the release waits for a voice call to end: the release stays exactly the commit it says.
 * `built` is the commit running now and `release` its folder; `skip` a commit that failed already (tried again only
 * when forced); `force` makes a release of the same commit (a plugin or a plugin list changed); `home` holds the
 * human's own plugins. Returns null when there is nothing to do.
 */
export async function update(
  root: string,
  o: { built: string | null; release?: string; skip?: string; force?: boolean; bun?: string; home?: string },
): Promise<Omit<UpdateStatus, 'at'> | null> {
  const { built, bun = process.execPath } = o
  const commit = await head(root)
  if (!commit || (!o.force && (commit === built || commit === o.skip))) return null
  const subject = (await git(root, 'log', '-1', '--format=%s', commit)).out.trim()
  const releases = releasesOf(root)
  const name = `${commit.slice(0, 12)}-${Date.now().toString(36)}`
  const dir = join(releases, name)
  const failed = (error: string) => {
    rmSync(dir, { recursive: true, force: true })
    return { state: 'failed' as const, commit, subject, built, error }
  }

  mkdirSync(dir, { recursive: true })
  const x = await run(['sh', '-c', 'git archive "$1" | tar -x -C "$2"', 'sh', commit, dir], root)
  if (!x.ok) return failed(`couldn't copy the commit:\n${tail(x.out)}`)
  if (existsSync(join(root, 'plugins.json'))) copyFileSync(join(root, 'plugins.json'), join(dir, 'plugins.json'))
  stageUserPlugins(dir, o.home)

  // dependencies are installed once per lockfile and shared by the releases that have it
  const lock = ['bun.lock', 'package.json'].map((f) => (existsSync(join(dir, f)) ? readFileSync(join(dir, f), 'utf8') : '')).join('\0')
  const deps = join(releases, `deps-${createHash('sha256').update(lock).digest('hex').slice(0, 16)}`)
  if (!existsSync(join(deps, 'node_modules'))) {
    const i = await run([bun, 'install', '--frozen-lockfile'], dir)
    if (!i.ok) return failed(`bun install failed:\n${tail(i.out)}`)
    rmSync(deps, { recursive: true, force: true })
    mkdirSync(deps, { recursive: true })
    if (existsSync(join(dir, 'node_modules'))) renameSync(join(dir, 'node_modules'), join(deps, 'node_modules'))
    else mkdirSync(join(deps, 'node_modules'))
  }
  rmSync(join(dir, 'node_modules'), { recursive: true, force: true })
  symlinkSync(join(deps, 'node_modules'), join(dir, 'node_modules'), 'dir')

  // A plugin of the human's own that breaks the build is set aside, and the build tried again: its last good version
  // (the running release's copy) comes in its place, or else it is left out. Only the base code failing fails the update.
  const last = o.release ? join(releases, o.release) : null
  for (let tries = stagedPlugins(dir).length * 2 + 2; ; tries--) {
    const why = await buildAndCheck(dir, bun)
    if (!why) break
    const mine = stagedPlugins(dir)
    if (!mine.length || tries <= 0) return failed(why.error)
    const named = mine.filter((n) => why.out.includes(`user-plugins/${n}/`))
    // a failure that names none of them: see whether the base builds without them all
    for (const n of named.length ? named : mine) setAside(dir, n, faultOf(why.out, n, dir), last)
  }
  return { state: 'ok', commit, subject, built: commit, release: name }
}

/** build the screen and check the server's code loads; why not, or null */
export async function buildAndCheck(dir: string, bun: string): Promise<{ error: string; out: string } | null> {
  const b = await run([bun, 'run', 'build'], dir)
  if (!b.ok || !existsSync(join(dir, 'dist', 'index.html'))) return { error: `the build failed:\n${tail(b.out)}`, out: b.out }
  return checkServer(dir, bun)
}

/** The server runs from source: check it and everything it imports still loads. */
export async function checkServer(dir: string, bun: string): Promise<{ error: string; out: string } | null> {
  const scratch = mkdtempSync(join(tmpdir(), 'shepherdr-check-'))
  const s = await run([bun, 'build', join(dir, 'server', 'main.ts'), '--target', 'bun', '--outdir', scratch], dir)
  rmSync(scratch, { recursive: true, force: true })
  return s.ok ? null : { error: `the server code doesn't load:\n${tail(s.out)}`, out: s.out }
}

/** what a failed build said about one of the human's own plugins: from the line that names it, its paths as the human
 *  knows them */
function faultOf(out: string, n: string, dir: string) {
  const lines = tail(out.replaceAll(`${dir}/`, ''))
    .split('\n')
    .filter((l) => !/^error: script ".*" exited/.test(l))
  const at = lines.findIndex((l) => l.includes(`user-plugins/${n}/`))
  return (at >= 0 ? lines.slice(at, at + 1) : lines.slice(-3)).join('\n').replaceAll(`user-plugins/${n}/`, `~/.shepherdr/plugins/${n}/`)
}

/** Take one of the human's own plugins out of a build that failed with it: in its place its last good version, the copy
 *  the running release built, unless that is what just failed. */
function setAside(dir: string, n: string, error: string, last: string | null) {
  const staged = readStaged(dir)
  const at = join(dir, 'user-plugins', n)
  rmSync(at, { recursive: true, force: true })
  const good = last ? join(last, 'user-plugins', n) : null
  const kept = !staged.broken[n] && !!good && existsSync(good)
  if (kept) cpSync(good!, at, { recursive: true })
  // what the human's own version did wrong is what they need to know, even once its last good version failed as well
  writeStaged(dir, { ...staged, broken: { ...staged.broken, [n]: { error: staged.broken[n]?.error ?? error, ...(kept ? { kept } : {}) } } })
}

/** Drop a release that won't run, and the releases and dependencies no one needs: all but the ones kept. */
export function prune(root: string, keep: (string | undefined)[]) {
  const releases = releasesOf(root)
  if (!existsSync(releases)) return
  const names = readdirSync(releases)
  const used = new Set<string>()
  for (const n of names) {
    if (n.startsWith('deps-')) continue
    if (!keep.includes(n)) {
      rmSync(join(releases, n), { recursive: true, force: true })
      continue
    }
    try {
      used.add(readlinkSync(join(releases, n, 'node_modules')))
    } catch {
      /* no dependencies */
    }
  }
  for (const n of names) if (n.startsWith('deps-') && !used.has(join(releases, n, 'node_modules'))) rmSync(join(releases, n), { recursive: true, force: true })
}

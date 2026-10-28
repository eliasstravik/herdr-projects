// Which lines of the screen's code each browser test file runs, so a change to the screen runs only the browser tests
// that reach the changed lines (scripts/test.ts), not all of them.
//
// Recorded on GitHub, on every push to main, by each test shard (bun run test --shard=1/3 --map): each page a browser
// test opens notes which of the built screen's functions ran, and the build's source maps turn them into lines of src/
// and plugins/. A test run here fetches the newest map from there, the shards' maps as one (and keeps it in the repo's
// .git, for every worktree), then sets the lines that changed since the map's commit against it:
//   - a line in a function: the test files that ran that function
//   - a whole new function: none by itself (whatever calls it changed too)
//   - types, imports, comments: none (they don't run)
//   - any other top-level code (a constant, a call): every test file that ran any of that file's code
//   - a file the screen is built from that isn't code (styles.css, index.html), or no map: every browser test
import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, statSync, utimesSync, writeFileSync } from 'node:fs'
import { basename, join, relative } from 'node:path'
import { decodedMappings, TraceMap } from '@jridgewell/trace-mapping'
import type { Browser, BrowserContext, Page } from 'playwright-core'

const root = join(import.meta.dir, '..')

/** the lines each test file ran ([first, last] spans) of each file the screen is built from, at one commit */
export type ScreenMap = { commit: string; sources: string[]; tests: Record<string, Record<string, [number, number][]>> }

/* ---------- recording, in a browser test's own process ---------- */

/** notes what each page of this browser runs, until the page (or its context, or the browser) closes; then writes it
 *  to $SHEPHERDR_TEST_MAP/<test file>.json */
export async function record(browser: Browser, test: string) {
  const { builtScreen } = await import('./web.ts')
  const dist = await builtScreen()
  const spans: Record<string, [number, number][]> = {}
  const sources = new Set<string>()
  const on = new Set<Page>()
  const stop = async (page: Page) => {
    if (!on.delete(page)) return
    for (const e of await page.coverage.stopJSCoverage().catch(() => [])) note(dist, e, spans, sources)
  }
  const watch = (ctx: BrowserContext) => {
    const newPage = ctx.newPage.bind(ctx)
    ctx.newPage = async () => {
      const page = await newPage()
      await page.coverage.startJSCoverage({ resetOnNavigation: false })
      on.add(page)
      const close = page.close.bind(page)
      page.close = async (o) => (await stop(page), close(o))
      return page
    }
    const close = ctx.close.bind(ctx)
    ctx.close = async (o) => (await Promise.all(ctx.pages().map(stop)), close(o))
    return ctx
  }
  const newContext = browser.newContext.bind(browser)
  browser.newContext = async (o) => watch(await newContext(o))
  const close = browser.close.bind(browser)
  browser.close = async (o) => {
    await Promise.all([...on].map(stop))
    const dir = process.env.SHEPHERDR_TEST_MAP!
    mkdirSync(dir, { recursive: true })
    const map: ScreenMap = { commit: '', sources: [...sources].sort(), tests: { [test]: Object.fromEntries(Object.entries(spans).map(([f, s]) => [f, merge(s)])) } }
    writeFileSync(join(dir, `${basename(test)}.json`), JSON.stringify(map))
    return close(o)
  }
}

type Coverage = { url: string; source?: string; functions: { ranges: { startOffset: number; endOffset: number; count: number }[] }[] }
const maps = new Map<string, { map: TraceMap; at: number[]; src: number[]; line: number[]; files: (string | null)[] } | null>()

/** the lines of our own files that the functions run in one of the built screen's scripts cover */
function note(dist: string, e: Coverage, spans: Record<string, [number, number][]>, sources: Set<string>) {
  const file = join(dist, new URL(e.url).pathname.replace(/^\//, '')) + '.map'
  if (!e.source || !e.url.startsWith('http')) return
  if (!maps.has(file)) maps.set(file, existsSync(file) ? flatten(file, e.source) : null)
  const m = maps.get(file)
  if (!m) return
  for (const f of m.files) if (f) sources.add(f)
  for (const fn of e.functions) {
    const r = fn.ranges[0]
    if (!r?.count || (r.startOffset === 0 && r.endOffset >= e.source.length)) continue
    // the mappings inside the function, in order of where they are in the built script
    let lo = 0
    let hi = m.at.length
    while (lo < hi) {
      const mid = (lo + hi) >> 1
      if (m.at[mid] < r.startOffset) lo = mid + 1
      else hi = mid
    }
    // a library's function (react's) isn't ours: skip it whole
    if (lo < m.at.length && m.files[m.src[lo]] === null) continue
    const seen: Record<number, [number, number]> = {}
    for (let i = lo; i < m.at.length && m.at[i] < r.endOffset; i++) {
      if (m.files[m.src[i]] === null) continue
      const s = (seen[m.src[i]] ??= [m.line[i], m.line[i]])
      s[0] = Math.min(s[0], m.line[i])
      s[1] = Math.max(s[1], m.line[i])
    }
    for (const [src, s] of Object.entries(seen)) (spans[m.files[+src]!] ??= []).push(s)
  }
}

/** a source map as flat arrays, in order of where each mapping is in the built script: its offset there, its source
 *  (our file, or null for a library's) and its line (1-based) */
function flatten(file: string, source: string) {
  const map = new TraceMap(readFileSync(file, 'utf8'), file)
  const starts = [0]
  for (let i = source.indexOf('\n'); i >= 0; i = source.indexOf('\n', i + 1)) starts.push(i + 1)
  const at: number[] = []
  const src: number[] = []
  const line: number[] = []
  decodedMappings(map).forEach((segs, l) => {
    for (const s of segs)
      if (s.length >= 4) {
        at.push(starts[l] + s[0])
        src.push(s[1]!)
        line.push(s[2]! + 1)
      }
  })
  const files = map.resolvedSources.map((p) => {
    const r = relative(root, p.replace(/^file:\/\//, ''))
    return r.startsWith('..') || r.includes('node_modules') ? null : r
  })
  return { map, at, src, line, files }
}

/** spans joined where they meet or overlap */
function merge(spans: [number, number][]) {
  const out: [number, number][] = []
  for (const [a, b] of [...spans].sort((x, y) => x[0] - y[0])) {
    const last = out.at(-1)
    if (last && a <= last[1] + 1) last[1] = Math.max(last[1], b)
    else out.push([a, b])
  }
  return out
}

/** the maps each test file wrote (bun run test --map), as one map of this commit */
export function gather(dir: string, commit: string, onto?: ScreenMap | null): ScreenMap {
  const map: ScreenMap = onto?.commit === commit ? onto : { commit, sources: [], tests: {} }
  const sources = new Set(map.sources)
  for (const n of existsSync(dir) ? readdirSync(dir) : []) {
    const part = JSON.parse(readFileSync(join(dir, n), 'utf8')) as ScreenMap
    for (const s of part.sources) sources.add(s)
    Object.assign(map.tests, part.tests)
  }
  map.sources = [...sources].sort()
  return map
}

/* ---------- picking, in scripts/test.ts ---------- */

const git = (...args: string[]) => {
  const r = Bun.spawnSync(['git', ...args], { cwd: root, stdout: 'pipe', stderr: 'pipe' })
  return r.success ? r.stdout.toString() : null
}

/** where the map is kept: the repo's own .git, shared by all its worktrees */
export function mapFile() {
  const common = git('rev-parse', '--path-format=absolute', '--git-common-dir')?.trim()
  return join(common || join(root, 'node_modules', '.cache'), 'shepherdr-test', 'screen-map.json')
}

export const readMap = (file: string): ScreenMap | null => {
  try {
    const m = JSON.parse(readFileSync(file, 'utf8')) as ScreenMap
    return m.commit && m.tests ? m : null
  } catch {
    return null
  }
}

/** the newest map: the one kept here, after asking GitHub (at most every 10 minutes) for the one its last check of main
 *  made. Null when there is none, or its commit can't be found here. */
export function screenMap(): ScreenMap | null {
  if (process.env.SHEPHERDR_SCREEN_MAP) return readMap(process.env.SHEPHERDR_SCREEN_MAP)
  const file = mapFile()
  const stamp = `${file}.checked`
  const fresh = existsSync(stamp) && Date.now() - statSync(stamp).mtimeMs < 10 * 60_000
  let map = readMap(file)
  if (!fresh && Bun.which('gh')) {
    mkdirSync(join(file, '..'), { recursive: true })
    writeFileSync(stamp, '')
    utimesSync(stamp, new Date(), new Date())
    // gh asks the host the repo is on (github.localhost here)
    const host = git('remote', 'get-url', 'origin')?.trim().match(/^(?:https?:\/\/|git@)([^/:]+)/)?.[1]
    const env = { ...process.env, ...(host && host !== 'github.com' && !process.env.GH_HOST ? { GH_HOST: host } : {}) }
    const gh = (...args: string[]) => {
      const r = Bun.spawnSync(['gh', ...args], { cwd: root, env, stdout: 'pipe', stderr: 'pipe', timeout: 20_000 })
      return r.success ? r.stdout : null
    }
    const runs = JSON.parse(gh('run', 'list', '--workflow', 'check.yml', '--branch', 'main', '--event', 'push', '--status', 'success', '--limit', '1', '--json', 'databaseId,headSha')?.toString() || '[]') as { databaseId: number; headSha: string }[]
    if (runs[0] && runs[0].headSha !== map?.commit) {
      const tmp = `${file}.${process.pid}`
      const parts = join(tmp, 'parts')
      rmSync(tmp, { recursive: true, force: true })
      mkdirSync(parts, { recursive: true })
      // each test shard's map (screen-map-1, -2…), or one of every browser test (screen-map, as runs once made); read
      // through the API, as gh run download skips a GitHub proxy (GH_HOST) for the files
      const ids = gh('api', '--jq', '.artifacts[] | select(.name | startswith("screen-map")) | .id', `repos/{owner}/{repo}/actions/runs/${runs[0].databaseId}/artifacts?per_page=100`)
      for (const id of ids?.toString().split('\n').filter(Boolean) ?? []) {
        const zip = gh('api', `repos/{owner}/{repo}/actions/artifacts/${id}/zip`)
        if (!zip) continue
        writeFileSync(join(tmp, `${id}.zip`), zip)
        const r = Bun.spawnSync(['unzip', '-p', join(tmp, `${id}.zip`), 'screen-map.json'], { stdout: 'pipe', stderr: 'ignore' })
        if (r.success && r.stdout.length) writeFileSync(join(parts, `${id}.json`), r.stdout)
      }
      const got = readdirSync(parts).length ? gather(parts, runs[0].headSha) : null
      if (got) {
        writeFileSync(`${tmp}.json`, JSON.stringify(got))
        renameSync(`${tmp}.json`, file)
        map = got
      }
      rmSync(tmp, { recursive: true, force: true })
    }
  }
  if (map && git('cat-file', '-e', `${map.commit}^{commit}`) === null) git('fetch', '--quiet', 'origin', map.commit)
  return map && git('cat-file', '-e', `${map.commit}^{commit}`) !== null ? map : null
}

type Kind = 'fn' | 'quiet' | 'code'

/** what each line of a file is: in a function, quiet (types, imports, comments, blank) or other top-level code; and
 *  each function's span (the outermost ones) */
async function kinds(text: string, tsx: boolean) {
  const ts = (await import('typescript')).default
  const sf = ts.createSourceFile('x.tsx', text, ts.ScriptTarget.Latest, true, tsx ? ts.ScriptKind.TSX : ts.ScriptKind.TS)
  const line = (pos: number) => sf.getLineAndCharacterOfPosition(pos).line + 1
  const n = text.split('\n').length
  const kind: Kind[] = Array(n + 2).fill('quiet')
  const fns: [number, number][] = []
  const quiet = (s: import('typescript').Statement) =>
    ts.isInterfaceDeclaration(s) ||
    ts.isTypeAliasDeclaration(s) ||
    (ts.isImportDeclaration(s) && !!s.importClause) ||
    (ts.isExportDeclaration(s) && (s.isTypeOnly || !s.moduleSpecifier)) ||
    !!ts.getModifiers(s as never)?.some((m) => m.kind === ts.SyntaxKind.DeclareKeyword)
  const walk = (node: import('typescript').Node) => {
    if (ts.isFunctionLike(node) && !ts.isFunctionTypeNode(node) && !ts.isMethodSignature(node) && !ts.isCallSignatureDeclaration(node) && !ts.isConstructSignatureDeclaration(node)) {
      const [a, b] = [line(node.getStart(sf)), line(node.end)]
      fns.push([a, b])
      for (let i = a; i <= b; i++) kind[i] = 'fn'
      return
    }
    ts.forEachChild(node, walk)
  }
  for (const s of sf.statements) {
    if (quiet(s)) continue
    for (let i = line(s.getStart(sf)); i <= line(s.end); i++) kind[i] = 'code'
    walk(s)
  }
  return { kind, fns }
}

/** the browser test files that run what changed in these files since the map's commit, or why every one runs */
export async function pick(map: ScreenMap, changed: string[]): Promise<{ tests: Set<string> } | { all: string }> {
  const tests = new Set<string>()
  const ran = (f: string, a: number, b: number) => {
    for (const [t, files] of Object.entries(map.tests)) if (files[f]?.some(([x, y]) => x <= b && y >= a)) tests.add(t)
  }
  for (const f of changed) {
    if (!/\.tsx?$/.test(f)) return { all: `${f} changes the whole screen` }
    // not part of the screen at the map's commit: new, or used by the server alone (what imports it changed too)
    if (!map.sources.includes(f)) continue
    const before = git('show', `${map.commit}:${f}`) ?? ''
    const now = existsSync(join(root, f)) ? readFileSync(join(root, f), 'utf8') : ''
    const [old, cur] = await Promise.all([kinds(before, f.endsWith('x')), kinds(now, f.endsWith('x'))])
    let whole = false
    for (const h of (git('diff', '-U0', '--no-renames', map.commit, '--', f) ?? '').matchAll(/^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/gm)) {
      const [a, n, b, m] = [+h[1], h[2] === undefined ? 1 : +h[2], +h[3], h[4] === undefined ? 1 : +h[4]]
      // lines taken out, as they were
      for (let i = a; i < a + n; i++) {
        if (old.kind[i] === 'fn') ran(f, i, i)
        else if (old.kind[i] === 'code') whole = true
      }
      // lines put in, as they are now: in a function that was there before, or new code at the top level
      for (let i = b; i < b + m; i++) {
        if (cur.kind[i] === 'code') whole = true
        else if (cur.kind[i] === 'fn' && !cur.fns.some(([x, y]) => x <= i && i <= y && x >= b && y < b + m)) ran(f, a, n ? a + n - 1 : a + 1)
      }
    }
    if (whole) {
      const any = Object.entries(map.tests).filter(([, files]) => files[f]?.length)
      if (!any.length) return { all: `${f} changed at its top level, and no test runs its functions` }
      for (const [t] of any) tests.add(t)
    }
  }
  return { tests }
}

// git for tasks: what a fresh worktree starts from and gets copied in, whether its work is in main, commits, rebasing
// on main and merging into it. herdr itself makes and removes task worktrees (worktrees.ts).
import { basename, join } from 'node:path'
import { BOX, run } from '../box.ts'
import { cpSync, existsSync, readFileSync } from '../mfs.ts'

export interface Run {
  ok: boolean
  code: number
  out: string
  err: string
}

/** a command on the task's machine: here, or over SSH on another one (box.ts) */
export async function sh(cmd: string[], cwd: string, opts: { timeoutMs?: number; env?: Record<string, string>; here?: boolean } = {}): Promise<Run> {
  if (BOX && !opts.here) {
    const r = await run(cmd, { cwd, env: { GIT_TERMINAL_PROMPT: '0', ...opts.env }, timeoutMs: opts.timeoutMs })
    return { ok: r.code === 0, code: r.code, out: r.out.trimEnd(), err: r.err.trimEnd() }
  }
  const p = Bun.spawn(cmd, { cwd, stdout: 'pipe', stderr: 'pipe', env: { ...process.env, GIT_TERMINAL_PROMPT: '0', ...opts.env } })
  let timedOut = false
  const t = opts.timeoutMs ? setTimeout(() => ((timedOut = true), p.kill()), opts.timeoutMs) : null
  const [out, err, code] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text(), p.exited])
  if (t) clearTimeout(t)
  return { ok: code === 0 && !timedOut, code: timedOut ? 124 : code, out: out.trimEnd(), err: timedOut ? `${err}\ntimed out` : err.trimEnd() }
}

export const git = (cwd: string, ...args: string[]) => sh(['git', ...args], cwd)

export async function isRepo(path: string) {
  return (await git(path, 'rev-parse', '--show-toplevel')).ok
}

export async function topLevel(path: string) {
  const r = await git(path, 'rev-parse', '--show-toplevel')
  return r.ok ? r.out : null
}

export async function defaultBranch(repo: string) {
  const r = await git(repo, 'symbolic-ref', '--short', 'refs/remotes/origin/HEAD')
  if (r.ok) return r.out.replace(/^origin\//, '')
  for (const b of ['main', 'master']) if ((await git(repo, 'rev-parse', '--verify', '--quiet', `refs/heads/${b}`)).ok) return b
  return (await git(repo, 'branch', '--show-current')).out || 'main'
}

/** Check a named destination before a task gets a worktree. Never change origin/HEAD. */
export async function validateBase(repo: string, base: string) {
  if (!base || base.startsWith('-') || !(await git(repo, 'check-ref-format', `refs/heads/${base}`)).ok)
    throw new Error('Choose a branch name, like main, develop or feature/login.')
  const common = await git(repo, 'rev-parse', '--path-format=absolute', '--git-common-dir')
  if (!common.ok || !(await sh(['test', '-w', common.out], repo)).ok) throw new Error('No permission to write to this repository.')
  const remote = (await git(repo, 'remote', 'get-url', 'origin')).ok
  if (remote) {
    const fetched = await git(repo, 'fetch', '--quiet', 'origin', `+refs/heads/${base}:refs/remotes/origin/${base}`)
    if (!fetched.ok) throw new Error(`Cannot fetch branch ${base} from origin: ${(fetched.err || fetched.out).split('\n')[0]}`)
    const g = await github(repo)
    if (g) {
      const r = await gh(g, repo, 'repo', 'view', `${g.host}/${g.repo}`, '--json', 'viewerPermission', '--jq', '.viewerPermission')
      if (!r.ok || !['WRITE', 'MAINTAIN', 'ADMIN'].includes(r.out.trim()))
        throw new Error(`Cannot confirm permission to merge into ${base} on GitHub. Sign in with repository write access first.`)
    } else {
      const r = await git(repo, 'push', '--dry-run', 'origin', `refs/remotes/origin/${base}:refs/heads/${base}`)
      if (!r.ok) throw new Error(`Cannot confirm permission to push ${base} to origin: ${(r.err || r.out).split('\n')[0]}`)
    }
  } else if (!(await branchExists(repo, base))) throw new Error(`No local branch ${base}.`)
}

/** a GitHub repo as gh reaches it: owner/name, and the host gh is logged in to for it (github.com, github.localhost) */
export interface GitHub {
  repo: string
  host: string
}

/** host and owner/name of a remote URL in any form git takes: https://host/o/r(.git), ssh://git@host/o/r, git@host:o/r */
export function parseRemote(url: string): GitHub | null {
  const m = url.trim().match(/^(?:[a-z+]+:\/\/)?(?:[^@/]+@)?([^/:]+)(?::\d+)?[:/]+([^/]+\/[^/]+?)(?:\.git)?\/?$/i)
  return m && /github/i.test(m[1]) ? { host: m[1].toLowerCase(), repo: m[2] } : null
}

/**
 * The repo's GitHub remote, read live (a repo may gain one after it was added): origin as written, and as git rewrites
 * it (url.insteadOf), so github.com mapped to github.localhost is found either way. gh talks to the host git really
 * reaches when that is a GitHub one, else to the one written.
 */
export async function github(repo: string): Promise<GitHub | null> {
  return (await githubs(repo))[0] ?? null
}

/** each GitHub repo origin may be: as git rewrites it first, then as written */
export async function githubs(repo: string): Promise<GitHub[]> {
  const [written, used] = await Promise.all([git(repo, 'config', '--get', 'remote.origin.url'), git(repo, 'remote', 'get-url', 'origin')])
  const all = [used.ok ? parseRemote(used.out) : null, written.ok ? parseRemote(written.out) : null].filter((g): g is GitHub => !!g)
  return all.filter((g, i) => all.findIndex((x) => x.host === g.host && x.repo === g.repo) === i)
}

export async function githubRemote(repo: string): Promise<string | null> {
  return (await github(repo))?.repo ?? null
}

export function slug(title: string, max = 30) {
  return (
    title
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '')
      .slice(0, max)
      .replace(/-+$/, '') || 'task'
  )
}

/** the commit a task's branch starts from: the remote's base when there is one (fetched first), else the local one.
 *  A commit, not a remote-tracking ref, so no upstream is written into the repo's shared config. */
export async function startPoint(repo: string, base: string) {
  await validateBase(repo, base)
  const from = (await git(repo, 'rev-parse', '--verify', '--quiet', `origin/${base}`)).ok ? `origin/${base}` : base
  if (!(await branchExists(repo, base)) && from !== base) {
    const made = await git(repo, 'branch', '--', base, from)
    if (!made.ok) throw new Error(`Cannot create local branch ${base}: ${made.err}`)
  }
  const r = await git(repo, 'rev-parse', '--verify', '--quiet', `${from}^{commit}`)
  return r.ok ? r.out : from
}

export async function branchExists(repo: string, branch: string) {
  return (await git(repo, 'rev-parse', '--verify', '--quiet', `refs/heads/${branch}`)).ok
}

/** the linked worktree `branch` is checked out in, if any (never the repo's own checkout, listed first) */
export async function worktreeOf(repo: string, branch: string) {
  const r = await git(repo, 'worktree', 'list', '--porcelain')
  const entry = r.out.split('\n\n').slice(1).find((e) => e.split('\n').includes(`branch refs/heads/${branch}`))
  return entry?.split('\n').find((l) => l.startsWith('worktree '))?.slice('worktree '.length)
}

/** files changed or new in the checkout (ignored ones don't count) */
export async function dirty(wt: string) {
  const r = await git(wt, 'status', '--porcelain')
  return !r.ok || !!r.out.trim()
}

/** merging `ref` into commit `into` makes exactly commit `is`'s tree */
async function mergesTo(repo: string, into: string, ref: string, is: string) {
  const m = await git(repo, 'merge-tree', '--write-tree', into, ref)
  const b = await git(repo, 'rev-parse', `${is}^{tree}`)
  return m.ok && b.ok && m.out.split('\n')[0] === b.out
}

/** merging `ref` into `base` goes through without a conflict: what GitHub calls mergeable */
export const mergesCleanly = async (repo: string, base: string, ref: string) => (await git(repo, 'merge-tree', '--write-tree', base, ref)).ok

/** the commits on `base` a task's squash merge made: by Shepherdr's subject, "<id>: <title>" */
export async function squashesOf(repo: string, base: string, id: string) {
  const r = await git(repo, 'log', '--format=%H %s', '-n', '5000', base)
  return r.ok ? r.out.split('\n').filter((l) => l.slice(41).startsWith(`${id}: `)).map((l) => l.slice(0, 40)) : []
}

/**
 * Everything on `ref` is in `base` already: its exact tree is one `base` had (a squash merge of the rebased branch
 * makes that commit; a branch with no work has its starting point's), squash-merging it now onto the parent of one of
 * `squashes` (its task's squash merges into `base`) makes that very commit, so nothing came after the merge (not even
 * a commit undoing part of it), however main has moved on since, or merging it into `base` would change nothing.
 */
export async function landed(repo: string, ref: string, base: string, squashes: string[] = []) {
  const tree = await git(repo, 'rev-parse', '--verify', '--quiet', `${ref}^{tree}`)
  if (!tree.ok) return false
  const trees = await git(repo, 'log', '--format=%T', '-n', '5000', base)
  if (trees.ok && trees.out.split('\n').includes(tree.out)) return true
  for (const s of squashes) {
    if (!(await git(repo, 'merge-base', '--is-ancestor', s, base)).ok) continue
    if (await mergesTo(repo, `${s}^`, ref, s)) return true
  }
  return mergesTo(repo, base, ref, base)
}

/** git's own pattern match is not needed here: `*` matches anything, and a pattern matches a path or its last part */
function globMatch(pattern: string, s: string) {
  const p = pattern.replace(/^\/+|\/+$/g, '')
  const re = new RegExp(`^${p.split('*').map((x) => x.replace(/[.+?^${}()|[\]\\]/g, '\\$&')).join('.*')}$`)
  return re.test(s)
}

/**
 * The repo's setup for a fresh worktree, as herdr-projects does it: ignored files it lacks (.env and the like) are
 * copied in when .worktreeinclude in the repo names them. Nothing else runs.
 */
export async function copyIncludes(repo: string, wt: string) {
  if (!existsSync(join(repo, '.worktreeinclude'))) return []
  const patterns = readFileSync(join(repo, '.worktreeinclude'), 'utf8')
    .split('\n')
    .map((l) => l.trim())
    .filter((l) => l && !l.startsWith('#'))
  if (!patterns.length) return []
  const ignored = await git(repo, 'ls-files', '--others', '--ignored', '--exclude-standard', '--directory')
  const copied: string[] = []
  for (const entry of ignored.out.split('\n').filter(Boolean)) {
    const rel = entry.replace(/\/+$/, '')
    if (!patterns.some((g) => globMatch(g, rel) || globMatch(g, basename(rel)))) continue
    try {
      cpSync(join(repo, rel), join(wt, rel))
      copied.push(rel)
    } catch {
      /* one that can't be copied: the rest still are */
    }
  }
  return copied
}

export async function removeWorktree(repo: string, path: string, force = false) {
  const r = force ? await git(repo, 'worktree', 'remove', '--force', path) : await git(repo, 'worktree', 'remove', path)
  await git(repo, 'worktree', 'prune')
  return r
}

/** commit whatever the agent left uncommitted */
export async function commitAll(wt: string, message: string) {
  const st = await git(wt, 'status', '--porcelain')
  if (!st.out.trim()) return false
  await git(wt, 'add', '-A')
  const r = await sh(['git', '-c', 'user.useConfigOnly=false', 'commit', '-q', '-m', message], wt)
  return r.ok
}

export async function mergeBase(wt: string, base: string) {
  const ref = (await git(wt, 'rev-parse', '--verify', '--quiet', `origin/${base}`)).ok ? `origin/${base}` : base
  const r = await git(wt, 'merge-base', ref, 'HEAD')
  return r.ok ? r.out : ref
}

/** commits were made on `branch` since it was made: its tip is no longer the commit its reflog starts at */
export async function workedOn(wt: string, branch: string) {
  const r = await git(wt, 'reflog', 'show', '--format=%H', `refs/heads/${branch}`)
  const first = r.out.split('\n').filter(Boolean).at(-1)
  const head = await git(wt, 'rev-parse', 'HEAD')
  return r.ok && !!first && head.ok && head.out !== first
}

export async function aheadCount(wt: string, base: string) {
  const mb = await mergeBase(wt, base)
  const r = await git(wt, 'rev-list', '--count', `${mb}..HEAD`)
  return Number(r.out || 0)
}

/** bring the local base up to the remote's when there is one; refused when the two have split */
export async function catchUp(repo: string, base: string): Promise<Run> {
  const ok: Run = { ok: true, code: 0, out: '', err: '' }
  if (!(await git(repo, 'remote', 'get-url', 'origin')).ok) return ok
  // offline: go on with what is here
  if (!(await git(repo, 'fetch', '--quiet', 'origin', base)).ok) return ok
  const remote = `origin/${base}`
  if (!(await git(repo, 'rev-parse', '--verify', '--quiet', remote)).ok) return ok
  if (!(await branchExists(repo, base))) return git(repo, 'branch', '--', base, remote)
  if ((await git(repo, 'merge-base', '--is-ancestor', remote, base)).ok) return ok
  if (!(await git(repo, 'merge-base', '--is-ancestor', base, remote)).ok) return { ...ok, ok: false, err: `${base} here and ${remote} have split; bring them together first` }
  const r = (await git(repo, 'branch', '--show-current')).out === base ? await git(repo, 'merge', '--ff-only', '--quiet', remote) : await git(repo, 'update-ref', `refs/heads/${base}`, remote)
  return r.ok ? r : { ...r, err: `could not bring ${base} up to ${remote}: ${(r.err || r.out).split('\n')[0]}` }
}

/** replay the branch on top of the base; a conflict leaves it as it was */
export async function rebase(wt: string, base: string) {
  const r = await git(wt, 'rebase', base)
  if (!r.ok) await git(wt, 'rebase', '--abort')
  return r
}

/** merge a commit (or branch) into the base on this machine, without disturbing the checkout you work in */
export async function mergeHere(repo: string, branch: string, base: string, message: string, squash: boolean): Promise<Run> {
  const current = (await git(repo, 'branch', '--show-current')).out
  if (current === base) {
    if ((await git(repo, 'status', '--porcelain', '--untracked-files=no')).out.trim()) return { ok: false, code: 1, out: '', err: `${base} has uncommitted changes in ${repo}` }
    const m = squash ? await git(repo, 'merge', '--squash', branch) : await git(repo, 'merge', '--no-ff', '-m', message, branch)
    if (!m.ok) {
      await git(repo, 'merge', '--abort')
      await git(repo, 'reset', '--hard', 'HEAD')
      return { ...m, err: `conflict: ${m.out || m.err}` }
    }
    return squash ? git(repo, 'commit', '-q', '-m', message) : m
  }
  // base is not checked out here: merge in a scratch worktree, then move the branch
  const tmp = join(repo, '.git', `shepherdr-merge-${Date.now()}`)
  const add = await git(repo, 'worktree', 'add', '--detach', tmp, base)
  if (!add.ok) return add
  try {
    const m = squash ? await git(tmp, 'merge', '--squash', branch) : await git(tmp, 'merge', '--no-ff', '-m', message, branch)
    if (!m.ok) return { ...m, err: `conflict: ${m.out || m.err}` }
    if (squash) {
      const c = await git(tmp, 'commit', '-q', '-m', message)
      if (!c.ok) return c
    }
    const head = (await git(tmp, 'rev-parse', 'HEAD')).out
    return git(repo, 'update-ref', `refs/heads/${base}`, head)
  } finally {
    await removeWorktree(repo, tmp, true)
  }
}

/* ---------- GitHub, through gh ---------- */

/** gh for one repo: its host in GH_HOST (gh can't tell github.localhost from the remote alone), never a prompt */
export const gh = (g: GitHub, cwd: string, ...args: string[]) =>
  sh(['gh', ...args], cwd, { timeoutMs: 120_000, env: { GH_HOST: g.host, GH_PROMPT_DISABLED: '1', GH_NO_UPDATE_NOTIFIER: '1', NO_COLOR: '1' } })

/** the repo as gh's --repo takes it, host included */
const R = (g: GitHub) => ['--repo', `${g.host}/${g.repo}`]

/**
 * gh and the repo's host: logged in ("yes"), not logged in there or no gh at all ("no"), or logged in but not answering
 * now ("down": an outage, a proxy away, a lapsed token), which is never taken as "no".
 */
export async function ghLogin(g: GitHub, cwd: string): Promise<{ state: 'yes' | 'no' | 'down'; said?: string }> {
  const r = await gh(g, cwd, 'auth', 'status', '--hostname', g.host).catch(() => null)
  if (!r || r.code === 127) return { state: 'no' }
  if (r.ok) return { state: 'yes' }
  const said = `${r.out}\n${r.err}`
  if (/not logged in/i.test(said)) return { state: 'no' }
  return { state: 'down', said: said.split('\n').map((l) => l.trim().replace(/^[-X✓] /, '')).find((l) => /invalid|fail|error|timeout|could not/i.test(l)) ?? said.trim().split('\n')[0] }
}

export interface Pr {
  number: number
  url: string
  state: 'OPEN' | 'CLOSED' | 'MERGED'
  isDraft: boolean
  /** the commit the pull request is at */
  headRefOid: string
  baseRefName: string
}

const PR_FIELDS = 'number,url,state,isDraft,headRefOid,baseRefName'

/** the branch's pull request: its open one, else one merged already */
export async function findPr(g: GitHub, cwd: string, branch: string): Promise<Pr | null> {
  const r = await gh(g, cwd, 'pr', 'list', ...R(g), '--head', branch, '--state', 'all', '--json', PR_FIELDS, '--limit', '10')
  if (!r.ok) return null
  try {
    const list = JSON.parse(r.out || '[]') as Pr[]
    return list.find((p) => p.state === 'OPEN') ?? list.find((p) => p.state === 'MERGED') ?? null
  } catch {
    return null
  }
}

export async function viewPr(g: GitHub, cwd: string, n: number): Promise<Pr | null> {
  const r = await gh(g, cwd, 'pr', 'view', String(n), ...R(g), '--json', PR_FIELDS)
  try {
    return r.ok ? (JSON.parse(r.out) as Pr) : null
  } catch {
    return null
  }
}

/** open a pull request from the branch (pushed already); its number and link, or gh's words */
export async function openPr(g: GitHub, cwd: string, o: { branch: string; base: string; title: string; body: string; draft: boolean }): Promise<{ number: number; url: string } | { error: string }> {
  const r = await gh(g, cwd, 'pr', 'create', ...R(g), '--head', o.branch, '--base', o.base, '--title', o.title, '--body', o.body, ...(o.draft ? ['--draft'] : []))
  const m = r.out.match(/https?:\/\/\S+\/pull\/(\d+)/)
  return m ? { number: Number(m[1]), url: m[0] } : { error: (r.err || r.out || `gh exited ${r.code}`).split('\n')[0] }
}

export const editPr = (g: GitHub, cwd: string, n: number, body: string) => gh(g, cwd, 'pr', 'edit', String(n), ...R(g), '--body', body)

export const closePr = (g: GitHub, cwd: string, n: number, comment: string) => gh(g, cwd, 'pr', 'close', String(n), ...R(g), '--comment', comment)

export const readyPr = (g: GitHub, cwd: string, n: number) => gh(g, cwd, 'pr', 'ready', String(n), ...R(g))

export type PrChecks = { state: 'pending' | 'pass' | 'fail' | 'none' | 'error'; passed: number; failed: { name: string; link?: string }[]; error?: string }

/** a check as gh pr checks buckets it: pass, fail, pending, skipping or cancel */
export type Check = { name: string; bucket: string; link?: string }

/** gh was refused GitHub's checks: a fine-grained token can't read them on a private repo (it can read Actions) */
export const noChecksAccess = (said: string) => /not accessible by personal access token/i.test(said)

/**
 * The jobs GitHub Actions ran on a commit, read through the Actions API, for a token that may read Actions but not
 * checks; null when GitHub did not answer.
 */
export async function actionJobs(g: GitHub, cwd: string, sha: string): Promise<Check[] | null> {
  const runs = await gh(g, cwd, 'api', '--paginate', '--jq', '.workflow_runs[].id', `repos/${g.repo}/actions/runs?head_sha=${sha}&per_page=100`)
  if (!runs.ok) return null
  const jobs = await Promise.all(
    runs.out
      .split('\n')
      .filter(Boolean)
      .map((id) => gh(g, cwd, 'api', '--paginate', '--jq', '.jobs[] | {name, status, conclusion, html_url}', `repos/${g.repo}/actions/runs/${id}/jobs?per_page=100`)),
  )
  if (jobs.some((r) => !r.ok)) return null
  try {
    return jobs
      .flatMap((r) => r.out.split('\n').filter(Boolean))
      .map((l) => JSON.parse(l) as { name: string; status: string; conclusion: string | null; html_url: string })
      .map((j) => ({ name: j.name, link: j.html_url, bucket: j.status !== 'completed' ? 'pending' : (JOB[j.conclusion ?? ''] ?? 'fail') }))
  } catch {
    return null
  }
}
const JOB: Record<string, string> = { success: 'pass', neutral: 'pass', skipped: 'skipping', cancelled: 'cancel' }

/** GitHub's checks on a pull request. "none" only when GitHub answered with no checks; anything unread is "error". */
export async function prChecks(g: GitHub, cwd: string, n: number): Promise<PrChecks> {
  const r = await gh(g, cwd, 'pr', 'checks', String(n), ...R(g), '--json', 'name,bucket,link')
  const out = r.out.trim()
  let list: Check[] | null = null
  if (!out.startsWith('[')) {
    // gh says "no checks reported" (and exits 1) when a branch simply has none
    if (/no checks reported/i.test(`${r.out}\n${r.err}`)) return { state: 'none', passed: 0, failed: [] }
    const pr = noChecksAccess(`${r.out}\n${r.err}`) ? await viewPr(g, cwd, n) : null
    list = pr && (await actionJobs(g, cwd, pr.headRefOid))
    if (!list) return { state: 'error', passed: 0, failed: [], error: (r.err || r.out || `gh exited ${r.code}`).split('\n')[0] }
  }
  try {
    list ??= JSON.parse(out) as Check[]
  } catch {
    return { state: 'error', passed: 0, failed: [], error: 'GitHub answered with something unreadable' }
  }
  if (!list.length) return { state: 'none', passed: 0, failed: [] }
  const failed = list.filter((c) => c.bucket === 'fail' || c.bucket === 'cancel').map((c) => ({ name: c.name, link: c.link || undefined }))
  const pending = list.some((c) => c.bucket === 'pending')
  const passed = list.filter((c) => c.bucket === 'pass').length
  return { state: failed.length ? 'fail' : pending ? 'pending' : 'pass', passed, failed }
}

/** how long GitHub's checks on pull requests take (ms): the middle of the last few runs that passed; none when unknown */
export async function checksTook(g: GitHub, cwd: string): Promise<number | undefined> {
  const r = await gh(g, cwd, 'api', '--jq', '.workflow_runs[] | [.run_started_at, .updated_at]', `repos/${g.repo}/actions/runs?event=pull_request&status=success&per_page=5`)
  if (!r.ok) return undefined
  const took = r.out
    .split('\n')
    .map((l) => {
      try {
        const [a, b] = JSON.parse(l) as [string, string]
        return Date.parse(b) - Date.parse(a)
      } catch {
        return Number.NaN
      }
    })
    .filter((ms) => ms > 0)
    .sort((a, b) => a - b)
  return took.length ? took[Math.floor(took.length / 2)] : undefined
}

/** GitHub squash-merges the pull request only while its head is still `sha`, the commit this machine has */
export const mergePr = (g: GitHub, cwd: string, n: number, sha: string, subject: string, body: string) =>
  gh(g, cwd, 'pr', 'merge', String(n), ...R(g), '--squash', '--match-head-commit', sha, '--subject', subject, '--body', body)

/** the repo has GitHub Actions workflows at this commit that run on pull requests: a pull request of it gets checks,
 *  even before they show */
export async function hasPrWorkflows(wt: string) {
  const r = await git(wt, 'ls-tree', '--name-only', 'HEAD', '.github/workflows/')
  if (!r.ok) return false
  for (const f of r.out.split('\n').filter((f) => /\.ya?ml$/.test(f))) {
    const y = await git(wt, 'show', `HEAD:${f}`)
    // on: pull_request, on: [push, pull_request], or a pull_request: key under on:
    if (y.ok && /^\s*(?:on:.*\bpull_request|-?\s*pull_request(?:_target)?\b)/m.test(y.out)) return true
  }
  return false
}

/** a commit's runs of GitHub Actions on a branch, as one verdict: all passed, one failed, or some still run */
export type BranchRun = { sha: string; state: 'pass' | 'fail' | 'pending'; failed: { name: string; link: string }[] }

/**
 * The verdicts of GitHub Actions' runs on pushes to a branch, its newest commit first; null when GitHub did not
 * answer. A run cancelled (a newer push took its place) counts for nothing: a commit with only those is left out.
 */
export async function branchRuns(g: GitHub, cwd: string, branch: string, limit = 30): Promise<BranchRun[] | null> {
  const r = await gh(g, cwd, 'run', 'list', ...R(g), '--branch', branch, '--event', 'push', '--limit', String(limit), '--json', 'headSha,status,conclusion,url,workflowName')
  if (!r.ok) return null
  let runs: { headSha: string; status: string; conclusion: string; url: string; workflowName: string }[]
  try {
    runs = JSON.parse(r.out || '[]')
  } catch {
    return null
  }
  const out: BranchRun[] = []
  for (const sha of [...new Set(runs.map((x) => x.headSha))]) {
    const of = runs.filter((x) => x.headSha === sha && x.conclusion !== 'cancelled')
    if (!of.length) continue
    const failed = of.filter((x) => x.status === 'completed' && !['success', 'neutral', 'skipped'].includes(x.conclusion)).map((x) => ({ name: x.workflowName, link: x.url }))
    out.push({ sha, failed, state: failed.length ? 'fail' : of.some((x) => x.status !== 'completed') ? 'pending' : 'pass' })
  }
  return out
}

/** what someone wrote on a pull request: a review, a comment on a line of its code, or one on the pull request itself */
export interface PrComment {
  /** unique across the three kinds: r123 (review), c123 (line), i123 (issue comment) */
  id: string
  /** when GitHub has it, ISO */
  at: string
  who: string
  body: string
  url: string
  /** a line comment's file and line ("server/x.ts:42") */
  where?: string
  /** a review's verdict: APPROVED, CHANGES_REQUESTED, COMMENTED */
  state?: string
}

/** each item of a GitHub list, one JSON per line, every page; null when GitHub did not answer */
async function items<T>(g: GitHub, cwd: string, path: string): Promise<T[] | null> {
  const r = await gh(g, cwd, 'api', '--paginate', '--jq', '.[]', path)
  if (!r.ok) return null
  try {
    return r.out.split('\n').filter((l) => l.trim()).map((l) => JSON.parse(l) as T)
  } catch {
    return null
  }
}

type Raw = { id: number; pull_request_review_id?: number | null; user?: { login?: string }; body?: string; html_url: string; created_at?: string; submitted_at?: string; state?: string; path?: string; line?: number | null; original_line?: number | null }

/** everything written on a pull request (reviews, line comments, comments), oldest first; null when unread */
export async function prComments(g: GitHub, cwd: string, n: number): Promise<PrComment[] | null> {
  const repo = `repos/${g.repo}`
  const [reviews, lines, notes] = await Promise.all([
    items<Raw>(g, cwd, `${repo}/pulls/${n}/reviews?per_page=100`),
    items<Raw>(g, cwd, `${repo}/pulls/${n}/comments?per_page=100`),
    items<Raw>(g, cwd, `${repo}/issues/${n}/comments?per_page=100`),
  ])
  if (!reviews || !lines || !notes) return null
  const of = (c: Raw, kind: string): PrComment => ({ id: `${kind}${c.id}`, at: c.submitted_at ?? c.created_at ?? '', who: c.user?.login ?? 'someone', body: (c.body ?? '').trim(), url: c.html_url })
  return [
    ...reviews.filter((c) => c.state !== 'PENDING').map((c) => ({ ...of(c, 'r'), state: c.state })),
    // a line comment is stamped when it was drafted: it is news when its review was sent
    ...lines.map((c) => ({ ...of(c, 'c'), at: reviews.find((r) => r.id === c.pull_request_review_id)?.submitted_at ?? c.created_at ?? '', where: c.path ? `${c.path}${(c.line ?? c.original_line) ? `:${c.line ?? c.original_line}` : ''}` : undefined })),
    ...notes.map((c) => of(c, 'i')),
  ].sort((a, b) => a.at.localeCompare(b.at))
}

/** the last lines a failed check's job logged before its last error (after it come clean-up steps), from its link
 *  (…/actions/runs/1/job/2, or a run's: its first failed job); nothing when gh can't tell. Read through the API, as
 *  gh run view's own calls skip a GitHub proxy (GH_HOST) for the jobs. */
export async function failedLog(g: GitHub, cwd: string, link: string, lines = 15): Promise<string | undefined> {
  const run = link.match(/\/actions\/runs\/(\d+)/)?.[1]
  let job = link.match(/\/actions\/runs\/\d+\/job\/(\d+)/)?.[1]
  if (!job && run) {
    const jobs = await gh(g, cwd, 'api', '--paginate', '--jq', '.jobs[] | select(.conclusion == "failure") | .id', `repos/${g.repo}/actions/runs/${run}/jobs?per_page=100`)
    job = jobs.ok ? jobs.out.split('\n').find(Boolean) : undefined
  }
  if (!job) return undefined
  // a log holds colours, which gh passes on only when told to (and a gh older than that flag goes without it)
  let r = await gh(g, cwd, 'api', '--allow-escape-sequences', `repos/${g.repo}/actions/jobs/${job}/logs`)
  if (!r.ok && /unknown flag/i.test(r.err)) r = await gh(g, cwd, 'api', `repos/${g.repo}/actions/jobs/${job}/logs`)
  if (!r.ok || !r.out.trim()) return undefined
  // each line is "time message" (gh run view's: "job<TAB>step<TAB>time message"): the message is what matters
  const said = r.out
    .split('\n')
    .map((l) => l.split('\t').at(-1)!.replace(/^\d{4}-\d\d-\d\dT[\d:.]+Z ?/, '').replace(/\x1b\[[\d;]*m/g, '').trimEnd())
    .filter((l) => l.trim())
  const err = said.findLastIndex((l) => l.includes('##[error]'))
  const tail = said.slice(0, err < 0 ? undefined : err + 1).slice(-lines).join('\n')
  return tail.length > 1500 ? `…${tail.slice(-1500)}` : tail
}

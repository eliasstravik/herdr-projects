// A stand-in for gh in tests: GitHub is a bare repo on disk (FAKE_GH_STATE's "origin"), its pull requests and checks
// live in that JSON file, and every call is written down there with the GH_HOST it came with. Only what the engine asks
// of gh is here: auth status, pr list/view/create/edit/ready/checks/merge, run list, and api (a pull request's reviews and
// comments, Actions' runs and jobs, a job's log).
import { execFileSync } from 'node:child_process'
import { mkdtempSync, openSync, readFileSync, renameSync, rmSync, unlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

export interface FakePr {
  number: number
  head: string
  base: string
  title: string
  body: string
  isDraft: boolean
  state: 'OPEN' | 'CLOSED' | 'MERGED'
}

export interface FakeGitHub {
  origin: string
  /** owner/name the URLs say */
  repo: string
  permission?: string
  authed?: boolean
  /** logged in, but GitHub doesn't answer (a proxy away): auth status fails as gh's does then */
  down?: boolean
  /** gh pr merge refuses with these words */
  mergeError?: string
  /** gh pr merge takes this long (ms) before it merges: GitHub slow, or stuck */
  mergeSlow?: number
  prs: FakePr[]
  /** what gh pr checks answers; none: "no checks reported" */
  checks?: { name: string; bucket: string; link?: string }[]
  /** the token can't read checks (a fine-grained one on a private repo): gh pr checks is refused */
  noChecks?: boolean
  /** what gh api answers for a path ("repos/acme/app/pulls/1/reviews"), without its query: each item on a line, as
   *  its --jq would pick them */
  api?: Record<string, unknown[]>
  /** what gh run list answers: Actions' runs, newest first */
  runs?: { headSha: string; status: string; conclusion: string; url: string; workflowName: string }[]
  /** a job's log, by job id (gh api repos/…/actions/jobs/<id>/logs) */
  logs?: Record<string, string>
  calls: { host?: string; args: string[] }[]
}

if (import.meta.main) {
  const file = process.env.FAKE_GH_STATE!
  // slow before it holds the state file, so the other calls meanwhile go on
  if (process.argv[2] === 'pr' && process.argv[3] === 'merge') Bun.sleepSync((JSON.parse(readFileSync(file, 'utf8')) as FakeGitHub).mergeSlow ?? 0)
  // the engine runs several gh at once (a pull request's reviews and comments): each holds the state file to itself
  // from reading it to writing it back, or one reads it half-written, or writes over what another wrote down
  const lock = `${file}.lock`
  for (let tries = 0; ; tries++) {
    try {
      openSync(lock, 'wx')
      break
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code !== 'EEXIST' || tries > 3000) throw e
      Bun.sleepSync(5)
    }
  }
  process.on('exit', () => unlinkSync(lock))
  const st = JSON.parse(readFileSync(file, 'utf8')) as FakeGitHub
  const args = process.argv.slice(2)
  st.calls.push({ host: process.env.GH_HOST, args })
  const save = () => {
    writeFileSync(`${file}.tmp`, JSON.stringify(st, null, 2))
    renameSync(`${file}.tmp`, file)
  }
  const opt = (name: string) => {
    const i = args.indexOf(`--${name}`)
    return i >= 0 ? args[i + 1] : undefined
  }
  const git = (...a: string[]) => execFileSync('git', a, { cwd: st.origin, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim()
  const headOf = (branch: string) => {
    try {
      return git('rev-parse', `refs/heads/${branch}`)
    } catch {
      return ''
    }
  }
  const url = (n: number) => `https://github.com/${st.repo}/pull/${n}`
  const json = (p: FakePr) => ({ number: p.number, url: url(p.number), state: p.state, isDraft: p.isDraft, headRefOid: p.state === 'OPEN' ? headOf(p.head) : (p as FakePr & { at?: string }).at, title: p.title, body: p.body, baseRefName: p.base })
  const out = (s: string, code = 0) => {
    save()
    process.stdout.write(`${s}\n`)
    process.exit(code)
  }
  const fail = (s: string) => {
    save()
    process.stderr.write(`${s}\n`)
    process.exit(1)
  }
  const [a0, a1, a2] = args
  if (a0 === 'auth' && a1 === 'status') {
    if (st.authed === false) fail(`You are not logged into any accounts on ${opt('hostname')}`)
    if (st.down) fail(`${opt('hostname')}\n  X Failed to log in to ${opt('hostname')} account t (hosts.yml)\n  - The token in hosts.yml is invalid.`)
    out('Logged in')
  }
  if (a0 === 'repo' && a1 === 'view') {
    if (st.authed === false || st.down) fail('Cannot read repository permissions')
    out(st.permission ?? 'WRITE')
  }
  if (a0 === 'api') {
    const path = args.at(-1)!.split('?')[0]
    const job = path.match(/\/actions\/jobs\/(\d+)\/logs$/)?.[1]
    if (job) out(st.logs?.[job] ?? fail('gh: Not Found (HTTP 404)')!)
    // --paginate --jq '.[]': one item per line
    out((st.api?.[path] ?? []).map((x) => JSON.stringify(x)).join('\n'))
  }
  if (a0 === 'run' && a1 === 'list') out(JSON.stringify(st.runs ?? []))
  if (a0 !== 'pr') fail(`fake gh: no ${a0}`)
  const pr = () => st.prs.find((p) => p.number === Number(a2)) ?? fail(`no pull requests found for ${a2}`)!
  switch (a1) {
    case 'list':
      out(JSON.stringify(st.prs.filter((p) => p.head === opt('head')).map(json)))
      break
    case 'view':
      out(JSON.stringify(json(pr())))
      break
    case 'create': {
      const head = opt('head')!
      if (!headOf(head)) fail(`pull request create failed: Head sha can't be blank, No commits between ${opt('base')} and ${head}`)
      const n = st.prs.length + 1
      st.prs.push({ number: n, head, base: opt('base')!, title: opt('title')!, body: opt('body') ?? '', isDraft: args.includes('--draft'), state: 'OPEN' })
      out(url(n))
      break
    }
    case 'edit':
      pr().body = opt('body') ?? pr().body
      out(url(pr().number))
      break
    case 'close':
      pr().state = 'CLOSED'
      out(`✓ Closed pull request #${a2}`)
      break
    case 'ready':
      pr().isDraft = false
      out(`✓ Pull request #${a2} is marked as "ready for review"`)
      break
    case 'checks':
      if (st.noChecks) fail('GraphQL: Resource not accessible by personal access token (repository.pullRequest.statusCheckRollup.nodes.0.commit.statusCheckRollup.contexts.nodes.0)')
      if (!st.checks?.length) fail(`no checks reported on the '${pr().head}' branch`)
      out(JSON.stringify(st.checks), st.checks!.some((c) => c.bucket === 'fail') ? 1 : st.checks!.some((c) => c.bucket === 'pending') ? 8 : 0)
      break
    case 'merge': {
      const p = pr()
      if (p.state !== 'OPEN') fail(`Pull request #${p.number} is not open`)
      if (p.isDraft) fail(`Pull request #${p.number} is still a draft`)
      if (st.mergeError) fail(st.mergeError)
      const at = headOf(p.head)
      if (opt('match-head-commit') && opt('match-head-commit') !== at) fail('Head branch was modified. Review and try the merge again.')
      // GitHub squashes the branch onto its base
      const tmp = mkdtempSync(join(tmpdir(), 'fake-gh-merge-'))
      try {
        const g = (...a: string[]) => execFileSync('git', a, { cwd: tmp, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] })
        g('clone', '-q', '-b', p.base, st.origin, '.')
        g('fetch', '-q', 'origin', p.head)
        try {
          g('merge', '--squash', 'FETCH_HEAD')
        } catch {
          fail(`Pull request #${p.number} is not mergeable: the merge commit cannot be cleanly created.`)
        }
        g('-c', 'user.email=gh@t', '-c', 'user.name=GitHub', 'commit', '-q', '-m', `${opt('subject') ?? p.title}\n\n${opt('body') ?? ''}`.trim())
        g('push', '-q', 'origin', `HEAD:refs/heads/${p.base}`)
      } finally {
        rmSync(tmp, { recursive: true, force: true })
      }
      Object.assign(p, { state: 'MERGED', at })
      out(`✓ Squashed and merged pull request #${p.number}`)
      break
    }
  }
  fail(`fake gh: no pr ${a1}`)
}

// shepherdr merge on a real git repo (engine-world.ts): it squash-merges into main and cleans up, here (rebased) or
// through a pull request on GitHub (a stand-in; rebased only on a conflict).
import { afterEach, describe, expect, setDefaultTimeout, test } from 'bun:test'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import { join } from 'node:path'
import { sh, repo, world, until, started, G, Settings } from './engine-world.ts'
import type { FakeGitHub } from './fake-gh.ts'

// each test runs git dozens of times and gh (a bun process of its own) up to fifty: a few seconds alone, past 20 on a
// machine as busy as a CI runner can get
setDefaultTimeout(60_000)

describe('shepherdr merge', () => {
  test('rebases on main, squash-merges into main, marks it merged and cleans up once the turn ends', async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'merges')
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    sh('git add -A && git commit -qm "add hello"', t.worktree!)
    // main moved on since the task started
    writeFileSync(join(dir, 'other.txt'), 'other\n')
    sh('git add -A && git commit -qm other', dir)
    await w.engine.done(t.id, 'Added hello.txt.')
    // the agent itself merges, mid-turn
    const r = await w.engine.merge(t.id, { self: true })
    expect(r).toEqual({ ok: true, note: undefined })
    expect(sh('git log --format=%s -n 3', dir).split('\n')).toEqual([`${t.id}: Say hello`, 'other', 'init'])
    expect(sh('git log -1 --format=%b', dir)).toBe('Added hello.txt.')
    expect(existsSync(join(dir, 'hello.txt'))).toBe(true)
    // its squash commit is recorded: what its branch is checked against when it is put away
    expect(t.squash).toBe(sh('git rev-parse main', dir))
    expect(w.engine.statusOf(t)).toBe('done')
    expect(w.engine.view(t).agent?.status).toBe('done')
    // its row says a few words; the summary is its card's
    expect(w.engine.subOf(t)).toBe('Merged')
    expect(w.engine.view(t).said).toBe('Added hello.txt.')
    // it keeps when it finished: the Done group's order and the row's age
    expect(Date.now() - t.doneAt!).toBeLessThan(5000)
    expect(w.engine.view(t)).toMatchObject({ doneAt: t.doneAt, age: 'now' })
    // its worktree stays while its turn runs, and goes once it ends
    expect(existsSync(t.worktree!)).toBe(true)
    w.agentIs(key, 'idle')
    // the branch goes last
    await until(() => !!t.cleaned)
    expect(existsSync(t.worktree!)).toBe(false)
    expect(w.records[key].finished).toBe(true)
    expect(sh('git branch --list "work/*"', dir)).toBe('')
    expect(w.news).toContain(`${t.id} was merged`)
    expect((await w.engine.merge(t.id)).ok).toBe(false)
    // one Done card for its project's chat, however often it settles again
    w.agentIs(key, 'idle')
    expect(w.cards.filter((c) => c.status === 'done')).toEqual([{ task: t.id, status: 'done', sub: 'Merged' }])
  })

  test('merged by you while its agent waits: cleaned up at once', async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'byyou')
    writeFileSync(join(t.worktree!, 'x.txt'), 'x\n')
    // left uncommitted: the merge commits it
    w.agentIs(key, 'idle')
    expect((await w.engine.merge(t.id)).ok).toBe(true)
    expect(existsSync(join(dir, 'x.txt'))).toBe(true)
    expect(existsSync(t.worktree!)).toBe(false)
  })

  test('a conflict with main stops it with what to do, and main stays as it was', async () => {
    const w = world()
    const dir = repo()
    const { t } = await started(w, dir, 'conflicts')
    writeFileSync(join(t.worktree!, 'README.md'), 'theirs\n')
    sh('git commit -qam theirs', t.worktree!)
    writeFileSync(join(dir, 'README.md'), 'mine\n')
    sh('git commit -qam mine', dir)
    const before = sh('git rev-parse HEAD', dir)
    const r = await w.engine.merge(t.id, { self: true })
    expect(r.ok).toBe(false)
    expect(r.error).toContain('Conflict with main')
    expect(r.error).toContain(`shepherdr merge ${t.id} again`)
    expect(sh('git rev-parse HEAD', dir)).toBe(before)
    expect(sh('git status --porcelain', t.worktree!)).toBe('')
    expect(t.merged).toBeFalsy()
  })

  test('a late message it has not read yet stops the merge until it has', async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'late')
    writeFileSync(join(t.worktree!, 'late.txt'), 'late\n')
    sh('git add -A && git commit -qm late', t.worktree!)
    // you wrote while it finished; it hasn't taken your words in yet
    w.chat.unseen = ['Also rename the button']
    const r = await w.engine.merge(t.id, { self: true })
    expect(r).toEqual({ ok: false, error: 'You have a new message in your chat: read and handle it, then merge again.' })
    expect(existsSync(join(dir, 'late.txt'))).toBe(false)
    expect(t.merged).toBeFalsy()
    w.agentIs(key, 'idle')
    expect((await w.engine.merge(t.id)).error).toBe(`${t.id}'s agent has a new message it hasn't read yet: let it handle that first.`)
    // it read it: the merge goes in
    w.chat.unseen = []
    expect((await w.engine.merge(t.id, { self: true })).ok).toBe(true)
    expect(existsSync(join(dir, 'late.txt'))).toBe(true)
  })

  test('anyone else waits for the agent to end its turn: no half-done work goes in', async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'busy')
    writeFileSync(join(t.worktree!, 'half.txt'), 'half\n')
    const r = await w.engine.merge(t.id)
    expect(r.ok).toBe(false)
    expect(r.error).toContain('is working')
    expect(existsSync(join(dir, 'half.txt'))).toBe(false)
    w.agentIs(key, 'idle')
    expect((await w.engine.merge(t.id)).ok).toBe(true)
  })

  test('two merges into one repo at once both go in, one after the other', async () => {
    const w = world()
    const dir = repo()
    const a = await started(w, dir, 'two', 'First', 'a')
    const b = w.engine.add({ project: a.p.name, title: 'Second', by: 'test' })
    await until(() => w.started.some((s) => s.key === `${b.id}/Agent`))
    writeFileSync(join(a.t.worktree!, 'a.txt'), 'a\n')
    writeFileSync(join(b.worktree!, 'b.txt'), 'b\n')
    const [ra, rb] = await Promise.all([w.engine.merge(a.t.id, { self: true }), w.engine.merge(b.id, { self: true })])
    expect([ra.ok, rb.ok]).toEqual([true, true])
    expect(existsSync(join(dir, 'a.txt')) && existsSync(join(dir, 'b.txt'))).toBe(true)
    expect(sh('git status --porcelain', dir)).toBe('')
  })

  test('with a remote, main here first catches up with it, so the merge sits on top of what is there', async () => {
    const w = world()
    const origin = repo()
    const dir = mkdtempSync(join(tmpdir(), 'shep-clone-'))
    sh(`git clone -q ${origin} . && git config user.email t@t && git config user.name T`, dir)
    const { t } = await started(w, dir, 'remote')
    // someone else's work lands on the remote; main here never pulled it
    writeFileSync(join(origin, 'theirs.txt'), 'theirs\n')
    sh('git add -A && git commit -qm theirs', origin)
    writeFileSync(join(t.worktree!, 'mine.txt'), 'mine\n')
    w.agentIs(`${t.id}/Agent`, 'idle')
    expect((await w.engine.merge(t.id)).ok).toBe(true)
    expect(sh('git log --format=%s -n 3', dir).split('\n')).toEqual([`${t.id}: Say hello`, 'theirs', 'init'])
    // the task's commit holds only its own change
    expect(sh('git show --name-only --format= HEAD', dir)).toBe('mine.txt')
  })

  test('a branch already merged by hand is only marked merged', async () => {
    const w = world()
    const dir = repo()
    const { t } = await started(w, dir, 'byhand')
    writeFileSync(join(t.worktree!, 'y.txt'), 'y\n')
    sh('git add -A && git commit -qm y', t.worktree!)
    sh(`git merge -q --squash ${t.branch} && git commit -qm "by hand"`, dir)
    const r = await w.engine.merge(t.id, { self: true })
    expect(r).toEqual({ ok: true, note: 'it was in main already' })
    expect(sh('git log --format=%s -n 1', dir)).toBe('by hand')
    expect(w.engine.statusOf(t)).toBe('done')
  })

  for (const how of ['squashed', 'with a merge commit'])
    test(`its pull request merged on GitHub ${how}, and main moved on since: only marked merged`, async () => {
      const w = world()
      const dir = repo()
      const { t } = await started(w, dir, `pr-${how.split(' ').at(-1)}`)
      // two commits, the second changing the first (a review fix)
      writeFileSync(join(t.worktree!, 'p.txt'), 'p\n')
      sh('git add -A && git commit -qm p', t.worktree!)
      writeFileSync(join(t.worktree!, 'p.txt'), 'p, fixed\n')
      sh('git add -A && git commit -qm fix', t.worktree!)
      sh(how === 'squashed' ? `git merge -q --squash ${t.branch} && git commit -qm "PR #1"` : `git merge -q --no-ff -m "PR #1" ${t.branch}`, dir)
      writeFileSync(join(dir, 'later.txt'), 'later\n')
      sh('git add -A && git commit -qm later', dir)
      const r = await w.engine.merge(t.id, { self: true })
      expect(r).toEqual({ ok: true, note: 'it was in main already' })
      expect(sh('git log --format=%s -n 2', dir).split('\n')).toEqual(['later', 'PR #1'])
      expect(w.engine.statusOf(t)).toBe('done')
    })

  test('nothing to merge is refused', async () => {
    const w = world()
    const { t } = await started(w, repo(), 'empty')
    const r = await w.engine.merge(t.id, { self: true })
    expect(r.ok).toBe(false)
    expect(r.error).toContain('no changes')
  })
})

describe('closed with nothing to merge', () => {
  const branches = (dir: string) => sh('git branch --list "work/*" --format="%(refname:short)"', dir)

  test('done on a branch with no changes goes straight to Done, closed, and is put away once its turn ends', async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'answer', 'Answer a question', 'Find out why. Report done.')
    expect(await w.engine.done(t.id, 'It is the clock.')).toBe('closed')
    expect(t.closed).toBe(true)
    expect(t.merged).toBeFalsy()
    expect(w.engine.statusOf(t)).toBe('done')
    expect(w.engine.subOf(t)).toBe('Nothing to merge')
    expect(w.engine.view(t).said).toBe('It is the clock.')
    expect(w.engine.view(t).state).toStartWith('Done · nothing to merge')
    expect(w.news).toContain(`${t.id} is done, nothing to merge: It is the clock.`)
    // mid-turn: its worktree stays until the turn ends
    expect(existsSync(t.worktree!)).toBe(true)
    w.agentIs(key, 'idle')
    await until(() => !!t.cleaned)
    expect(existsSync(t.worktree!)).toBe(false)
    expect(branches(dir)).toBe('')
    expect(w.records[key].finished).toBe(true)
    w.agentIs(key, 'idle')
    expect(w.cards.filter((c) => c.status === 'done')).toEqual([{ task: t.id, status: 'done', sub: 'Nothing to merge' }])
    expect(sh('git log --format=%s', dir)).toBe('init')
    const r = await w.engine.merge(t.id, { self: true })
    expect(r.error).toContain('it had nothing to merge')
    expect(await w.engine.done(t.id, 'again')).toBe('ended')
    expect(() => w.engine.todo(t.id)).toThrow('Done already (nothing to merge)')
  })

  test('done with changes, committed or not, is Ready as before', async () => {
    const w = world()
    const { t } = await started(w, repo(), 'readyagain')
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    expect(await w.engine.done(t.id, 'Added hello.txt.')).toBe('ready')
    sh('git add -A && git commit -qm hello', t.worktree!)
    expect(await w.engine.done(t.id, 'Added hello.txt.')).toBe('ready')
    expect(t.closed).toBeFalsy()
  })

  test('task close stops its agent, puts its worktree and branch away and marks it Done, Nothing to merge by default', async () => {
    const w = world()
    const dir = repo()
    const { t, key } = await started(w, dir, 'closes')
    // its agent is at work
    expect(await w.engine.close(t.id)).toEqual({ kept: [] })
    expect(w.records[key].finished).toBe(true)
    expect(w.engine.statusOf(t)).toBe('done')
    expect(w.engine.subOf(t)).toBe('Nothing to merge')
    // closed is when it finished, as merged is
    expect(w.engine.view(t)).toMatchObject({ doneAt: t.doneAt, age: 'now' })
    expect(existsSync(t.worktree!)).toBe(false)
    expect(branches(dir)).toBe('')
    expect(w.herdr.spaces.some((x) => x.label === `${t.id} Say hello`)).toBe(false)
    expect(w.news.some((n) => n.startsWith(`${t.id} is done, nothing to merge`))).toBe(true)
    expect(w.cards.filter((c) => c.status === 'done')).toEqual([{ task: t.id, status: 'done', sub: 'Nothing to merge' }])
    await expect(w.engine.close(t.id)).rejects.toThrow('Done already (nothing to merge)')
  })

  test('task close keeps unmerged work and says so; the sweep leaves it be', async () => {
    const w = world()
    const dir = repo()
    const { t } = await started(w, dir, 'closekeeps')
    writeFileSync(join(t.worktree!, 'wip.txt'), 'wip\n')
    sh('git add -A && git commit -qm wip', t.worktree!)
    const { kept } = await w.engine.close(t.id, 'Not needed any more.')
    expect(kept).toEqual([`${t.worktree!.replace(homedir(), '~')} (work on ${t.branch} that main lacks)`])
    expect(w.engine.view(t).said).toBe('Not needed any more.')
    expect(existsSync(join(t.worktree!, 'wip.txt'))).toBe(true)
    expect(branches(dir)).toBe(t.branch!)
    expect(w.engine.keptLeftovers()[0]).toStartWith(`${t.id} `)
    await w.engine.sweep()
    expect(existsSync(t.worktree!)).toBe(true)
  })

  test('a Ready task closed without a summary keeps what it said when done', async () => {
    const w = world()
    const { t, key } = await started(w, repo(), 'closeready')
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    expect(await w.engine.done(t.id, 'Drafted it; not wanted after all.')).toBe('ready')
    w.agentIs(key, 'idle')
    const { kept } = await w.engine.close(t.id)
    expect(kept[0]).toEndWith('(uncommitted changes)')
    expect(w.engine.view(t).said).toBe('Drafted it; not wanted after all.')
  })

  test('a stopped task stuck in To do, its worktree already gone, closes', async () => {
    const w = world()
    const dir = repo()
    const { t } = await started(w, dir, 'stuck', 'Look into it', "Don't start yet, wait for a go.")
    w.engine.todo(t.id)
    await w.engine.stopTask(t.id)
    expect(w.engine.statusOf(t)).toBe('todo')
    expect(await w.engine.close(t.id, 'Answered elsewhere.')).toEqual({ kept: [] })
    expect(w.engine.statusOf(t)).toBe('done')
    expect(t.cleaned).toBe(true)
  })
})

/** a GitHub stand-in: a bare repo is GitHub, gh is test/fake-gh.ts; the clone's origin reads as GitHub's */
const ghBin = mkdtempSync(join(tmpdir(), 'shep-ghbin-'))
writeFileSync(join(ghBin, 'gh'), `#!/bin/sh\nexec "${process.execPath}" "${join(import.meta.dir, 'fake-gh.ts')}" "$@"\n`, { mode: 0o755 })

function onGitHub(o: { authed?: boolean; down?: boolean; checks?: FakeGitHub['checks']; remote?: string } = {}) {
  process.env.PATH = `${ghBin}:${process.env.PATH}`
  const bare = mkdtempSync(join(tmpdir(), 'shep-gh-origin-'))
  sh('git init -q --bare -b main', bare)
  sh(`git push -q ${bare} main`, repo())
  const dir = mkdtempSync(join(tmpdir(), 'shep-gh-clone-'))
  const remote = o.remote ?? 'https://github.com/acme/app.git'
  sh(`git clone -q ${bare} . && git config user.email t@t && git config user.name T && git remote set-url origin ${remote} && git config url.${bare}.insteadOf ${remote}`, dir)
  const file = join(mkdtempSync(join(tmpdir(), 'shep-gh-state-')), 'github.json')
  writeFileSync(file, JSON.stringify({ origin: bare, repo: 'acme/app', prs: [], calls: [], authed: o.authed, down: o.down, checks: o.checks }))
  process.env.FAKE_GH_STATE = file
  const gh = () => JSON.parse(readFileSync(file, 'utf8')) as FakeGitHub
  const set = (patch: Partial<FakeGitHub>) => writeFileSync(file, JSON.stringify({ ...gh(), ...patch }))
  return { bare, dir, gh, set }
}

/** Settings → How tasks work → Merge after GitHub checks, for one test (it is kept on disk: back to its default after) */
const waitForChecks = (w: ReturnType<typeof world>) => w.settings.set('brief.merge', 'after GitHub checks')
afterEach(() => void new Settings().set('brief.merge', null))

describe('on GitHub: a pull request per task', () => {
  test("its origin is read as written and as git rewrites it: github.com mapped to github.localhost talks to github.localhost", async () => {
    const dir = repo()
    sh('git remote add origin https://github.com/acme/app.git && git config url.http://github.localhost/.insteadOf https://github.com/', dir)
    expect(await G.github(dir)).toEqual({ host: 'github.localhost', repo: 'acme/app' })
    sh('git remote set-url origin http://github.localhost/eliasstravik/shepherdr.git && git config --unset url.http://github.localhost/.insteadOf', dir)
    expect(await G.github(dir)).toEqual({ host: 'github.localhost', repo: 'eliasstravik/shepherdr' })
    sh('git remote set-url origin ssh://git@github.com/acme/app.git', dir)
    expect(await G.github(dir)).toEqual({ host: 'github.com', repo: 'acme/app' })
    sh('git remote set-url origin https://gitlab.com/acme/app.git', dir)
    expect(await G.github(dir)).toBeNull()
  })

  test('a remote-only develop override starts there and merges there, preserving main and the space default', async () => {
    const w = world()
    const g = onGitHub()
    sh('git branch develop main && git push -q origin develop && git branch -D develop', g.dir)
    const main = sh('git rev-parse main', g.dir)
    const p = await w.places.add(g.dir, 'gitflow')
    const t = w.engine.add({ project: p.name, title: 'Develop work', by: 'test', base: 'develop' })
    await until(() => w.started.some((s) => s.key === `${t.id}/Agent`))
    expect(w.started.find((s) => s.key === `${t.id}/Agent`)!.system).toContain('(from develop)')
    expect(w.engine.view(t).base).toBe('develop')
    expect(p.base).toBe('main')
    writeFileSync(join(t.worktree!, 'develop.txt'), 'work')
    sh('git add -A && git commit -qm work', t.worktree!)
    expect((await w.engine.openPr(t.id)).ok).toBe(true)
    expect(g.gh().prs[0].base).toBe('develop')
    expect((await w.engine.merge(t.id, { self: true })).ok).toBe(true)
    expect(sh('git show develop:develop.txt', g.bare)).toBe('work')
    expect(sh('git rev-parse main', g.bare)).toBe(main)
    expect(sh('git rev-parse develop', g.dir)).toBe(sh('git rev-parse develop', g.bare))
  })

  test('a pull request retargeted on GitHub cannot merge into a different branch', async () => {
    const w = world()
    const g = onGitHub()
    const { t } = await started(w, g.dir, 'wrong-target')
    writeFileSync(join(t.worktree!, 'work.txt'), 'work')
    sh('git add -A && git commit -qm work', t.worktree!)
    expect((await w.engine.openPr(t.id)).ok).toBe(true)
    g.set({ prs: g.gh().prs.map((p) => ({ ...p, base: 'develop' })) })
    const r = await w.engine.merge(t.id, { self: true })
    expect(r.ok).toBe(false)
    expect(r.error).toContain('targets develop')
    expect(g.gh().calls.some((c) => c.args[0] === 'pr' && c.args[1] === 'merge')).toBe(false)
  })

  test('read-only GitHub permission and deleted remote branches refuse work before an agent starts', async () => {
    const w = world()
    const g = onGitHub()
    const p = await w.places.add(g.dir, 'read-only')
    g.set({ permission: 'READ' })
    const t = w.engine.add({ project: p.name, title: 'No write access', by: 'test' })
    await until(() => !!t.trouble)
    expect(t.trouble).toContain('permission')
    expect(t.worktree).toBeUndefined()
    expect(w.started.some((s) => s.key === `${t.id}/Agent`)).toBe(false)
    g.set({ permission: 'WRITE' })
    sh('git branch gone && git push -q origin gone && git push -q origin --delete gone', g.dir)
    const missing = w.engine.add({ project: p.name, title: 'Gone', by: 'test', base: 'gone' })
    await until(() => !!missing.trouble)
    expect(missing.trouble).toContain('Cannot fetch branch gone')
    expect(missing.worktree).toBeUndefined()
  })

  test('shepherdr pr opens a draft; done makes it ready with the summary; merge squash-merges it on GitHub and pulls main here', async () => {
    const w = world()
    const g = onGitHub({ remote: 'http://github.localhost/acme/app.git' })
    const { t, key } = await started(w, g.dir, 'gh')
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    sh('git add -A && git commit -qm "add hello"', t.worktree!)
    const r = await w.engine.openPr(t.id)
    expect(r.ok).toBe(true)
    expect(r.text).toContain('Opened PR #1 as a draft')
    expect(r.text).toContain('https://github.com/acme/app/pull/1')
    expect(t.pr).toEqual({ number: 1, url: 'https://github.com/acme/app/pull/1' })
    expect(w.engine.view(t).pr).toEqual(t.pr)
    const pr = g.gh().prs[0]
    expect(pr).toMatchObject({ head: t.branch, base: 'main', title: 'Say hello', isDraft: true })
    expect(pr.body).toContain('> Add hello.txt. Merge when done.')
    expect(pr.body).toContain('Generated with [Claude Code]')
    // gh is told the host git really talks to
    expect(g.gh().calls.every((c) => c.host === 'github.localhost')).toBe(true)
    expect(sh(`git rev-parse refs/heads/${t.branch}`, g.bare)).toBe(sh('git rev-parse HEAD', t.worktree!))
    // no checks on this repo: nothing to wait for
    expect((await w.engine.checks(t.id, 0)).text).toContain('no checks')
    await w.engine.done(t.id, 'Added hello.txt.')
    await until(() => !g.gh().prs[0].isDraft)
    expect(g.gh().prs[0].body.startsWith('Added hello.txt.')).toBe(true)
    const m = await w.engine.merge(t.id, { self: true })
    expect(m).toEqual({ ok: true, note: 'merged PR #1 on GitHub, not waiting for its checks' })
    expect(g.gh().prs[0].state).toBe('MERGED')
    expect(sh('git log --format=%s -n 2 main', g.bare).split('\n')).toEqual([`${t.id}: Say hello (#1)`, 'init'])
    // its branch on GitHub is gone, and main here is GitHub's (what Shepherdr's own update watches)
    expect(sh(`git branch --list ${t.branch}`, g.bare)).toBe('')
    expect(sh('git rev-parse main', g.dir)).toBe(sh('git rev-parse main', g.bare))
    expect(existsSync(join(g.dir, 'hello.txt'))).toBe(true)
    expect(w.engine.statusOf(t)).toBe('done')
    w.agentIs(key, 'idle')
    await until(() => !!t.cleaned)
    expect(existsSync(t.worktree!)).toBe(false)
    expect(sh('git branch --list "work/*"', g.dir)).toBe('')
  })

  test('merge opens the pull request itself when the agent never did, and merges it at the commit here', async () => {
    const w = world()
    const g = onGitHub({ checks: [{ name: 'test', bucket: 'pass' }] })
    const { t } = await started(w, g.dir, 'ghmerge')
    // main moved on GitHub since the task started
    const other = mkdtempSync(join(tmpdir(), 'shep-gh-other-'))
    sh(`git clone -q ${g.bare} . && git config user.email t@t && git config user.name T && echo o > other.txt && git add -A && git commit -qm other && git push -q origin main`, other)
    writeFileSync(join(t.worktree!, 'mine.txt'), 'mine\n')
    w.agentIs(`${t.id}/Agent`, 'idle')
    const m = await w.engine.merge(t.id)
    expect(m.ok).toBe(true)
    const pr = g.gh().prs[0]
    expect(pr).toMatchObject({ number: 1, isDraft: false, state: 'MERGED' })
    expect(g.gh().calls.find((c) => c.args[1] === 'merge')!.args).toContain('--match-head-commit')
    expect(g.gh().calls.every((c) => c.host === 'github.com')).toBe(true)
    expect(sh('git log --format=%s -n 3 main', g.dir).split('\n')).toEqual([`${t.id}: Say hello (#1)`, 'other', 'init'])
    expect(sh('git show --name-only --format= main', g.dir)).toBe('mine.txt')
  })

  test('main moved on GitHub without a conflict: merged as it is, its passed checks kept, no new push', async () => {
    const w = world()
    const g = onGitHub({ checks: [{ name: 'test', bucket: 'pass' }] })
    // its brief's Settings block says to wait: set before it is added
    waitForChecks(w)
    const { t } = await started(w, g.dir, 'ghbehind')
    writeFileSync(join(t.worktree!, 'mine.txt'), 'mine\n')
    sh('git add -A && git commit -qm mine', t.worktree!)
    await w.engine.openPr(t.id)
    const head = sh('git rev-parse HEAD', t.worktree!)
    const other = mkdtempSync(join(tmpdir(), 'shep-gh-other-'))
    sh(`git clone -q ${g.bare} . && git config user.email t@t && git config user.name T && echo o > other.txt && git add -A && git commit -qm other && git push -q origin main`, other)
    const pushes = g.gh().calls.length
    const m = await w.engine.merge(t.id, { self: true })
    expect(m).toEqual({ ok: true, note: 'merged PR #1 on GitHub' })
    // not rebased: GitHub merged the very commit its checks passed on
    expect(sh('git rev-parse HEAD', t.worktree!)).toBe(head)
    const merge = g.gh().calls.slice(pushes).find((c) => c.args[1] === 'merge')!.args
    expect(merge[merge.indexOf('--match-head-commit') + 1]).toBe(head)
    expect(sh('git log --format=%s -n 3 main', g.dir).split('\n')).toEqual([`${t.id}: Say hello (#1)`, 'other', 'init'])
    expect(existsSync(join(g.dir, 'other.txt')) && existsSync(join(g.dir, 'mine.txt'))).toBe(true)
  })

  test('a conflict with main is rebased here when git can, pushed, and waits for fresh checks; else it goes back', async () => {
    const w = world()
    const g = onGitHub({ checks: [{ name: 'test', bucket: 'pass' }] })
    waitForChecks(w)
    const { p: place, t } = await started(w, g.dir, 'ghclash')
    writeFileSync(join(t.worktree!, 'README.md'), 'mine\n')
    sh('git add -A && git commit -qm mine', t.worktree!)
    await w.engine.openPr(t.id)
    const head = sh('git rev-parse HEAD', t.worktree!)
    // main took this very change, then changed the line again: merging conflicts, but a rebase drops the change it has
    const other = mkdtempSync(join(tmpdir(), 'shep-gh-other-'))
    sh(`git clone -q ${g.bare} . && git config user.email t@t && git config user.name T && git fetch -q origin ${t.branch} && git cherry-pick -x FETCH_HEAD >/dev/null && echo again > README.md && git commit -qam again && git push -q origin main`, other)
    writeFileSync(join(t.worktree!, 'new.txt'), 'new\n')
    sh('git add -A && git commit -qm new', t.worktree!)
    g.set({ checks: [{ name: 'test', bucket: 'pending' }] })
    const p = await w.engine.merge(t.id, { self: true })
    expect(p.ok).toBe(false)
    expect(p.error).toContain('rebased on main for a conflict')
    expect(sh('git log --format=%s -n 4 HEAD', t.worktree!).split('\n')).toEqual(['new', 'again', 'mine', 'init'])
    expect(sh(`git rev-parse refs/heads/${t.branch}`, g.bare)).not.toBe(head)
    expect(sh(`git rev-parse refs/heads/${t.branch}`, g.bare)).toBe(sh('git rev-parse HEAD', t.worktree!))
    g.set({ checks: [{ name: 'test', bucket: 'pass' }] })
    expect((await w.engine.merge(t.id, { self: true })).ok).toBe(true)
    // the same lines changed on both sides: git can't rebase it, so its agent does, by hand
    const u = w.engine.add({ project: place.name, title: 'Say it too', desc: 'Change README.md.', by: 'test' })
    await until(() => w.started.some((s) => s.key === `${u.id}/Agent`))
    writeFileSync(join(u.worktree!, 'README.md'), 'mine too\n')
    sh('git add -A && git commit -qm mine', u.worktree!)
    await w.engine.openPr(u.id)
    const at = sh('git rev-parse HEAD', u.worktree!)
    sh('git pull -q --rebase origin main && echo theirs > README.md && git commit -qam theirs && git push -q origin main', other)
    const c = await w.engine.merge(u.id, { self: true })
    expect(c.ok).toBe(false)
    expect(c.error).toContain('Conflict with main: rebase')
    expect(sh('git rev-parse HEAD', u.worktree!)).toBe(at)
    expect(g.gh().prs[1].state).toBe('OPEN')
  })

  test("failed checks go back to the agent; merging waits while they run, and never on what it can't read", async () => {
    const w = world()
    const g = onGitHub({ checks: [{ name: 'test', bucket: 'fail', link: 'https://github.com/acme/app/actions/runs/7' }, { name: 'lint', bucket: 'pass' }] })
    waitForChecks(w)
    const { t, key } = await started(w, g.dir, 'ghfail')
    writeFileSync(join(t.worktree!, 'x.txt'), 'x\n')
    sh('git add -A && git commit -qm x', t.worktree!)
    await w.engine.openPr(t.id)
    const c = await w.engine.checks(t.id, 0)
    expect(c.ok).toBe(false)
    expect(c.text).toContain("GitHub's checks failed on PR #1: test.")
    expect(c.text).toContain('actions/runs/7')
    await w.engine.done(t.id, 'x')
    w.agentIs(key, 'idle')
    // you (or a coordinator) merge: refused, and the agent is told what to fix
    const m = await w.engine.merge(t.id)
    expect(m.ok).toBe(false)
    expect(m.error).toContain('Sent back')
    expect(w.sent.at(-1)!.key).toBe(key)
    expect(w.sent.at(-1)!.text).toContain(`shepherdr pr ${t.id}`)
    expect(w.engine.statusOf(t)).toBe('work')
    expect(g.gh().prs[0].state).toBe('OPEN')
    // still running: wait for them
    g.set({ checks: [{ name: 'test', bucket: 'pending' }] })
    const p = await w.engine.merge(t.id, { self: true })
    expect(p.error).toContain(`shepherdr checks ${t.id}`)
    expect((await w.engine.checks(t.id, 0)).text).toContain('still run')
    // fixed and passed
    g.set({ checks: [{ name: 'test', bucket: 'pass' }] })
    expect((await w.engine.checks(t.id, 0)).text).toContain("checks passed on PR #1")
    expect((await w.engine.merge(t.id, { self: true })).ok).toBe(true)
    expect(t.merged).toBe(true)
  })

  test('Merging, not Ready, from its first merge until it is in: checks running, a rebase, the merge itself', async () => {
    const w = world()
    const g = onGitHub({ checks: [{ name: 'test', bucket: 'pending' }] })
    waitForChecks(w)
    // the checks of its last pull requests took 2, 3 and 9 minutes
    const run = (m: number) => ['2026-10-26T10:00:00Z', `2026-10-26T10:0${m}:00Z`]
    g.set({ api: { 'repos/acme/app/actions/runs': [run(2), run(9), run(3)] } })
    const { t, key } = await started(w, g.dir, 'lands')
    writeFileSync(join(t.worktree!, 'x.txt'), 'x\n')
    sh('git add -A && git commit -qm x', t.worktree!)
    await w.engine.done(t.id, 'Added x, a long summary that belongs in the chat card and never on the row.')
    w.chat.reply = 'Merging once the checks pass.'
    // reported ready, not yet merging: Ready, a few words on its row
    w.agentIs(key, 'idle')
    expect(w.engine.statusOf(t)).toBe('ready')
    expect(w.engine.subOf(t)).toBe('Ready to merge')
    w.agentIs(key, 'working')
    const r = await w.engine.merge(t.id, { self: true })
    expect(r.error).toContain(`shepherdr checks ${t.id}`)
    expect(w.engine.statusOf(t)).toBe('merging')
    expect(w.engine.subOf(t)).toBe('Checks running · ~3m')
    expect(w.engine.view(t)).toMatchObject({ status: 'merging', state: expect.stringMatching(/^Merging · /) })
    // its turn ends while it waits for them: still Merging, nothing for you
    w.agentIs(key, 'idle')
    expect(w.engine.statusOf(t)).toBe('merging')
    // a question in its words alone is no fact: still Merging
    w.chat.reply = 'Merge it anyway?'
    expect(w.engine.statusOf(t)).toBe('merging')
    // it asks you on a card meanwhile: Ready, as it waits for you
    w.chat.card = 'ask'
    expect(w.engine.statusOf(t)).toBe('ready')
    w.chat.card = undefined
    w.chat.reply = 'Merging once the checks pass.'
    // the checks pass while it rests with nothing running to wake it: it is told once to merge
    g.set({ checks: [{ name: 'test', bucket: 'pass' }] })
    await w.engine.watchPrs()
    expect(w.sent.at(-1)!.text).toContain(`merge ${t.id} now (shepherdr merge ${t.id})`)
    const told = w.sent.length
    await w.engine.watchPrs()
    expect(w.sent.length).toBe(told)
    expect(w.engine.statusOf(t)).toBe('merging')
    // a message from you: back at work, no longer merging
    w.engine.youWrote(key)
    expect(t.landing).toBeUndefined()
    await w.engine.done(t.id, 'x')
    w.agentIs(key, 'working')
    expect((await w.engine.merge(t.id, { self: true })).error).toBeUndefined()
    expect(w.engine.statusOf(t)).toBe('done')
    expect(w.engine.subOf(t)).toBe('Merged')
  })

  test('failed checks on its way in: back at work, not Merging', async () => {
    const w = world()
    const g = onGitHub({ checks: [{ name: 'test', bucket: 'pending' }] })
    waitForChecks(w)
    const { t, key } = await started(w, g.dir, 'landfail')
    writeFileSync(join(t.worktree!, 'x.txt'), 'x\n')
    sh('git add -A && git commit -qm x', t.worktree!)
    await w.engine.done(t.id, 'x')
    w.agentIs(key, 'working')
    await w.engine.merge(t.id, { self: true })
    expect(w.engine.subOf(t)).toBe('Checks running')
    g.set({ checks: [{ name: 'test', bucket: 'fail' }] })
    expect((await w.engine.checks(t.id, 0)).ok).toBe(false)
    expect(t.landing).toBeUndefined()
    expect(w.engine.statusOf(t)).toBe('work')
  })

  test("a token that can't read GitHub's checks reads the Actions jobs of the pull request's commit instead", async () => {
    const w = world()
    const g = onGitHub()
    const { t, key } = await started(w, g.dir, 'ghjobs')
    writeFileSync(join(t.worktree!, 'x.txt'), 'x\n')
    sh('git add -A && git commit -qm x', t.worktree!)
    await w.engine.openPr(t.id)
    const head = sh('git rev-parse HEAD', t.worktree!)
    const job = (status: string, conclusion: string | null) => ({ name: 'check', status, conclusion, html_url: 'https://github.com/acme/app/actions/runs/7/job/9' })
    g.set({ noChecks: true, api: { 'repos/acme/app/actions/runs': [7], 'repos/acme/app/actions/runs/7/jobs': [job('in_progress', null)] } })
    expect((await w.engine.checks(t.id, 0)).text).toContain('still run')
    expect(g.gh().calls.some((c) => c.args.includes(`repos/acme/app/actions/runs?head_sha=${head}&per_page=100`))).toBe(true)
    // reported done while they run: a failure is news, with the end of its log
    await w.engine.done(t.id, 'x')
    g.set({ api: { 'repos/acme/app/actions/runs': [7], 'repos/acme/app/actions/runs/7/jobs': [job('completed', 'failure')] }, logs: { '9': '2026-10-26T10:00:00.1Z ##[error]boom' } })
    w.agentIs(key, 'idle')
    await w.engine.watchPrs()
    expect(w.sent.at(-1)!.text).toContain('- Check failed: check (https://github.com/acme/app/actions/runs/7/job/9). Its log ends:\n    ##[error]boom')
    const c = await w.engine.checks(t.id, 0)
    expect(c.ok).toBe(false)
    expect(c.text).toContain("GitHub's checks failed on PR #1: check.")
    // fixed: passed, a skipped job aside, and it merges
    g.set({ api: { 'repos/acme/app/actions/runs': [7], 'repos/acme/app/actions/runs/7/jobs': [job('completed', 'success'), { ...job('completed', 'skipped'), name: 'deploy' }] } })
    expect((await w.engine.checks(t.id, 0)).text).toContain('checks passed on PR #1 (1)')
    await w.engine.done(t.id, 'x')
    expect((await w.engine.merge(t.id, { self: true })).ok).toBe(true)
  })

  test("news on a done task's pull request wakes its agent once: a failed check with its log's end, new comments", async () => {
    const w = world()
    const g = onGitHub({ checks: [{ name: 'test', bucket: 'pass' }] })
    const { t, key } = await started(w, g.dir, 'ghnews')
    writeFileSync(join(t.worktree!, 'x.txt'), 'x\n')
    sh('git add -A && git commit -qm x', t.worktree!)
    await w.engine.openPr(t.id)
    // not done yet: nothing is watched
    g.set({ checks: [{ name: 'lint', bucket: 'fail' }] })
    w.agentIs(key, 'idle')
    await w.engine.watchPrs()
    expect(w.sent.length).toBe(0)
    // a failure it saw itself (shepherdr checks) and reported done with is no news
    expect((await w.engine.checks(t.id, 0)).ok).toBe(false)
    await w.engine.done(t.id, 'x; lint is red on main too')
    await w.engine.watchPrs()
    expect(w.sent.length).toBe(0)
    // a check fails and people write on it; a comment from before it was done is old news
    const pulls = 'repos/acme/app/pulls/1'
    g.set({
      checks: [{ name: 'test', bucket: 'fail', link: 'https://github.com/acme/app/actions/runs/7/job/9' }, { name: 'lint', bucket: 'pass' }],
      // as GitHub logs a whole job: what failed, then the clean-up steps after it
      logs: { '9': ['ok 1', 'expected 1 to be 2', '##[error]Process completed with exit code 1.', 'Post job cleanup.', 'Cleaning up orphan processes'].map((l) => `2026-10-26T10:00:00.1234567Z ${l}`).join('\n') },
      api: {
        [`${pulls}/reviews`]: [
          { id: 1, user: { login: 'ann' }, body: 'Please rename it.', state: 'CHANGES_REQUESTED', html_url: 'https://github.com/acme/app/pull/1#r1', submitted_at: '2099-01-01T00:00:00Z' },
          { id: 2, user: { login: 'bob' }, body: '', state: 'COMMENTED', html_url: 'https://github.com/acme/app/pull/1#r2', submitted_at: '2099-01-01T00:00:01Z' },
        ],
        // drafted long before its review was sent: news all the same
        [`${pulls}/comments`]: [{ id: 3, pull_request_review_id: 2, user: { login: 'bob' }, body: 'Typo here.', path: 'x.txt', line: 1, html_url: 'https://github.com/acme/app/pull/1#c3', created_at: '2000-01-01T00:00:00Z' }],
        ['repos/acme/app/issues/1/comments']: [{ id: 4, user: { login: 'cy' }, body: 'Old news.', html_url: 'https://github.com/acme/app/pull/1#i4', created_at: '2000-01-01T00:00:00Z' }],
      },
    })
    await w.engine.watchPrs()
    expect(w.sent.length).toBe(1)
    const told = w.sent[0]
    expect(told.key).toBe(key)
    expect(told.text).toMatch(/^<shepherdr><github task="[^"]+" pr="1">GitHub news on PR #1 \(https:\/\/github.com\/acme\/app\/pull\/1\)/)
    expect(told.text).toContain('- Check failed: test (https://github.com/acme/app/actions/runs/7/job/9). Its log ends:\n    ok 1\n    expected 1 to be 2\n    ##[error]Process completed with exit code 1.\n- ')
    expect(told.text).not.toContain('clean')
    expect(told.text).toContain('- Changes requested by ann (https://github.com/acme/app/pull/1#r1):\n    Please rename it.')
    expect(told.text).toContain('- Comment by bob on x.txt:1 (https://github.com/acme/app/pull/1#c3):\n    Typo here.')
    expect(told.text).not.toContain('Old news')
    expect(told.text).not.toContain('#r2')
    expect(told.text).toContain('shepherdr-agent skill')
    expect(w.posted.at(-1)).toMatchObject({ key, card: { status: 'work', text: 'woken: test failed and 2 new comments on PR #1' } })
    // it is at work again, and hears nothing more until it reports done
    expect(w.engine.statusOf(t)).toBe('work')
    await w.engine.watchPrs()
    expect(w.sent.length).toBe(1)
    // done again: the same failure and comments are not told twice
    await w.engine.done(t.id, 'x')
    await w.engine.watchPrs()
    expect(w.sent.length).toBe(1)
    // an agent at work hears news only once it rests
    const notes = g.gh().api!
    g.set({ api: { ...notes, ['repos/acme/app/issues/1/comments']: [{ id: 5, user: { login: 'cy' }, body: 'Ship it?', html_url: 'https://github.com/acme/app/pull/1#i5', created_at: '2099-01-02T00:00:00Z' }] } })
    w.agentIs(key, 'working')
    await w.engine.watchPrs()
    expect(w.sent.length).toBe(1)
    w.agentIs(key, 'idle')
    await w.engine.watchPrs()
    expect(w.sent.length).toBe(2)
    expect(w.sent[1].text).toContain('- Comment by cy (https://github.com/acme/app/pull/1#i5):\n    Ship it?')
    expect(w.sent[1].text).not.toContain('Check failed')
  })

  test('a branch just pushed whose repo has workflows on pull requests waits for its checks to show, never taking none as passed', async () => {
    const w = world()
    const g = onGitHub()
    waitForChecks(w)
    const { t } = await started(w, g.dir, 'ghwf')
    mkdirSync(join(t.worktree!, '.github', 'workflows'), { recursive: true })
    writeFileSync(join(t.worktree!, '.github', 'workflows', 'ci.yml'), 'on:\n  pull_request:\n  push:\n')
    sh('git add -A && git commit -qm ci', t.worktree!)
    const m = await w.engine.merge(t.id, { self: true })
    expect(m.ok).toBe(false)
    expect(m.error).toContain('just pushed')
    expect(g.gh().prs[0].state).toBe('OPEN')
    // a while later still none: they didn't run on it after all, and it merges
    t.pushed!.at = 0
    expect((await w.engine.merge(t.id, { self: true })).ok).toBe(true)
  })

  test('a branch just pushed whose workflows run only on main has no checks to wait for', async () => {
    const w = world()
    const g = onGitHub()
    waitForChecks(w)
    const { t } = await started(w, g.dir, 'ghmain')
    mkdirSync(join(t.worktree!, '.github', 'workflows'), { recursive: true })
    writeFileSync(join(t.worktree!, '.github', 'workflows', 'ci.yml'), '# no run on pull requests\non:\n  push:\n    branches: [main]\n  workflow_dispatch:\n')
    sh('git add -A && git commit -qm ci', t.worktree!)
    expect((await w.engine.merge(t.id, { self: true })).ok).toBe(true)
  })

  test("by default it merges without waiting for GitHub's checks, but never on one already failed at its commit", async () => {
    const w = world()
    const g = onGitHub({ checks: [{ name: 'test', bucket: 'pending' }] })
    const { t } = await started(w, g.dir, 'ghnowait')
    writeFileSync(join(t.worktree!, 'x.txt'), 'x\n')
    sh('git add -A && git commit -qm x', t.worktree!)
    expect((await w.engine.openPr(t.id)).text).toContain("Merging doesn't wait for GitHub's checks")
    g.set({ checks: [{ name: 'test', bucket: 'fail', link: 'https://github.com/acme/app/actions/runs/7' }] })
    const f = await w.engine.merge(t.id, { self: true })
    expect(f.ok).toBe(false)
    expect(f.error).toContain("GitHub's checks failed on PR #1: test.")
    expect(g.gh().prs[0].state).toBe('OPEN')
    // fixed: a new commit, its checks just started
    writeFileSync(join(t.worktree!, 'x.txt'), 'fixed\n')
    sh('git commit -qam fix', t.worktree!)
    g.set({ checks: [{ name: 'test', bucket: 'pending' }] })
    expect(await w.engine.merge(t.id, { self: true })).toEqual({ ok: true, note: 'merged PR #1 on GitHub, not waiting for its checks' })
    expect(g.gh().prs[0].state).toBe('MERGED')
    // the brief's Merge line says so too
    expect(t.desc).toContain("Don't wait for GitHub's checks.")
  })

  test("main going red on GitHub after merges tells its coordinator once, with the run and what merged since green", async () => {
    const w = world()
    const g = onGitHub()
    const { t } = await started(w, g.dir, 'ghred')
    writeFileSync(join(t.worktree!, 'x.txt'), 'x\n')
    sh('git add -A && git commit -qm x', t.worktree!)
    const green = sh('git rev-parse main', g.bare)
    // other tests' tasks merged lately too (one store): only this one's project is watched
    for (const x of w.engine.tasks) x.doneAt = 0
    expect((await w.engine.merge(t.id, { self: true })).ok).toBe(true)
    const red = sh('git rev-parse main', g.bare)
    const run = (headSha: string, conclusion: string, n: number, status = 'completed') => ({ headSha, status, conclusion, url: `https://github.com/acme/app/actions/runs/${n}`, workflowName: 'Check' })
    // still running on the merge: nothing yet
    g.set({ runs: [run(red, '', 3, 'in_progress'), run(green, 'success', 2)] })
    await w.engine.watchMains()
    const before = w.news.length
    expect(w.news.length).toBe(before)
    // a newer push cancelled one run; the other failed
    g.set({ runs: [run(red, 'cancelled', 4), run(red, 'failure', 3), run(green, 'success', 2)], api: { 'repos/acme/app/actions/runs/3/jobs': [9] }, logs: { '9': 'expected 1 to be 2\n##[error]Process completed with exit code 1.' } })
    await w.engine.watchMains()
    expect(w.news.length).toBe(before + 1)
    const told = w.news.at(-1)!
    expect(told).toContain(`main went red on GitHub at ${red.slice(0, 7)}: Check failed (https://github.com/acme/app/actions/runs/3).`)
    expect(told).toContain(`Merged since it was last green (${green.slice(0, 7)}): ${t.id}: Say hello (#1).`)
    expect(told).toContain('    expected 1 to be 2')
    expect(told).toContain('fix-forward task')
    // for the project's own coordinator, by its place id (a folder's path reached no chat)
    expect(w.newsTo.at(-1)).toBe(t.project)
    expect(w.places.space(t.project)).toBeDefined()
    // told once while it stays red
    await w.engine.watchMains()
    expect(w.news.length).toBe(before + 1)
    // green again, then red again: told again
    g.set({ runs: [run('b'.repeat(40), 'success', 5), run(red, 'failure', 3)] })
    await w.engine.watchMains()
    g.set({ runs: [run('c'.repeat(40), 'failure', 6), run('b'.repeat(40), 'success', 5)] })
    await w.engine.watchMains()
    expect(w.news.length).toBe(before + 2)
  })

  test("GitHub stuck on the merge: shepherdr merge still answers by its deadline, and asked again waits for that same merge", async () => {
    const w = world()
    const g = onGitHub()
    const { t } = await started(w, g.dir, 'mergeslow')
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    sh('git add -A && git commit -qm "add hello"', t.worktree!)
    await w.engine.done(t.id, 'Added hello.txt.')
    g.set({ mergeSlow: 4000 })
    w.engine.mergeDeadline = 1000
    const first = await w.engine.merge(t.id, { self: true })
    expect(first.ok).toBe(false)
    expect(first.error).toContain(`${t.id}'s merge isn't through after 1 s`)
    expect(first.error).toContain(`Run shepherdr merge ${t.id} again`)
    expect(w.engine.statusOf(t)).toBe('merging')
    // asked again while it still runs: no second merge beside it, the same one's answer
    w.engine.mergeDeadline = 30_000
    expect(await w.engine.merge(t.id, { self: true })).toEqual({ ok: true, note: 'merged PR #1 on GitHub, not waiting for its checks' })
    expect(g.gh().calls.filter((c) => c.args[0] === 'pr' && c.args[1] === 'merge')).toHaveLength(1)
    expect(w.engine.statusOf(t)).toBe('done')
  })

  test('Shepherdr stopping for an update lets a merge on its way finish and answer first, and starts no new one', async () => {
    const w = world()
    const g = onGitHub()
    const { t } = await started(w, g.dir, 'mergedrain')
    const other = w.engine.add({ project: t.project, title: 'Other', by: 'test' })
    writeFileSync(join(t.worktree!, 'hello.txt'), 'hello\n')
    sh('git add -A && git commit -qm "add hello"', t.worktree!)
    await w.engine.done(t.id, 'Added hello.txt.')
    g.set({ mergeSlow: 1500 })
    const merging = w.engine.merge(t.id, { self: true })
    await until(() => g.gh().calls.some((c) => c.args[0] === 'pr' && c.args[1] === 'view'))
    const drained = w.engine.drain()
    expect(await w.engine.merge(other.id)).toEqual({ ok: false, error: `Shepherdr is restarting: nothing was merged. Run shepherdr merge ${other.id} again in a minute.` })
    expect(await drained).toBe(true)
    // through by the time it may stop: merged on GitHub and marked so, its answer given
    expect(g.gh().prs[0].state).toBe('MERGED')
    expect(t.merged).toBe(true)
    expect((await merging).ok).toBe(true)
  })

  test("logged in but GitHub doesn't answer: nothing merges, here or there", async () => {
    const w = world()
    const g = onGitHub()
    const { t } = await started(w, g.dir, 'ghdown')
    g.set({ down: true })
    writeFileSync(join(t.worktree!, 'd.txt'), 'd\n')
    sh('git add -A && git commit -qm d', t.worktree!)
    const before = sh('git rev-parse main', g.dir)
    const r = await w.engine.openPr(t.id)
    expect(r.ok).toBe(false)
    expect(r.text).toContain("gh can't reach GitHub")
    const m = await w.engine.merge(t.id, { self: true })
    expect(m.ok).toBe(false)
    expect(m.error).toContain("gh can't reach GitHub now (github.com: Failed to log in")
    expect(sh('git rev-parse main', g.dir)).toBe(before)
    expect(sh('git rev-parse main', g.bare)).toBe(before)
    expect(t.merged).toBeFalsy()
    // its pull request open, and gh logged out here: never merged around it
    g.set({ down: false, authed: false })
    t.pr = { number: 9, url: 'https://github.com/acme/app/pull/9' }
    expect((await w.engine.merge(t.id, { self: true })).error).toContain('PR #9 is on GitHub')
    expect(sh('git rev-parse main', g.dir)).toBe(before)
  })

  test('shepherdr pr again pushes to the open one, and readies it once done; a merge GitHub refuses merges nothing', async () => {
    const w = world()
    const g = onGitHub()
    const { t } = await started(w, g.dir, 'ghagain')
    writeFileSync(join(t.worktree!, 'a.txt'), 'a\n')
    sh('git add -A && git commit -qm a', t.worktree!)
    await w.engine.openPr(t.id)
    writeFileSync(join(t.worktree!, 'b.txt'), 'b\n')
    sh('git add -A && git commit -qm b', t.worktree!)
    t.reported = true
    t.said = 'Added a and b.'
    const r = await w.engine.openPr(t.id)
    expect(r.text).toContain('Pushed to PR #1')
    expect(g.gh().prs).toHaveLength(1)
    expect(g.gh().prs[0]).toMatchObject({ isDraft: false })
    expect(g.gh().prs[0].body.startsWith('Added a and b.')).toBe(true)
    expect(sh(`git rev-parse refs/heads/${t.branch}`, g.bare)).toBe(sh('git rev-parse HEAD', t.worktree!))
    g.set({ mergeError: 'Pull request #1 is not mergeable: the merge commit cannot be cleanly created.' })
    const m = await w.engine.merge(t.id, { self: true })
    expect(m.ok).toBe(false)
    expect(m.error).toContain('Conflict with main on GitHub')
    expect(t.merged).toBeFalsy()
    expect(g.gh().prs[0].state).toBe('OPEN')
  })

  test('a pull request merged on GitHub by hand: main here catches up and it is marked merged', async () => {
    const w = world()
    const g = onGitHub()
    const { t } = await started(w, g.dir, 'ghhand')
    writeFileSync(join(t.worktree!, 'y.txt'), 'y\n')
    sh('git add -A && git commit -qm y', t.worktree!)
    await w.engine.openPr(t.id)
    const ready = mkdtempSync(join(tmpdir(), 'shep-gh-hand-'))
    g.set({ prs: g.gh().prs.map((p) => ({ ...p, isDraft: false })) })
    sh(`"${join(ghBin, 'gh')}" pr merge 1 --squash`, ready)
    const m = await w.engine.merge(t.id, { self: true })
    expect(m).toEqual({ ok: true, note: 'PR #1 was merged on GitHub already' })
    expect(existsSync(join(g.dir, 'y.txt'))).toBe(true)
  })

  test('main here ahead of GitHub\'s is refused before anything merges; work main has already closes its pull request', async () => {
    const w = world()
    const g = onGitHub()
    const { t } = await started(w, g.dir, 'ghahead')
    writeFileSync(join(t.worktree!, 'v.txt'), 'v\n')
    sh('git add -A && git commit -qm v', t.worktree!)
    await w.engine.openPr(t.id)
    sh('echo l > local.txt && git add -A && git commit -qm local', g.dir)
    const m = await w.engine.merge(t.id, { self: true })
    expect(m.error).toContain('1 commit GitHub lacks')
    expect(g.gh().prs[0].state).toBe('OPEN')
    // pushed, and the same work landed on main another way: its pull request is closed, the task marked merged
    sh(`git push -q origin main && git merge -q --squash ${t.branch} && git commit -qm "by hand" && git push -q origin main`, g.dir)
    const r = await w.engine.merge(t.id, { self: true })
    expect(r).toEqual({ ok: true, note: 'it was in main already; closed PR #1' })
    expect(g.gh().calls.some((c) => c.args[0] === 'pr' && c.args[1] === 'close')).toBe(true)
  })

  test("a GitHub remote gh isn't logged in to: merged here and main pushed, as before", async () => {
    const w = world()
    const g = onGitHub()
    const { t } = await started(w, g.dir, 'ghnoauth')
    g.set({ authed: false })
    writeFileSync(join(t.worktree!, 'z.txt'), 'z\n')
    sh('git add -A && git commit -qm z', t.worktree!)
    expect((await w.engine.openPr(t.id)).text).toContain('No pull request')
    const m = await w.engine.merge(t.id, { self: true })
    expect(m).toEqual({ ok: true, note: 'pushed main to GitHub' })
    expect(g.gh().prs).toEqual([])
    expect(sh('git log --format=%s -n 1 main', g.bare)).toBe(`${t.id}: Say hello`)
  })

  test('a project that gains its GitHub remote after it was added goes through a pull request', async () => {
    const w = world()
    const g = onGitHub()
    sh('git remote remove origin', g.dir)
    const { p, t } = await started(w, g.dir, 'ghlater')
    expect(p.github).toBeNull()
    sh(`git remote add origin https://github.com/acme/app.git && git fetch -q origin`, g.dir)
    writeFileSync(join(t.worktree!, 'w.txt'), 'w\n')
    sh('git add -A && git commit -qm w', t.worktree!)
    expect((await w.engine.merge(t.id, { self: true })).ok).toBe(true)
    expect(p.github).toBe('acme/app')
    expect(g.gh().prs[0].state).toBe('MERGED')
  })
})

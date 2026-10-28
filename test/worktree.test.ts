// A task's worktree, read only, for its side panel: on a real git repo, its files are listed and read only inside it
// (no path tricks, no links out, nothing ignored), its diff against main counts what it changed (committed or not, new
// files too), and its pull request comes from gh (a stand-in here).
import { describe, expect, test } from 'bun:test'
import { execSync } from 'node:child_process'
import { chmodSync, mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { checksOf, diffFiles, diffPatch, ghRepo, listFiles, pullRequest, readFile, safeRel } from '../server/live/worktree.ts'

const sh = (cmd: string, cwd: string) => execSync(cmd, { cwd, shell: '/bin/sh', encoding: 'utf8' }).trim()

/** a repo on main, and a task's worktree off it with every kind of change */
function setup() {
  const dir = mkdtempSync(join(tmpdir(), 'shep-wt-'))
  const repo = join(dir, 'repo')
  mkdirSync(repo)
  sh('git init -q -b main && git config user.email t@t && git config user.name T', repo)
  writeFileSync(join(repo, 'README.md'), '# x\n')
  mkdirSync(join(repo, 'src'))
  writeFileSync(join(repo, 'src/a.ts'), 'export const a = 1\nexport const b = 2\n')
  writeFileSync(join(repo, 'src/old.ts'), 'export const old = true\nexport const keep = 1\nexport const more = 2\n')
  writeFileSync(join(repo, 'gone.txt'), 'bye\n')
  writeFileSync(join(repo, '.gitignore'), '.env\nnode_modules/\n')
  sh('git add -A && git commit -qm init', repo)
  const wt = join(dir, 'wt')
  sh(`git worktree add -q -b work/1-x ${wt} main`, repo)
  // committed: a change, a rename, a delete
  writeFileSync(join(wt, 'src/a.ts'), 'export const a = 1\nexport const b = 3\nexport const c = 4\n')
  sh('git mv src/old.ts src/renamed.ts && git rm -q gone.txt && git commit -qam work', wt)
  // main moves on after the task started: not part of what the task changed
  writeFileSync(join(repo, 'README.md'), '# x\n\nmore on main\n')
  sh('git commit -qam main-moves', repo)
  // not committed: an edit, a new file, a binary, an ignored secret, a link that leads out
  writeFileSync(join(wt, 'README.md'), '# x\nedited\n')
  writeFileSync(join(wt, 'src/new.ts'), 'one\ntwo\n')
  writeFileSync(join(wt, 'logo.png'), new Uint8Array([0x89, 0x50, 0, 0, 1, 2]))
  writeFileSync(join(wt, '.env'), 'SECRET=1\n')
  mkdirSync(join(wt, 'node_modules'))
  writeFileSync(join(wt, 'node_modules/x.js'), 'x\n')
  writeFileSync(join(dir, 'outside.txt'), 'not yours\n')
  symlinkSync(join(dir, 'outside.txt'), join(wt, 'escape.txt'))
  return { dir, repo, wt }
}

describe("a task's files", () => {
  test('listed: tracked and new ones, never ignored ones or .git', async () => {
    const { dir, wt } = setup()
    const { files } = await listFiles(wt)
    expect(files).toEqual(['.gitignore', 'README.md', 'escape.txt', 'logo.png', 'src/a.ts', 'src/new.ts', 'src/renamed.ts'])
    rmSync(dir, { recursive: true, force: true })
  })

  test('read: text as it is, a binary file said to be one', async () => {
    const { dir, wt } = setup()
    expect(await readFile(wt, 'src/new.ts')).toEqual({ path: 'src/new.ts', size: 8, text: 'one\ntwo\n' })
    expect(await readFile(wt, 'logo.png')).toEqual({ path: 'logo.png', size: 6, binary: true })
    rmSync(dir, { recursive: true, force: true })
  })

  test('no path leads outside it: .., absolute paths, .git, ignored files, links out, odd pathspecs', async () => {
    const { dir, wt } = setup()
    for (const p of ['../outside.txt', 'src/../../outside.txt', '/etc/passwd', join(dir, 'outside.txt'), '.git/config', '.git/HEAD', '.env', 'node_modules/x.js', 'escape.txt', 'src', '', './README.md', 'src//a.ts', '*', ':(top)README.md', 'README.md\0', 'src\\a.ts'])
      await expect(readFile(wt, p)).rejects.toThrow('No such file.')
    expect(() => safeRel(42)).toThrow()
    rmSync(dir, { recursive: true, force: true })
  })

  test('a link inside it is read only when what it leads to is listed too: never an ignored file through a link', async () => {
    const { dir, wt } = setup()
    symlinkSync('.env', join(wt, 'leak'))
    symlinkSync('src/new.ts', join(wt, 'alias.ts'))
    sh('git add leak alias.ts && git commit -qm links', wt)
    symlinkSync('.env', join(wt, 'leak2'))
    await expect(readFile(wt, 'leak')).rejects.toThrow('No such file.')
    await expect(readFile(wt, 'leak2')).rejects.toThrow('No such file.')
    expect((await readFile(wt, 'alias.ts')).text).toBe('one\ntwo\n')
    rmSync(dir, { recursive: true, force: true })
  })
})

describe('what a task changed', () => {
  test('against where it left main: committed or not, renames, deletes, new and binary files, with lines in and out', async () => {
    const { dir, wt } = setup()
    const d = await diffFiles(wt, 'main')
    const by = Object.fromEntries(d.files.map((f) => [f.path, f]))
    expect(Object.keys(by).sort()).toEqual(['README.md', 'escape.txt', 'gone.txt', 'logo.png', 'src/a.ts', 'src/new.ts', 'src/renamed.ts'])
    expect(by['src/a.ts']).toEqual({ path: 'src/a.ts', status: 'modified', add: 2, del: 1 })
    expect(by['src/renamed.ts']).toMatchObject({ status: 'renamed', from: 'src/old.ts', add: 0, del: 0 })
    expect(by['gone.txt']).toMatchObject({ status: 'deleted', del: 1 })
    expect(by['src/new.ts']).toMatchObject({ status: 'added', add: 2 })
    expect(by['logo.png']).toMatchObject({ status: 'added', binary: true })
    // main's own later change to README is not the task's: only its edit counts
    expect(by['README.md']).toMatchObject({ status: 'modified', add: 1, del: 0 })
    // a link out is listed as the link it is, but never followed
    expect(by['escape.txt']).toMatchObject({ status: 'added', add: 1 })
    expect(d.add).toBe(d.files.reduce((x, f) => x + f.add, 0))
    rmSync(dir, { recursive: true, force: true })
  })

  test("a file's unified diff: only for a changed file, a new one against nothing", async () => {
    const { dir, wt } = setup()
    const a = await diffPatch(wt, 'main', 'src/a.ts')
    expect(a.text).toContain('-export const b = 2')
    expect(a.text).toContain('+export const c = 4')
    expect(a.text).not.toContain('more on main')
    expect((await diffPatch(wt, 'main', 'src/new.ts')).text).toContain('+two')
    expect((await diffPatch(wt, 'main', 'gone.txt')).text).toContain('-bye')
    const link = await diffPatch(wt, 'main', 'escape.txt')
    expect(link.text).toContain('new file mode 120000')
    expect(link.text).not.toContain('not yours')
    expect(await diffPatch(wt, 'main', 'logo.png')).toEqual({ path: 'logo.png', text: '', binary: true })
    await expect(diffPatch(wt, 'main', '.gitignore')).rejects.toThrow('That file has no changes.')
    await expect(diffPatch(wt, 'main', '../outside.txt')).rejects.toThrow('No such file.')
    rmSync(dir, { recursive: true, force: true })
  })
})

describe('its pull request', () => {
  test('the GitHub host and repo of a remote', () => {
    expect(ghRepo('git@github.com:acme/gtm.git')).toEqual({ host: 'github.com', repo: 'acme/gtm' })
    expect(ghRepo('https://github.com/acme/gtm')).toEqual({ host: 'github.com', repo: 'acme/gtm' })
    expect(ghRepo('http://github.localhost/eliasstravik/shepherdr.git')).toEqual({ host: 'github.localhost', repo: 'eliasstravik/shepherdr' })
    expect(ghRepo('ssh://git@ghe.corp:2222/team/app.git')).toEqual({ host: 'ghe.corp', repo: 'team/app' })
    expect(ghRepo('/local/path')).toBeNull()
  })

  test("gh's checks in plain states", () => {
    expect(
      checksOf([
        { __typename: 'CheckRun', name: 'test', status: 'COMPLETED', conclusion: 'SUCCESS', detailsUrl: 'https://x/1' },
        { __typename: 'CheckRun', name: 'lint', status: 'COMPLETED', conclusion: 'FAILURE' },
        { __typename: 'CheckRun', name: 'e2e', status: 'IN_PROGRESS', conclusion: '' },
        { __typename: 'CheckRun', name: 'docs', status: 'COMPLETED', conclusion: 'SKIPPED' },
        { __typename: 'StatusContext', context: 'vercel', state: 'PENDING', targetUrl: 'https://v' },
        { __typename: 'StatusContext', context: 'ci/legacy', state: 'SUCCESS' },
      ]),
    ).toEqual([
      { name: 'test', state: 'pass', url: 'https://x/1' },
      { name: 'lint', state: 'fail', url: undefined },
      { name: 'e2e', state: 'pending', url: undefined },
      { name: 'docs', state: 'skip', url: undefined },
      { name: 'vercel', state: 'pending', url: 'https://v' },
      { name: 'ci/legacy', state: 'pass', url: undefined },
    ])
  })

  test('asked of gh for the branch, on the remote\'s host; none without a remote or a PR', async () => {
    const { dir, wt } = setup()
    expect(await pullRequest(wt, 'work/1-x')).toEqual({ pr: null, why: 'This space has no GitHub remote.' })
    sh('git remote add origin http://github.localhost/acme/gtm.git', wt)
    // a stand-in gh: says what it was asked and on which host, and has a PR for one branch only
    const bin = join(dir, 'bin')
    mkdirSync(bin)
    writeFileSync(
      join(bin, 'gh'),
      `#!/bin/sh\necho "$GH_HOST $*" >> ${join(dir, 'asked')}\nif [ "$3" = "work/1-x" ]; then echo '{"number":7,"title":"X","state":"OPEN","isDraft":true,"url":"https://github.localhost/acme/gtm/pull/7","statusCheckRollup":[{"name":"t","status":"COMPLETED","conclusion":"SUCCESS"}]}'; else echo 'no pull requests found for branch "'$3'"' >&2; exit 1; fi\n`,
    )
    chmodSync(join(bin, 'gh'), 0o755)
    const path = process.env.PATH
    process.env.PATH = `${bin}:${path}`
    try {
      expect(await pullRequest(wt, 'work/1-x')).toEqual({ pr: { number: 7, title: 'X', state: 'draft', url: 'https://github.localhost/acme/gtm/pull/7', checks: [{ name: 't', state: 'pass', url: undefined }] } })
      expect(await pullRequest(wt, 'work/2-y')).toEqual({ pr: null, why: 'No pull request yet.' })
      expect(sh(`cat ${join(dir, 'asked')}`, dir).split('\n')[0]).toBe('github.localhost pr view work/1-x -R acme/gtm --json number,title,state,isDraft,url,statusCheckRollup')
    } finally {
      process.env.PATH = path
    }
    rmSync(dir, { recursive: true, force: true })
  })
  test("a token that can't read checks: the pull request's Actions jobs instead", async () => {
    const { dir, wt } = setup()
    sh('git remote add origin http://github.localhost/acme/gtm.git', wt)
    // a stand-in gh refused the checks, as GitHub refuses a fine-grained token on a private repo
    const bin = join(dir, 'bin')
    mkdirSync(bin)
    writeFileSync(
      join(bin, 'gh'),
      `#!/bin/sh
case "$*" in
  *statusCheckRollup*) echo 'GraphQL: Resource not accessible by personal access token (repository.pullRequest.statusCheckRollup)' >&2; exit 1 ;;
  *headRefOid*) echo '{"number":7,"title":"X","state":"OPEN","isDraft":false,"url":"https://github.localhost/acme/gtm/pull/7","headRefOid":"abc"}' ;;
  *"actions/runs?head_sha=abc"*) echo 41 ;;
  *actions/runs/41/jobs*) echo '{"name":"check","status":"completed","conclusion":"failure","html_url":"https://j/1"}'; echo '{"name":"e2e","status":"queued","conclusion":null,"html_url":"https://j/2"}' ;;
  *) exit 1 ;;
esac
`,
    )
    chmodSync(join(bin, 'gh'), 0o755)
    const path = process.env.PATH
    process.env.PATH = `${bin}:${path}`
    try {
      expect((await pullRequest(wt, 'work/1-x')).pr?.checks).toEqual([
        { name: 'check', state: 'fail', url: 'https://j/1' },
        { name: 'e2e', state: 'pending', url: 'https://j/2' },
      ])
    } finally {
      process.env.PATH = path
    }
    rmSync(dir, { recursive: true, force: true })
  })
})

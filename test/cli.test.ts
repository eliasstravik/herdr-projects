// The shepherdr command, here (server/cli.ts) and on another machine (the helper's shepherdr, over the SSH tunnel's
// main.sock): shepherdr merge never waits on and on for an answer that isn't coming, as when Shepherdr restarted
// halfway or the tunnel dropped and left its end open (SHE-212's agent sat on one for 40 minutes).
import { describe, expect, test } from 'bun:test'
import { spawn, spawnSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, statSync, symlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const root = join(import.meta.dir, '..')
const perl = spawnSync('perl', ['-MJSON::PP', '-e1']).status === 0

/** run it to its end: what it printed, its exit code, and how long it took */
function run(argv: string[], env: Record<string, string>) {
  return new Promise<{ out: string; err: string; code: number | null; ms: number }>((done) => {
    const at = Date.now()
    const c = spawn(argv[0], argv.slice(1), { stdio: ['ignore', 'pipe', 'pipe'], env: { ...process.env, ...env } })
    let out = ''
    let err = ''
    c.stdout!.on('data', (d) => (out += d))
    c.stderr!.on('data', (d) => (err += d))
    c.on('close', (code) => done({ out, err, code, ms: Date.now() - at }))
  })
}

const noAnswer = (id: string) => [`shepherdr merge ${id} got no answer from Shepherdr in 1 s`, `shepherdr task show ${id}`, `run shepherdr merge ${id} again`]

describe('shepherdr merge with no answer coming', () => {
  test('here: it gives up after its wait and says what to do', async () => {
    // takes the call and never answers
    const server = Bun.serve({ hostname: '127.0.0.1', port: 0, idleTimeout: 0, fetch: () => new Promise<Response>(() => {}) })
    try {
      const env = { SHEPHERDR_URL: `http://127.0.0.1:${server.port}`, SHEPHERDR_MERGE_WAIT: '1000' }
      const r = await run([process.execPath, join(root, 'server', 'cli.ts'), 'merge', 'SHE-9'], env)
      expect(r.code).toBe(1)
      for (const words of noAnswer('SHE-9')) expect(r.err).toContain(words)
      expect(r.ms).toBeLessThan(8000)
      // task merge, its other name
      expect((await run([process.execPath, join(root, 'server', 'cli.ts'), 'task', 'merge', 'SHE-9'], env)).err).toContain(noAnswer('SHE-9')[0])
    } finally {
      server.stop(true)
    }
  })

  test.if(perl)('on another machine: the open end of a dropped tunnel ends it too', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-cli-'))
    // main.sock as a dropped tunnel left it: it takes the call and never answers, nor closes
    const server = Bun.listen({ unix: join(dir, 'main.sock'), socket: { data() {} } })
    try {
      const helper = join(dir, 'bin', 'shepherdr')
      mkdirSync(join(dir, 'bin'))
      copyFileSync(join(root, 'helper', 'shepherdr'), helper)
      const r = await run(['perl', helper, 'merge', 'SHE-9'], { SHEPHERDR_MERGE_WAIT: '1000' })
      expect(r.code).toBe(1)
      for (const words of noAnswer('SHE-9')) expect(r.err).toContain(words)
      expect(r.ms).toBeLessThan(8000)
    } finally {
      server.stop(true)
    }
  })
})

describe('shepherdr secret run and env', () => {
  // Shepherdr's answer to secret run and env: the values, for the command to use here; it never prints them
  const values = { TOKEN: "a b'c$d", PLAIN: 'xyz' }
  const answer = async (req: Request) => {
    const b = (await req.json()) as { args: string[]; stdin?: string }
    return Response.json({ out: '', code: 0, ...(b.args[1] === 'run' || b.args[1] === 'env' ? { secrets: values } : {}), args: b.args, stdin: b.stdin ?? null })
  }

  /** each shepherdr, here and on another machine, against the answer above */
  async function each(check: (cmd: string[], cwd: string) => Promise<void>) {
    const server = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch: answer })
    const dir = mkdtempSync(join(tmpdir(), 'shep-sec-'))
    mkdirSync(join(dir, 'bin'))
    copyFileSync(join(root, 'helper', 'shepherdr'), join(dir, 'bin', 'shepherdr'))
    const sock = perl ? Bun.serve({ unix: join(dir, 'main.sock'), fetch: answer }) : undefined
    process.env.SHEPHERDR_URL = `http://127.0.0.1:${server.port}`
    try {
      const repo = join(dir, 'repo')
      mkdirSync(repo)
      spawnSync('git', ['init', '-q'], { cwd: repo })
      writeFileSync(join(repo, '.gitignore'), '.env\n')
      await check([process.execPath, join(root, 'server', 'cli.ts')], repo)
      if (perl) await check(['perl', join(dir, 'bin', 'shepherdr')], repo)
    } finally {
      delete process.env.SHEPHERDR_URL
      server.stop(true)
      sock?.stop(true)
    }
  }

  /** run to its end, never blocking: Shepherdr's answer comes from this same process */
  const sh = (argv: string[], cwd: string) =>
    new Promise<{ stdout: string; stderr: string; status: number | null }>((done) => {
      const c = spawn(argv[0], argv.slice(1), { cwd, stdio: ['ignore', 'pipe', 'pipe'], env: process.env })
      let stdout = ''
      let stderr = ''
      c.stdout!.on('data', (d) => (stdout += d))
      c.stderr!.on('data', (d) => (stderr += d))
      c.on('close', (status) => done({ stdout, stderr, status }))
    })

  test('run: the command gets each one in its environment, what it prints is masked, and its exit code goes back', async () => {
    await each(async (cmd, cwd) => {
      const check = `[ "$TOKEN" = "${values.TOKEN.replace(/\$/g, '\\$')}" ] && echo ok`
      // printed whole, split over two writes, and to stderr; one under 4 characters stays
      const script = `${check}; printf "[%s][%s]" "$TOKEN" "$PLAIN"; printf "a b"; sleep 0.2; printf "%s" "'c\\$d."; echo "$TOKEN" >&2; exit 3`
      const r = await sh([...cmd, 'secret', 'run', 'TOKEN', '--', 'sh', '-c', script], cwd)
      expect(r.stdout).toBe('ok\n[********][xyz]********.')
      expect(r.stderr).toBe('********\n')
      expect(r.status).toBe(3)
      // no command: nothing asked
      expect((await sh([...cmd, 'secret', 'run', 'TOKEN'], cwd)).stderr).toContain('Put the command after --')
    })
  })

  test('env: NAME=value lines in a file git ignores, readable by you only; refused where git would keep it', async () => {
    await each(async (cmd, cwd) => {
      writeFileSync(join(cwd, '.env'), 'OTHER=1\nexport TOKEN=old\n')
      const r = await sh([...cmd, 'secret', 'env', 'TOKEN', 'PLAIN'], cwd)
      expect(r.status).toBe(0)
      expect(r.stdout).not.toContain(values.TOKEN)
      const lines = readFileSync(join(cwd, '.env'), 'utf8').trim().split('\n').sort()
      expect(lines).toEqual(['OTHER=1', 'PLAIN=xyz', 'TOKEN="a b\'c\\$d"'])
      expect(statSync(join(cwd, '.env')).mode & 0o777).toBe(0o600)
      const kept = await sh([...cmd, 'secret', 'env', 'TOKEN', '--file', 'kept.env'], cwd)
      expect(kept.status).toBe(1)
      expect(kept.stdout).toContain('not ignored by git')
      expect(existsSync(join(cwd, 'kept.env'))).toBe(false)
      // a link, ignored or not, would carry it into the file it leads to
      writeFileSync(join(cwd, 'tracked.txt'), 'tracked\n')
      writeFileSync(join(cwd, '.gitignore'), '.env\nlink.env\n')
      if (!existsSync(join(cwd, 'link.env'))) symlinkSync('tracked.txt', join(cwd, 'link.env'))
      expect((await sh([...cmd, 'secret', 'env', 'TOKEN', '--file', 'link.env'], cwd)).stdout).toContain('is a link')
      expect(readFileSync(join(cwd, 'tracked.txt'), 'utf8')).toBe('tracked\n')
    })
  })
})

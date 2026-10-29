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

describe.if(perl)('session-owned command routing', () => {
  // A real child process owns each binding. Neither cwd nor an inherited pane id identifies a session.
  async function session(route: Record<string, string>, commands: string[][], extra: Record<string, string> = {}) {
    const home = mkdtempSync(join(tmpdir(), 'shep-routing-'))
    const helperDir = join(home, '.shepherdr', 'helper')
    mkdirSync(join(helperDir, 'bin'), { recursive: true })
    copyFileSync(join(root, 'helper', 'route.pl'), join(helperDir, 'route.pl'))
    copyFileSync(join(root, 'helper', 'shepherdr'), join(helperDir, 'bin', 'shepherdr'))
    // Copy keeps the executable mode of the source helper.
    const answer = async (req: Request) => Response.json({ out: JSON.stringify(await req.json()) + '\n', code: 0 })
    const socket = Bun.serve({ unix: join(helperDir, 'main.sock'), fetch: answer })
    const spec = route.helper === 'installed' ? { ...route, helper: join(helperDir, 'bin', 'shepherdr') } : route
    const code = `
      const router = ${JSON.stringify(join(root, 'helper', 'route.pl'))};
      const spec = ${JSON.stringify(spec)};
      function bind() {
        const p = Bun.spawnSync(['perl', router, '--bind', String(process.pid), JSON.stringify(spec)]);
        if (p.exitCode) throw new Error(p.stderr.toString());
      }
      bind();
      for (const args of ${JSON.stringify(commands)}) {
        if (args[0] === 'rebind') { spec.chat = args[1]; bind(); continue; }
        const p = Bun.spawnSync(args, { env: { ...process.env }, stdout: 'pipe', stderr: 'pipe' });
        process.stdout.write(p.stdout); process.stderr.write(p.stderr);
        if (p.exitCode) process.exit(p.exitCode);
      }
    `
    try {
      return await run([process.execPath, '-e', code], {
        HOME: home, SHEPHERDR_HOME: join(home, '.shepherdr'),
        SHEPHERDR_CHAT: 'ST-1/Reviewer', SHEPHERDR_URL: 'http://127.0.0.1:7787', HERDR_PANE_ID: 'another-pane', ...extra,
      })
    } finally { socket.stop(true) }
  }

  test('repo command before helper, stale URL/chat and login shell all reach the remote owner', async () => {
    const command = join(root, 'bin', 'shepherdr')
    const r = await session({ helper: 'installed', chat: 'SHE-9/Agent' }, [
      [command, 'show'],
      ['/bin/bash', '-lc', `PATH=${JSON.stringify(join(root, 'bin'))}:$PATH shepherdr show`],
      ['rebind', 'space-key'], [command, 'show'],
      ['rebind', 'SHE-10/Reviewer'], [command, 'show'],
    ])
    expect(r.code).toBe(0)
    expect(r.err).toBe('')
    expect(r.out.trim().split('\n').map((l) => JSON.parse(l).chat)).toEqual(['SHE-9/Agent', 'SHE-9/Agent', 'space-key', 'SHE-10/Reviewer'])
  })

  test('local live owner replaces a stale demo URL, while explicit dev/demo targets stay selected', async () => {
    const live = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch: async (req) => Response.json({ out: 'live ' + (await req.json() as { chat: string }).chat + '\n', code: 0 }) })
    const demo = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch: () => Response.json({ out: 'explicit demo\n', code: 0 }) })
    const url = `http://127.0.0.1:${demo.port}`
    const command = join(root, 'bin', 'shepherdr')
    try {
      const r = await session({ url: `http://127.0.0.1:${live.port}`, chat: 'SHE-9/Agent' }, [
        [command, 'show'],
        [command, '--url', url, 'show'],
        ['env', `SHEPHERDR_URL=${url}`, command, 'show'],
        ['env', `SHEPHERDR_URL=${url}`, '/bin/bash', '-lc', `${JSON.stringify(command)} show`],
        ['env', 'SHEPHERDR_CHAT=SHE-9/Reviewer', command, 'show'],
      ])
      expect(r.code).toBe(0)
      expect(r.out).toBe('live SHE-9/Agent\nexplicit demo\nexplicit demo\nexplicit demo\nlive SHE-9/Reviewer\n')
    } finally { live.stop(true); demo.stop(true) }
  })

  test('explicit unrelated URL is never redirected to the remote helper', async () => {
    const server = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch: () => Response.json({ out: 'explicit local\n', code: 0 }) })
    try {
      const r = await session({ helper: 'installed', chat: 'space-key' }, [
        ['env', `SHEPHERDR_URL=http://127.0.0.1:${server.port}`, join(root, 'bin', 'shepherdr'), 'show'],
      ])
      expect(r.code).toBe(0)
      expect(r.out).toBe('explicit local\n')
    } finally { server.stop(true) }
  })

  test('shared executor tools use the live owning folder, ignoring another session’s env and pane id', async () => {
    const home = mkdtempSync(join(tmpdir(), 'shep-shared-executor-'))
    const folder = join(home, 'worktree')
    mkdirSync(folder)
    const other = join(home, 'coordinator')
    mkdirSync(other)
    const server = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch: async (req) => Response.json({ out: (await req.json() as { chat: string }).chat + '\n', code: 0 }) })
    const code = `
      // This owner is a sibling, not an ancestor of the tool subprocess.
      const owner = Bun.spawn(['/bin/sleep', '20'], { env: { ...process.env, SHEPHERDR_CHAT: 'SHE-9/Agent', SHEPHERDR_URL: '' } });
      try {
        const b = Bun.spawnSync(['perl', ${JSON.stringify(join(root, 'helper', 'route.pl'))}, '--bind', String(owner.pid), JSON.stringify({cwd:${JSON.stringify(folder)}, chat:'SHE-9/Agent', url:${JSON.stringify(`http://127.0.0.1:${server.port}`)}})]);
        if (b.exitCode) throw new Error(b.stderr.toString());
        const first = Bun.spawnSync([${JSON.stringify(join(root, 'bin', 'shepherdr'))}, 'show'], {cwd:${JSON.stringify(folder)}});
        if (first.exitCode) throw new Error(first.stderr.toString());
        process.stdout.write(first.stdout);
        const b2 = Bun.spawnSync(['perl', ${JSON.stringify(join(root, 'helper', 'route.pl'))}, '--bind', String(process.pid), JSON.stringify({cwd:${JSON.stringify(other)}, chat:'space-key', url:${JSON.stringify(`http://127.0.0.1:${server.port}`)}})]);
        if (b2.exitCode) throw new Error(b2.stderr.toString());
        // Even with a binding for its original chat, the shared daemon is not the task's CLI ancestor.
        const unchanged = Bun.spawnSync([${JSON.stringify(join(root, 'bin', 'shepherdr'))}, 'show'], {cwd:${JSON.stringify(folder)}});
        if (unchanged.exitCode) throw new Error(unchanged.stderr.toString());
        process.stdout.write(unchanged.stdout);
        const p = Bun.spawnSync(['env', 'SHEPHERDR_CHAT=SHE-9/Agent', ${JSON.stringify(join(root, 'bin', 'shepherdr'))}, 'show'], {cwd:${JSON.stringify(other)}});
        process.stdout.write(p.stdout); process.stderr.write(p.stderr); process.exitCode = p.exitCode;
      } finally { owner.kill(); await owner.exited; }
    `
    try {
      // A separate ordinary CLI started the shared daemon. Its binding is also above the ownership boundary.
      const parentCode = `
        const b = Bun.spawnSync(['perl', ${JSON.stringify(join(root, 'helper', 'route.pl'))}, '--bind', String(process.pid), JSON.stringify({cwd:${JSON.stringify(join(other, 'parent'))}, chat:'original-owner', url:${JSON.stringify(`http://127.0.0.1:${server.port}`)}})]);
        if (b.exitCode) throw new Error(b.stderr.toString());
        const p = Bun.spawnSync(['/bin/bash', '-c', 'exec -a codex "$@"', 'daemon', process.execPath, '-e', ${JSON.stringify(code)}, 'app-server']);
        process.stdout.write(p.stdout); process.stderr.write(p.stderr); process.exit(p.exitCode);
      `
      const r = await run(['/bin/bash', '-c', 'exec -a codex "$@"', 'original-cli', process.execPath, '-e', parentCode], { HOME: home, SHEPHERDR_CHAT: 'ST-1/Reviewer', SHEPHERDR_URL: 'http://127.0.0.1:7787', HERDR_PANE_ID: 'stale-pane' })
      expect(r.code).toBe(0)
      expect(r.out).toBe('SHE-9/Agent\nSHE-9/Agent\nSHE-9/Agent\n')
    } finally { server.stop(true) }
  })

  test('a reused pid with another start time does not inherit a route', async () => {
    const server = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch: () => Response.json({ out: 'explicit local\n', code: 0 }) })
    const home = mkdtempSync(join(tmpdir(), 'shep-reused-pid-'))
    const dir = join(home, '.shepherdr', 'helper', 'routes')
    mkdirSync(dir, { recursive: true })
    const code = `
      await Bun.write(${JSON.stringify(dir)} + '/' + process.pid + '.json', JSON.stringify({stamp:'wrong start', chat:'wrong', helper:'/missing'}));
      const r = Bun.spawnSync([${JSON.stringify(join(root, 'bin', 'shepherdr'))}, 'show']);
      process.stdout.write(r.stdout); process.exit(r.exitCode);
    `
    try {
      const r = await run([process.execPath, '-e', code], { HOME: home, SHEPHERDR_URL: `http://127.0.0.1:${server.port}` })
      expect(r.code).toBe(0)
      expect(r.out).toBe('explicit local\n')
    } finally { server.stop(true) }
  })
})

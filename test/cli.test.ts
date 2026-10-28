// The shepherdr command, here (server/cli.ts) and on another machine (the helper's shepherdr, over the SSH tunnel's
// main.sock): shepherdr merge never waits on and on for an answer that isn't coming, as when Shepherdr restarted
// halfway or the tunnel dropped and left its end open (SHE-212's agent sat on one for 40 minutes).
import { describe, expect, test } from 'bun:test'
import { spawn, spawnSync } from 'node:child_process'
import { copyFileSync, mkdirSync, mkdtempSync } from 'node:fs'
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

import { expect, test } from 'bun:test'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { prepareHome } from '../server/core/home.ts'
import { buildProblem, launcherStatus } from '../server/core/launcher.ts'

test('a live unconfirmed process never reports stopped or gets signaled', async () => {
  const home = mkdtempSync(join(tmpdir(), 'shep-launcher-'))
  const server = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch: () => new Response('not ready', { status: 503 }) })
  try {
    prepareHome(home)
    writeFileSync(join(home, 'run.json'), JSON.stringify({ pid: process.pid, boot: 'starting', home, port: server.port, root: '/unused' }), { mode: 0o600 })
    const text = await launcherStatus({ home, port: server.port!, binary: 'hc', session: 'test', socket: '/unused' })
    expect(text).toContain('process is alive')
    expect(text).not.toContain('is stopped')
    expect(text).toContain('not confirmed yet')
    expect(() => process.kill(process.pid, 0)).not.toThrow()
  } finally { server.stop(true); rmSync(home, { recursive: true, force: true }) }
})

test('start refuses an unbuilt application and names the step; a Vite preview serves its own page', () => {
  const root = mkdtempSync(join(tmpdir(), 'shep-launcher-build-'))
  try {
    expect(buildProblem({}, root)).toContain(`bun run build in ${root}`)
    expect(buildProblem({ plugins: false }, root)).toBeUndefined()
    mkdirSync(join(root, 'dist')); writeFileSync(join(root, 'dist/index.html'), '<!doctype html>')
    expect(buildProblem({}, root)).toBeUndefined()
  } finally { rmSync(root, { recursive: true, force: true }) }
})

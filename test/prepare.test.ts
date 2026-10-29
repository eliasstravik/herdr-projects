import { afterEach, expect, test } from 'bun:test'
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { dependenciesPresent, prepare } from '../scripts/prepare.ts'

const dirs: string[] = []
afterEach(() => { for (const dir of dirs.splice(0)) rmSync(dir, { recursive: true, force: true }) })

// All installation/build effects stay in a scratch checkout. No live install, server or state is involved.
function fixture(mode = '') {
  const root = mkdtempSync(join(tmpdir(), 'shep-prepare-'))
  dirs.push(root)
  writeFileSync(join(root, 'package.json'), JSON.stringify({ dependencies: { example: '1.0.0' } }))
  writeFileSync(join(root, 'mode'), mode)
  const bun = join(root, 'bun')
  writeFileSync(bun, `#!${process.execPath}\n
import { appendFileSync, existsSync, mkdirSync, writeFileSync, readFileSync } from 'node:fs';
const args = process.argv.slice(2);
appendFileSync('calls', args.join(' ') + '\\n');
const mode = readFileSync('mode', 'utf8');
if (args[0] === 'install') {
  if (mode === 'install fails') { console.error('registry unavailable'); process.exit(1); }
  writeFileSync('installed', 'yes');
  if (mode !== 'install incomplete') {
    mkdirSync('node_modules/example', { recursive: true });
    writeFileSync('node_modules/example/package.json', JSON.stringify({ main: 'index.js' }));
    writeFileSync('node_modules/example/index.js', '');
  }
} else if (args[0] === 'run') {
  if (mode === 'build fails') { console.error('broken screen'); process.exit(1); }
  if (mode !== 'no screen') { mkdirSync('dist', { recursive: true }); writeFileSync('dist/index.html', 'screen'); }
} else if (mode === 'nested missing' && !existsSync('installed')) { console.error('missing nested import'); process.exit(1);
} else if (mode === 'server fails') { console.error('missing server import'); process.exit(1); }
`)
  chmodSync(bun, 0o755)
  const calls = () => existsSync(join(root, 'calls')) ? readFileSync(join(root, 'calls'), 'utf8') : ''
  return { root, bun, calls }
}

test('clean first start installs before building and checks the server', async () => {
  const f = fixture()
  expect(await prepare(f.root, f.bun)).toBeNull()
  expect(f.calls().split('\n').slice(0, 2)).toEqual(['install --frozen-lockfile', 'run build'])
  expect(f.calls()).toContain('build ')
  expect(existsSync(join(f.root, 'dist/index.html'))).toBe(true)
})

test('an empty node_modules is incomplete, including when a screen exists', async () => {
  const f = fixture()
  mkdirSync(join(f.root, 'node_modules'))
  mkdirSync(join(f.root, 'dist'))
  writeFileSync(join(f.root, 'dist/index.html'), 'old screen')
  expect(dependenciesPresent(f.root)).toBe(false)
  expect(await prepare(f.root, f.bun)).toBeNull()
  expect(f.calls()).toStartWith('install --frozen-lockfile\nrun build\n')
})

test('a package manifest without its entry file is incomplete', async () => {
  const f = fixture()
  mkdirSync(join(f.root, 'node_modules/example'), { recursive: true })
  writeFileSync(join(f.root, 'node_modules/example/package.json'), '{"main":"index.js"}')
  expect(dependenciesPresent(f.root)).toBe(false)
  expect(await prepare(f.root, f.bun)).toBeNull()
})

for (const [mode, message] of [
  ['install fails', 'registry unavailable'],
  ['install incomplete', 'without the required dependencies'],
  ['build fails', 'broken screen'],
  ['no screen', 'the build failed'],
  ['server fails', "server code doesn't load"],
]) test(mode + ' stops preparation clearly', async () => {
  const f = fixture(mode)
  expect(await prepare(f.root, f.bun)).toContain(message)
  if (mode.startsWith('install')) expect(f.calls()).toBe('install --frozen-lockfile\n')
})

test('a missing installer is a clear failure', async () => {
  const f = fixture()
  expect(await prepare(f.root, join(f.root, 'missing-bun'))).toContain('bun install failed')
})

test('healthy starts do not install or build again', async () => {
  const f = fixture()
  expect(await prepare(f.root, f.bun)).toBeNull()
  const calls = f.calls()
  expect(await prepare(f.root, f.bun)).toBeNull()
  expect(f.calls().slice(calls.length)).toStartWith('build ')
  expect(f.calls().slice(calls.length)).not.toContain('install')
  expect(f.calls().slice(calls.length)).not.toContain('run build')
})

test('failed preparation leaves an existing server answering and reports failure', async () => {
  const f = fixture()
  // Real lifecycle command with a scratch checkout; a separate disposable process represents the old server.
  writeFileSync(join(f.root, 'package.json'), JSON.stringify({ version: 'new', scripts: { build: 'echo broken-checkout >&2; exit 1' } }))
  const server = join(f.root, 'old-server.ts')
  writeFileSync(server, `const s = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch(req) {
    return Response.json(req.url.endsWith('/api/version') ? { root: '/old-checkout', version: 'old', supervisor: process.pid } : {});
  } }); console.log(s.port);`)
  const old = Bun.spawn([process.execPath, server], { stdout: 'pipe', stderr: 'pipe' })
  try {
    const reader = old.stdout.getReader()
    const first = await reader.read()
    const port = Number(new TextDecoder().decode(first.value).trim())
    reader.releaseLock()
    const command = Bun.spawn([process.execPath, join(import.meta.dir, '../scripts/plugin.ts'), 'start'], {
      env: { ...process.env, HOME: f.root, SHEPHERDR_HOME: join(f.root, 'home'), HERDR_PLUGIN_ROOT: f.root,
        HERDR_PLUGIN_STATE_DIR: join(f.root, 'state'), HERDR_PLUGIN_ID: '', SHEPHERDR_PORT: String(port) },
      stdout: 'pipe', stderr: 'pipe',
    })
    const [code, stderr] = await Promise.all([command.exited, new Response(command.stderr).text()])
    expect(code).toBe(1)
    expect(stderr).toContain('Shepherdr preparation failed')
    expect(stderr).toContain('broken-checkout')
    expect(old.exitCode).toBeNull()
    expect((await fetch(`http://127.0.0.1:${port}/api/snapshot`)).ok).toBe(true)
    expect(existsSync(join(f.root, 'state/supervisor.pid'))).toBe(false)
  } finally {
    old.kill()
    await old.exited
  }
})


test('an exports-only package with missing files is incomplete', async () => {
  const f = fixture()
  mkdirSync(join(f.root, 'node_modules/example'), { recursive: true })
  writeFileSync(join(f.root, 'node_modules/example/package.json'), '{"exports":{".":"./dist/index.js"}}')
  expect(dependenciesPresent(f.root)).toBe(false)
  expect(await prepare(f.root, f.bun)).toBeNull()
  expect(f.calls()).toStartWith('install --frozen-lockfile\n')
})

test('a missing nested server import is repaired even with an existing screen and package entry', async () => {
  const f = fixture('nested missing')
  mkdirSync(join(f.root, 'dist'))
  writeFileSync(join(f.root, 'dist/index.html'), 'screen')
  mkdirSync(join(f.root, 'node_modules/example'), { recursive: true })
  writeFileSync(join(f.root, 'node_modules/example/package.json'), '{"main":"index.js"}')
  writeFileSync(join(f.root, 'node_modules/example/index.js'), '')
  expect(dependenciesPresent(f.root)).toBe(true)
  expect(await prepare(f.root, f.bun)).toBeNull()
  expect(f.calls()).toContain('install --frozen-lockfile\nrun build\n')
})

// Agents' secrets: kept at a level (Everywhere, a machine, a space) in a file only you can read, picked most specific
// first, asked for with a masked card, and never in a snapshot, a chat or the command's printed answer.
import { afterAll, describe, expect, test } from 'bun:test'
import { mkdtempSync, readFileSync, statSync, symlinkSync, mkdirSync, copyFileSync, unlinkSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawn } from 'node:child_process'
import { randomUUID } from 'node:crypto'

process.env.SHEPHERDR_HOME = mkdtempSync(join(tmpdir(), 'shep-secrets-'))
process.env.HERDR_SOCKET_PATH = join(process.env.SHEPHERDR_HOME, 'no-herdr.sock')
delete process.env.SHEPHERDR_MAIN
const { Vault, secretPages } = await import('../server/live/secrets.ts')
const { LiveSource } = await import('../server/live/source.ts')
const { placeId } = await import('../server/live/places.ts')

describe('the vault', () => {
  test('the most specific level wins, and its file is readable by you only', () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-vault-'))
    const v = new Vault(dir)
    expect(v.set('all', 'TOKEN', 'everywhere').ok).toBe(true)
    expect(v.set('p:box:api', 'TOKEN', 'for api').ok).toBe(true)
    expect(v.set('m:box', 'OTHER', 'box').ok).toBe(true)
    const chain = ['p:box:api', 'm:box', 'all']
    expect(v.pick(['TOKEN', 'OTHER', 'NONE'], chain)).toEqual({ found: { TOKEN: { value: 'for api', level: 'p:box:api' }, OTHER: { value: 'box', level: 'm:box' } }, missing: ['NONE'] })
    expect(v.pick(['TOKEN'], ['p:box:web', 'm:box', 'all']).found.TOKEN.value).toBe('everywhere')
    expect(v.visible(chain).map((x) => [x.name, x.level])).toEqual([
      ['OTHER', 'm:box'],
      ['TOKEN', 'p:box:api'],
    ])
    expect(statSync(join(dir, 'vault.json')).mode & 0o777).toBe(0o600)
    // removed: the level above counts again
    expect(v.set('p:box:api', 'TOKEN', null).ok).toBe(true)
    expect(v.pick(['TOKEN'], chain).found.TOKEN.value).toBe('everywhere')
    expect(v.set('p:box:api', 'TOKEN', null).ok).toBe(false)
  })

  test('a name is one an environment variable may have, and a value is not empty', () => {
    const v = new Vault('', {})
    expect(v.set('all', 'BAD NAME', 'x').ok).toBe(false)
    expect(v.set('all', '1TOKEN', 'x').ok).toBe(false)
    expect(v.set('all', 'TOKEN', '').ok).toBe(false)
    expect(v.set('nowhere', 'TOKEN', 'x').ok).toBe(false)
    expect(v.set('all', 'github_token', 'x').ok).toBe(true)
  })

  test("Settings gets a Secrets page for each level, with names and never a value", () => {
    const v = new Vault('', { all: { A: { value: 'shh-1', at: 1 } }, 'p:box:api': { B: { value: 'shh-2', at: 2 } } })
    const pages = secretPages([{ name: 'Tasks', rows: [], level: 'all' }, { name: 'Tasks', rows: [], level: 'p:box:api', machine: 'box' }, { name: 'Alerts', rows: [], global: true }], v.names())
    const secrets = pages.filter((p) => p.secrets)
    expect(secrets.map((p) => [p.level, p.machine, p.secrets!.map((s) => s.name)])).toEqual([
      ['all', undefined, ['A']],
      ['p:box:api', 'box', ['B']],
    ])
    expect(JSON.stringify(pages)).not.toContain('shh-')
  })
})

describe('the shepherdr secret command', () => {
  const src = new LiveSource()
  afterAll(() => src.stop())
  const home = placeId.home()
  let proof = src.agents.secretAccess.issue(home, 'fake-process', [home, placeId.machine(), 'all'])

  test('an agent asks with a card, and is told it is kept already', async () => {
    const r = await src.cli(['secret', 'ask', 'API_KEY', '--why', 'To call the API'], { chat: home, proof })
    expect(r.code).toBe(0)
    const card = src.chats.log(home).posted.find((p) => p.secret?.name === 'API_KEY')!.secret!
    expect(card.why).toBe('To call the API')
    expect(card.levels.map((l) => l.id)).toEqual([home, placeId.machine(), 'all'])
    // not from a terminal: there is no chat to show the card in
    expect((await src.cli(['secret', 'ask', 'API_KEY'], {})).code).toBe(1)

    // a paste's line end goes; anything else is the secret's own
    expect(await src.act({ type: 'secret', level: placeId.machine(), name: 'API_KEY', value: 'k-123456\n' })).toEqual({ ok: true })
    await src.act({ type: 'secret', level: placeId.machine(), name: 'SPACED', value: ' pass word ' })
    expect((await src.cli(['secret', 'run', 'SPACED'], { chat: home, proof })).secrets).toEqual({ SPACED: ' pass word ' })
    expect((await src.cli(['secret', 'ask', 'API_KEY'], { chat: home, proof })).out).toContain('kept already')
    expect((await src.cli(['secret', 'ask', 'API_KEY', '--force'], { chat: home, proof })).code).toBe(0)
  })

  test('run and env get the values; nothing else prints or shows one', async () => {
    await src.act({ type: 'secret', level: 'all', name: 'OTHER', value: 'o-987654' })
    const r = await src.cli(['secret', 'run', 'API_KEY', 'OTHER'], { chat: home, proof })
    expect(r).toEqual({ out: '', code: 0, secrets: { API_KEY: 'k-123456', OTHER: 'o-987654' } })
    expect((await src.cli(['secret', 'env', 'API_KEY,OTHER', '--file', '.env'], { chat: home, proof })).secrets).toEqual({ API_KEY: 'k-123456', OTHER: 'o-987654' })
    const missing = await src.cli(['secret', 'run', 'NOPE'], { chat: home, proof })
    expect(missing.code).toBe(1)
    expect(missing.out).toContain('shepherdr secret ask NOPE')

    const list = await src.cli(['secret', 'list'], { chat: home, proof })
    expect(list.out).toContain('API_KEY')
    expect(list.out).toContain('OTHER  (Everywhere')
    const shown = JSON.stringify([list, src.snapshot(), src.chats.log(home)])
    expect(shown).not.toContain('k-123456')
    expect(shown).not.toContain('o-987654')
    expect(src.snapshot().settings.find((x) => x.name === 'Secrets' && x.level === 'all')?.secrets?.map((s) => s.name)).toEqual(['OTHER'])
    expect(readFileSync(join(process.env.SHEPHERDR_HOME!, 'vault.json'), 'utf8')).toContain('k-123456')
  })

  test('headers and localhost never replace proof, and body scope cannot broaden it', async () => {
    const { startApp } = await import('../server/app.ts')
    const { createServer, connect } = await import('node:net')
    const server = startApp({ source: src, dist: '/nonexistent', port: 0 })
    const proxy = createServer((incoming) => {
      const upstream = connect(server.port!, '127.0.0.1')
      incoming.pipe(upstream).pipe(incoming)
      incoming.on('error', () => upstream.destroy())
      upstream.on('error', () => incoming.destroy())
    })
    await new Promise<void>((done) => proxy.listen(0, '127.0.0.1', done))
    const address = proxy.address() as { port: number }
    const request = async (port: number, headers: Record<string, string> = {}, body = {}) => {
      const r = await fetch(`http://127.0.0.1:${port}/api/cli`, { method: 'POST', headers: { 'content-type': 'application/json', ...headers }, body: JSON.stringify({ args: ['secret', 'run', 'API_KEY'], chat: home, ...body }) })
      return await r.json() as { code: number; secrets?: Record<string, string> }
    }
    try {
      for (const port of [server.port!, address.port]) {
        for (const headers of ([{}, { 'x-forwarded-for': '127.0.0.1', 'x-forwarded-host': 'localhost', forwarded: 'for=127.0.0.1', 'tailscale-user-login': 'owner' }, { authorization: `Bearer ${'0'.repeat(64)}` }] as Record<string, string>[])) {
          const r = await request(port, headers)
          expect(r.code).toBe(1)
          expect(r.secrets).toBeUndefined()
        }
        expect((await request(port, { authorization: `Bearer ${proof}` })).secrets).toEqual({ API_KEY: 'k-123456' })
      }
      await src.act({ type: 'secret', level: 'p:other:private', name: 'ONLY_OTHER', value: 'fake-other-secret' })
      const forged = await request(server.port!, { authorization: `Bearer ${proof}` }, { args: ['secret', 'env', 'ONLY_OTHER'], chat: 'p:other:private', cwd: '/another/space' })
      expect(forged.code).toBe(1)
      expect(forged.secrets).toBeUndefined()
      expect((await request(server.port!, { authorization: `Bearer ${proof}` }, { chat: 'p:other:private', cwd: '/another/space' })).secrets).toEqual({ API_KEY: 'k-123456' })
      const temporary = src.agents.secretAccess.issue('revoked', 'fake-process', ['all'])
      src.agents.secretAccess.revoke('revoked')
      expect((await request(server.port!, { authorization: `Bearer ${temporary}` })).code).toBe(1)
    } finally {
      proxy.close()
      server.stop(true)
    }
  })

  test('normal repo and remote helper secret run use private process credentials', async () => {
    const { startApp } = await import('../server/app.ts')
    const { SecretAccess } = await import('../server/live/secret-access.ts')
    const root = join(import.meta.dir, '..')
    const dir = mkdtempSync(join(tmpdir(), 'shep-auth-cli-'))
    const helper = join(dir, '.shepherdr', 'helper')
    mkdirSync(join(helper, 'bin'), { recursive: true })
    copyFileSync(join(root, 'helper', 'route.pl'), join(helper, 'route.pl'))
    copyFileSync(join(root, 'helper', 'shepherdr'), join(helper, 'bin', 'shepherdr'))
    const credentialFile = join(helper, 'credentials', randomUUID())
    const server = startApp({ source: src, dist: '/nonexistent', port: 0 })
    // main.sock is the same byte-preserving HTTP transport as the SSH remote forward.
    const socket = Bun.serve({ unix: join(helper, 'main.sock'), fetch: (req) => fetch(`http://127.0.0.1:${server.port}/api/cli`, { method: 'POST', headers: req.headers, body: req.body }) })
    const env = { ...process.env, HOME: dir, SHEPHERDR_CHAT: home, SHEPHERDR_SECRET_FILE: credentialFile, SHEPHERDR_URL: `http://127.0.0.1:${server.port}` }
    const bind = (capability: string, pid = process.pid) => {
      const r = Bun.spawnSync(['perl', join(root, 'helper', 'route.pl'), '--bind', String(pid)], { env, stdin: Buffer.from(JSON.stringify({ chat: home, cwd: dir, url: env.SHEPHERDR_URL, capability, credentialFile })) })
      expect(r.exitCode).toBe(0)
      expect(statSync(credentialFile).mode & 0o777).toBe(0o600)
      expect(statSync(join(helper, 'credentials')).mode & 0o777).toBe(0o700)
    }
    const run = (command: string[]) => new Promise<{ code: number | null; out: string }>((done) => {
      const child = spawn(command[0], [...command.slice(1), 'secret', 'run', 'API_KEY', '--', 'sh', '-c', '[ "$API_KEY" = "k-123456" ] && [ -z "$SHEPHERDR_SECRET_CAPABILITY" ] && echo authenticated'], { cwd: dir, env, stdio: ['ignore', 'pipe', 'pipe'] })
      let out = ''
      child.stdout.on('data', (data) => out += data)
      child.stderr.on('data', (data) => out += data)
      child.on('close', (code) => done({ code, out }))
    })
    try {
      bind(proof)
      for (const command of [[join(root, 'bin', 'shepherdr')], ['perl', join(helper, 'bin', 'shepherdr')]]) {
        expect(await run(command)).toEqual({ code: 0, out: 'authenticated\n' })
      }
      // Worker restart/repair rewrites the same inherited file reference with a fresh capability.
      const stale = proof
      src.agents.secretAccess.revoke(home)
      expect((await run(['perl', join(helper, 'bin', 'shepherdr')])).code).toBe(1)
      proof = src.agents.secretAccess.issue(home, 'new-worker-process', [home, placeId.machine(), 'all'])
      bind(proof)
      expect(src.agents.secretAccess.authenticate(stale)).toBeUndefined()
      expect(await run(['perl', join(helper, 'bin', 'shepherdr')])).toEqual({ code: 0, out: 'authenticated\n' })
      // A worker for a different machine cannot accept this worker's proof.
      expect(new SecretAccess().authenticate(proof)).toBeUndefined()
      // A forged chat/cwd may select routing, but never the other live process's credential.
      const owner = Bun.spawn([process.execPath, '-e', 'setInterval(() => {}, 1000)'], { env, stdout: 'ignore', stderr: 'ignore' })
      try {
        unlinkSync(join(helper, 'routes', `${process.pid}.json`))
        bind(proof, owner.pid)
        const fileReference = env.SHEPHERDR_SECRET_FILE
        env.SHEPHERDR_SECRET_FILE = ''
        expect((await run(['perl', join(helper, 'bin', 'shepherdr')])).code).toBe(1)
        env.SHEPHERDR_SECRET_FILE = fileReference
      } finally { owner.kill(); await owner.exited }
      bind('0'.repeat(64))
      for (const command of [[join(root, 'bin', 'shepherdr')], ['perl', join(helper, 'bin', 'shepherdr')]]) {
        const refused = await run(command)
        expect(refused.code).toBe(1)
        expect(refused.out).not.toContain('k-123456')
      }
    } finally {
      socket.stop(true)
      server.stop(true)
    }
  })

  test('artifact imports cannot publish vaults or credentials, even through links', async () => {
    const { Artifacts } = await import('../server/live/artifacts.ts')
    const dir = mkdtempSync(join(tmpdir(), 'shep-import-'))
    const vault = join(process.env.SHEPHERDR_HOME!, 'vault.json')
    symlinkSync(vault, join(dir, 'innocent.json'))
    expect(() => Artifacts.read(undefined, vault, dir)).toThrow('Private Shepherdr')
    expect(() => Artifacts.read(undefined, 'innocent.json', dir)).toThrow('Private Shepherdr')
  })

  test('the human sets and removes one from a terminal, its value on stdin', async () => {
    expect((await src.cli(['secret', 'set', 'TYPED'], {})).code).toBe(1)
    expect((await src.cli(['secret', 'set', 'TYPED', '--machine'], { stdin: 'v-1\n' })).out).toContain('Saved TYPED')
    expect((await src.cli(['secret', 'run', 'TYPED'], { chat: home, proof })).secrets).toEqual({ TYPED: 'v-1' })
    expect((await src.cli(['secret', 'rm', 'TYPED', '--machine'], {})).code).toBe(0)
    expect((await src.cli(['secret', 'run', 'TYPED'], { chat: home, proof })).code).toBe(1)
  })
})

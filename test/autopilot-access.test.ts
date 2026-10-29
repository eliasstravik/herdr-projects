// Real OS enforcement, with fake executables and fake data. CI installs bubblewrap; never real agents/accounts.
import { describe, expect, test } from 'bun:test'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { request } from 'node:http'
process.env.SHEPHERDR_HOME = mkdtempSync(join(tmpdir(), 'pilot-access-store-'))
process.env.HERDR_SOCKET_PATH = join(process.env.SHEPHERDR_HOME, 'absent.sock')
const { PilotRuns } = await import('../server/live/autopilot.ts')
const { prepareSandbox, sandboxArgs } = await import('../server/live/autopilot-sandbox.ts')

const bwrap = Bun.which('bwrap')
describe.skipIf(!bwrap)('Autopilot actual OS boundary', () => {
  test('cross-harness reviewer executable/login and fixed broker identity share the same boundary', async () => {
    const root = mkdtempSync(join(tmpdir(), 'pilot-reviewer-os-'))
    const providerHome = join(root, 'fake-login'), cwd = join(root, 'project')
    for (const dir of [join(providerHome, '.claude'), join(providerHome, '.codex'), cwd]) mkdirSync(dir, { recursive: true })
    writeFileSync(join(providerHome, '.claude/.credentials.json'), '{"fixture":"FAKE_CLAUDE"}')
    writeFileSync(join(providerHome, '.codex/auth.json'), '{"fixture":"FAKE_CODEX"}')
    const executables: Record<string,string> = {}
    for (const kind of ['claude', 'codex']) {
      const file = join(root, kind); executables[kind] = file
      writeFileSync(file, kind === 'claude' ? '#!/bin/sh\ntest -f "$HOME/.claude/.credentials.json" && codex --review\n' : `#!/bin/sh\ntest -f "$HOME/.codex/auth.json" && echo same-boundary > '${cwd}/reviewer-result'\n`, { mode: 0o700 })
    }
    const p = new PilotRuns(); p.runs = []
    const run = p.begin('fixture', { harness: { kind: 'claude' }, hosts: [], secrets: [], approvals: ['merge'], parallel: 1 }); run.state = 'on'; p.claim(run, 'OWN-1')
    const calls: { chat: string; cwd: string }[] = []
    const box = await prepareSandbox({ key: 'OWN-1/Agent', name: 'fixture', kind: 'claude', cwd, workspace: 'fixture' }, run, cwd, async (_args, ctx) => { calls.push(ctx); return { out: 'ok', code: 0 } }, 'codex', { providerHome, executable: (kind) => executables[kind] })
    try {
      expect(existsSync(join(box.logsHome, '.codex/auth.json'))).toBe(true)
      const child = Bun.spawn([box.wrapper], { stdout: 'pipe', stderr: 'pipe' })
      const [code, err] = await Promise.all([child.exited, new Response(child.stderr).text()])
      expect({ code, stderr: code ? err : '' }).toEqual({ code: 0, stderr: '' })
      expect(await Bun.file(join(cwd, 'reviewer-result')).text()).toBe('same-boundary\n')
      const send = (args: string[]) => new Promise<{ code: number }>((resolve, reject) => {
        const req = request({ socketPath: box.gateway!, path: '/api/cli', method: 'POST', headers: { 'content-type': 'application/json' } }, (res) => { let body = ''; res.on('data', (chunk) => body += chunk); res.on('end', () => resolve(JSON.parse(body))) })
        req.on('error', reject); req.end(JSON.stringify({ args, chat: 'OTHER/Agent', cwd: '/host', proof: 'fake' }))
      })
      expect((await send(['status'])).code).toBe(0)
      expect(calls).toHaveLength(1); expect(calls[0]).toMatchObject({ chat: 'OWN-1/Agent', cwd })
      expect((await send(['task', 'tell', 'OTHER-1', 'go'])).code).toBe(1)
      expect(calls).toHaveLength(1)
    } finally { await box.close() }
  })
  for (const kind of ['claude', 'codex']) test(`${kind} and its child shell cannot access host files, control files, Git metadata or host network`, async () => {
    const root = mkdtempSync(join(tmpdir(), 'shep-pilot-os-'))
    const repo = join(root, 'repo'), cwd = join(root, 'task'), state = join(root, 'state'), control = join(root, 'control')
    for (const p of [repo, cwd, state, control, join(cwd, '.git')]) mkdirSync(p, { recursive: true })
    const secret = join(root, 'fake-host-secret')
    writeFileSync(secret, 'FAKE NEVER A REAL SECRET')
    writeFileSync(join(control, 'policy'), 'immutable')
    for (const name of ['bun', kind]) writeFileSync(join(control, name), '')
    copyFileSync(join(import.meta.dir, '../helper/autopilot/bootstrap.ts'), join(control, 'bootstrap.ts'))
    const fake = join(root, kind)
    writeFileSync(fake, `#!/bin/sh
set -eu
printf '%s\\n' "$@" > '${cwd}/launch-args'
test ! -e '${secret}'
if echo bad > '${repo}/escape' 2>/dev/null; then exit 11; fi
if echo bad > '${cwd}/.git/config' 2>/dev/null; then exit 12; fi
if echo bad > /run/shepherdr/policy 2>/dev/null; then exit 13; fi
/bin/sh -c "test ! -e '${secret}' && echo child > '${cwd}/child'"
/run/shepherdr/bun -e 'try { await fetch("http://127.0.0.1:9", {signal: AbortSignal.timeout(200)}); process.exit(14) } catch {}'
echo safe > '${cwd}/result'
`, { mode: 0o700 })
    const args = sandboxArgs({ cwd, repo, state, control, home: '/home/fixture-agent', binary: fake, bun: process.execPath, kind, writable: true, git: [join(cwd, '.git')] })
    const child = Bun.spawn([bwrap!, ...args, '--model', 'fixture-model'], { stdout: 'pipe', stderr: 'pipe' })
    const [code, err] = await Promise.all([child.exited, new Response(child.stderr).text()])
    expect({ code, stderr: code ? err : '' }).toEqual({ code: 0, stderr: '' })
    expect(await Bun.file(join(cwd, 'result')).text()).toBe('safe\n')
    expect(await Bun.file(join(cwd, 'launch-args')).text()).toContain('fixture-model')
    expect(await Bun.file(join(control, 'policy')).text()).toBe('immutable')
    expect(await Bun.file(secret).text()).toBe('FAKE NEVER A REAL SECRET')
  })
})

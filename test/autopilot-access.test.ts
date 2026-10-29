// Real OS enforcement, with fake executables and fake data. CI installs bubblewrap; never real agents/accounts.
import { describe, expect, test } from 'bun:test'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, symlinkSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { request } from 'node:http'
process.env.SHEPHERDR_HOME = mkdtempSync(join(tmpdir(), 'pilot-access-store-'))
process.env.HERDR_SOCKET_PATH = join(process.env.SHEPHERDR_HOME, 'absent.sock')
const { PilotRuns } = await import('../server/live/autopilot.ts')
const { prepareSandbox, sandboxArgs, capturedTranscript } = await import('../server/live/autopilot-sandbox.ts')
const { isolatedShellCommand } = await import('../server/live/agents.ts')

const bwrap = Bun.which('bwrap')
describe.skipIf(!bwrap)('Autopilot actual OS boundary', () => {
  test('shared absolute runtime deadline kills a fake agent and its sleeping descendant', async () => {
    const root = mkdtempSync(join(tmpdir(), 'pilot-deadline-'))
    const cwd = join(root, 'project'), providerHome = join(root, 'fake-login')
    mkdirSync(cwd); mkdirSync(providerHome)
    const executable = join(root, 'claude')
    writeFileSync(executable, `#!/bin/sh\n(while true; do echo alive >> '${cwd}/child-progress'; sleep 0.05; done) &\nwait\n`, { mode: 0o700 })
    const p = new PilotRuns(); p.runs = []
    const run = p.begin('deadline', { harness: { kind: 'claude' }, hosts: [], secrets: [], approvals: ['merge'], parallel: 1, runtime: 3000 }); run.state = 'on'
    const box = await prepareSandbox({ key: 'OWN-2/Agent', name: 'deadline', kind: 'claude', cwd, workspace: 'fixture' }, run, cwd, async () => ({ out: '', code: 0 }), undefined, { providerHome, executable: () => executable })
    const start = Date.now()
    try {
      const child = Bun.spawn([box.wrapper], { stdout: 'ignore', stderr: 'ignore' })
      expect(await child.exited).not.toBe(0)
      expect(Date.now() - start).toBeLessThan(5000)
      const progress = await Bun.file(join(cwd, 'child-progress')).text()
      expect(progress).toContain('alive')
      await Bun.sleep(200)
      expect(await Bun.file(join(cwd, 'child-progress')).text()).toBe(progress)
    } finally { await box.close() }
  })
  test('cross-harness reviewer executable/login and fixed broker identity share the same boundary', async () => {
    const root = mkdtempSync(join(tmpdir(), 'pilot-reviewer-os-'))
    const providerHome = join(root, 'fake-login'), cwd = join(root, 'project')
    for (const dir of [join(providerHome, '.claude'), join(providerHome, '.codex'), cwd]) mkdirSync(dir, { recursive: true })
    writeFileSync(join(providerHome, '.claude/.credentials.json'), '{"fixture":"FAKE_CLAUDE"}')
    writeFileSync(join(providerHome, '.codex/auth.json'), '{"fixture":"FAKE_CODEX"}')
    const executables: Record<string,string> = {}
    for (const kind of ['claude', 'codex']) {
      const file = join(root, kind); executables[kind] = file
      writeFileSync(file, kind === 'claude' ? '#!/bin/sh\ntest -f "$HOME/.claude/.credentials.json" && codex --review\n' : `#!/bin/sh\nmkdir -p "$HOME/.codex/sessions"\nprintf '%s\\n' '{"fixture":"captured inside namespace"}' > "$HOME/.codex/sessions/fixture.jsonl"\ntest -f "$HOME/.codex/auth.json" && echo same-boundary > '${cwd}/reviewer-result'\n`, { mode: 0o700 })
    }
    const p = new PilotRuns(); p.runs = []
    const run = p.begin('fixture', { harness: { kind: 'claude' }, hosts: [], secrets: [], approvals: ['merge'], parallel: 1 }); run.state = 'on'; p.claim(run, 'OWN-1')
    const calls: { chat: string; cwd: string }[] = []
    const box = await prepareSandbox({ key: 'OWN-1/Agent', name: 'fixture', kind: 'claude', cwd, workspace: 'fixture' }, run, cwd, async (_args, ctx) => { calls.push(ctx); return { out: 'ok', code: 0 } }, 'codex', { providerHome, executable: (kind) => executables[kind] })
    try {
      expect(existsSync(join(box.logsHome, '.codex/auth.json'))).toBe(true)
      // The real launch handover must fix even a profile-altered PATH before the canonical name is run.
      const child = Bun.spawn(['/bin/sh', '-c', isolatedShellCommand(box.wrapper)], { env: { ...process.env, PATH: '/unusable-profile-path' }, stdin: 'pipe', stdout: 'pipe', stderr: 'pipe' })
      child.stdin.write('claude\nexit\n'); child.stdin.end()
      const [code, err] = await Promise.all([child.exited, new Response(child.stderr).text()])
      expect({ code, stderr: code ? err : '' }).toEqual({ code: 0, stderr: '' })
      expect(await Bun.file(join(cwd, 'reviewer-result')).text()).toBe('same-boundary\n')
      expect(await Bun.file(capturedTranscript(box.captures!, 'codex', '.codex/sessions/fixture.jsonl')).text()).toContain('captured inside namespace')
      const send = (args: string[]) => new Promise<{ code: number }>((resolve, reject) => {
        const req = request({ socketPath: box.gateway!, path: '/api/cli', method: 'POST', headers: { 'content-type': 'application/json' } }, (res) => { let body = ''; res.on('data', (chunk) => body += chunk); res.on('end', () => resolve(JSON.parse(body))) })
        req.on('error', reject); req.end(JSON.stringify({ args, chat: 'OTHER/Agent', cwd: '/host', proof: 'fake' }))
      })
      expect((await send(['status'])).code).toBe(0)
      expect(calls).toHaveLength(1); expect(calls[0]).toMatchObject({ chat: 'OWN-1/Agent', cwd })
      expect((await send(['task', 'tell', 'OTHER-1', 'go'])).code).toBe(1)
      expect(calls).toHaveLength(1)
      // Paths are opaque names, never host paths to open; replayed chunks are idempotent.
      const target = join(root, 'sentinel'); writeFileSync(target, 'unchanged')
      const capture = { kind: 'codex', file: target, offset: 0, data: Buffer.from('fixture log\n').toString('base64') }
      const upload = () => new Promise<number>((resolve, reject) => {
        const req = request({ socketPath: box.gateway!, path: '/api/transcript', method: 'POST' }, (res) => { res.resume(); res.on('end', () => resolve(res.statusCode!)) })
        req.on('error', reject); req.end(JSON.stringify(capture))
      })
      expect(await upload()).toBe(200); expect(await upload()).toBe(200)
      expect(await Bun.file(target).text()).toBe('unchanged')
      expect(await Bun.file(capturedTranscript(box.captures!, 'codex', target)).text()).toBe('fixture log\n')
    } finally { await box.close() }
    // A worker-planted provider-directory link must not turn a restart into a host write.
    rmSync(join(box.logsHome, '.codex'), { recursive: true, force: true })
    symlinkSync(providerHome, join(box.logsHome, '.codex'))
    await expect(prepareSandbox({ key: 'OWN-1/Agent', name: 'fixture', kind: 'claude', cwd, workspace: 'fixture' }, run, cwd, async () => ({ out: '', code: 0 }), 'codex', { providerHome, executable: (kind) => executables[kind] })).rejects.toThrow('symlink')
    expect(existsSync(join(providerHome, 'auth.json'))).toBe(false)
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

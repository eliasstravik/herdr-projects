// Real OS enforcement, with fake executables and fake data. CI installs bubblewrap; never real agents/accounts.
import { describe, expect, test } from 'bun:test'
import { copyFileSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { sandboxArgs } from '../server/live/autopilot-sandbox.ts'

const bwrap = Bun.which('bwrap')
describe.skipIf(!bwrap)('Autopilot actual OS boundary', () => {
  for (const kind of ['claude', 'codex']) test(`${kind} and its child shell cannot access host files, control files, Git metadata or host network`, async () => {
    const root = mkdtempSync(join(tmpdir(), 'shep-pilot-os-'))
    const repo = join(root, 'repo'), cwd = join(root, 'task'), state = join(root, 'state'), control = join(root, 'control')
    for (const p of [repo, cwd, state, control, join(cwd, '.git')]) mkdirSync(p, { recursive: true })
    const secret = join(root, 'fake-host-secret')
    writeFileSync(secret, 'FAKE NEVER A REAL SECRET')
    writeFileSync(join(control, 'policy'), 'immutable')
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

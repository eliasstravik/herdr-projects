// One install for every machine: the helper Shepherdr puts on another machine (helper/), run here as it runs there.
// The SSH link itself (box.ts, machines.ts) was checked by hand against a machine saved in herdr.
import { describe, expect, test } from 'bun:test'
import { spawn, spawnSync } from 'node:child_process'
import { appendFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, utimesSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { parseHookLines } from '../server/live/readers/hooks.ts'
import { helperFiles, helperVersion } from '../server/live/install.ts'
import { Mirror, mirrorHomes } from '../server/live/mirror.ts'
import { carryClaudeSession } from '../server/live/agents.ts'
import type { BoxInfo } from '../server/live/box.ts'

const root = join(import.meta.dir, '..')
const perl = spawnSync('perl', ['-MJSON::PP', '-e1']).status === 0

describe('the helper', () => {
  test.if(perl)('hook.pl records the hook as it came, with when; the main machine cuts it down as it reads', () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-hook-'))
    const f = join(dir, 'a.jsonl')
    const input = { hook_event_name: 'PreToolUse', tool_name: 'AskUserQuestion', tool_input: { questions: [{ question: 'Ship it?' }] }, tool_response: { big: 'x'.repeat(1000) } }
    expect(spawnSync('perl', [join(root, 'helper', 'hook.pl'), f], { input: JSON.stringify(input) }).status).toBe(0)
    const line = JSON.parse(readFileSync(f, 'utf8'))
    expect(line.raw.tool_response).toBeUndefined()
    expect(typeof line.at).toBe('number')
    expect(parseHookLines(readFileSync(f, 'utf8'))).toEqual([{ at: line.at, event: 'PreToolUse', tool: 'AskUserQuestion', question: 'Ship it?' }])
  })

  test.if(perl)('mirror.pl sends what a followed file gains, and a file that starts over from its start', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-mirror-'))
    const f = join(dir, 'log.jsonl')
    writeFileSync(f, 'one\n')
    const p = spawn('perl', [join(root, 'helper', 'mirror.pl')])
    let out = ''
    p.stdout.on('data', (d) => (out += d.toString()))
    p.stdin.write(`S 0\nD ${dir}\n`)
    const until = async (s: string) => {
      for (let i = 0; i < 50 && !out.includes(s); i++) await Bun.sleep(100)
      expect(out).toContain(s)
    }
    await until(`${f}\t0\t4\none\n`)
    appendFileSync(f, 'two\n')
    await until(`${f}\t4\t4\ntwo\n`)
    writeFileSync(f, 'x\n')
    await until(`${f}\t0\t2\nx\n`)
    p.kill()
  })

  test.if(perl)('mirror.pl sends what an agent writes now before the rest of a backlog', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-mirror-'))
    const big = join(dir, 'old.jsonl')
    const live = join(dir, 'live.jsonl')
    writeFileSync(big, 'x'.repeat(16 << 20))
    utimesSync(big, new Date(Date.now() - 3_600_000), new Date(Date.now() - 3_600_000))
    writeFileSync(live, 'one\n')
    const p = spawn('perl', [join(root, 'helper', 'mirror.pl')])
    p.stdin.write(`S 0\nD ${dir}\n`)
    // the main machine is slow to read: mirror.pl waits on it, halfway through the backlog, while the agent writes
    await Bun.sleep(500)
    appendFileSync(live, 'two\n')
    let out = ''
    p.stdout.setEncoding('latin1')
    p.stdout.on('data', (d) => (out += d))
    for (let i = 0; i < 100 && !out.includes(`${big}\t${15 << 20}\t`); i++) await Bun.sleep(100)
    p.kill()
    expect(out.indexOf(`${live}\t4\t4\ntwo\n`)).toBeGreaterThan(0)
    expect(out.indexOf(`${live}\t4\t4\ntwo\n`)).toBeLessThan(out.indexOf(`${big}\t${8 << 20}\t`))
  })

  test.if(perl)('a mirror started again sends only what its copy lacks', async () => {
    const there = mkdtempSync(join(tmpdir(), 'shep-mirror-'))
    const here = mkdtempSync(join(tmpdir(), 'shep-copy-'))
    const log = join(there, 'log.jsonl')
    writeFileSync(log, 'one\n')
    const copy = (f: string) => join(here, f)
    const mirror = () => {
      const m = new Mirror({} as BoxInfo, () => Bun.spawn(['perl', join(root, 'helper', 'mirror.pl')], { stdin: 'pipe', stdout: 'pipe', stderr: 'ignore' }), copy)
      m.follow([there], [])
      m.start()
      return m
    }
    const until = async (s: string) => {
      for (let i = 0; i < 50 && !(existsSync(copy(log)) && readFileSync(copy(log), 'utf8') === s); i++) await Bun.sleep(100)
      expect(readFileSync(copy(log), 'utf8')).toBe(s)
    }
    const first = mirror()
    await until('one\n')
    first.stop()
    // Shepherdr starts again (an update): the new mirror goes on from what is here, and the copy shows it was not sent again
    writeFileSync(copy(log), 'ONE\n')
    appendFileSync(log, 'two\n')
    const again = mirror()
    await until('ONE\ntwo\n')
    again.stop()
  })

  test.if(perl)('health.pl says the cores, load, memory and disks', () => {
    const r = spawnSync('perl', [join(root, 'helper', 'health.pl')], { encoding: 'utf8' })
    const j = JSON.parse(r.stdout)
    expect(j.cores).toBeGreaterThan(0)
    expect(j.memTotal).toBeGreaterThan(0)
    expect(j.disks.length).toBeGreaterThan(0)
  })

  test('its files: the command, the scripts and the skills; its version changes with them', () => {
    const files = helperFiles('Linux', 'x86_64').map(([to]) => to)
    for (const f of ['bin/shepherdr', 'hook.pl', 'mirror.pl', 'health.pl', 'relay.pl', 'skill/shepherdr/SKILL.md', 'skill/shepherdr-coordinator/SKILL.md', 'skill/shepherdr-agent/SKILL.md']) expect(files).toContain(f)
    expect(helperVersion('Linux', 'x86_64')).toMatch(/^[0-9a-f]{16}$/)
  })

  test.if(perl)('the shepherdr command there says plainly when the way back is down', () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-shim-'))
    mkdirSync(join(dir, 'bin'))
    writeFileSync(join(dir, 'bin', 'shepherdr'), readFileSync(join(root, 'helper', 'shepherdr')), { mode: 0o755 })
    const r = spawnSync('perl', [join(dir, 'bin', 'shepherdr'), 'status'], { encoding: 'utf8' })
    expect(r.status).toBe(2)
    expect(r.stderr).toContain("can't be reached from this machine")
  })
})

describe('a machine run from the main one', () => {
  test("its home space's coordinator runs in its home folder there, and its log is read from the mirror of that machine's", () => {
    const home = mkdtempSync(join(tmpdir(), 'shep-worker-'))
    const box: BoxInfo = { label: 'vm-demo', target: 'vm-demo', session: 'default', herdr: 'herdr', home: '/home/exedev', path: '/usr/bin:/bin', os: 'Linux' }
    const code = `const { Places } = await import(${JSON.stringify(join(root, 'server/live/places.ts'))}); const { claudeLogPath } = await import(${JSON.stringify(join(root, 'server/live/readers/find.ts'))}); const c = new Places().coordinator('p:vm-demo:~'); console.log(JSON.stringify({ cwd: c?.cwd, log: claudeLogPath(c?.cwd ?? '', 'abc') }))`
    const r = spawnSync(process.execPath, ['-e', code], { encoding: 'utf8', env: { ...process.env, ...mirrorHomes(home, box), SHEPHERDR_BOX: JSON.stringify(box), SHEPHERDR_HOME: home, SHEPHERDR_MACHINE: 'vm-demo' } })
    expect(JSON.parse(r.stdout)).toEqual({ cwd: '/home/exedev', log: `${home}/mirror/home/exedev/.claude/projects/-home-exedev/abc.jsonl` })
  })

  test("its coordinator's heartbeat holds no path of the main machine: the checklist comes in it", () => {
    const home = mkdtempSync(join(tmpdir(), 'shep-worker-'))
    const box: BoxInfo = { label: 'vm-demo', target: 'vm-demo', session: 'default', herdr: 'herdr', home: '/home/other', path: '/usr/bin:/bin', os: 'Linux' }
    const code = `const { Heartbeats } = await import(${JSON.stringify(join(root, 'server/live/heartbeat.ts'))}); const h = new Heartbeats(); const first = h.beat('p:vm-demo:tor', 'space'); h.write('p:vm-demo:tor', '- Check the ROM builds'); console.log(JSON.stringify([first, h.beat('p:vm-demo:tor', 'space'), h.beat('p:vm-demo:~', 'home')]))`
    const r = spawnSync(process.execPath, ['-e', code], { encoding: 'utf8', env: { ...process.env, ...mirrorHomes(home, box), SHEPHERDR_BOX: JSON.stringify(box), SHEPHERDR_HOME: home, SHEPHERDR_MACHINE: 'vm-demo' } })
    const [first, edited, machine] = JSON.parse(r.stdout) as string[]
    for (const text of [first, edited, machine]) {
      expect(text).not.toContain(home)
      expect(text).not.toContain('.shepherdr')
      expect(text).not.toMatch(/\/(Users|home|root|tmp|var|private)\//)
      expect(text).toContain('shepherdr heartbeat set')
    }
    expect(first).toContain('Tasks stuck or stalled')
    expect(edited).toContain('Your HEARTBEAT.md:\n- Check the ROM builds')
    expect(machine).toContain('Disk, memory and load')
  })

  test("an agent's artifact commands there name no path of the main machine: the pictures and pages are copied over", () => {
    const home = mkdtempSync(join(tmpdir(), 'shep-worker-'))
    const there = mkdtempSync(join(tmpdir(), 'shep-there-'))
    const files = mkdtempSync(join(tmpdir(), 'shep-there-files-'))
    writeFileSync(join(files, 'board.html'), '<h1>Board</h1>')
    // ssh to the machine runs its command here, so its files are these folders
    const bin = mkdtempSync(join(tmpdir(), 'shep-ssh-'))
    writeFileSync(join(bin, 'ssh'), '#!/bin/sh\nfor a; do last=$a; done\nexec /bin/sh -c "$last"\n', { mode: 0o755 })
    const box: BoxInfo = { label: 'vm-demo', target: 'vm-demo', session: 'default', herdr: '/nonexistent/herdr', home: there, path: `${bin}:/usr/bin:/bin`, os: 'Linux' }
    const code = `
      const { mkdirSync, writeFileSync } = await import('node:fs')
      const { LiveSource } = await import(${JSON.stringify(join(root, 'server/live/source.ts'))})
      const src = new LiveSource()
      const shot = (n) => { mkdirSync(${JSON.stringify(join(home, 'checks'))}, { recursive: true }); const f = ${JSON.stringify(join(home, 'checks'))} + '/' + n; writeFileSync(f, 'png ' + n); return f }
      src.checker = async () => ({ errors: [], warnings: [], shots: { light: shot('a.png'), dark: shot('a-dark.png') } })
      const cwd = ${JSON.stringify(files)}
      const outs = [(await src.cli(['artifact', 'check', 'board.html'], { cwd })).out]
      const made = (await src.cli(['artifact', 'new', 'board.html', '--title', 'Board'], { cwd })).out
      const id = made.match(/artifact (\\w+)/)[1]
      outs.push(made, (await src.cli(['artifact', 'update', id, 'board.html'], { cwd })).out, (await src.cli(['artifact', 'show', id], { cwd })).out)
      // no browser on the main machine: its error names its own paths
      src.checker = async () => ({ errors: [], warnings: [], skipped: 'could not open a browser here (no chrome at ' + ${JSON.stringify(join(home, 'chrome'))} + ')' })
      outs.push((await src.cli(['artifact', 'check', 'board.html'], { cwd })).out)
      console.log(JSON.stringify(outs))
      process.exit(0)`
    const r = spawnSync(process.execPath, ['-e', code], { encoding: 'utf8', env: { ...process.env, ...mirrorHomes(home, box), PATH: `${bin}:${process.env.PATH}`, SHEPHERDR_BOX: JSON.stringify(box), SHEPHERDR_HOME: home, SHEPHERDR_MACHINE: 'vm-demo', HERDR_SOCKET_PATH: join(home, 'no-herdr.sock') } })
    const outs = JSON.parse(r.stdout) as string[]
    for (const out of outs) expect(out).not.toContain(home)
    const checks = join(there, '.shepherdr', 'helper', 'checks')
    for (const out of outs.slice(0, 3)) expect(out).toContain(`Pictures at phone width: ${checks}/a.png (dark: ${checks}/a-dark.png)`)
    expect(readFileSync(join(checks, 'a-dark.png'), 'utf8')).toBe('png a-dark.png')
    const page = outs[3].match(/HTML: (\S+)/)![1]
    expect(page.startsWith(join(there, '.shepherdr', 'helper', 'artifacts') + '/')).toBe(true)
    expect(readFileSync(page, 'utf8')).toBe('<h1>Board</h1>')
    expect(outs[4]).toContain('No pictures: the main machine could not open its browser')
  })

  test("its shepherdr set and unset for Everywhere run on the main machine; its machine's own stays here", async () => {
    const home = mkdtempSync(join(tmpdir(), 'shep-worker-'))
    const main = mkdtempSync(join(tmpdir(), 'shep-main-'))
    const there = mkdtempSync(join(tmpdir(), 'shep-there-'))
    const bin = mkdtempSync(join(tmpdir(), 'shep-ssh-'))
    writeFileSync(join(bin, 'ssh'), '#!/bin/sh\nfor a; do last=$a; done\nexec /bin/sh -c "$last"\n', { mode: 0o755 })
    // the main machine's Shepherdr: it keeps Everywhere in its settings file
    const asked: string[][] = []
    const everywhere: Record<string, string> = {}
    const server = Bun.serve({
      port: 0,
      hostname: '127.0.0.1',
      async fetch(req) {
        const [verb, key, value] = ((await req.json()) as { args: string[] }).args
        asked.push([verb, key, value].filter(Boolean))
        if (verb === 'set') everywhere[key] = value
        else delete everywhere[key]
        writeFileSync(join(main, 'settings.json'), JSON.stringify({ everywhere, machine: {}, spaces: {} }))
        return Response.json({ out: 'ok', code: 0 })
      },
    })
    writeFileSync(join(main, 'server.json'), JSON.stringify({ port: server.port, pid: process.pid }))
    const box: BoxInfo = { label: 'vm-demo', target: 'vm-demo', session: 'default', herdr: '/nonexistent/herdr', home: there, path: `${bin}:/usr/bin:/bin`, os: 'Linux' }
    const code = `
      const { LiveSource } = await import(${JSON.stringify(join(root, 'server/live/source.ts'))})
      const src = new LiveSource()
      const run = async (args) => { const r = await src.cli(args, {}); return [r.code, r.out.trim(), src.settings.pick(args[1], 'machine')] }
      const outs = [await run(['set', 'brief.review', 'off']), await run(['set', 'machine.maxTasks', '3', '--machine']), await run(['unset', 'brief.review'])]
      // the main machine's Shepherdr is away (it restarts)
      ;(await import('node:fs')).rmSync(${JSON.stringify(join(main, 'server.json'))})
      outs.push(await run(['set', 'brief.review', 'off']))
      console.log(JSON.stringify(outs))
      process.exit(0)`
    try {
      const w = Bun.spawn([process.execPath, '-e', code], { stdout: 'pipe', stderr: 'pipe', env: { ...process.env, ...mirrorHomes(home, box), PATH: `${bin}:${process.env.PATH}`, SHEPHERDR_BOX: JSON.stringify(box), SHEPHERDR_HOME: home, SHEPHERDR_MACHINE: 'vm-demo', SHEPHERDR_MAIN_HOME: main, SHEPHERDR_SHARED_HOME: main, HERDR_SOCKET_PATH: join(home, 'no-herdr.sock') } })
      const out = await new Response(w.stdout).text()
      const [set, mine, unset, away] = JSON.parse(out) as [number, string, { value: string; from: string }][]
      expect(set).toEqual([0, "brief.review = off (Everywhere's)", { value: 'off', from: 'all' }])
      expect(mine).toEqual([0, "machine.maxTasks = 3 (vm-demo's)", { value: '3', from: 'machine' }])
      expect(unset[0]).toBe(0)
      expect(unset[1]).toStartWith("brief.review: Everywhere's own is unset")
      expect(unset[2].from).toBe('default')
      // only Everywhere went to the main machine
      expect(asked).toEqual([['set', 'brief.review', 'off'], ['unset', 'brief.review']])
      expect(away[0]).toBe(1)
      expect(away[1]).toContain("The main machine's Shepherdr can't be reached right now")
    } finally {
      server.stop(true)
    }
  })

  test.if(perl)('the shepherdr command there sends a new HEARTBEAT.md on stdin', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-shim-'))
    mkdirSync(join(dir, 'bin'))
    writeFileSync(join(dir, 'bin', 'shepherdr'), readFileSync(join(root, 'helper', 'shepherdr')), { mode: 0o755 })
    let got: { args: string[]; stdin?: string } | undefined
    const server = Bun.listen({
      unix: join(dir, 'main.sock'),
      socket: {
        data(sock, data) {
          got = JSON.parse(data.toString().split('\r\n\r\n')[1])
          sock.end('HTTP/1.0 200 OK\r\n\r\n{"out":"Saved.\\n","code":0}')
        },
      },
    })
    try {
      const run = (args: string[], input?: string) => new Promise<string>((done) => {
        const c = spawn('perl', [join(dir, 'bin', 'shepherdr'), ...args], { stdio: [input === undefined ? 'ignore' : 'pipe', 'pipe', 'pipe'] })
        let out = ''
        c.stdout!.on('data', (d) => (out += d))
        c.on('close', () => done(out))
        if (input !== undefined) c.stdin!.end(input)
      })
      expect(await run(['heartbeat', 'set'], '- Check the ROM builds\n')).toBe('Saved.\n')
      expect(got).toMatchObject({ args: ['heartbeat', 'set'], stdin: '- Check the ROM builds\n' })
      // other words read no stdin
      await run(['heartbeat'], 'ignored')
      expect(got!.stdin).toBeUndefined()
    } finally {
      server.stop(true)
    }
  })

  test.if(perl)("a /clear there starts another session: the chat follows it, from what the agent's hooks say", () => {
    const home = mkdtempSync(join(tmpdir(), 'shep-worker-'))
    const box: BoxInfo = { label: 'vm-demo', target: 'vm-demo', session: 'default', herdr: 'herdr', home: '/home/exedev', path: '/usr/bin:/bin', os: 'Linux' }
    const hooks = '/home/exedev/.shepherdr/hooks/shep-machine.jsonl'
    const project = '/home/exedev/.claude/projects/-home-exedev--shepherdr-coordinators-machine'
    const copy = `${home}/mirror${hooks}`
    mkdirSync(join(copy, '..'), { recursive: true })
    // as the helper's hook.pl records them there, and the mirror brings them: an earlier run's start, then its /clear
    const hook = (raw: object) => expect(spawnSync('perl', [join(root, 'helper', 'hook.pl'), copy], { input: JSON.stringify(raw) }).status).toBe(0)
    const at = (id: string, source: string) => ({ hook_event_name: 'SessionStart', source, session_id: id, transcript_path: `${project}/${id}.jsonl` })
    hook(at('old', 'startup'))
    const code = `
      const { Agents } = await import(${JSON.stringify(join(root, 'server/live/agents.ts'))})
      const a = new Agents()
      const old = ${JSON.stringify(`${home}/mirror${project}/old.jsonl`)}
      const r = { key: 'm:vm-demo', name: 'shep-machine', kind: 'claude', cwd: '/home/exedev/.shepherdr/coordinators/machine', workspace: 'w', logs: [old], sessionId: 'd071', started: Date.now() + 60000, hooks: ${JSON.stringify(hooks)} }
      a.records[r.key] = r
      const seen = []
      // its own run started after that: what the copy says of an earlier run is not followed
      a.followSession(r.key)
      seen.push({ id: r.sessionId, logs: [...r.logs] })
      r.started = 0
      a.followSession(r.key)
      seen.push({ id: r.sessionId, logs: [...r.logs] })
      a.followSession(r.key)
      seen.push({ id: r.sessionId, logs: [...r.logs] })
      console.log(JSON.stringify(seen))
      process.exit(0)`
    const env = { ...process.env, ...mirrorHomes(home, box), SHEPHERDR_BOX: JSON.stringify(box), SHEPHERDR_HOME: home, SHEPHERDR_MACHINE: 'vm-demo' }
    // /clear: the session ends, one starts and another right after; prompts go to the last
    hook({ hook_event_name: 'SessionEnd', reason: 'clear', session_id: 'old' })
    hook(at('79420d27', 'clear'))
    hook(at('8ed46adf', 'clear'))
    hook({ hook_event_name: 'UserPromptSubmit', prompt: 'hi', session_id: '8ed46adf' })
    const r = spawnSync(process.execPath, ['-e', code], { encoding: 'utf8', env })
    const m = (id: string) => `${home}/mirror${project}/${id}.jsonl`
    const seen = JSON.parse(r.stdout)
    expect(seen[0]).toEqual({ id: 'd071', logs: [m('old')] })
    expect(seen[1]).toEqual({ id: '8ed46adf', logs: [m('old'), m('8ed46adf')] })
    expect(seen[2]).toEqual(seen[1])
  })

  test('a session resumed in another folder takes its transcript along, wherever Claude Code wrote it', () => {
    const claude = mkdtempSync(join(tmpdir(), 'shep-carry-'))
    const was = process.env.CLAUDE_CONFIG_DIR
    process.env.CLAUDE_CONFIG_DIR = claude
    try {
      // it ran in the home folder, not the folder its record named
      mkdirSync(join(claude, 'projects', '-home-exedev'), { recursive: true })
      writeFileSync(join(claude, 'projects', '-home-exedev', 'abc.jsonl'), '{"type":"user"}\n')
      const log = carryClaudeSession('abc', '/home/exedev/.shepherdr/coordinators/machine')
      expect(log).toBe(join(claude, 'projects', '-home-exedev--shepherdr-coordinators-machine', 'abc.jsonl'))
      expect(readFileSync(log!, 'utf8')).toBe('{"type":"user"}\n')
      // one that is there already stays as it is; one written nowhere has nothing to carry
      writeFileSync(log!, 'kept\n')
      expect(carryClaudeSession('abc', '/home/exedev/.shepherdr/coordinators/machine')).toBe(log)
      expect(readFileSync(log!, 'utf8')).toBe('kept\n')
      expect(carryClaudeSession('nope', '/home/exedev/elsewhere')).toBeUndefined()
    } finally {
      if (was === undefined) delete process.env.CLAUDE_CONFIG_DIR
      else process.env.CLAUDE_CONFIG_DIR = was
    }
  })
})

describe('a restart of the main server (an update)', () => {
  const saved = { id: 'far', label: 'far', target: 'far.invalid', session: 'default', enabled: true }
  const snap = { name: 'S', host: 'far', places: [{ id: 'm:far', kind: 'machine', level: 1, name: 'far', machine: 'far', status: 'idle', mode: 'regular', chat: 'm:far' }, { id: 'p:far:web', kind: 'project', level: 2, name: 'web', machine: 'far', status: 'need', mode: 'regular', chat: 'p:far:web' }], tasks: [], automations: {}, asks: {}, settings: [], overrides: {}, user: 'u' }

  /** a worker left running from before the restart: a process of its own, serving its snapshot */
  function oldWorker() {
    const dir = mkdtempSync(join(tmpdir(), 'shep-old-worker-'))
    mkdirSync(join(dir, 'server'))
    writeFileSync(join(dir, 'server', 'main.ts'), `const s = Bun.serve({ port: 0, fetch: () => Response.json(${JSON.stringify(snap)}) }); console.log(s.port)`)
    const p = spawn(process.execPath, [join(dir, 'server', 'main.ts')])
    return new Promise<{ pid: number; port: number; alive: () => boolean }>((done) =>
      p.stdout.once('data', (d) => done({ pid: p.pid!, port: Number(String(d).trim()), alive: () => p.exitCode === null && p.signalCode === null })),
    )
  }

  async function before(w: { pid: number; port: number }, code: string) {
    const { HOME, writeJson } = await import('../server/store.ts')
    const { Machines, codeVersion } = await import('../server/live/machines.ts')
    writeJson('machines/far/last.json', { saved, snap, chats: [{ key: 'p:far:web', cli: 'Claude Code', messages: [{ id: 'm1', who: 'agent', at: 1, text: 'Done.' }] }] })
    writeJson('machines/far/server.json', { port: w.port, pid: w.pid, code: code || codeVersion() })
    return { HOME, Machines, writeJson }
  }

  test('what each machine showed comes back at once, connecting, and its worker on the same code is taken back', async () => {
    const w = await oldWorker()
    const { HOME, Machines } = await before(w, '')
    const m = new Machines(async () => [saved])
    try {
      const l = m.list().find((x) => x.rec.machine === 'far')!
      // at once: its places and the chats on screen, as before the restart, connecting rather than out of reach
      expect(l.snap?.places.map((p) => p.id)).toEqual(['m:far', 'p:far:web'])
      expect(l.chats.get('p:far:web')?.messages[0].text).toBe('Done.')
      expect(l.connecting).toBe(true)
      expect(m.rows()[0].state).toBe('Connecting…')
      for (let i = 0; i < 250 && !l.reachable; i++) await Bun.sleep(20)
      expect(l.reachable).toBe(true)
      expect(l.connecting).toBe(false)
      expect(l.worker?.pid).toBe(w.pid)
      m.stop()
      // the worker goes on after this server stops, and what the machine showed is kept for the next start
      await Bun.sleep(100)
      expect(w.alive()).toBe(true)
      expect(existsSync(join(HOME, 'machines', 'far', 'last.json'))).toBe(true)
    } finally {
      m.stop()
      process.kill(w.pid)
    }
  })

  test('a worker started again out of its hands (the doctor, by hand) is followed to its new address', async () => {
    const w = await oldWorker()
    const { Machines, writeJson } = await before(w, '')
    const m = new Machines(async () => [saved])
    const next = await oldWorker()
    try {
      const l = m.list().find((x) => x.rec.machine === 'far')!
      for (let i = 0; i < 250 && !l.reachable; i++) await Bun.sleep(20)
      expect(l.reachable).toBe(true)
      // the old worker stops, and a new one serves on another port, as the doctor leaves it
      process.kill(w.pid)
      writeJson('machines/far/server.json', { port: next.port, pid: next.pid })
      for (let i = 0; i < 500 && !(l.reachable && l.rec.url.endsWith(`:${next.port}`)); i++) await Bun.sleep(20)
      expect(l.rec.url).toBe(`http://127.0.0.1:${next.port}`)
      expect(l.reachable).toBe(true)
      expect(l.worker?.pid).toBe(next.pid)
    } finally {
      m.stop()
      if (w.alive()) process.kill(w.pid)
      process.kill(next.pid)
    }
  })

  test('a worker on other code is stopped, not taken back', async () => {
    const w = await oldWorker()
    const { Machines } = await before(w, 'older')
    const m = new Machines(async () => [saved])
    try {
      for (let i = 0; i < 250 && w.alive(); i++) await Bun.sleep(20)
      expect(w.alive()).toBe(false)
      expect(m.list()[0].worker?.pid).not.toBe(w.pid)
    } finally {
      m.stop()
      if (w.alive()) process.kill(w.pid)
    }
  })
})

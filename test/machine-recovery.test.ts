import { describe, expect, test } from 'bun:test'
import { chmodSync, existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { LinkBackoff, WorkerHealth } from '../server/live/recovery.ts'
import { EventEmitter } from 'node:events'
import { Subscription } from '../server/live/herdr.ts'
import { Tunnel } from '../server/live/tunnel.ts'
import { Machines } from '../server/live/machines.ts'
import type { Snapshot } from '../shared/types.ts'

const until = async (f: () => boolean) => {
  for (let i = 0; i < 400 && !f(); i++) await Bun.sleep(10)
  expect(f()).toBe(true)
}

describe('machine link recovery', () => {
  test('retry delay is bounded and only stable success resets it', () => {
    const b = new LinkBackoff()
    expect([b.fail(0), b.fail(1), b.fail(2), b.fail(3), b.fail(4), b.fail(5)]).toEqual([3000, 6000, 12000, 24000, 30000, 30000])
    b.success(10)
    expect(b.fail(20)).toBe(30000)
    b.success(30)
    expect(b.fail(60_030)).toBe(3000)
  })

  test('watchdog gives three minutes, backs off restarts, and ignores healthy idle', () => {
    const h = new WorkerHealth()
    h.observe(false, 0)
    expect(h.restartDue(179_999)).toBe(false)
    expect(h.restartDue(180_000)).toBe(true)
    h.restarting(180_000)
    expect(h.restartDue(359_999)).toBe(false)
    expect(h.restartDue(360_000)).toBe(true)
    h.restarting(360_000)
    expect(h.restartDue(540_000)).toBe(false)
    expect(h.restartDue(720_000)).toBe(true)
    h.observe(true, 720_000)
    expect(h.restartDue(900_000)).toBe(false)
    h.observe(true, 780_000)
    h.observe(false, 800_000)
    h.restarting(980_000)
    expect(h.restartDue(1_160_000)).toBe(true)
  })

  test('failed SSH/name lookup recovers inside the same worker with fresh bridge connections', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-dns-recovery-'))
    const root = join(import.meta.dir, '..')
    // Executed in an isolated child: never real SSH, DNS, sockets or the live store.
    writeFileSync(join(dir, 'ssh'), `#!/usr/bin/env bun
import { existsSync, appendFileSync } from 'node:fs'
appendFileSync(${JSON.stringify(join(dir, 'calls'))}, JSON.stringify(process.argv.slice(2))+'\\n')
if (!existsSync(${JSON.stringify(join(dir, 'online'))})) { console.error('Could not resolve hostname fake.invalid'); process.exit(255) }
for await (const line of console) {
 const f = JSON.parse(line)
 console.log(JSON.stringify({id:f.id,result:f.method==='agent.list'?{agents:[]}:{subscribed:true}}))
 if(f.method !== 'events.subscribe') break
}
`)
    chmodSync(join(dir, 'ssh'), 0o700)
    writeFileSync(join(dir, 'verify.ts'), `
import { writeFileSync, readFileSync } from 'node:fs'
import { Agents } from ${JSON.stringify(join(root, 'server/live/agents.ts'))}
const a = new Agents()
// Exercise polling separately from the event subscription.
;(a as any).sub.set = () => undefined
const now = Date.now
let clock = now()
Date.now = () => clock
try {
 await Promise.all([a.refresh(), a.refresh(), a.refresh()])
 if(a.reachable) throw Error('false healthy after DNS failure')
 writeFileSync(${JSON.stringify(join(dir, 'online'))}, '')
 await a.refresh()
 if(a.reachable) throw Error('retry ignored backoff')
 clock += 4000
 await a.refresh()
 if(!a.reachable) throw Error('did not reconnect')
 const calls = readFileSync(${JSON.stringify(join(dir, 'calls'))}, 'utf8').trim().split('\\n').map(x=>JSON.parse(x))
 if(calls.length !== 2) throw Error('duplicate bridge attempts: '+calls.length)
 if(calls.some(x=>!x.includes('ControlPath=none'))) throw Error('reused cached SSH master')
 let release!: () => void
 ;(a as any).readBackground = () => new Promise<void>(resolve => { release = resolve })
 await a.refresh()
 // A slow pane read may still be running, but it cannot block the next health probe.
 await a.refresh()
 release()
 const beforeStop = readFileSync(${JSON.stringify(join(dir, 'calls'))}, 'utf8').trim().split('\\n').length
 a.stop()
 clock += 60000
 await a.refresh()
 if(readFileSync(${JSON.stringify(join(dir, 'calls'))}, 'utf8').trim().split('\\n').length !== beforeStop) throw Error('retried after stop')
} finally { a.stop(); Date.now = now }
`)
    try {
      const p = Bun.spawn([process.execPath, join(dir, 'verify.ts')], {
        env: { ...process.env, PATH: `${dir}:${process.env.PATH}`, SHEPHERDR_HOME: join(dir, 'state'), SHEPHERDR_BOX: JSON.stringify({ label: 'fake', target: 'fake.invalid', session: 'default', herdr: 'herdr', home: '/fake', path: '/bin', os: 'Linux' }) },
        stdout: 'pipe', stderr: 'pipe', timeout: 10_000,
      })
      const [code, err] = await Promise.all([p.exited, new Response(p.stderr).text()])
      expect({ code, err }).toEqual({ code: 0, err: '' })
    } finally { rmSync(dir, { recursive: true, force: true }) }
  })

  test('a tunnel retries failed setup once, never overlaps, and cancels pending retry', async () => {
    let attempts = 0, active = 0, max = 0
    let finish!: () => void
    const tunnel = new Tunnel(async (track, stopped) => {
      attempts++
      active++
      max = Math.max(max, active)
      try {
        if (attempts === 1) throw Error('Could not resolve hostname')
        await new Promise<void>((resolve) => {
          finish = resolve
          track({ exited: Promise.resolve(), kill: resolve })
          if (stopped()) resolve()
        })
      } finally { active-- }
    })
    try {
      tunnel.start(); tunnel.start()
      await until(() => attempts === 2)
      tunnel.start()
      expect(max).toBe(1)
      finish()
      await Bun.sleep(20)
      tunnel.stop()
      await Bun.sleep(6100)
      expect(attempts).toBe(2)
    } finally { tunnel.stop() }
  }, 15_000)

  test('subscription retries a failed lookup, ignores stale close events, and cancels on stop', async () => {
    let attempts = 0, opened = 0
    const sockets: (EventEmitter & { write(s: string): void; destroy(): void })[] = []
    const sub = new Subscription(() => undefined, () => { opened++ }, () => {
      attempts++
      if (attempts === 1) throw Error('name lookup failed')
      const socket = Object.assign(new EventEmitter(), {
        write() { queueMicrotask(() => socket.emit('data', Buffer.from(JSON.stringify({ result: {} }) + '\n'))) },
        destroy() { socket.emit('close') },
      })
      sockets.push(socket)
      queueMicrotask(() => socket.emit('connect'))
      return socket
    })
    try {
      sub.set([{ type: 'pane.closed' }])
      await until(() => opened === 1)
      expect(attempts).toBe(2)
      sub.set([{ type: 'pane.created' }])
      await until(() => opened === 2)
      expect(attempts).toBe(3)
      sockets[0].emit('close')
      sockets[1].emit('error', Error('SSH gone'))
      sockets[1].emit('close')
      sub.stop()
      await Bun.sleep(6100)
      expect(attempts).toBe(3)
    } finally { sub.stop() }
  }, 15_000)

  test('shutdown during socket cleanup cannot open a new tunnel', async () => {
    let release!: () => void, opened = false
    const t = new Tunnel(async (track, stopped) => {
      await new Promise<void>((resolve) => { release = resolve; track({ exited: Promise.resolve(), kill: resolve }) })
      if (!stopped()) opened = true
    })
    t.start(); t.stop(); release()
    await Bun.sleep(10)
    expect(opened).toBe(false)
  })

  test('HTTP snapshot with herdr off triggers one watchdog restart, waits for exit, and queues work', async () => {
    let healthy = false
    const s = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch: () => Response.json({ machines: [{ status: healthy ? 'idle' : 'off' }], tasks: [] } as unknown as Snapshot) })
    const m = new Machines(null, async () => ({}))
    const l = m.adopt('fake-watchdog', `http://127.0.0.1:${s.port}`)
    // Access lifecycle entry points without running real probes, worker processes or SSH.
    type TestLink = typeof l
    const internals = m as unknown as { poll(link: TestLink): Promise<void>; start(link: TestLink): Promise<void> }
    let starts = 0, kills = 0, exit!: () => void
    internals.start = async () => { starts++ }
    const worker = { pid: -1, kill: () => { kills++ }, exited: new Promise<void>((r) => { exit = r }) }
    l.worker = worker
    const originalNow = Date.now
    let clock = originalNow()
    Date.now = () => clock
    try {
      await until(() => !l.polling)
      expect(l.reachable).toBe(false)
      expect(await m.act(l, { type: 'noop' } as never)).toEqual({ waiting: true })
      expect(l.waiting).toHaveLength(1)
      l.waiting.length = 0
      clock += 179_000
      await internals.poll(l)
      expect(kills).toBe(0)
      clock += 2000
      await Promise.all([internals.poll(l), internals.poll(l)])
      expect(kills).toBe(1)
      expect(starts).toBe(0)
      await internals.poll(l)
      expect(kills).toBe(1)
      exit()
      await until(() => starts === 1)
      l.worker = worker
      await internals.poll(l)
      expect(kills).toBe(1)
      healthy = true
      await internals.poll(l)
      expect(l.reachable).toBe(true)
      clock += 900_000
      await internals.poll(l)
      expect(kills).toBe(1)
      m.stop()
      healthy = false
      await internals.poll(l)
      expect(kills).toBe(1)
    } finally { Date.now = originalNow; exit(); m.stop(); s.stop(true) }
  })
  test('removing a machine cancels cached writes and late probes cannot restore it', async () => {
    const { HOME } = await import('../server/store.ts')
    const m = new Machines(null, async () => ({}))
    const l = m.adopt('removed-fake', 'http://127.0.0.1:1')
    type TestLink = typeof l
    const internal = m as unknown as {
      saved: () => Promise<never[]>
      keep(l: TestLink): void
      keepTimers: Map<string, ReturnType<typeof setTimeout>>
    }
    internal.saved = async () => []
    try {
      internal.keep(l)
      expect(internal.keepTimers.has(l.saved.label)).toBe(true)
      await m.sync()
      expect(m.byName(l.saved.label)).toBeUndefined()
      expect(internal.keepTimers.has(l.saved.label)).toBe(false)
      internal.keep(l)
      expect(internal.keepTimers.has(l.saved.label)).toBe(false)
      expect(existsSync(join(HOME, 'machines', l.saved.label, 'last.json'))).toBe(false)
    } finally { m.stop() }
  })

})

// Starts Shepherdr: SHEPHERDR_SOURCE=demo for the fictional world (review, tests), otherwise live on herdr.
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { startApp } from './app.ts'
import { DemoWorld } from './demo/world.ts'
import { Push } from './push.ts'
import type { Source } from './source.ts'
import { HOME, writeJson } from './store.ts'

const root = join(import.meta.dir, '..')
const port = Number(process.env.SHEPHERDR_PORT ?? 7787)
const kind = process.env.SHEPHERDR_SOURCE === 'demo' ? 'demo' : 'live'

async function makeSource(): Promise<Source> {
  if (kind === 'demo') return new DemoWorld()
  const { LiveSource } = await import('./live/source.ts')
  return new LiveSource()
}

const source = await makeSource()
const push = kind === 'live' && !process.env.SHEPHERDR_BOX ? new Push() : undefined
if (push && 'devices' in source) (source as { devices: () => string[] }).devices = () => push.devices.map(deviceName)
// a worker for another machine (machines.ts) serves only its main machine, which pushes alerts itself
const worker = !!process.env.SHEPHERDR_BOX
const server = startApp({ source, dist: join(root, 'dist'), push: worker ? undefined : push, port, updates: worker ? undefined : join(HOME, 'update.json') })
// a worker says which code it runs: the main server keeps it across its own restart while that is the same
if (kind === 'live') writeJson('server.json', { port: server.port, pid: process.pid, code: worker ? (await import('./live/machines.ts')).codeVersion() : undefined })
let tunnel: { stop(): void } | undefined
if (worker) {
  // the agents there reach this worker's shepherdr command through an SSH tunnel to it
  tunnel = (await import('./live/tunnel.ts')).keepTunnel(server.port!)
  orphaned()
}
console.log(`shepherdr (${kind}) on http://127.0.0.1:${server.port}, ready ${Math.round(performance.now())} ms after start`)

/** A worker outlives a restart of its main server, which takes it back; one whose main server stays gone stops. */
function orphaned() {
  let gone = 0
  setInterval(() => {
    try {
      process.kill((JSON.parse(readFileSync(join(process.env.SHEPHERDR_MAIN_HOME ?? '', 'server.json'), 'utf8')) as { pid: number }).pid, 0)
      gone = 0
    } catch {
      if (++gone >= 10) {
        tunnel?.stop()
        source.stop?.()
        process.exit(0)
      }
    }
  }, 60_000)
}

// an update restarts it, as one right after a merge into Shepherdr's own repo does: a merge on its way finishes and
// its answer goes out first, so none is cut halfway with its agent waiting on an answer that never comes
let stopping = false
for (const sig of ['SIGINT', 'SIGTERM'] as const)
  process.on(sig, async () => {
    // asked again: at once
    if (stopping) process.exit(0)
    stopping = true
    if (await source.drain?.()) await Bun.sleep(500)
    tunnel?.stop()
    source.stop?.()
    server.stop()
    process.exit(0)
  })

/** "this phone", "this Mac": what the browser says it is */
function deviceName(ua: string) {
  if (/iPhone|Android.*Mobile/.test(ua)) return 'a phone'
  if (/iPad|Android/.test(ua)) return 'a tablet'
  if (/Mac OS X/.test(ua)) return 'a Mac'
  if (/Windows/.test(ua)) return 'a PC'
  return 'a computer'
}

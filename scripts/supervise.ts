// Keeps the server running and up to date. When Shepherdr's own checkout moves to a new commit (a task merged into
// main), the plugin list file changes, or the human's own plugins or their list (~/.shepherdr/plugins) change, it makes a release (scripts/update.ts) and restarts the server on it, once no
// voice call is on (scripts/supervisor.ts); open screens then reload themselves. The server always runs from a frozen
// release, so a later merge can't change what runs. A failed update keeps the running version, and update.json says
// why (the screen shows it). A merge that changes the supervisor's own code starts it again on that code.
import { spawn, type ChildProcess } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, watch, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { HOME, readJson, writeJson } from '../server/store.ts'
import type { UpdateStatus } from '../shared/types.ts'
import { codeOf, Supervisor } from './supervisor.ts'
import { refDirs, run as runCmd } from './update.ts'

const ROOT = join(import.meta.dir, '..')
const PORT = Number(process.env.SHEPHERDR_PORT ?? 7787)
let child: ChildProcess | null = null
let stopping = false
/** the code this supervisor runs, as it started */
const CODE = codeOf(import.meta.path)

/** stop new voice calls (or let them start again) and say how many are on; a server that doesn't answer has none */
async function drain(on: boolean) {
  try {
    const r = await fetch(`http://127.0.0.1:${PORT}/api/drain`, { method: 'POST', body: JSON.stringify({ on }), signal: AbortSignal.timeout(2000) })
    return ((await r.json()) as { calls: number }).calls
  } catch {
    return 0
  }
}

const sup = new Supervisor({
  root: ROOT,
  home: HOME,
  status: readJson<UpdateStatus | null>('update.json', null),
  save: (s) => writeJson('update.json', s),
  drain,
  restart: () => child?.kill('SIGTERM'),
  sleep: (ms) => Bun.sleep(ms),
  log: (m) => console.log(m),
})

function run() {
  // the agents' shepherdr command and skill stay in the checkout (SHEPHERDR_ROOT); the server runs from the release
  child = spawn(process.execPath, [join(sup.dir(), 'server', 'main.ts')], { cwd: ROOT, stdio: 'inherit', env: { ...process.env, SHEPHERDR_ROOT: ROOT, SHEPHERDR_SUPERVISOR: String(process.pid) } })
  child.on('exit', (code) => {
    child = null
    if (stopping) return
    sup.stopped(code)
    setTimeout(run, code === 0 ? 200 : 2000)
  })
}

let timer: ReturnType<typeof setTimeout> | null = null
let forced = false
/** a merge writes several files: wait for it to settle */
function soon(force = false) {
  forced ||= force
  if (timer) clearTimeout(timer)
  timer = setTimeout(() => {
    timer = null
    const f = forced
    forced = false
    void sup.check(f).then(renew)
  }, 1000)
}

let renewing = false
/** the supervisor on new code while it takes over: a stop meanwhile stops it too */
let handover: ChildProcess | null = null
/** new code of its own that didn't load: not tried again */
let broken = ''
/** A merge changed the supervisor's own code (it ran on older code until herdr started it again): it starts itself
 *  again on the new code, with the server, once the new release is past its trial, no update is being made and no
 *  voice call is on. A release that failed, or new code that doesn't load or start Shepherdr, leaves this one running. */
async function renew() {
  const state = () => sup.status?.state
  const now = codeOf(import.meta.path)
  if (renewing || stopping || now === CODE || now === broken || state() !== 'ok') return
  renewing = true
  while (state() !== 'ok' || sup.onTrial || sup.working || (await drain(true)) > 0) {
    await drain(false)
    if (state() === 'failed') return void (renewing = false)
    await Bun.sleep(5000)
  }
  if (!(await loads())) {
    broken = now
    await drain(false)
    return void (renewing = false)
  }
  console.log("the supervisor's own code changed: starting it again on it")
  stopping = true
  const was = child
  if (was && was.exitCode === null && was.signalCode === null) {
    was.kill('SIGTERM')
    await new Promise((done) => was.once('exit', done))
  }
  const next = spawn(process.execPath, [import.meta.path], { cwd: process.cwd(), detached: true, stdio: ['ignore', 'inherit', 'inherit'], env: process.env })
  handover = next
  // herdr's stop and start (scripts/plugin.ts) find the new one
  const pidFile = join(process.env.HERDR_PLUGIN_STATE_DIR ?? join(HOME, 'run'), 'supervisor.pid')
  const mine = (() => {
    try {
      return existsSync(pidFile) && Number(readFileSync(pidFile, 'utf8')) === process.pid
    } catch {
      return false
    }
  })()
  const pidIs = (pid: number) => {
    try {
      if (mine) writeFileSync(pidFile, String(pid))
    } catch {
      /* plugin.ts asks the server for its supervisor as well */
    }
  }
  pidIs(next.pid!)
  // the new one has its server answering in time, or this one takes back over as it was
  let gone = false
  next.once('exit', () => (gone = true))
  let up = false
  for (let i = 0; i < 60 && !gone && !up; i++) {
    await Bun.sleep(1000)
    up = await fetch(`http://127.0.0.1:${PORT}/api/snapshot`, { signal: AbortSignal.timeout(1500) }).then((r) => r.ok, () => false)
  }
  if (!up) {
    console.log("the supervisor's new code didn't start Shepherdr; it goes on as it was")
    try {
      process.kill(-next.pid!, 'SIGTERM')
    } catch {}
    pidIs(process.pid)
    handover = null
    broken = now
    stopping = false
    renewing = false
    await Bun.sleep(1000)
    return run()
  }
  next.unref()
  process.exit(0)
}

/** the supervisor's new code and everything it imports load (as a release's server is checked, update.ts) */
async function loads() {
  const scratch = mkdtempSync(join(tmpdir(), 'shepherdr-check-'))
  const r = await runCmd([process.execPath, 'build', import.meta.path, '--target', 'bun', '--outdir', scratch], ROOT)
  rmSync(scratch, { recursive: true, force: true })
  if (!r.ok) console.log(`the supervisor's new code doesn't load; it goes on as it is\n${r.out}`)
  return r.ok
}

watch(join(ROOT, 'plugins.json'), () => soon(true))
// the human's own plugins and their on/off list: a release of the same commit with them as they are now
mkdirSync(join(HOME, 'plugins'), { recursive: true })
watch(join(HOME, 'plugins'), { recursive: true }, () => soon(true))
watch(HOME, (_, f) => f === 'plugins.json' && soon(true))
for (const dir of await refDirs(ROOT)) {
  try {
    watch(dir, { recursive: dir.endsWith('heads') }, () => soon())
  } catch (e) {
    console.log(`can't watch ${dir}: ${(e as Error).message}`)
  }
}
// a slow look as well, in case a change slipped past the watchers
setInterval(() => soon(), 60_000)

for (const sig of ['SIGTERM', 'SIGINT'] as const)
  process.on(sig, () => {
    stopping = true
    child?.kill('SIGTERM')
    try {
      if (handover?.pid) process.kill(-handover.pid, 'SIGTERM')
    } catch {}
    process.exit(0)
  })

run()
// no release yet (the first start), or merged while Shepherdr was down: make one now
soon()

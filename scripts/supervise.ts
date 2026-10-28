// Keeps the server running and up to date. When Shepherdr's own checkout moves to a new commit (a task merged into
// main), the plugin list file changes, or the human's own plugins or their list (~/.shepherdr/plugins) change, it makes a release (scripts/update.ts) and restarts the server on it, once no
// voice call is on (scripts/supervisor.ts); open screens then reload themselves. The server always runs from a frozen
// release, so a later merge can't change what runs. A failed update keeps the running version, and update.json says
// why (the screen shows it).
import { spawn, type ChildProcess } from 'node:child_process'
import { mkdirSync, watch } from 'node:fs'
import { join } from 'node:path'
import { HOME, readJson, writeJson } from '../server/store.ts'
import type { UpdateStatus } from '../shared/types.ts'
import { Supervisor } from './supervisor.ts'
import { refDirs } from './update.ts'

const ROOT = join(import.meta.dir, '..')
const PORT = Number(process.env.SHEPHERDR_PORT ?? 7787)
let child: ChildProcess | null = null
let stopping = false

const sup = new Supervisor({
  root: ROOT,
  home: HOME,
  status: readJson<UpdateStatus | null>('update.json', null),
  save: (s) => writeJson('update.json', s),
  // a server that doesn't answer has no call to cut
  drain: async (on) => {
    try {
      const r = await fetch(`http://127.0.0.1:${PORT}/api/drain`, { method: 'POST', body: JSON.stringify({ on }), signal: AbortSignal.timeout(2000) })
      return ((await r.json()) as { calls: number }).calls
    } catch {
      return 0
    }
  },
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
    sup.check(f)
  }, 1000)
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
    process.exit(0)
  })

run()
// no release yet (the first start), or merged while Shepherdr was down: make one now
soon()

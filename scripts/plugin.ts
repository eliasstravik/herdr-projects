// herdr plugin lifecycle: start (idempotent: the same Shepherdr already answering is left alone), open, stop, status,
// share. herdr's startup hooks are one-shot, so start detaches a small supervisor that runs the server, and rebuilds
// and restarts it within seconds whenever the checkout moves to a new commit or the plugin list file changes. A herdr
// plugin update builds the new version in a new folder and starts nothing: the next start (herdr's, or the open
// action) finds the old one answering from another folder or version, and swaps it for this one.
import { spawn, spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, openSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { join, resolve } from 'node:path'
import { takeOver } from './take-over.ts'
import { prepare } from './prepare.ts'

const ROOT = resolve(process.env.HERDR_PLUGIN_ROOT ?? join(import.meta.dir, '..'))
const STATE = process.env.HERDR_PLUGIN_STATE_DIR ?? join(process.env.SHEPHERDR_HOME ?? join(homedir(), '.shepherdr'), 'run')
const PORT = Number(process.env.SHEPHERDR_PORT ?? 7787)
const PID = join(STATE, 'supervisor.pid')
const LOG = join(STATE, 'server.log')
const PATH_ON_TAILNET = '/shepherdr'
const VERSION = (JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8')) as { version: string }).version

function pid(): number | null {
  try {
    const p = Number(readFileSync(PID, 'utf8'))
    process.kill(p, 0)
    return p
  } catch {
    return null
  }
}

async function answering() {
  try {
    const r = await fetch(`http://127.0.0.1:${PORT}/api/snapshot`, { signal: AbortSignal.timeout(1500) })
    return r.ok
  } catch {
    return false
  }
}

/** which Shepherdr answers (none: an older one that doesn't say) */
async function running(): Promise<{ root: string; version: string; supervisor: number | null } | null> {
  try {
    const r = await fetch(`http://127.0.0.1:${PORT}/api/version`, { signal: AbortSignal.timeout(1500) })
    return r.ok ? ((await r.json()) as { root: string; version: string; supervisor: number | null }) : null
  } catch {
    return null
  }
}

const real = (p: string) => {
  try {
    return realpathSync(p)
  } catch {
    return resolve(p)
  }
}

/** stop a supervisor with the server it runs (its process group), and wait until the port is free */
async function kill(p: number) {
  try {
    process.kill(-p, 'SIGTERM')
  } catch {
    try {
      process.kill(p, 'SIGTERM')
    } catch {}
  }
  for (let i = 0; i < 40 && (await answering()); i++) await Bun.sleep(250)
}

async function start(): Promise<boolean> {
  // a failure here never keeps Shepherdr from starting; it is tried again at the next start
  try {
    takeOver(ROOT)
  } catch (e) {
    console.log(`Tidying away herdr-projects failed: ${(e as Error).message}`)
  }
  let replace: Awaited<ReturnType<typeof running>> = null
  if (await answering()) {
    const r = await running()
    const same = !r || (real(r.root) === real(ROOT) && r.version === VERSION)
    // started by hand (no supervisor) is left alone too: it isn't ours to stop
    if (same || !r?.supervisor) {
      console.log(`Shepherdr is running on http://127.0.0.1:${PORT}`)
      return true
    }
    console.log(`Shepherdr ${r.version} is running from ${r.root}: restarting it on ${VERSION} from ${ROOT}`)
    replace = r
  }
  const error = await prepare(ROOT)
  if (error) {
    console.error(`Shepherdr preparation failed: ${error}`)
    return false
  }
  if (replace) {
    await kill(replace.supervisor!)
    if (await answering()) {
      console.log(`Shepherdr ${replace.version} did not stop; see ${LOG}`)
      return false
    }
  }
  mkdirSync(STATE, { recursive: true })
  const out = openSync(LOG, 'a')
  const child = spawn(process.execPath, [join(ROOT, 'scripts', 'supervise.ts')], {
    cwd: ROOT,
    detached: true,
    stdio: ['ignore', out, out],
    env: { ...process.env, SHEPHERDR_PORT: String(PORT), HERDR_SOCKET_PATH: process.env.HERDR_SOCKET_PATH ?? '' },
  })
  child.unref()
  writeFileSync(PID, String(child.pid))
  for (let i = 0; i < 40; i++) {
    if (await answering()) {
      console.log(`Shepherdr is running on http://127.0.0.1:${PORT}. Share it on your tailnet with the "Share Shepherdr" action.`)
      return true
    }
    await Bun.sleep(250)
  }
  console.log(`Shepherdr did not answer yet; see ${LOG}`)
  return false
}

async function stop() {
  const r = await running()
  const p = r?.supervisor ?? pid()
  if (!p) return console.log('Shepherdr is not running.')
  await kill(p)
  rmSync(PID, { force: true })
  console.log('Stopped Shepherdr.')
}

async function status() {
  console.log((await answering()) ? `Shepherdr is running on http://127.0.0.1:${PORT}` : 'Shepherdr is not running.')
}

/** its link on the tailnet when it is shared there, else on this machine */
function link() {
  const ts = spawnSync('tailscale', ['serve', 'status', '--json'], { encoding: 'utf8' })
  const shared = ts.status === 0 && ts.stdout.includes(`127.0.0.1:${PORT}`) && ts.stdout.includes(PATH_ON_TAILNET)
  return shared ? `https://${tailnetName()}${PATH_ON_TAILNET}/` : `http://127.0.0.1:${PORT}/`
}

function tailnetName() {
  const st = spawnSync('tailscale', ['status', '--json'], { encoding: 'utf8' })
  return st.status === 0 ? String(JSON.parse(st.stdout).Self?.DNSName ?? '').replace(/\.$/, '') : '<this machine>'
}

/** the open action (herdr-projects' old key opens it too): start it when needed, and show its link over the pane */
async function open() {
  const up = await start()
  if (!up) process.exitCode = 1
  const say = up ? `Shepherdr is at ${link()}` : `Shepherdr did not start yet; see ${LOG}`
  console.log(say)
  // an action's own output shows nowhere in herdr: a small pane says it until Enter closes it
  const text = `${say}\n\nOpen it in a browser. To use it from your phone, run the action "Share Shepherdr on my tailnet".`
  spawnSync(process.env.HERDR_BIN_PATH || 'herdr', ['plugin', 'pane', 'open', '--plugin', process.env.HERDR_PLUGIN_ID ?? 'herdr-projects', '--entrypoint', 'link', '--env', `SHEPHERDR_LINK_TEXT=${text}`], { stdio: 'ignore' })
}

/** private to the tailnet only (tailscale serve), never funnel */
function share() {
  const ts = spawnSync('tailscale', ['serve', '--bg', '--set-path', PATH_ON_TAILNET, `http://127.0.0.1:${PORT}`], { encoding: 'utf8' })
  if (ts.status !== 0) return console.log(`tailscale serve failed: ${ts.stderr || ts.stdout}`)
  console.log(`Shepherdr is on your tailnet: https://${tailnetName()}${PATH_ON_TAILNET}/ (open it on your phone and add it to the home screen for alerts)`)
}

const cmd = process.argv[2]
if (cmd === 'start') { if (!(await start())) process.exitCode = 1 }
else if (cmd === 'open') await open()
else if (cmd === 'stop') await stop()
else if (cmd === 'status') await status()
else if (cmd === 'share') share()
else console.log('usage: plugin.ts start|open|stop|status|share')

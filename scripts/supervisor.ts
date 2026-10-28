// What the supervisor decides: when to make a release, when to restart on it (only once no voice call is on, with new
// calls stopped), and when to take a release back. scripts/supervise.ts wires it to the server process and watchers.
import { createHash } from 'node:crypto'
import { existsSync, readFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import type { UpdateStatus } from '../shared/types.ts'
import { prune, releasesOf, update } from './update.ts'

/** a new release that stops within this long of starting is taken back */
export const TRIAL = 15_000

export interface SupervisorDeps {
  root: string
  /** where the human's own plugins live (none: ~/.shepherdr) */
  home?: string
  status: UpdateStatus | null
  save(s: UpdateStatus): void
  /** stop new voice calls (or let them start again) and say how many are on, in one step at the server */
  drain(on: boolean): Promise<number>
  /** stop the server; it comes back on `current` */
  restart(): void
  sleep(ms: number): Promise<void>
  log(msg: string): void
  now?: () => number
}

export class Supervisor {
  status: UpdateStatus | null
  /** the release running now (none before the first: the checkout as herdr built it) */
  current: string | undefined
  /** while a new release is on trial: the status to go back to */
  private trial: { from: Omit<UpdateStatus, 'at'>; until: number } | null = null
  private busy = false
  private again: { force: boolean } | null = null
  private now: () => number

  constructor(private d: SupervisorDeps) {
    this.status = d.status
    this.current = d.status?.release
    this.now = d.now ?? Date.now
    if (this.status?.state === 'restarting') this.save({ ...this.status, state: 'ok' })
  }

  private save(s: Omit<UpdateStatus, 'at'>) {
    this.status = { ...s, at: this.now() }
    this.d.save(this.status)
  }

  /** a release is being made or started */
  get working() {
    return this.busy
  }

  /** a new release runs on trial: it is taken back if it stops now */
  get onTrial() {
    return !!this.trial && this.now() <= this.trial.until
  }

  /** the folder the server runs from */
  dir(): string {
    const name = this.current
    const dir = name ? join(releasesOf(this.d.root), name) : null
    return dir && existsSync(join(dir, 'server', 'main.ts')) ? dir : this.d.root
  }

  /** Make a release of the commit the checkout is on and restart on it. Calls while one runs are folded into one
   *  more pass after it. */
  async check(force = false): Promise<void> {
    if (this.busy) {
      this.again = { force: force || !!this.again?.force }
      return
    }
    this.busy = true
    try {
      await this.pass(force)
    } finally {
      this.busy = false
      const again = this.again
      this.again = null
      if (again) await this.check(again.force)
    }
  }

  private async pass(force: boolean) {
    const before = this.status
    const running = this.dir() !== this.d.root
    const r = await update(this.d.root, {
      built: running ? (before?.built ?? null) : null,
      // the human's own plugins that built there are their last good versions
      release: running ? this.current : undefined,
      skip: before?.state === 'failed' ? before.commit : undefined,
      force,
      home: this.d.home,
    })
    if (!r) return
    if (r.state === 'failed') {
      this.d.log(`update to ${r.commit.slice(0, 7)} failed; the running Shepherdr stays as it was\n${r.error}`)
      return this.save({ ...r, release: this.current })
    }
    // a restart would cut a voice call short: wait until none is on, with new ones stopped from that moment on
    let waited = false
    while ((await this.d.drain(true)) > 0) {
      await this.d.drain(false)
      if (!waited) this.d.log(`${r.commit.slice(0, 7)} is ready; restarting once the voice call ends`)
      waited = true
      await this.d.sleep(5000)
      // newer news came in while waiting: make that release instead
      if (this.again) return prune(this.d.root, [this.current, this.trial?.from.release])
    }
    this.d.log(`updated to ${r.commit.slice(0, 7)} (${r.subject}): restarting`)
    // screens hear it is an update, not an outage, before the server goes; new calls stay stopped until it does
    this.save({ ...r, state: 'restarting' })
    await this.d.sleep(300)
    const previous = this.current
    this.trial = { from: { ...(before ?? { state: 'ok', commit: '', subject: '', built: null }), release: previous }, until: this.now() + TRIAL }
    this.current = r.release
    this.d.restart()
    prune(this.d.root, [this.current, previous])
    this.save(r)
  }

  /** The server stopped (it is started again on `current`). A new release that stops as it starts is taken back. */
  stopped(code: number | null) {
    if (code === 0 || !this.trial || this.now() > this.trial.until) return
    const r = this.status!
    this.d.log(`${r.commit.slice(0, 7)} stopped as it started; back to the version before`)
    this.current = this.trial.from.release
    this.save({ ...this.trial.from, state: 'failed', commit: r.commit, subject: r.subject, error: "the new version stopped as it started:\nsee Shepherdr's server.log" })
    this.trial = null
  }
}

/** The code a script runs: its file and every file it imports from the checkout (types only don't count), as one
 *  hash. The supervisor starts itself again when its own changes (supervise.ts). */
export function codeOf(entry: string): string {
  const h = createHash('sha256')
  const seen = new Set<string>()
  const walk = (f: string) => {
    if (seen.has(f)) return
    seen.add(f)
    let text: string
    try {
      text = readFileSync(f, 'utf8')
    } catch {
      return h.update(`${f}: gone`)
    }
    h.update(f).update(text)
    for (const m of text.matchAll(/^\s*(?:import|export)\s+(?!type\b)[^'"]*?from\s+['"](\.[^'"]+)['"]|import\(\s*['"](\.[^'"]+)['"]\s*\)/gm)) walk(resolve(dirname(f), m[1] ?? m[2]))
  }
  walk(resolve(entry))
  return h.digest('hex').slice(0, 16)
}

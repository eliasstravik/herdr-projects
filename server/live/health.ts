// Machine health: every 30 seconds Shepherdr reads how loaded this machine is (its 1-minute load against its cores,
// memory in use, how full home and /tmp are, a GPU's use where nvidia-smi is there) and whether herdr answers. Cheap:
// /proc, statfs and one nvidia-smi call. It only measures and tells: past a limit in Settings (machine.*) its
// coordinator hears once, with the numbers and what uses the machine most, and again only after it came back under.
// What to do about it (pause or queue tasks, clean the disk, tell the human) is the coordinator's call. The same
// numbers decide when Shepherdr starts more tasks (engine.release).
import { readdirSync, readFileSync, readlinkSync, statfsSync, statSync } from 'node:fs'
import { cpus, homedir, tmpdir } from 'node:os'
import type { HealthLevel, HealthMeter } from '../../shared/types.ts'
import type { MachineLimits } from './settings.ts'
import { BOX, helperDir, runSync, type BoxInfo } from './box.ts'
import { join } from 'node:path'

export const SAMPLE_MS = 30_000
/** the load must stay over its limit this long before the coordinator hears of it: one test run's spike is no news */
export const LOAD_FOR = 120_000
/** herdr must stay silent this long: a restart of it takes a few seconds */
export const HERDR_FOR = 30_000
/** back under: below this share of its limit, so a value hovering at the limit is told once */
export const BACK_UNDER = 0.9

export interface Disk {
  path: string
  /** % full, as df counts it: used over what an ordinary user could have */
  pct: number
  freeGb: number
  dev: number
}

export interface Gpu {
  name: string
  util: number
  memPct: number
}

export interface Sample {
  at: number
  cores: number
  /** 1-minute load average */
  load: number
  memTotal: number
  memUsed: number
  disks: Disk[]
  gpus: Gpu[]
}

/** 1-minute load from /proc/loadavg */
export const parseLoad = (text: string) => Number.parseFloat(text.split(/\s+/)[0]) || 0

/** total and in-use memory (bytes) from /proc/meminfo: in use is what is not available */
export function parseMem(text: string) {
  const kb = (name: string) => Number(text.match(new RegExp(`^${name}:\\s+(\\d+)`, 'm'))?.[1] ?? 0) * 1024
  const total = kb('MemTotal')
  const avail = kb('MemAvailable') || kb('MemFree')
  return { total, used: Math.max(0, total - avail) }
}

/** nvidia-smi --query-gpu=name,utilization.gpu,memory.used,memory.total --format=csv,noheader,nounits */
export function parseGpus(text: string): Gpu[] {
  return text
    .split('\n')
    .map((l) => l.split(',').map((x) => x.trim()))
    .filter((f) => f.length >= 4 && f[0])
    .map(([name, util, used, total]) => ({ name, util: Number(util) || 0, memPct: Number(total) ? Math.round((Number(used) / Number(total)) * 100) : 0 }))
}

/** how full a folder's disk is, as df says it */
export function diskOf(path: string): Disk | null {
  try {
    const s = statfsSync(path)
    const used = s.blocks - s.bfree
    const pct = used + s.bavail ? Math.round((used / (used + s.bavail)) * 100) : 0
    return { path, pct, freeGb: (s.bavail * s.bsize) / 2 ** 30, dev: statSync(path).dev }
  } catch {
    return null
  }
}

let smi: string | null | undefined
function gpus(): Gpu[] {
  smi ??= Bun.which('nvidia-smi')
  if (!smi) return []
  try {
    const r = Bun.spawnSync([smi, '--query-gpu=name,utilization.gpu,memory.used,memory.total', '--format=csv,noheader,nounits'], { stdout: 'pipe', stderr: 'ignore', timeout: 5000 })
    return r.exitCode === 0 ? parseGpus(r.stdout.toString()) : []
  } catch {
    return []
  }
}

/** another machine now, as the helper's health.pl reads it there (box.ts) */
function boxSample(b: BoxInfo): Sample {
  const r = runSync(['perl', join(helperDir(b.home), 'health.pl')], { timeoutMs: 15_000 })
  const j = r.code === 0 ? (JSON.parse(r.out.toString()) as Omit<Sample, 'at'>) : null
  if (!j) return { at: Date.now(), cores: 1, load: 0, memTotal: 0, memUsed: 0, disks: [], gpus: [] }
  return { ...j, at: Date.now(), disks: j.disks.map((d) => ({ ...d, path: d.path === b.home ? homedir() : d.path, dev: Number(d.dev) || 0 })) }
}

/** this machine now: home and /tmp once each (one disk when they are the same) */
export function sample(): Sample {
  if (BOX) return boxSample(BOX)
  const read = (f: string) => {
    try {
      return readFileSync(f, 'utf8')
    } catch {
      return ''
    }
  }
  const mem = parseMem(read('/proc/meminfo'))
  const disks: Disk[] = []
  for (const p of [homedir(), tmpdir()]) {
    const d = diskOf(p)
    if (d && !disks.some((x) => x.dev === d.dev)) disks.push(d)
  }
  return { at: Date.now(), cores: cpus().length || 1, load: parseLoad(read('/proc/loadavg')), memTotal: mem.total, memUsed: mem.used, disks, gpus: gpus() }
}

export const cpuPct = (s: Sample) => Math.round((s.load / s.cores) * 100)
export const memPct = (s: Sample) => (s.memTotal ? Math.round((s.memUsed / s.memTotal) * 100) : 0)
const gb = (b: number) => `${Math.round(b / 2 ** 30)} GB`
const tilde = (p: string) => (p === homedir() ? 'home' : p)

/* ---------- limits ---------- */

/** new tasks wait: the load or memory is over its start limit (nothing when the limit is by number only) */
export function busy(s: Sample | undefined, l: MachineLimits): 'cpu' | 'memory' | undefined {
  if (!s) return undefined
  if (l.startLoad !== null && s.load > l.startLoad * s.cores) return 'cpu'
  if (l.startMemory !== null && memPct(s) > l.startMemory) return 'memory'
  return undefined
}

export type Kind = 'cpu' | 'memory' | 'disk' | 'herdr'

/** what is past its alert limit now (over), and what is well back under it (back) */
export function levels(s: Sample, herdr: boolean, l: MachineLimits): Record<Kind, { over: boolean; back: boolean }> {
  const disk = Math.max(0, ...s.disks.map((d) => d.pct))
  const vs = (v: number, limit: number | null) => ({ over: limit !== null && v > limit, back: limit === null || v < limit * BACK_UNDER })
  return {
    cpu: vs(s.load, l.alertLoad === null ? null : l.alertLoad * s.cores),
    memory: vs(memPct(s), l.alertMemory),
    disk: vs(disk, l.alertDisk),
    herdr: { over: l.alertHerdr && !herdr, back: herdr || !l.alertHerdr },
  }
}

/** one kind's watch: over since when, and whether the coordinator was told */
export interface Watch {
  since?: number
  told?: boolean
}

/**
 * Each kind, from what it was and what it is now: told once when it has been over its limit long enough, and again
 * (that it is back) once it is well under. In between, nothing.
 */
export function judge(was: Partial<Record<Kind, Watch>>, now: Record<Kind, { over: boolean; back: boolean }>, at: number) {
  const hold: Record<Kind, number> = { cpu: LOAD_FOR, memory: 0, disk: 0, herdr: HERDR_FOR }
  const next: Partial<Record<Kind, Watch>> = {}
  const events: { kind: Kind; what: 'over' | 'back' }[] = []
  for (const kind of Object.keys(now) as Kind[]) {
    const w = { ...was[kind] }
    const { over, back } = now[kind]
    if (w.told) {
      if (back) events.push({ kind, what: 'back' })
      else next[kind] = w
      continue
    }
    if (!over) continue
    w.since ??= at
    if (at - w.since >= hold[kind]) {
      w.told = true
      events.push({ kind, what: 'over' })
    }
    next[kind] = w
  }
  return { next, events }
}

/** each meter for the screen: CPU, Memory, Disk, and each GPU */
export function meters(s: Sample, l: MachineLimits, told: Partial<Record<Kind, Watch>>): HealthMeter[] {
  const level = (kind: Kind, start: boolean): HealthLevel => (told[kind]?.told ? 'over' : start ? 'busy' : 'ok')
  const disk = s.disks.reduce<Disk | undefined>((a, d) => (!a || d.pct > a.pct ? d : a), undefined)
  return [
    { name: 'CPU', pct: cpuPct(s), detail: `load ${s.load.toFixed(1)} on ${s.cores} cores`, level: level('cpu', l.startLoad !== null && s.load > l.startLoad * s.cores) },
    { name: 'Memory', pct: memPct(s), detail: `${gb(s.memUsed)} of ${gb(s.memTotal)}`, level: level('memory', l.startMemory !== null && memPct(s) > l.startMemory) },
    ...(disk ? [{ name: 'Disk', pct: disk.pct, detail: s.disks.map((d) => `${tilde(d.path)} ${d.pct}%`).join(' · '), level: level('disk', false) }] : []),
    ...s.gpus.map((g, i) => ({ name: s.gpus.length > 1 ? `GPU ${i + 1}` : 'GPU', pct: g.util, detail: `${g.name} · memory ${g.memPct}%`, level: 'ok' as const })),
  ]
}

/* ---------- what uses it most ---------- */

export interface Proc {
  pid: number
  /** % of one core over the moment measured */
  cpu: number
  rss: number
  cmd: string
  cwd?: string
}

/** cpu ticks and resident memory of every process, from /proc/<pid>/stat */
function procStats() {
  const out = new Map<number, { ticks: number; rss: number }>()
  for (const name of readdirSync('/proc')) {
    if (!/^\d+$/.test(name)) continue
    try {
      const st = readFileSync(`/proc/${name}/stat`, 'utf8')
      const f = st.slice(st.lastIndexOf(')') + 2).split(' ')
      out.set(Number(name), { ticks: Number(f[11]) + Number(f[12]), rss: Number(f[21]) * 4096 })
    } catch {
      /* gone meanwhile */
    }
  }
  return out
}

/** the processes using this machine most over a moment (one second): by CPU, or by memory */
export async function topProcs(by: 'cpu' | 'memory', n = 8, ms = 1000): Promise<Proc[]> {
  // another machine's processes aren't listed: its numbers alone say how it is doing
  if (BOX) return []
  const a = procStats()
  await Bun.sleep(ms)
  const b = procStats()
  const hz = 100
  const all: Proc[] = []
  for (const [pid, x] of b) {
    const cpu = Math.round(((x.ticks - (a.get(pid)?.ticks ?? x.ticks)) / hz / (ms / 1000)) * 100)
    all.push({ pid, cpu, rss: x.rss, cmd: '' })
  }
  const top = all.sort((p, q) => (by === 'cpu' ? q.cpu - p.cpu || q.rss - p.rss : q.rss - p.rss)).slice(0, n)
  for (const p of top) {
    try {
      p.cmd = readFileSync(`/proc/${p.pid}/cmdline`, 'utf8').split('\0').filter(Boolean).join(' ').slice(0, 80)
      p.cwd = readlinkSync(`/proc/${p.pid}/cwd`)
    } catch {
      /* gone meanwhile, or not ours to read */
    }
  }
  return top.filter((p) => p.cmd)
}

/** each task's share: the processes running in its worktree, summed */
export function byTask(procs: Proc[], tasks: { id: string; worktree?: string }[]) {
  const of = (p: Proc) => tasks.find((t) => t.worktree && p.cwd && (p.cwd === t.worktree || p.cwd.startsWith(`${t.worktree}/`)))?.id
  const sums = new Map<string, { cpu: number; rss: number }>()
  for (const p of procs) {
    const id = of(p)
    if (!id) continue
    const s = sums.get(id) ?? { cpu: 0, rss: 0 }
    s.cpu += p.cpu
    s.rss += p.rss
    sums.set(id, s)
  }
  return { of, tasks: [...sums].map(([id, s]) => ({ id, ...s })) }
}

const mb = (b: number) => (b >= 2 ** 30 ? `${(b / 2 ** 30).toFixed(1)} GB` : `${Math.round(b / 2 ** 20)} MB`)

/** the lines a coordinator reads: the processes using the machine most, each with its task where it runs in one */
export function procLines(procs: Proc[], tasks: { id: string; worktree?: string }[], by: 'cpu' | 'memory') {
  const t = byTask(procs, tasks)
  const lines = procs.slice(0, 6).map((p) => {
    const id = t.of(p)
    return `  ${by === 'cpu' ? `${p.cpu}% CPU` : mb(p.rss)}  ${p.cmd}${id ? `  (${id})` : ''}`
  })
  const per = t.tasks.sort((a, b) => (by === 'cpu' ? b.cpu - a.cpu : b.rss - a.rss)).map((x) => `${x.id} ${by === 'cpu' ? `${x.cpu}% CPU` : mb(x.rss)}`)
  return [...(per.length ? [`Tasks among them: ${per.join(', ')}.`] : []), 'Using it most now:', ...lines]
}

/** the numbers in one line: "CPU 170% (load 27.2 on 16 cores) · Memory 62% (19 of 31 GB) · Disk home 22%" */
export function summary(s: Sample) {
  return [`CPU ${cpuPct(s)}% (load ${s.load.toFixed(1)} on ${s.cores} cores)`, `Memory ${memPct(s)}% (${gb(s.memUsed)} of ${gb(s.memTotal)})`, `Disk ${s.disks.map((d) => `${tilde(d.path)} ${d.pct}% (${Math.round(d.freeGb)} GB free)`).join(', ')}`, ...s.gpus.map((g) => `GPU ${g.util}% (${g.name})`)].join(' · ')
}

/* ---------- the watch ---------- */

export interface HealthEvent {
  kind: Kind
  what: 'over' | 'back'
  sample: Sample
  /** herdr: since when it stopped answering */
  since?: number
}

/**
 * Samples this machine every SAMPLE_MS and looks at herdr every few seconds; tells `on` when something crossed its
 * limit or came back under. What was told is kept on disk: a restart (Shepherdr restarts on every merge) tells
 * nothing again while it is still over.
 */
export class Monitor {
  last?: Sample
  private watch: Partial<Record<Kind, Watch>>

  constructor(
    private limits: () => MachineLimits,
    private herdr: () => boolean,
    private on: (e: HealthEvent) => void,
    private take: () => Sample = sample,
    private store: { read: () => Partial<Record<Kind, Watch>>; write: (w: Partial<Record<Kind, Watch>>) => void } = { read: () => ({}), write: () => undefined },
  ) {
    this.watch = store.read()
  }

  /** a look: a fresh sample when the last is old, then each limit judged */
  tick(now = Date.now()) {
    if (!this.last || now - this.last.at >= SAMPLE_MS - 1000) this.last = { ...this.take(), at: now }
    const was = this.watch
    const { next, events } = judge(was, levels(this.last, this.herdr(), this.limits()), now)
    this.watch = next
    if (JSON.stringify(next) !== JSON.stringify(was)) this.store.write(next)
    for (const e of events) this.on({ ...e, sample: this.last, since: was[e.kind]?.since })
  }

  /** what is past its alert limit now, and was told */
  told() {
    return this.watch
  }

  /** over a start limit: new tasks wait */
  busy() {
    return busy(this.last, this.limits())
  }
}

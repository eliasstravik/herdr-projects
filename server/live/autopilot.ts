// Trusted run ownership and limits. None of this state is mounted into an isolated agent.
import { randomUUID } from 'node:crypto'
import { readJson, writeJson } from '../store.ts'
import { CHAT_KINDS, chatProblem } from './readers/index.ts'
import type { Harness } from './restart.ts'

export const AUTOPILOT_KINDS = CHAT_KINDS.filter((k) => ['claude', 'codex', 'pi'].includes(k))
export interface PilotConfig {
  harness: Harness
  reviewer?: Harness
  hosts: string[]
  secrets: string[]
  approvals: string[]
  traffic?: number
  tokens?: number
  runtime?: number
  parallel: number
}
export interface PilotRun {
  id: string
  place: string
  config: PilotConfig
  started: number
  state: 'starting' | 'on' | 'stopping' | 'stopped' | 'failed' | 'workers'
  error?: string
  tasks: string[]
  traffic?: number
  used: number
  last?: { at: number; reason: string; outcome: string; ended?: number }
}
export function pilotProblem(kind: string) {
  return chatProblem(kind) ?? (!AUTOPILOT_KINDS.includes(kind) ? `${kind} has no isolated Autopilot adapter.` : undefined)
}
export function hostList(text: string): string[] {
  const hosts = [...new Set(text.split(/[\s,]+/).filter(Boolean).map((s) => s.toLowerCase()))]
  if (hosts.some((s) => !/^(?=.{1,253}$)(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z]{2,}$/.test(s))) throw new Error('Use exact public host names, separated by commas. No wildcards, URLs or local addresses.')
  return hosts
}
export class PilotRuns {
  runs: PilotRun[] = readJson('autopilot-runs.json', [])
  private transitions = new Set<string>()
  save() { writeJson('autopilot-runs.json', this.runs) }
  current(place: string) { return this.runs.findLast((r) => r.place === place && ['starting', 'on', 'stopping', 'failed'].includes(r.state)) }
  owner(task: string) { return this.runs.findLast((r) => r.tasks.includes(task)) }
  busy(place: string) { return this.transitions.has(place) }
  async transition<T>(place: string, fn: () => Promise<T>): Promise<T> {
    if (this.busy(place)) throw new Error('The coordinator is changing. Messages will wait; try again shortly.')
    this.transitions.add(place)
    try { return await fn() } finally { this.transitions.delete(place) }
  }
  begin(place: string, config: PilotConfig, now = Date.now()): PilotRun {
    const problem = pilotProblem(config.harness.kind)
    if (problem) throw new Error(problem)
    const r: PilotRun = { id: randomUUID(), place, config, started: now, state: 'starting', tasks: [], used: 0 }
    this.runs.push(r); this.save(); return r
  }
  claim(run: PilotRun, task: string) {
    if (run.tasks.includes(task)) return
    if (run.state !== 'on') throw new Error('Autopilot is not accepting new tasks.')
    if (!run.tasks.includes(task)) run.tasks.push(task)
    this.save()
  }
  check(run: PilotRun, active: number, now = Date.now()) {
    if (!['on', 'workers'].includes(run.state)) throw new Error(run.error || 'Autopilot is stopped.')
    if (run.config.runtime && now - run.started >= run.config.runtime) throw new Error('Autopilot reached its shared runtime limit.')
    if (run.config.traffic && (run.traffic ?? 0) >= run.config.traffic) throw new Error('Autopilot reached its shared network allowance.')
    if (run.config.tokens && run.used >= run.config.tokens) throw new Error('Autopilot reached its reported token allowance.')
    if (active >= run.config.parallel) throw new Error('Autopilot is at its maximum parallel tasks. Wait for one to finish.')
  }
  wake(run: PilotRun, reason: string, now = Date.now()) {
    this.check(run, -1, now)
    run.last = { at: now, reason, outcome: 'Queued' }; this.save()
  }
  outcome(run: PilotRun, outcome: string, ended = false) {
    if (!run.last) return
    run.last.outcome = outcome
    if (ended) run.last.ended = Date.now()
    this.save()
  }
}

// This is an allowlist, not a prompt instruction. The gateway fixes chat identity itself.
export function pilotCommand(run: PilotRun, chat: string, args: string[]) {
  if (!['on', 'workers'].includes(run.state)) throw new Error(run.error || 'Autopilot is stopped.')
  const [cmd, op, id] = args
  const task = chat.includes('/') ? chat.split('/')[0] : undefined
  const own = (target?: string) => !!target && run.tasks.includes(target.split('/')[0])
  if (cmd === 'complete' && !task || cmd === 'progress' || cmd === 'checkpoint' && task && own(task)) return
  if (['help', 'show', 'models'].includes(cmd)) return
  if (cmd === 'status' && (!op || own(op))) return
  if (cmd === 'heartbeat' && !op) return
  if (cmd === 'secret' && ['list', 'ask', 'run', 'env'].includes(op)) {
    return
  }
  if (['space', 'project'].includes(cmd) && (!op || ['list', 'show'].includes(op))) return
  if (cmd === 'task' && ['list', 'show', 'chat'].includes(op)) return
  if (cmd === 'task' && op === 'add' && run.state === 'on') return
  if (cmd === 'task' && ['tell', 'start', 'pause', 'queue', 'edit', 'close'].includes(op) && own(id)) return
  if (['ready', 'todo', 'pr', 'checks'].includes(cmd) && own(op) && (!task || op === task)) return
  if ((cmd === 'merge' || cmd === 'task' && op === 'merge') && own(cmd === 'merge' ? op : id) && !run.config.approvals.includes('merge')) return
  if (cmd === 'ask' || cmd === 'alert') return
  if (cmd === 'artifact' && ['list', 'show'].includes(op)) return
  throw new Error('This action needs the human. Ask in the chat; the permission profile does not allow it.')
}

/** Trusted gateway traffic, including every helper/reviewer, is measured before bytes leave the runtime. */
export function reserveTraffic(run: PilotRun, bytes: number) {
  if (!['starting', 'on', 'workers'].includes(run.state)) throw new Error('Autopilot has stopped.')
  if (run.config.traffic && (run.traffic ?? 0) + bytes > run.config.traffic) {
    run.traffic = run.config.traffic
    throw new Error('Autopilot reached its shared network allowance.')
  }
  run.traffic = (run.traffic ?? 0) + bytes
}

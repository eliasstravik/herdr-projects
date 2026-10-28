// A coordinator or task agent that stopped, or runs without the files Shepherdr starts it with now, started again by
// Shepherdr itself with its session. After herdr restarts it restores each pane's last command, which may name files
// that moved since (hooks and prompts once lived in ~/.shepherdr, now in ~/.shepherdr/helper): that CLI exits at once,
// or runs without the hooks that tell Shepherdr what it does. Tried a few times, further apart each time, then left to
// the human: a start that keeps failing never loops.
import { basename } from 'node:path'
import type { ProcessInfo } from './herdr.ts'

/** gone from herdr; its CLI exited, the shell at its prompt; or it runs on other files than it would start with now */
export type Wrong = 'gone' | 'exited' | 'stale'

/** what is wrong with an agent's pane from what runs in it (want: the path each flag should name); nothing while it is
 *  fine, or when its CLI isn't found in it */
export function paneWrong(kind: string, p: ProcessInfo, want: Record<string, string>): Wrong | undefined {
  if (p.shell_pid && p.foreground_process_group_id === p.shell_pid) return 'exited'
  const argv = p.foreground_processes?.find((x) => x.argv?.some((a) => basename(a) === kind))?.argv
  if (!argv) return undefined
  for (const [flag, path] of Object.entries(want)) {
    const i = argv.indexOf(flag)
    if (i < 0 || argv[i + 1] !== path) return 'stale'
  }
  return undefined
}

export interface RevivalTimes {
  /** how long it is seen wrong before it is started again: one starting, or herdr just back, settles first */
  grace: number
  /** how long after the try before each try may go: as many tries as gaps */
  gaps: number[]
  /** how long it runs well without a break before its tries count again from none */
  steady: number
}

/** every agent's tries at starting it again: seen() while it is wrong says when to try and when to give up */
export class Revivals {
  private wrong = new Map<string, number>()
  private tries = new Map<string, number[]>()
  private given = new Set<string>()
  /** since when it has been seen running well, without a break */
  private well = new Map<string, number>()
  /** whether it was at work when last seen running: told to carry on once it is back */
  private working = new Map<string, boolean>()

  constructor(readonly times: RevivalTimes = { grace: 20_000, gaps: [0, 60_000, 5 * 60_000], steady: 10 * 60_000 }) {}

  /** its status as herdr and its hooks tell it, while it runs */
  status(key: string, st: string) {
    if (st !== 'gone') this.working.set(key, st === 'working')
  }

  wasWorking(key: string) {
    return !!this.working.get(key)
  }

  /** seen running well: once that lasts a while without a break, its tries are forgotten */
  fine(key: string, now = Date.now()) {
    this.wrong.delete(key)
    const since = this.well.get(key) ?? now
    this.well.set(key, since)
    if (now - since < this.times.steady) return
    this.tries.delete(key)
    this.given.delete(key)
  }

  /** not looked at (herdr out of reach, or an agent not to keep running): seen wrong from the next look */
  skip(key: string) {
    this.wrong.delete(key)
    this.well.delete(key)
  }

  /** seen wrong: 'go' to start it again now, 'give-up' once when its tries are used up, else nothing yet */
  seen(key: string, now = Date.now()): 'go' | 'give-up' | undefined {
    this.well.delete(key)
    const since = this.wrong.get(key) ?? now
    this.wrong.set(key, since)
    if (now - since < this.times.grace || this.given.has(key)) return undefined
    const tries = this.tries.get(key) ?? []
    if (tries.length >= this.times.gaps.length) {
      this.given.add(key)
      return 'give-up'
    }
    if (tries.length && now - tries.at(-1)! < this.times.gaps[tries.length]) return undefined
    this.tries.set(key, [...tries, now])
    // the start it sets going settles before it is judged again
    this.wrong.delete(key)
    return 'go'
  }
}

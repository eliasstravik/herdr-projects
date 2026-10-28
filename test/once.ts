// One process at a time: test files run in parallel, and some work (installing, building the screen) must happen
// once for all of them. The first takes a lock file and does it; the others wait and use what it made.
import { closeSync, openSync, readFileSync, rmSync, writeFileSync } from 'node:fs'

const alive = (pid: number) => {
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

/** `done()` once it is true, else `work()` run under `lock` by one process while the others wait for it */
export async function once(lock: string, done: () => boolean, work: () => Promise<void> | void) {
  while (!done()) {
    let fd: number
    try {
      fd = openSync(lock, 'wx')
    } catch {
      // another one is at it: wait, unless it died at it
      let pid = 0
      try {
        pid = Number(readFileSync(lock, 'utf8').trim() || 0)
      } catch {}
      if (pid && !alive(pid)) rmSync(lock, { force: true })
      else await Bun.sleep(100)
      continue
    }
    try {
      writeFileSync(fd, String(process.pid))
      closeSync(fd)
      if (!done()) await work()
    } finally {
      rmSync(lock, { force: true })
    }
  }
}

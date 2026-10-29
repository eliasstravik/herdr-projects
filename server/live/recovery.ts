/** Retry failed links slowly, resetting only after a minute of continuous success. No timers or global state. */
export class LinkBackoff {
  private delay = 3000
  private healthySince?: number
  fail(now = Date.now()) {
    if (this.healthySince !== undefined && now - this.healthySince >= 60_000) this.delay = 3000
    this.healthySince = undefined
    const delay = this.delay
    this.delay = Math.min(30_000, this.delay * 2)
    return delay
  }
  success(now = Date.now()) {
    this.healthySince ??= now
  }
}

/** An answering HTTP server is insufficient: its herdr link must work too. */
export class WorkerHealth {
  private down?: number
  private healthySince?: number
  private nextRestart = 0
  private cooldown = 180_000
  observe(healthy: boolean, now = Date.now()) {
    if (healthy) {
      this.down = undefined
      this.healthySince ??= now
      if (now - this.healthySince >= 60_000) {
        this.cooldown = 180_000
        this.nextRestart = 0
      }
    } else {
      this.healthySince = undefined
      this.down ??= now
    }
  }
  restartDue(now = Date.now()) {
    return this.down !== undefined && now - this.down >= 180_000 && now >= this.nextRestart
  }
  restarting(now = Date.now()) {
    this.down = now
    this.nextRestart = now + this.cooldown
    this.cooldown = Math.min(15 * 60_000, this.cooldown * 2)
  }
}

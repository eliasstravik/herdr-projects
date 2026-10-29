// The way back from another machine: its agents' shepherdr command (the helper's bin/shepherdr) talks to the socket
// main.sock in the helper's folder there, which an SSH tunnel kept open from here leads to this worker's API. The API
// stays on 127.0.0.1 here; SSH is the only way in.
import { LinkBackoff } from './recovery.ts'
import { join } from 'node:path'
import { BOX, helperDir, q, sshArgs } from './box.ts'

type TunnelProcess = { exited: Promise<unknown>; kill(): void }

/** Exactly one opening attempt or tunnel, including during cleanup and shutdown. */
export class Tunnel {
  private stopped = false
  private proc?: TunnelProcess
  private timer?: ReturnType<typeof setTimeout>
  private recovery = new LinkBackoff()
  constructor(private launch: (track: (p: TunnelProcess) => void, stopped: () => boolean) => Promise<void>) {}
  start() { if (!this.timer) void this.open() }
  private opening = false
  private async open() {
    if (this.stopped || this.opening) return
    this.opening = true
    try {
      await this.launch((p) => { this.proc = p; if (this.stopped) p.kill() }, () => this.stopped)
    } catch {
      // Failure to spawn is retried just like DNS/SSH failure.
    } finally {
      this.opening = false
      this.proc = undefined
      if (!this.stopped) this.timer = setTimeout(() => { this.timer = undefined; void this.open() }, this.recovery.fail())
    }
  }
  connected() { this.recovery.success() }
  stop() {
    this.stopped = true
    if (this.timer) clearTimeout(this.timer)
    this.proc?.kill()
  }
}

export function keepTunnel(port: number) {
  const b = BOX!
  const sock = join(helperDir(b.home), 'main.sock')
  const tunnel = new Tunnel(async (track, stopped) => {
    // a socket left from before stops the forward: it goes first (sshd doesn't remove it on its own); without
    // holding up the worker meanwhile
    const rm = Bun.spawn(sshArgs(b.target, `rm -f ${q(sock)}`), { stdout: 'ignore', stderr: 'ignore', timeout: 20_000 })
    track(rm)
    const code = await rm.exited
    if (code !== 0 || stopped()) return
    // some sshds (exe.dev's) make the socket as root, so the user's own agents can't open it: hand it back first.
    // Held while the socket is there: one removed under it (an overlapping worker's rm after an update) opens again
    const keep = `[ -O ${q(sock)} ] || sudo -n chown "$(id -u):$(id -g)" ${q(sock)} 2>/dev/null; while [ -S ${q(sock)} ] && printf . 2>/dev/null; do sleep 10; done`
    const args = sshArgs(b.target, keep, {
      extra: ['-o', 'ControlPath=none', '-o', 'ExitOnForwardFailure=yes', '-R', `${sock}:127.0.0.1:${port}`],
    })
    const p = Bun.spawn(args, { stdin: 'ignore', stdout: 'pipe', stderr: 'ignore' })
    track(p)
    // HOLD emits a byte only after SSH has established the forward. Drain it continuously.
    const reader = p.stdout.getReader()
    try {
      while (!(await reader.read()).done) tunnel.connected()
    } finally {
      p.kill()
      await p.exited
    }
  })
  tunnel.start()
  return tunnel
}

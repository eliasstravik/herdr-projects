// The way back from another machine: its agents' shepherdr command (the helper's bin/shepherdr) talks to the socket
// main.sock in the helper's folder there, which an SSH tunnel kept open from here leads to this worker's API. The API
// stays on 127.0.0.1 here; SSH is the only way in.
import { join } from 'node:path'
import { BOX, helperDir, q, sshArgs } from './box.ts'

export function keepTunnel(port: number) {
  const b = BOX!
  const sock = join(helperDir(b.home), 'main.sock')
  const open = async () => {
    // a socket left from before stops the forward: it goes first (sshd doesn't remove it on its own); without
    // holding up the worker meanwhile
    const rm = Bun.spawn(sshArgs(b.target, `rm -f ${q(sock)}`), { stdout: 'ignore', stderr: 'ignore', timeout: 20_000 })
    await rm.exited
    // some sshds (exe.dev's) make the socket as root, so the user's own agents can't open it: hand it back first.
    // Held while the socket is there: one removed under it (an overlapping worker's rm after an update) opens again
    const keep = `[ -O ${q(sock)} ] || sudo -n chown "$(id -u):$(id -g)" ${q(sock)} 2>/dev/null; while [ -S ${q(sock)} ] && printf . 2>/dev/null; do sleep 10; done`
    const args = sshArgs(b.target, keep, {
      extra: ['-o', 'ControlPath=none', '-o', 'ExitOnForwardFailure=yes', '-R', `${sock}:127.0.0.1:${port}`],
    })
    const p = Bun.spawn(args, { stdin: 'ignore', stdout: 'ignore', stderr: 'ignore' })
    p.exited.then(() => setTimeout(open, 3000))
  }
  void open()
}

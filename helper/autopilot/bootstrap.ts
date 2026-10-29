// Runs INSIDE the network and PID namespace. Its only connection out is the trusted Unix gateway.
import { existsSync, openSync, readSync, closeSync, statSync } from 'node:fs'
import { join, relative } from 'node:path'
import { connect, createServer } from 'node:net'
const relay = createServer((socket) => {
  const upstream = connect('/run/shepherdr/gateway.sock')
  socket.pipe(upstream).pipe(socket)
  socket.on('error', () => upstream.destroy())
  upstream.on('error', () => socket.destroy())
})
await new Promise<void>((resolve, reject) => { relay.once('error', reject); relay.listen(18765, '127.0.0.1', resolve) })
const [binary, ...args] = process.argv.slice(2)
const env = { ...process.env, HTTP_PROXY: 'http://127.0.0.1:18765', HTTPS_PROXY: 'http://127.0.0.1:18765', ALL_PROXY: 'http://127.0.0.1:18765', NO_PROXY: '127.0.0.1', http_proxy: 'http://127.0.0.1:18765', https_proxy: 'http://127.0.0.1:18765', SHEPHERDR_URL: 'http://127.0.0.1:18765' }
// Open agent-written paths only INSIDE the namespace. The host receives bytes, never a path to open.
const positions = new Map<string, number>()
let capturing = false
async function capture() {
  if (capturing) return
  capturing = true
  try {
    const files = ['.claude/projects', '.codex/sessions', '.pi/agent/sessions'].flatMap((dir) => {
      const cwd = join(process.env.HOME!, dir)
      return existsSync(cwd) ? Array.from(new Bun.Glob('**/*.jsonl').scanSync({ cwd, dot: true, absolute: true, onlyFiles: true })) : []
    })
    for (const file of files) {
      let fd: number | undefined
      try {
        const size = statSync(file).size
        const old = positions.get(file) ?? 0, offset = old > size ? 0 : old
        if (offset === size) continue
        fd = openSync(file, 'r')
        const buffer = Buffer.alloc(Math.min(size - offset, 128 * 1024)), n = readSync(fd, buffer, 0, buffer.length, offset)
        const path = relative(process.env.HOME!, file), kind = path.startsWith('.claude/') ? 'claude' : path.startsWith('.codex/') ? 'codex' : 'pi'
        const reply = await fetch('http://127.0.0.1:18765/api/transcript', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ kind, file: path, offset, data: buffer.subarray(0, n).toString('base64') }), signal: AbortSignal.timeout(2000) })
        if (reply.ok) positions.set(file, offset + n)
      } catch { /* a file being rotated, or the gateway stopping */ }
      finally { if (fd !== undefined) closeSync(fd) }
    }
  } finally { capturing = false }
}
const captureTimer = setInterval(() => void capture(), 250)
const child = Bun.spawn([binary, ...args], { env, stdin: 'inherit', stdout: 'inherit', stderr: 'inherit' })
const code = await child.exited
clearInterval(captureTimer)
while (capturing) await Bun.sleep(20)
await capture()
process.exit(code)

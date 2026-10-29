// Read the existing daemon only: never start/resume a thread or start another daemon.
import { spawn } from 'node:child_process'
import { Duplex } from 'node:stream'
import type WebSocketType from 'ws'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'
// Bun's built-in `ws` shim ignores createConnection. Use ws's implementation for the CLI proxy transport.
const require = createRequire(import.meta.url)
const WebSocket: typeof WebSocketType = require(join(dirname(require.resolve('ws/package.json')), 'lib/websocket.js'))
import type { AgentRuntime } from '../../shared/types.ts'
import { BOX, script, sshArgs } from './box.ts'

/** Null effort means unset OR unavailable in Codex's protocol, not a known model default. */
export function codexRuntime(thread: { model?: unknown; reasoningEffort?: unknown; status?: { type?: string } }, at: number): AgentRuntime | undefined {
  if (!thread.status?.type || thread.status.type === 'notLoaded') return undefined
  return {
    source: 'session', at,
    ...(typeof thread.model === 'string' && thread.model !== 'default' ? { model: thread.model } : {}),
    ...(typeof thread.reasoningEffort === 'string' ? { effort: thread.reasoningEffort } : {}),
  }
}

/** One short-lived, read-only connection for all currently tracked Codex sessions. Works through the remote CLI's
 * documented raw socket proxy too. Unsupported/old CLIs simply fall back to their transcripts. */
export async function readCodexRuntime(ids: string[]): Promise<Map<string, AgentRuntime>> {
  const out = new Map<string, AgentRuntime>()
  if (!ids.length) return out
  const argv = ['codex', 'app-server', 'proxy']
  const p = BOX ? spawn('ssh', sshArgs(BOX.target, script(BOX, argv)).slice(1), { stdio: ['pipe', 'pipe', 'ignore'] })
    : spawn(argv[0], argv.slice(1), { stdio: ['pipe', 'pipe', 'ignore'] })
  const socket = new Duplex({
    read() { p.stdout?.resume() },
    write(chunk, encoding, callback) { p.stdin!.write(chunk, encoding, callback) },
    final(callback) { p.stdin!.end(callback) },
  })
  p.stdout!.on('data', (chunk) => { if (!socket.push(chunk)) p.stdout!.pause() })
  p.stdout!.on('end', () => socket.push(null))
  const capabilities = new Map<string, boolean>()
  return new Promise((resolve) => {
    let done = false
    const pending = new Map<number, string>()
    const ws = new WebSocket('ws://localhost/', { createConnection: () => socket as never, handshakeTimeout: 2000, maxPayload: 1 << 20 })
    const finish = () => {
      if (done) return
      done = true
      clearTimeout(timer)
      ws.terminate()
      socket.destroy()
      p.kill()
      for (const state of out.values()) {
        if (state.model && capabilities.get(state.model) === false) state.effortUnsupported = true
      }
      resolve(out)
    }
    const timer = setTimeout(finish, 3000)
    p.on('error', finish)
    socket.on('error', finish)
    ws.on('error', finish)
    ws.on('close', finish)
    ws.on('open', () => ws.send(JSON.stringify({ id: 0, method: 'initialize', params: { clientInfo: { name: 'shepherdr_runtime', version: '1' } } })))
    ws.on('message', (data) => {
      try {
        const r = JSON.parse(data.toString())
        if (r.id === 0) {
          if (r.error) return finish()
          ws.send(JSON.stringify({ method: 'initialized', params: {} }))
          pending.set(-1, '')
          ws.send(JSON.stringify({ id: -1, method: 'model/list', params: { includeHidden: true } }))
          ids.forEach((id, i) => {
            pending.set(i + 1, id)
            ws.send(JSON.stringify({ id: i + 1, method: 'thread/read', params: { threadId: id, includeTurns: false } }))
          })
        } else if (r.id === -1) {
          for (const m of r.result?.data ?? []) {
            if (typeof m.model === 'string' && Array.isArray(m.supportedReasoningEfforts)) capabilities.set(m.model, m.supportedReasoningEfforts.length > 0)
          }
          pending.delete(-1)
          if (!pending.size) finish()
        } else if (pending.has(r.id)) {
          const id = pending.get(r.id)!
          const runtime = r.result?.thread && codexRuntime(r.result.thread, Date.now())
          if (runtime) out.set(id, runtime)
          pending.delete(r.id)
          if (!pending.size) finish()
        }
      } catch { finish() }
    })
  })
}

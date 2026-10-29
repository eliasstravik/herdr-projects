import { LinkBackoff } from './recovery.ts'
// herdr's socket API: newline-delimited JSON over a Unix socket. One request per connection (herdr closes it after
// the reply); a subscription keeps its connection open and streams events. On another machine (box.ts) the same JSON
// goes through herdr's own remote bridge (herdr --session <s> remote-api-bridge over SSH), one bridge per connection.
import { connect } from 'node:net'
import { homedir } from 'node:os'
import { join } from 'node:path'
import { BOX, script, sshArgs } from './box.ts'

export function socketPath() {
  return process.env.HERDR_SOCKET_PATH ?? process.env.HERDR_SOCKET ?? join(homedir(), '.config', 'herdr', 'herdr.sock')
}

export class HerdrError extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(message)
    this.name = 'HerdrError'
  }
}

let n = 0

/** a connection to herdr's API: its socket here, or a bridge to the other machine's */
interface Conn {
  write(s: string): void
  destroy(): void
  on(e: 'connect', f: () => void): void
  on(e: 'data', f: (d: Buffer) => void): void
  on(e: 'error' | 'close', f: (e?: Error) => void): void
}

function open(path: string): Conn {
  if (!BOX || path !== socketPath()) return connect(path)
  const p = Bun.spawn(sshArgs(BOX.target, script(BOX, [BOX.herdr, '--session', BOX.session, 'remote-api-bridge']), { extra: ['-o', 'ControlPath=none'] }), { stdin: 'pipe', stdout: 'pipe', stderr: 'ignore' })
  const fns: Record<string, ((x?: unknown) => void)[]> = { connect: [], data: [], error: [], close: [] }
  const fire = (e: string, x?: unknown) => fns[e].forEach((f) => f(x))
  let closed = false
  const close = () => {
    if (closed) return
    closed = true
    fire('close')
  }
  queueMicrotask(() => fire('connect'))
  ;(async () => {
    try {
      const r = p.stdout.getReader()
      for (;;) {
        const { done, value } = await r.read()
        if (done) break
        fire('data', Buffer.from(value))
      }
    } catch (e) {
      fire('error', e)
    }
    close()
  })()
  p.exited.then(close)
  return {
    write: (s) => {
      try {
        p.stdin.write(s)
        p.stdin.flush()
      } catch (e) {
        fire('error', e as Error)
      }
    },
    destroy: () => {
      closed = true
      try {
        p.kill()
      } catch {
        /* gone already */
      }
    },
    on: (e: string, f: (x?: never) => void) => void fns[e].push(f as (x?: unknown) => void),
  } as Conn
}

/** herdr's socket refused the connection or is not there: herdr is down or restarting */
const gone = (e: Error & { code?: string }) => ['ECONNREFUSED', 'ENOENT', 'ECONNRESET', 'EPIPE'].includes(e.code ?? '') || /ECONNREFUSED|ENOENT/.test(e.message)

export async function rpc<T = unknown>(method: string, params: Record<string, unknown> = {}, timeoutMs = 10_000, path = socketPath()): Promise<T> {
  const id = `shep-${Date.now().toString(36)}-${++n}`
  return new Promise<T>((resolve, reject) => {
    let buf = ''
    let done = false
    const sock = open(path)
    const finish = (f: () => void) => {
      if (done) return
      done = true
      clearTimeout(timer)
      sock.destroy()
      f()
    }
    const timer = setTimeout(() => finish(() => reject(new HerdrError('timeout', `herdr ${method} timed out`))), timeoutMs)
    sock.on('connect', () => sock.write(`${JSON.stringify({ id, method, params })}\n`))
    sock.on('data', (d) => {
      buf += d.toString('utf8')
      let i: number
      while ((i = buf.indexOf('\n')) >= 0) {
        const line = buf.slice(0, i).trim()
        buf = buf.slice(i + 1)
        if (!line) continue
        const f = JSON.parse(line) as { id?: string; result?: T; error?: { code: string; message: string } }
        if (f.id !== id) continue
        if (f.error) finish(() => reject(new HerdrError(f.error!.code, f.error!.message)))
        else finish(() => resolve(f.result as T))
      }
    })
    // its socket refusing, or gone: herdr stopped or is starting again. Said plainly, never "connect ECONNREFUSED …"
    sock.on('error', (e) => finish(() => reject(new HerdrError('unreachable', !e || gone(e) ? "herdr isn't answering" : e.message))))
    sock.on('close', () => finish(() => reject(new HerdrError('closed', `herdr closed the connection during ${method}`))))
  })
}

export type AgentStatus = 'idle' | 'working' | 'blocked' | 'done' | 'unknown'

export interface AgentSession {
  source: string
  agent: string
  kind: 'id' | 'path'
  value: string
}

export interface AgentInfo {
  agent: string | null
  agent_status: AgentStatus
  agent_session?: AgentSession | null
  cwd: string | null
  foreground_cwd?: string | null
  name?: string | null
  pane_id: string
  tab_id: string
  workspace_id: string
  title?: string | null
  terminal_title_stripped?: string | null
  interactive_ready?: boolean
}

/** what runs in a pane (pane.process_info) */
export interface ProcessInfo {
  shell_pid?: number
  foreground_process_group_id?: number
  foreground_processes?: { argv?: string[]; name?: string; pid?: number }[]
}

/** a workspace's place in a herdr worktree group: a repo's own checkout, or a linked worktree nested under it */
export interface WorktreeOf {
  checkout_path: string
  is_linked_worktree: boolean
  repo_root: string
}

export interface WorkspaceInfo {
  workspace_id: string
  label: string
  focused?: boolean
  tab_count?: number
  pane_count?: number
  worktree?: WorktreeOf
}

export interface PaneInfo {
  pane_id: string
  workspace_id: string
  cwd?: string | null
  foreground_cwd?: string | null
}

/** the workspace a worktree's space nests under, or the repo folder herdr finds (or opens) one for */
export type WorktreeFrom = { workspace_id: string } | { cwd: string }

/** what herdr gives back when it creates or opens a worktree workspace */
export interface WorktreeSpace {
  workspace: WorkspaceInfo
  root_pane: { pane_id: string }
  worktree?: { path: string }
  already_open?: boolean
}

export const herdr = {
  ping: () => rpc<{ version?: string; protocol?: number }>('ping', {}, 3000),
  agents: () => rpc<{ agents: AgentInfo[] }>('agent.list').then((r) => r.agents),
  agent: (target: string) => rpc<{ agent: AgentInfo }>('agent.get', { target }).then((r) => r.agent),
  workspaces: () => rpc<{ workspaces: WorkspaceInfo[] }>('workspace.list').then((r) => r.workspaces),
  createWorkspace: (label: string, cwd: string, env: Record<string, string> = {}) =>
    rpc<{ workspace: WorkspaceInfo; root_pane: { pane_id: string } }>('workspace.create', { label, cwd, env, focus: false }),
  renameWorkspace: (workspace_id: string, label: string) => rpc('workspace.rename', { workspace_id, label }),
  createTab: (workspace_id: string, label: string, cwd: string, env: Record<string, string> = {}) =>
    rpc<{ tab: { tab_id: string }; root_pane: { pane_id: string } }>('tab.create', { workspace_id, label, cwd, env, focus: false }),
  start: (name: string, kind: string, pane_id: string, args: string[] = []) => rpc<{ agent: AgentInfo }>('agent.start', { name, kind, pane_id, args, timeout_ms: 60_000 }, 70_000),
  prompt: (target: string, text: string) => rpc('agent.prompt', { target, text }, 20_000),
  sendKeys: (target: string, keys: string[]) => rpc('agent.send_keys', { target, keys }),
  paneKeys: (pane_id: string, keys: string[]) => rpc('pane.send_keys', { pane_id, keys }),
  paneText: (pane_id: string, text: string) => rpc('pane.send_text', { pane_id, text }),
  read: (pane_id: string, format: 'ansi' | 'text' = 'ansi', lines = 200) =>
    rpc<{ read?: { text: string }; text?: string }>('pane.read', { pane_id, source: 'recent', format, lines, strip_ansi: format !== 'ansi' }).then((r) => r.read?.text ?? r.text ?? ''),
  /** what the pane shows right now, plain: unlike a deep read it never scrolls a working agent's view */
  visible: (pane_id: string) =>
    rpc<{ read?: { text: string }; text?: string }>('pane.read', { pane_id, source: 'visible', format: 'text', strip_ansi: true }).then((r) => r.read?.text ?? r.text ?? ''),
  /** a pane's size in cells */
  size: (pane_id: string) =>
    rpc<{ layout: { panes: { pane_id: string; rect: { width: number; height: number } }[] } }>('pane.layout', { pane_id }).then((r) => {
      const rect = r.layout.panes.find((p) => p.pane_id === pane_id)?.rect
      return rect ? { cols: rect.width, rows: rect.height } : null
    }),
  panes: (workspace_id: string) => rpc<{ panes: PaneInfo[] }>('pane.list', { workspace_id }).then((r) => r.panes),
  /** what runs in a pane: its shell, and the processes in its foreground with their command lines */
  processInfo: (pane_id: string) =>
    rpc<{ process_info: ProcessInfo }>('pane.process_info', { pane_id }).then((r) => r.process_info),
  /** the shell sits at its prompt: nothing runs in the foreground */
  paneIdle: (pane_id: string) =>
    rpc<{ process_info: { shell_pid?: number; foreground_process_group_id?: number } }>('pane.process_info', { pane_id }).then(
      (r) => !!r.process_info.shell_pid && r.process_info.foreground_process_group_id === r.process_info.shell_pid,
    ),
  /** a checkout on a new (or existing) branch, opened as a workspace nested under `from`: the repo's own workspace, or
   *  the one herdr finds (or opens) for the repo's folder. path: where (herdr picks one when it is left out) */
  worktreeCreate: (from: WorktreeFrom, branch: string, base: string, label: string, path?: string) =>
    rpc<WorktreeSpace>('worktree.create', { ...from, branch, base, label, ...(path ? { path } : {}), focus: false }, 120_000),
  /** an existing checkout as a workspace nested under `from` (the one already open, if it is) */
  worktreeOpen: (from: WorktreeFrom, path: string, label: string) => rpc<WorktreeSpace>('worktree.open', { ...from, path, label, focus: false }, 60_000),
  /** git worktree remove of a linked worktree's workspace, which closes it; never forced, never the branch */
  worktreeRemove: (workspace_id: string, force = false) => rpc('worktree.remove', { workspace_id, force }, 120_000),
  closePane: (pane_id: string) => rpc('pane.close', { pane_id }),
  closeWorkspace: (workspace_id: string) => rpc('workspace.close', { workspace_id }),
  notify: (title: string, body: string) => rpc('notification.show', { title, body }).catch(() => undefined),
}

export type HerdrEvent = { event: string; data: Record<string, unknown> }

/**
 * Keeps one subscription open and reopens it whenever it closes (herdr closes a subscriber that falls behind).
 * Callers re-read state from a snapshot after `onOpen`, so nothing missed in between stays missed.
 */
export class Subscription {
  private sock: Conn | null = null
  private stopped = false
  private retry?: ReturnType<typeof setTimeout>
  private handshake?: ReturnType<typeof setTimeout>
  private recovery = new LinkBackoff()
  private wanted: Record<string, unknown>[] = []
  constructor(
    private onEvent: (e: HerdrEvent) => void,
    private onOpen: () => void = () => undefined,
    private connect: () => Conn = () => open(socketPath()),
  ) {}

  set(subs: Record<string, unknown>[]) {
    const same = JSON.stringify(subs) === JSON.stringify(this.wanted)
    this.wanted = subs
    if (!same) this.reopen()
  }

  private reopen() {
    if (this.retry) clearTimeout(this.retry)
    if (this.handshake) clearTimeout(this.handshake)
    const previous = this.sock
    this.sock = null
    previous?.destroy()
    if (this.stopped || !this.wanted.length) return
    let sock: Conn
    try { sock = this.connect() } catch {
      this.retry = setTimeout(() => this.reopen(), this.recovery.fail())
      return
    }
    this.sock = sock
    let buf = ''
    let opened = false
    sock.on('connect', () => {
      sock.write(`${JSON.stringify({ id: `sub-${++n}`, method: 'events.subscribe', params: { subscriptions: this.wanted } })}\n`)
      // SSH spawn is not a successful bridge connection; wait for an actual frame.
    })
    sock.on('data', (d) => {
      if (this.sock !== sock || this.stopped) return
      buf += d.toString('utf8')
      let i: number
      while ((i = buf.indexOf('\n')) >= 0) {
        const line = buf.slice(0, i).trim()
        buf = buf.slice(i + 1)
        if (!line) continue
        try {
          const f = JSON.parse(line)
          if (f.error) { again(); return }
          if (!opened && ('result' in f || f.event)) {
            opened = true
            if (this.handshake) clearTimeout(this.handshake)
            this.recovery.success()
            this.onOpen()
          }
          if (f.event) this.onEvent(f as HerdrEvent)
        } catch {
          /* a frame we can't read */
        }
      }
    })
    const again = () => {
      if (this.sock !== sock || this.stopped) return
      this.sock = null
      if (this.handshake) clearTimeout(this.handshake)
      sock.destroy()
      this.retry = setTimeout(() => this.sock === null && this.reopen(), this.recovery.fail())
    }
    this.handshake = setTimeout(again, 15_000)
    sock.on('error', again)
    sock.on('close', again)
  }

  stop() {
    this.stopped = true
    if (this.handshake) clearTimeout(this.handshake)
    if (this.retry) clearTimeout(this.retry)
    this.sock?.destroy()
    this.sock = null
  }
}

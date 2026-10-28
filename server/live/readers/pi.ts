// pi's log: ~/.pi/agent/sessions/--<cwd>--/<time>_<id>.jsonl. The first line is {type: 'session', id, cwd}; then
// entries with their own id: message {message: {role, content}}, model_change, custom, ... The chat reads the user's
// and the assistant's text; system prompts, tool calls and results, and thinking stay out.
import type { LogEvent, Reader } from './types.ts'

type Json = any

const parseLine = (text: string): Json => {
  try {
    return JSON.parse(text)
  } catch {
    return undefined
  }
}

/** pi expands "/skill:name args" into the skill's body in tags, then the args: show it as typed */
function cleanUser(t: string): string {
  const m = /^\s*<skill name="([^"]*)"[^>]*>[\s\S]*?<\/skill>\s*/.exec(t)
  if (m) return `/skill:${m[1]} ${t.slice(m[0].length)}`.trim()
  return t.trim()
}

function texts(c: Json): string[] {
  if (typeof c === 'string') return [c]
  if (!Array.isArray(c)) return []
  return c.filter((b) => b?.type === 'text' && typeof b.text === 'string').map((b) => b.text)
}

export class PiReader implements Reader {
  cli = 'pi'
  private sessions = new Set<string>()
  /** files whose last turn ended: a reply that comes next (pi retrying an error) starts a turn of its own */
  private ended = new Set<string>()

  reset(file: string) {
    this.sessions.delete(file)
    this.ended.delete(file)
  }

  parse(lines: { text: string; offset: number }[], file: string): LogEvent[] {
    const out: LogEvent[] = []
    for (const { text, offset } of lines) {
      const l = parseLine(text)
      if (!l || typeof l !== 'object') continue
      const t = Date.parse(l.timestamp ?? '') || (typeof l.message?.timestamp === 'number' ? l.message.timestamp : 0)

      if (l.type === 'session') {
        if (this.sessions.has(file) || typeof l.id !== 'string') continue
        this.sessions.add(file)
        out.push({ kind: 'session', id: `session-${l.id}`, at: t, sessionId: l.id })
      } else if (l.type === 'message') {
        const m = l.message ?? {}
        const id = String(l.id ?? `m${offset}`)
        if (m.role === 'user' || (m.role === 'assistant' && this.ended.has(file))) {
          this.ended.delete(file)
          out.push({ kind: 'turn-start', id: `start-${id}`, at: t })
        }
        if (m.role === 'user') {
          const words = texts(m.content).map(cleanUser).filter(Boolean).join('\n\n')
          if (words) out.push({ kind: 'user', id, at: t, text: words })
        } else if (m.role === 'assistant') {
          const words = texts(m.content)
            .map((s) => s.trim())
            .filter(Boolean)
            .join('\n\n')
          if (words) out.push({ kind: 'agent', id, at: t, text: words })
          const u = m.usage
          const tokens = typeof u?.totalTokens === 'number' ? u.totalTokens : (u?.input ?? 0) + (u?.output ?? 0) + (u?.cacheRead ?? 0) + (u?.cacheWrite ?? 0)
          if (tokens > 0) out.push({ kind: 'usage', id: `usage-${id}`, at: t, tokens })
          // toolUse means the agent goes on; stop, aborted, error and length all end the turn
          if (m.stopReason && m.stopReason !== 'toolUse') {
            if (m.stopReason === 'error') out.push({ kind: 'error', id: `error-${id}`, at: t, text: String(m.errorMessage ?? '') })
            this.ended.add(file)
            out.push({ kind: 'turn-end', id: `turn-${id}`, at: t })
          }
        }
      }
    }
    return out
  }
}

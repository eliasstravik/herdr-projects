// A reply as it streams onto the CLI's screen. The logs get a reply only once it is whole, but Claude Code and Codex
// draw it as it comes: blocks headed by a bullet at the left edge (● or • a reply or a tool step, whose output hangs
// under ⎿ or └), a status line while it thinks or runs a tool ("✻ Honking…", "• Working (3s • esc to interrupt)"),
// and the message box below. A reply is streaming when the last block above the box is text with no status line under
// it. pi draws replies without a mark of their own, so its screen tells nothing.

const HEAD = /^[●⏺•] (.*)$/
const STEP = /^\s+[⎿└]/
const BOX = /^[❯›](\s|$)/
const RULE = /^[─━]{3,}/
const TIP = /^\s+⎿\s+Tip:/
const CODEX_STATUS = /^• .*\(.*esc to interrupt\)/

/** the reply text streaming at the bottom of the screen, its lines as shown; none while it thinks or runs a tool */
export function replyOnScreen(cli: string, screen: string): string | undefined {
  if (cli !== 'claude' && cli !== 'codex') return undefined
  const lines = screen.split('\n').map((l) => l.trimEnd())
  const box = lines.findLastIndex((l) => BOX.test(l))
  if (box < 0) return undefined
  let i = box - 1
  while (i >= 0 && (!lines[i].trim() || RULE.test(lines[i]) || TIP.test(lines[i]))) i--
  const body: string[] = []
  for (; i >= 0; i--) {
    const l = lines[i]
    if (!l.trim()) continue
    if (/^\s/.test(l)) {
      if (STEP.test(l)) return undefined
      body.unshift(l.trim())
      continue
    }
    // the block's head, or a status line: thinking, a tool running, a turn's "done"
    const m = HEAD.exec(l)
    if (!m || (cli === 'codex' && CODEX_STATUS.test(l))) return undefined
    return [m[1].trim(), ...body].join('\n')
  }
  return undefined
}

// A reply as it streams onto the CLI's screen, on screens shortened from real Claude Code 2.1 and Codex 0.163 panes.
import { describe, expect, test } from 'bun:test'
import { sawReply } from '../server/live/chats.ts'
import { replyOnScreen } from '../server/live/streaming.ts'

const RULE = '─'.repeat(40)
const claude = (...above: string[]) => [...above, '', RULE, '❯ ', RULE, '  ⏵⏵ bypass permissions on (shift+tab to cycle) · esc to interrupt'].join('\n')
const codex = (...above: string[]) => [...above, '', '› Ask Codex to do anything', '', '  GPT-6.1-Sol default fast · /work/app', '  ← for agents · ? for shortcuts'].join('\n')

describe('a reply on the screen', () => {
  test('Claude Code: none while it thinks or runs a tool', () => {
    expect(replyOnScreen('claude', claude('❯ <shepherdr><news task="SHE-9" event="ready">SHE-9 is ready.</news></shepherdr>', '', '✻ Infusing… (1s · thinking)'))).toBeUndefined()
    expect(
      replyOnScreen('claude', claude('❯ Run ls', '● I will run it now.', '', '● Listing files in current directory', '  ⎿  $ ls', '', '✢ Infusing… (4s · ↓ 390 tokens · thinking)')),
    ).toBeUndefined()
    // a background job that finished wakes it: the job's line, then it thinks
    expect(
      replyOnScreen('claude', claude('● started', '', '✻ Crunched for 2s · done 9:41 PM', '', '● Background command "Sleep 6 seconds" completed (exit code 0)', '', '· Hullaballooing…', '  ⎿  Tip: Connect Claude to your IDE · /ide')),
    ).toBeUndefined()
    // its turn is done
    expect(replyOnScreen('claude', claude('● All set.', '', '✻ Sautéed for 7s · done 9:39 PM'))).toBeUndefined()
  })

  test('Claude Code: the text block it is writing, once no status line is under it', () => {
    expect(replyOnScreen('claude', claude('● Background command "Sleep" completed (exit code 0)', '', '● Six seconds passed, then a quiet hi,', '  the background shell gave its word'))).toBe(
      'Six seconds passed, then a quiet hi,\nthe background shell gave its word',
    )
    expect(replyOnScreen('claude', claude('● <no-reply>SHE-9 merged</no-reply>'))).toBe('<no-reply>SHE-9 merged</no-reply>')
  })

  test('Codex: none while it works or explores; its text block as it streams', () => {
    expect(replyOnScreen('codex', codex('› Run ls first', '', '• Explored', '  └ List ls', '', '• Working (1s • esc to interrupt)'))).toBeUndefined()
    expect(replyOnScreen('codex', codex('• Explored', '  └ List ls', '    + Show details', '', '• Sheep wake', '  Mists drift', '  Hooves pat'))).toBe('Sheep wake\nMists drift\nHooves pat')
  })

  test('pi draws no mark of its own for a reply: its screen tells nothing', () => {
    expect(replyOnScreen('pi', [' Sheep drift', '── ⠴ Working ──', RULE].join('\n'))).toBeUndefined()
  })
})

describe('a reply counts once it streams', () => {
  const reads = (...texts: (string | undefined)[]) => texts.reduce((s, t) => sawReply(s, t), { on: false } as { on: boolean; last?: string }).on
  test('it must grow between two reads, so a still line never counts', () => {
    expect(reads('Six seconds')).toBe(false)
    expect(reads('Six seconds', 'Six seconds')).toBe(false)
    expect(reads('Six seconds', 'Six seconds passed')).toBe(true)
    expect(reads(undefined, 'Six seconds passed')).toBe(false)
  })
  test('a hidden <no-reply> or older [[...]] marker never counts, nor a lone letter; once on, it stays on for the turn', () => {
    expect(reads('<n', '<no-re', '<no-reply>SHE', '<no-reply>SHE-9 merged', '<no-reply>SHE-9 merged</no-reply>')).toBe(false)
    expect(reads('`<No', '`<No-Reply>SHE-9', '`<No-Reply>SHE-9 merged</No-Reply>`')).toBe(false)
    expect(reads('[[', '[[noted', '[[noted]]')).toBe(false)
    expect(reads('<b>Six</b>', '<b>Six</b> seconds')).toBe(true)
    expect(reads('[', 'S')).toBe(false)
    expect(reads('<re', '<react emoji="👍" to="SH')).toBe(false)
    // its reaction line first, then the marker: still hidden; a reply line first, then words: those count
    expect(reads('<react emoji="👍" to="SHE-9"/>\n<no-re', '<react emoji="👍" to="SHE-9"/>\n<no-reply>a 👍</no-reply>')).toBe(false)
    expect(reads('<reply to="SHE-9"/>\nIt is', '<reply to="SHE-9"/>\nIt is merged')).toBe(true)
    expect(reads('Six', 'Six seconds', undefined)).toBe(true)
  })
})

// The terminal is drawn at the pane's own size: herdr's lines go in as one write, never wrapped, and the font scales to
// the pane's columns.
import { afterAll, describe, expect, test } from 'bun:test'
import { mkdtempSync, rmSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { colsOf, fitFont, frame, MAX, MIN, PAD } from '../plugins/terminal/screen.ts'
import { herdr } from '../server/live/herdr.ts'

describe('the screen', () => {
  test("its width is the pane's; from an older machine (no size), the widest line without its colours", () => {
    expect(colsOf({ ansi: 'hi', cols: 138, rows: 43 })).toBe(138)
    // the pane grew between the read and the size: the line still fits
    expect(colsOf({ ansi: 'x'.repeat(140), cols: 138, rows: 43 })).toBe(140)
    expect(colsOf({ ansi: '\x1b[38;2;1;2;3mabcdef\x1b[0m\r\nabc' })).toBe(6)
    expect(colsOf({ ansi: '' })).toBe(80)
  })

  test('one write from a clear screen, no cursor, rows joined as a terminal moves; blank rows at the end go', () => {
    const { rows, data } = frame('one\r\n\x1b[1mtwo\x1b[0m\r\n\r\n   \x1b[0m\r\n')
    expect(rows).toBe(2)
    expect(data).toBe('\x1b[?25l\x1b[0m\x1b[H\x1b[2Jone\r\n\x1b[1mtwo\x1b[0m')
    expect(frame('').rows).toBe(1)
  })

  test('the font fills the width with the columns, padding aside', () => {
    // 138 columns of 0.6 px per px of font in a 560 px panel
    const f = fitFont(560, 138, 0.6)
    expect(f * 0.6 * 138).toBeLessThanOrEqual(560 - 2 * PAD)
    expect((f + 0.25) * 0.6 * 138).toBeGreaterThan(560 - 2 * PAD)
    // a phone's is below the floor: the screen pans sideways; a wide desktop's is above the cap
    expect(fitFont(390, 138, 0.6)).toBeLessThan(MIN)
    expect(fitFont(1600, 80, 0.6)).toBeGreaterThan(MAX)
  })
})

describe("herdr's pane size", () => {
  const dir = mkdtempSync(join(tmpdir(), 'shep-sock-'))
  const path = join(dir, 'herdr.sock')
  const asked: { method: string; params: Record<string, unknown> }[] = []
  const server = createServer((sock) => {
    sock.on('data', (d) => {
      const req = JSON.parse(d.toString().trim())
      asked.push(req)
      const panes = [
        { pane_id: 'w1:p1', rect: { x: 0, y: 0, width: 90, height: 40 } },
        { pane_id: 'w1:p2', rect: { x: 91, y: 0, width: 47, height: 40 } },
      ]
      sock.end(`${JSON.stringify({ id: req.id, result: { type: 'pane_layout', layout: { panes } } })}\n`)
    })
  })
  const listening = new Promise<void>((r) => server.listen(path, r))
  const before = process.env.HERDR_SOCKET_PATH
  afterAll(() => {
    server.close()
    rmSync(dir, { recursive: true, force: true })
    if (before === undefined) delete process.env.HERDR_SOCKET_PATH
    else process.env.HERDR_SOCKET_PATH = before
  })

  test("is its own rect in its tab's layout, in cells", async () => {
    await listening
    process.env.HERDR_SOCKET_PATH = path
    expect(await herdr.size('w1:p2')).toEqual({ cols: 47, rows: 40 })
    expect(asked.at(-1)).toMatchObject({ method: 'pane.layout', params: { pane_id: 'w1:p2' } })
    expect(await herdr.size('w9:p9')).toBeNull()
  })
})

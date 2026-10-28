// The terminal's screen as herdr reads it, for xterm.js: its width in cells, one write per read, and the font that fits.
import type { TermScreen } from '@shared/types.ts'

/** font sizes in px: the fit never goes below MIN (the screen pans sideways instead) nor above MAX (a wide panel keeps
 *  it calm); a pinch goes from the whole width up to ZOOM */
export const MIN = 8
export const MAX = 13
export const ZOOM = 24
export const LINE = 1.2
/** the screen's padding, each side, in px */
export const PAD = 12

/** the pane's width in cells: herdr's, or the widest line when that is wider (the pane grew between the read and the
 *  size) or there is none (a machine on an older build sends none). A line wider than the screen would wrap and push
 *  the top line off. */
export function colsOf(screen: TermScreen) {
  const widest = Math.max(0, ...screen.ansi.split('\n').map((l) => l.replace(/\x1b\[[0-9;:?]*[A-Za-z]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\r/g, '').length))
  return Math.max(screen.cols ?? 0, widest) || 80
}

/** herdr's lines as one write: from a clear screen, no cursor (the CLI draws its own), rows joined as a terminal moves */
export function frame(ansi: string) {
  const lines = ansi.replace(/\r/g, '').split('\n')
  while (lines.length > 1 && !lines.at(-1)!.replace(/\x1b\[[0-9;:]*m/g, '').trim()) lines.pop()
  return { rows: lines.length, data: `\x1b[?25l\x1b[0m\x1b[H\x1b[2J${lines.join('\r\n')}` }
}

/** the font size whose cells fill `width` px with `cols` cells, unclamped */
export function fitFont(width: number, cols: number, cell: number) {
  return Math.floor(((width - 2 * PAD) / (cols * cell)) * 4) / 4
}

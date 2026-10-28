// Picks the reader for a CLI, and tails a log file: each read() returns the events of the lines written since the
// last one.
import { closeSync, openSync, readSync, statSync } from 'node:fs'
import { ClaudeReader } from './claude.ts'
import { CodexReader } from './codex.ts'
import { PiReader } from './pi.ts'
import type { LogEvent, Reader } from './types.ts'

export { ClaudeReader, CodexReader, PiReader }

/** a fresh reader for herdr's agent kind (readers remember open questions and split replies per file) */
export function readerFor(cli: string): Reader | undefined {
  switch (cli.toLowerCase()) {
    case 'claude':
    case 'claude-code':
    case 'claudecode':
      return new ClaudeReader()
    case 'codex':
      return new CodexReader()
    case 'pi':
      return new PiReader()
  }
  return undefined
}

// read big files a piece at a time; Claude Code logs reach tens of MB
const CHUNK = 4 << 20

export class LogTail {
  /** byte offset of the first line not read yet (a partial last line is read again next time) */
  private offset = 0
  /** the file's inode: a new one means the file was replaced */
  private ino = -1

  constructor(
    readonly file: string,
    readonly reader: Reader,
  ) {}

  /** events of the complete lines written since the last read; starts over when the file shrank or was replaced */
  read(): LogEvent[] {
    let size: number
    try {
      const st = statSync(this.file)
      if (st.ino !== this.ino || st.size < this.offset) this.restart(st.ino)
      size = st.size
    } catch {
      return []
    }
    if (size <= this.offset) return []

    const out: LogEvent[] = []
    let fd: number
    try {
      fd = openSync(this.file, 'r')
    } catch {
      return []
    }
    try {
      let carry = Buffer.alloc(0)
      let pos = this.offset
      while (pos < size) {
        const buf = Buffer.alloc(Math.min(CHUNK, size - pos))
        const got = readSync(fd, buf, 0, buf.length, pos)
        if (got <= 0) break
        pos += got
        const data = carry.length ? Buffer.concat([carry, buf.subarray(0, got)]) : buf.subarray(0, got)
        const start = pos - data.length
        const lines: { text: string; offset: number }[] = []
        let from = 0
        for (let nl = data.indexOf(10); nl !== -1; nl = data.indexOf(10, from)) {
          let text = data.toString('utf8', from, nl)
          if (text.endsWith('\r')) text = text.slice(0, -1)
          if (text.trim()) lines.push({ text, offset: start + from })
          from = nl + 1
        }
        carry = data.subarray(from)
        this.offset = start + from
        if (lines.length) out.push(...this.reader.parse(lines, this.file))
      }
    } finally {
      closeSync(fd)
    }
    return out
  }

  /** every event in the file, from the start (later read() calls go on from the end) */
  readAll(): LogEvent[] {
    this.ino = -1
    return this.read()
  }

  private restart(ino: number) {
    this.offset = 0
    this.ino = ino
    this.reader.reset?.(this.file)
  }
}

import { LinkBackoff } from './recovery.ts'
// The main machine's copy of another machine's agent logs, kept as they grow: the helper's mirror.pl there sends what
// each followed file gains, over one SSH connection, and it is written here under the worker's store (mfs.mirrored).
// The readers then read these copies as they read this machine's own logs: Claude Code's, Codex's and pi's
// transcripts, Claude Code's session files, and the hook events of the agents Shepherdr started there.
import { closeSync, mkdirSync, openSync, readdirSync, statSync, writeSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { helperDir, spawnOn, type BoxInfo } from './box.ts'
import { mirrored } from './mfs.ts'

/** a file in a followed folder is sent whole when it changed this recently (or later, once it changes) */
const SINCE_MS = 3 * 86_400_000

export class Mirror {
  private proc: Proc | null = null
  private dirs = new Set<string>()
  private files = new Set<string>()
  /** how much of each file is here: after the link drops, it goes on from there */
  private have = new Map<string, number>()
  private recovery = new LinkBackoff()
  private stopped = false
  private timer: ReturnType<typeof setTimeout> | null = null
  /** a piece arrived: the readers may look again */
  onData: () => void = () => undefined

  constructor(
    box: BoxInfo,
    /** mirror.pl running there, and where its copy of a file is here */
    private run: () => Proc = () => spawnOn(box, ['perl', join(helperDir(box.home), 'mirror.pl')], { stdin: 'pipe', extra: ['-C'] }),
    private copy: (file: string) => string = mirrored,
  ) {}

  /** follow these folders' files and these files; the ones followed already stay followed */
  follow(dirs: string[], files: string[]) {
    const kept: string[] = []
    const lines: string[] = []
    for (const d of dirs) if (!this.dirs.has(d)) (this.dirs.add(d), kept.push(...this.kept(d, true)), lines.push(`D ${d}`))
    for (const f of files) if (!this.files.has(f)) (this.files.add(f), kept.push(...this.kept(f, false)), lines.push(`F ${f}`))
    if (lines.length) this.write([...kept, ...lines])
  }

  /** how much of a newly followed folder's files (or file) is here already, from before Shepherdr started again: only
   *  the rest is sent, so what the agents write now isn't stuck behind days of logs sent again */
  private kept(path: string, dir: boolean): string[] {
    const lines: string[] = []
    const add = (f: string) => {
      if (this.have.has(f)) return
      try {
        const s = statSync(this.copy(f))
        if (!s.isFile()) return
        this.have.set(f, s.size)
        lines.push(`K ${s.size} ${f}`)
      } catch {
        /* not here yet */
      }
    }
    if (!dir) add(path)
    else
      try {
        for (const n of readdirSync(this.copy(path))) add(`${path}/${n}`)
      } catch {
        /* nothing of it here yet */
      }
    return lines
  }

  start() {
    if (this.stopped || this.proc) return
    let p: Proc
    try { p = this.run() } catch {
      this.timer = setTimeout(() => this.start(), this.recovery.fail())
      return
    }
    this.proc = p
    this.write([`S ${Date.now() - SINCE_MS}`, ...[...this.have].map(([f, n]) => `K ${n} ${f}`), ...[...this.dirs].map((d) => `D ${d}`), ...[...this.files].map((f) => `F ${f}`)])
    void this.read(p).catch(() => p.kill())
    p.exited.then(() => {
      if (this.proc !== p) return
      this.proc = null
      if (!this.stopped) this.timer = setTimeout(() => this.start(), this.recovery.fail())
    })
  }

  stop() {
    this.stopped = true
    if (this.timer) clearTimeout(this.timer)
    this.proc?.kill()
  }

  private write(lines: string[]) {
    try {
      ;(this.proc?.stdin as import("bun").FileSink | undefined)?.write(`${lines.join('\n')}\n`)
      ;(this.proc?.stdin as import("bun").FileSink | undefined)?.flush()
    } catch {
      /* it starts again and is told everything */
    }
  }

  /** "<path>\t<offset>\t<length>\n" and the bytes, over and over */
  private async read(p: Proc) {
    let buf = Buffer.alloc(0)
    let want: { file: string; offset: number; len: number } | null = null
    // the chunks of a piece still coming: joined once it is all here, not on every chunk
    let parts: Buffer[] = []
    let got = 0
    const reader = p.stdout.getReader()
    for (;;) {
      const { done, value } = await reader.read()
      if (done) return
      parts.push(Buffer.from(value))
      got += value.length
      if (want && buf.length + got < want.len) continue
      buf = Buffer.concat([buf, ...parts])
      parts = []
      got = 0
      let wrote = false
      for (;;) {
        if (!want) {
          const nl = buf.indexOf(10)
          if (nl < 0) break
          const [file, offset, len] = buf.toString('utf8', 0, nl).split('\t')
          buf = buf.subarray(nl + 1)
          want = { file, offset: Number(offset), len: Number(len) }
        }
        if (buf.length < want.len) break
        this.put(want.file, want.offset, buf.subarray(0, want.len))
        buf = buf.subarray(want.len)
        want = null
        wrote = true
      }
      if (wrote) { this.recovery.success(); this.onData() }
    }
  }

  private put(file: string, offset: number, data: Buffer) {
    const to = this.copy(file)
    try {
      mkdirSync(dirname(to), { recursive: true })
      // a file that starts over is written anew
      const fd = openSync(to, offset === 0 ? 'w' : 'r+')
      try {
        writeSync(fd, data, 0, data.length, offset)
        this.have.set(file, offset + data.length)
      } finally {
        closeSync(fd)
      }
    } catch {
      /* the next piece of it, or the next start, brings it */
    }
  }
}

type Proc = { stdin: unknown; stdout: ReadableStream<Uint8Array>; exited: Promise<number>; kill(): void }

/** where the readers find another machine's CLI homes: in its mirror (a worker's environment) */
export function mirrorHomes(home: string, box: BoxInfo): Record<string, string> {
  const m = (p: string) => `${home}/mirror${box.home}/${p}`
  return { CLAUDE_CONFIG_DIR: m('.claude'), CODEX_HOME: m('.codex'), PI_CODING_AGENT_DIR: m('.pi/agent') }
}

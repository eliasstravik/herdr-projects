// The file calls that read a machine's own files (repos, worktrees, an agent's page, its inbox, its hooks), named as
// node:fs names them. On the main machine they are node:fs itself. In a worker for another machine (box.ts) a path
// under the worker's own store on the main machine (HOME: its records, its mirror of the logs) stays here, and every
// other path is the other machine's, read over SSH.
import * as fs from 'node:fs'
import { HOME } from '../store.ts'
import { BOX, helperDir, q, runSync } from './box.ts'

const local = (p: string) => !BOX || p === HOME || p.startsWith(`${HOME}/`)

const sh = (script: string, stdin?: string | Uint8Array) => runSync(['/bin/sh', '-c', script], { stdin })

function fail(p: string, err: string): never {
  const e = new Error(`${err.trim() || 'failed'}: ${p}`) as Error & { code: string }
  e.code = /no such file|not found|cannot stat|ENOENT/i.test(err) || !err.trim() ? 'ENOENT' : 'EIO'
  throw e
}

interface Stat {
  size: number
  mtimeMs: number
  ino: number
  dev: number
  isDirectory(): boolean
  isFile(): boolean
  isSymbolicLink(): boolean
}

// perl: on every Linux and macOS, with one way to stat a file on both
const STAT = `perl -e '@s = $ARGV[1] ? lstat($ARGV[0]) : stat($ARGV[0]); exit 1 unless @s; print join(" ", $s[7], $s[9], $s[1], $s[0], (-d _ ? 1 : 0), (-f _ ? 1 : 0), (-l $ARGV[0] ? 1 : 0))'`

function remoteStat(p: string, link: boolean): Stat {
  const r = sh(`${STAT} ${q(p)} ${link ? 1 : 0}`)
  if (r.code) fail(p, 'No such file or directory')
  const [size, mtime, ino, dev, d, f, l] = r.out.toString().trim().split(' ').map(Number)
  return { size, mtimeMs: mtime * 1000, ino, dev, isDirectory: () => d === 1, isFile: () => f === 1, isSymbolicLink: () => l === 1 }
}

export function existsSync(p: string) {
  return local(p) ? fs.existsSync(p) : sh(`test -e ${q(p)}`).code === 0
}

export function statSync(p: string): Stat {
  return local(p) ? fs.statSync(p) : remoteStat(p, false)
}

export function lstatSync(p: string): Stat {
  return local(p) ? fs.lstatSync(p) : remoteStat(p, true)
}

export function readFileSync(p: string): Buffer
export function readFileSync(p: string, enc: 'utf8'): string
export function readFileSync(p: string, enc?: 'utf8'): Buffer | string {
  if (local(p)) return enc ? fs.readFileSync(p, enc) : fs.readFileSync(p)
  const r = sh(`cat ${q(p)}`)
  if (r.code) fail(p, r.err)
  return enc ? r.out.toString('utf8') : r.out
}

/** the first n bytes of a file */
export function readHead(p: string, n: number): Uint8Array {
  if (local(p)) {
    const buf = new Uint8Array(n)
    const fd = fs.openSync(p, 'r')
    try {
      let at = 0
      while (at < n) {
        const got = fs.readSync(fd, buf, at, n - at, at)
        if (!got) break
        at += got
      }
      return buf.subarray(0, at)
    } finally {
      fs.closeSync(fd)
    }
  }
  const r = sh(`head -c ${Math.max(0, Math.floor(n))} ${q(p)}`)
  if (r.code) fail(p, r.err)
  return new Uint8Array(r.out)
}

export function writeFileSync(p: string, data: string | Uint8Array, o?: { mode?: number }) {
  if (local(p)) return fs.writeFileSync(p, data, o)
  const r = sh(`cat > ${q(p)}${o?.mode ? ` && chmod ${o.mode.toString(8)} ${q(p)}` : ''}`, data)
  if (r.code) fail(p, r.err)
}

export function mkdirSync(p: string, o?: { recursive?: boolean; mode?: number }) {
  if (local(p)) return void fs.mkdirSync(p, o)
  const r = sh(`mkdir ${o?.recursive ? '-p ' : ''}${q(p)}`)
  if (r.code && !o?.recursive) fail(p, r.err)
}

export function rmSync(p: string, o?: { recursive?: boolean; force?: boolean }) {
  if (local(p)) return fs.rmSync(p, o)
  const r = sh(`rm ${o?.recursive ? '-r ' : ''}${o?.force ? '-f ' : ''}${q(p)}`)
  if (r.code && !o?.force) fail(p, r.err)
}

export function renameSync(from: string, to: string) {
  if (local(from) && local(to)) return fs.renameSync(from, to)
  const r = sh(`mv -f ${q(from)} ${q(to)}`)
  if (r.code) fail(from, r.err)
}

export function readdirSync(p: string): string[] {
  if (local(p)) return fs.readdirSync(p)
  const r = sh(`cd ${q(p)} && ls -A1`)
  if (r.code) fail(p, r.err)
  return r.out.toString().split('\n').filter(Boolean)
}

export function realpathSync(p: string): string {
  if (local(p)) return fs.realpathSync(p)
  const r = sh(`perl -MCwd=abs_path -e 'my $r = abs_path($ARGV[0]); exit 1 unless defined $r && -e $r; print $r' ${q(p)}`)
  if (r.code) fail(p, 'No such file or directory')
  return r.out.toString()
}

/** cp -R, links kept as links */
export function cpSync(from: string, to: string) {
  if (local(from) && local(to)) return fs.cpSync(from, to, { recursive: true, verbatimSymlinks: true })
  const r = sh(`mkdir -p "$(dirname ${q(to)})" && cp -RP ${q(from)} ${q(to)}`)
  if (r.code) fail(from, r.err)
}

/** the folder of what an agent there reads or runs from Shepherdr (hooks, its instructions, files sent to it): the
 *  helper's folder on another machine, Shepherdr's own folder here */
export const boxDir = () => (BOX ? helperDir(BOX.home) : HOME)

/** where the main machine keeps its copy of another machine's file as it grows (mirror.ts); a file here is itself */
export const mirrored = (p: string) => (BOX && !local(p) ? `${HOME}/mirror${p}` : p)

/** the other machine's path of a mirrored file */
export const unmirrored = (p: string) => (BOX && p.startsWith(`${HOME}/mirror/`) ? p.slice(`${HOME}/mirror`.length) : p)

/** a file Shepherdr made on the main machine (a check's picture, an artifact's page), for an agent on the other machine
 *  to open: copied into a folder of the helper's there, where those over a day old go. Here it is itself */
export function forAgent(p: string, sub: string, name = p.split('/').pop()!): string {
  if (!BOX) return p
  const dir = `${boxDir()}/${sub}`
  const to = `${dir}/${name}`
  const r = sh(`mkdir -p ${q(dir)} && { find ${q(dir)} -type f -mtime +0 -exec rm -f {} + 2>/dev/null; cat > ${q(to)}; }`, fs.readFileSync(p))
  if (r.code) fail(to, r.err)
  return to
}

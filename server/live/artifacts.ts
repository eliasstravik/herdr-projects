// Artifacts: what an agent shows beyond words (a brief, a dashboard, a deck, a draft screen, a small app). One
// self-contained HTML file plus optional JSON data, with a short id, a title and the place it lives in; every update is
// a new version of the same one. Kept in ~/.shepherdr/artifacts until deleted, the last 20 versions each.
import { randomBytes } from 'node:crypto'
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import * as task from './mfs.ts'
import { isAbsolute, join, resolve, sep } from 'node:path'
import { boxHome } from './box.ts'
import { HOME, readJson, writeJson } from '../store.ts'

export const MAX_HTML = 2 * 1024 * 1024
export const MAX_DATA = 1024 * 1024
export const MAX_IMAGE = 1024 * 1024
export const KEEP_VERSIONS = 20

export interface VersionRec {
  n: number
  at: number
  /** who made it ("SHE-56 agent"), and the chat it came from (none from a terminal) */
  by: string
  chat?: string
  note?: string
  /** the version whose HTML and data files it shows: an update with only data keeps the HTML before it */
  html: number
  data?: number
}

export interface ArtifactRec {
  id: string
  title: string
  place: string
  at: number
  versions: VersionRec[]
  /** what its page last sent, for the HTML version it was sent from */
  sent?: { html: number; text: string }
  /** the check's picture of its latest version, light and dark */
  thumb?: boolean
}

export interface CheckResult {
  /** what broke: script errors, and anything it tried to load from the network */
  errors: string[]
  /** console.error lines: said, not refused */
  warnings: string[]
  /** pictures, videos and the like wider than the page or taller than the screen at phone width: said, not refused */
  media?: string[]
  /** pictures at phone width, light and dark; none when there is no browser here */
  shots?: { light: string; dark: string }
  /** why it could not be checked */
  skipped?: string
}

export type Checker = (html: string, data?: unknown) => Promise<CheckResult>

const ID_CHARS = 'abcdefghijkmnpqrstuvwxyz23456789'

function newId(taken: Record<string, unknown>) {
  for (;;) {
    const id = [...randomBytes(4)].map((b) => ID_CHARS[b % ID_CHARS.length]).join('')
    if (!taken[id]) return id
  }
}

const IMAGE_TYPES: Record<string, string> = { png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', gif: 'image/gif', webp: 'image/webp', svg: 'image/svg+xml', avif: 'image/avif' }

/** a file's bytes are a picture (not only its name), so a renamed file can't slip in */
function isImage(b: Buffer, ext: string) {
  const head = b.subarray(0, 12).toString('latin1')
  if (ext === 'svg') return /<svg[\s>]/i.test(b.subarray(0, 4096).toString('utf8'))
  return head.startsWith('\x89PNG') || head.startsWith('\xff\xd8\xff') || head.startsWith('GIF8') || (head.startsWith('RIFF') && head.slice(8, 12) === 'WEBP') || /^ftyp(avif|avis|mif1)$/.test(head.slice(4, 12))
}

/** Imports must never publish the vault or private process routing (including via symlinks). */
export function publicArtifactInput(path: string) {
  const real = task.realpathSync(path)
  const roots = [HOME, process.env.SHEPHERDR_MAIN_HOME, join(boxHome(), '.shepherdr')].filter((x): x is string => !!x)
  for (const root of roots) {
    let base = resolve(root)
    try { base = task.realpathSync(root) } catch { /* a missing remote/local root has no files */ }
    if (real === base || real.startsWith(base + sep)) {
      if (['inbox', 'artifacts'].some((dir) => real.startsWith(join(base, dir) + sep))) continue
      throw new Error('Private Shepherdr files cannot be imported into artifacts.')
    }
  }
  return real
}

/** pictures named by an absolute path on this machine (src="/home/…/chart.png", url(/…/bg.jpg)) go in as data: the
 *  frame can't load anything */
export function inlineImages(html: string): { html: string; missing: string[] } {
  const missing: string[] = []
  const ext = Object.keys(IMAGE_TYPES).join('|')
  const re = new RegExp(String.raw`(["'(])(/[^"'()\s<>]{1,1024}?\.(${ext}))(?=["')])`, 'gi')
  const out = html.replace(re, (all, open: string, path: string, x: string) => {
    const e = x.toLowerCase()
    try {
      if (!task.existsSync(path) || task.statSync(path).size > MAX_IMAGE) return (missing.push(path), all)
      const b = task.readFileSync(publicArtifactInput(path))
      if (!isImage(b, e)) return (missing.push(path), all)
      return `${open}data:${IMAGE_TYPES[e]};base64,${b.toString('base64')}`
    } catch {
      missing.push(path)
      return all
    }
  })
  return { html: out, missing }
}

/** a word that is lowercase on purpose stays so: code, commands, files, names like iOS or eBay */
const KEEP_CASE = /^[^\s]*([A-Z]|[./_@#`$<>=(){}\[\]~\\])/

/** a title starts with a capital, and so does what follows a task id ("SHE-90: proof" → "SHE-90: Proof") */
export function titleCase(title: string) {
  const up = (s: string) => (KEEP_CASE.test(s) ? s : s.replace(/^\p{Ll}/u, (c) => c.toUpperCase()))
  const t = title.trim()
  const m = /^((?:[\w-]+:)?[A-Za-z][\w-]*-\d+:\s*)(.*)$/s.exec(t)
  return m ? m[1] + up(m[2]) : up(t)
}

export class Artifacts {
  private dir = join(HOME, 'artifacts')
  list: Record<string, ArtifactRec> = readJson('artifacts.json', {})

  constructor() {
    mkdirSync(this.dir, { recursive: true })
    // titles made before they were capitalized for the agent
    let fixed = false
    for (const a of Object.values(this.list)) {
      const t = titleCase(a.title ?? '')
      if (t === a.title) continue
      a.title = t
      fixed = true
    }
    if (fixed) this.save()
  }

  save() {
    writeJson('artifacts.json', this.list)
  }

  get(id: string): ArtifactRec | undefined {
    return this.list[id.trim().toLowerCase()]
  }

  latest(a: ArtifactRec) {
    return a.versions[a.versions.length - 1]
  }

  version(a: ArtifactRec, n?: number) {
    return n ? a.versions.find((v) => v.n === n) : this.latest(a)
  }

  file(id: string, n: number, kind: 'html' | 'json') {
    return join(this.dir, id, `${n}.${kind}`)
  }

  thumbFile(id: string, dark = false) {
    return join(this.dir, id, dark ? 'thumb-dark.png' : 'thumb.png')
  }

  /** read and check what an agent hands over: the HTML (pictures inlined) and the data */
  static read(file: string | undefined, dataFile: string | undefined, cwd: string): { html?: string; data?: string; missing: string[] } {
    const at = (f: string) => (isAbsolute(f) ? f : resolve(cwd, f))
    let html: string | undefined
    let data: string | undefined
    let missing: string[] = []
    if (file) {
      const f = at(file)
      if (!task.existsSync(f)) throw new Error(`No file at ${f}`)
      const r = inlineImages(task.readFileSync(publicArtifactInput(f), 'utf8'))
      html = r.html
      missing = r.missing
      if (!html.trim()) throw new Error(`${f} is empty`)
      if (Buffer.byteLength(html) > MAX_HTML) throw new Error(`${f} is over 2 MB with its pictures inlined; make the pictures smaller`)
    }
    if (dataFile) {
      const f = at(dataFile)
      if (!task.existsSync(f)) throw new Error(`No file at ${f}`)
      data = task.readFileSync(publicArtifactInput(f), 'utf8')
      if (Buffer.byteLength(data) > MAX_DATA) throw new Error(`${f} is over 1 MB`)
      try {
        JSON.parse(data)
      } catch (e) {
        throw new Error(`${f} is not JSON: ${(e as Error).message}`)
      }
    }
    return { html, data, missing }
  }

  create(o: { title: string; place: string; by: string; chat?: string; html: string; data?: string; note?: string }): ArtifactRec {
    const id = newId(this.list)
    const a: ArtifactRec = { id, title: titleCase(o.title), place: o.place, at: Date.now(), versions: [] }
    this.list[id] = a
    this.write(a, { by: o.by, chat: o.chat, html: o.html, data: o.data, note: o.note })
    return a
  }

  /** a new version: new HTML, new data, or both; what it doesn't bring stays as it was */
  update(a: ArtifactRec, o: { by: string; chat?: string; html?: string; data?: string; note?: string; title?: string }) {
    if (!o.html && o.data === undefined && !o.title) throw new Error('Nothing to update: give an HTML file, --data or --title')
    if (o.title) a.title = titleCase(o.title)
    return this.write(a, o)
  }

  private write(a: ArtifactRec, o: { by: string; chat?: string; html?: string; data?: string; note?: string }) {
    const prev = a.versions.length ? this.latest(a) : undefined
    const n = (prev?.n ?? 0) + 1
    mkdirSync(join(this.dir, a.id), { recursive: true })
    if (o.html) writeFileSync(this.file(a.id, n, 'html'), o.html)
    if (o.data !== undefined) writeFileSync(this.file(a.id, n, 'json'), o.data)
    const v: VersionRec = { n, at: Date.now(), by: o.by, chat: o.chat, note: o.note, html: o.html ? n : prev!.html, data: o.data !== undefined ? n : prev?.data }
    a.versions.push(v)
    a.at = v.at
    if (o.html) delete a.sent
    this.prune(a)
    this.save()
    return v
  }

  /** the last 20 versions stay; files no kept version shows go */
  private prune(a: ArtifactRec) {
    if (a.versions.length <= KEEP_VERSIONS) return
    const gone = a.versions.splice(0, a.versions.length - KEEP_VERSIONS)
    const used = new Set(a.versions.flatMap((v) => [`${v.html}.html`, v.data ? `${v.data}.json` : '']))
    for (const v of gone)
      for (const f of [`${v.n}.html`, `${v.n}.json`]) if (!used.has(f)) rmSync(join(this.dir, a.id, f), { force: true })
  }

  html(a: ArtifactRec, n?: number) {
    const v = this.version(a, n)
    const f = v && this.file(a.id, v.html, 'html')
    return f && existsSync(f) ? readFileSync(f, 'utf8') : undefined
  }

  data(a: ArtifactRec, n?: number): unknown {
    const v = this.version(a, n)
    if (!v?.data) return undefined
    try {
      return JSON.parse(readFileSync(this.file(a.id, v.data, 'json'), 'utf8'))
    } catch {
      return undefined
    }
  }

  setThumbs(a: ArtifactRec, shots: { light: string; dark: string }) {
    try {
      writeFileSync(this.thumbFile(a.id), readFileSync(shots.light))
      writeFileSync(this.thumbFile(a.id, true), readFileSync(shots.dark))
      a.thumb = true
      this.save()
    } catch {
      /* the pictures went missing: the list shows no picture */
    }
  }

  sent(a: ArtifactRec, n: number | undefined, text: string) {
    const v = this.version(a, n)
    if (!v) return
    a.sent = { html: v.html, text }
    this.save()
  }

  /** what its page already sent, when the page shown is the one it was sent from */
  sentFor(a: ArtifactRec, n?: number) {
    const v = this.version(a, n)
    return v && a.sent?.html === v.html ? a.sent.text : undefined
  }

  remove(id: string) {
    const a = this.get(id)
    if (!a) return false
    rmSync(join(this.dir, a.id), { recursive: true, force: true })
    delete this.list[a.id]
    this.save()
    return true
  }

  /** the pages Shepherdr kept before artifacts: each becomes an artifact with the same id, so its chat card still opens */
  migratePages(placeOf: (chat: string) => string, repost: (chat: string, id: string) => void) {
    const old = readJson<Record<string, { id: string; title: string; by: string; chat: string; at: number; sent?: string }>>('pages.json', {})
    const pagesDir = join(HOME, 'pages')
    for (const p of Object.values(old)) {
      const f = join(pagesDir, `${p.id}.html`)
      if (!existsSync(f) || this.list[p.id]) continue
      mkdirSync(join(this.dir, p.id), { recursive: true })
      writeFileSync(this.file(p.id, 1, 'html'), readFileSync(f))
      this.list[p.id] = { id: p.id, title: p.title, place: placeOf(p.chat), at: p.at, versions: [{ n: 1, at: p.at, by: p.by, chat: p.chat, html: 1 }], sent: p.sent ? { html: 1, text: p.sent } : undefined }
      repost(p.chat, p.id)
    }
    if (Object.keys(old).length || existsSync(pagesDir)) {
      this.save()
      rmSync(join(HOME, 'pages.json'), { force: true })
      rmSync(pagesDir, { recursive: true, force: true })
    }
  }
}

/** opens the page the way the app frames it, at phone width, light then dark: script errors and anything it tried to
 *  load are what breaks it */
export function browserChecker(run: (args: string[]) => Promise<{ ok: boolean; out: string; err: string }>, wrap: (html: string, data?: unknown) => string): Checker {
  // one browser, one check at a time: two at once would mix up their pages, pictures and errors
  let queue: Promise<unknown> = Promise.resolve()
  return (html, data) => {
    const turn = queue.then(() => checkOne(run, wrap, html, data))
    queue = turn.catch(() => undefined)
    return turn
  }
}

async function checkOne(run: (args: string[]) => Promise<{ ok: boolean; out: string; err: string }>, wrap: (html: string, data?: unknown) => string, html: string, data: unknown): Promise<CheckResult> {
  const dir = join(HOME, 'checks')
  mkdirSync(dir, { recursive: true })
  // the pictures are for the agent to look at now: a day later they go
  for (const f of readdirSync(dir))
    try {
      if (statSync(join(dir, f)).mtimeMs < Date.now() - 86_400_000) rmSync(join(dir, f), { force: true })
    } catch {
      /* gone already */
    }
  const stem = join(dir, `${Date.now().toString(36)}-${randomBytes(3).toString('hex')}`)
  const page = `${stem}.html`
  writeFileSync(page, wrap(html, data))
  const light = `${stem}.png`
  const dark = `${stem}-dark.png`
  const r = await run([
    '--session',
    'shep-artifacts',
    'batch',
    'set viewport 390 844',
    'set media light',
    `open file://${page}`,
    'wait 400',
    `screenshot ${light}`,
    'set media dark',
    'wait 150',
    `screenshot ${dark}`,
    'eval JSON.stringify((window.__shepherdrMedia&&__shepherdrMedia(),window.__shepherdrCheck||[]))',
  ])
  rmSync(page, { force: true })
  if (!r.ok && !existsSync(light)) return { errors: [], warnings: [], skipped: `could not open a browser here (${(r.err || r.out).trim().split('\n')[0] || 'agent-browser failed'})` }
  // the last line is the list the page kept, printed as a JSON string
  let said: string[] = []
  for (const l of r.out.trim().split('\n').reverse()) {
    try {
      const v = JSON.parse(JSON.parse(l.trim()))
      if (Array.isArray(v)) {
        said = v.map(String)
        break
      }
    } catch {
      /* not that line */
    }
  }
  const of = (k: string) => said.filter((s) => s.startsWith(k)).map((s) => s.slice(k.length))
  const errors = said.filter((s) => !s.startsWith('console: ') && !s.startsWith('media: ')).slice(0, 12)
  const warnings = of('console: ').slice(0, 8)
  const media = of('media: ').slice(0, 8)
  return { errors, warnings, media, shots: existsSync(light) && existsSync(dark) ? { light, dark } : undefined }
}

/** for the check: what the page itself reports, written into it before anything else runs */
export const CHECK_PROBE = `<script>(function(){var k=window.__shepherdrCheck=[];addEventListener('error',function(e){if(e.message)k.push('script error: '+e.message+(e.lineno?' (line '+e.lineno+')':''))});addEventListener('unhandledrejection',function(e){k.push('script error: '+(e.reason&&e.reason.message||e.reason))});addEventListener('securitypolicyviolation',function(e){k.push('blocked: '+(e.blockedURI||e.violatedDirective)+' (artifacts load nothing: put it inline)')});window.__shepherdrMedia=function(){var de=document.documentElement,W=de.clientWidth,H=innerHeight,n={};[].forEach.call(document.querySelectorAll('img,video,canvas,svg,iframe,embed,object'),function(el){var t=el.tagName.toLowerCase();n[t]=(n[t]||0)+1;if(el.ownerSVGElement)return;var r=el.getBoundingClientRect(),why=[];if(!r.width||!r.height)return;if(r.right>W+1||r.left<-1)why.push('wider than the page ('+W+' px)');if(r.height>H+1)why.push('taller than the screen ('+H+' px)');if(why.length)k.push('media: '+t+' '+n[t]+' ('+Math.round(r.width)+'×'+Math.round(r.height)+' px) is '+why.join(' and '))})};var ce=console.error;console.error=function(){k.push('console: '+[].map.call(arguments,String).join(' '));return ce.apply(console,arguments)}})()</script>`


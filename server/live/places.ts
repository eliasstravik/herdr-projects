// Spaces, as herdr's: a folder on a machine with a name and its own coordinator. Every machine has its home space at
// its home folder (~), first under it until dragged and never removed; its coordinator also keeps an eye on the machine (and the main
// machine's on every machine). Any other space is a git repo with its own herdr space, added from the sidebar or by
// asking a coordinator; its tasks are made in its repo. Each space has its own mode.
import { execFileSync } from 'node:child_process'
import { hostname } from 'node:os'
import { readFileSync } from 'node:fs'
import { basename, join, resolve } from 'node:path'
import { setOrder, type PlaceOrder, type SectionDefs } from '../../shared/order.ts'
import { nameProblem, slugify, type Mode } from '../../shared/types.ts'
import { readJson, writeJson } from '../store.ts'
import { Agents } from './agents.ts'
import { boxHome } from './box.ts'
import { validateBase, defaultBranch, github, githubRemote, isRepo, topLevel, type GitHub } from './engine/git.ts'
import { migrateToSpaces } from './migrate.ts'

export interface Space {
  /** the name it was added under: its id and herdr space keep it; the name that shows may change (names.json) */
  name: string
  path: string
  /** short task key: GTM, HP */
  key: string
  github: string | null
  base: string
  added: number
}

/** a mode read from disk, the CLI or another machine, any case */
export const asMode = (m: unknown): Mode | undefined => {
  const w = typeof m === 'string' ? m.trim().toLowerCase() : ''
  return w === 'regular' || w === 'autopilot' ? w : undefined
}

export const HOST = (process.env.SHEPHERDR_MACHINE ?? hostname()).split('.')[0]

/** the name the human reads for this machine, while HOST stays its key (its place, chat, mode, settings,
 *  coordinator): a Mac's model, "Mac mini", as herdr's sidebar shows it (macOS gives a bare "Mac" for its hostname).
 *  A machine run over SSH keeps its herdr label; any other, or a Mac that doesn't say, its HOST. */
export function machineName(env: Record<string, string | undefined> = process.env, os: string = process.platform, model = macModel) {
  if (env.SHEPHERDR_MACHINE || os !== 'darwin') return HOST
  return model() || HOST
}

/** the Mac's model name ("Mac mini", "MacBook Pro"), as About This Mac says it; '' where it can't be read */
function macModel() {
  try {
    const out = execFileSync('/usr/sbin/system_profiler', ['SPHardwareDataType', '-json'], { encoding: 'utf8', timeout: 10_000, stdio: ['ignore', 'pipe', 'ignore'] })
    const name = JSON.parse(out)?.SPHardwareDataType?.[0]?.machine_name
    return typeof name === 'string' ? name.trim() : ''
  } catch {
    return ''
  }
}

export const NAME = machineName()

/** a home space's folder key, in its id: p:<machine>:~ */
export const HOME_KEY = '~'

/** the herdr agent of this machine's home space's coordinator (no space's is called so: theirs start shep-) */
export const HOME_AGENT = 'shepherdr-home'

/** the herdr space it runs in, and the one the machine's and All machines' coordinators shared before */
export const HOME_SPACE = { label: 'home', was: 'Coordinators' }

export const placeId = {
  /** a machine's heading, what its spaces' order and its fold are kept under */
  machine: (m = HOST) => `m:${m}`,
  space: (name: string, m = HOST) => `p:${m}:${name}`,
  home: (m = HOST) => `p:${m}:${HOME_KEY}`,
}

export function keyFor(name: string, taken: string[]) {
  const words = name.split(/[^a-zA-Z0-9]+/).filter(Boolean)
  const cands = [
    words.length > 1 ? words.map((w) => w[0]).join('') : name.replace(/[^a-zA-Z]/g, '').slice(0, 3),
    name.replace(/[^a-zA-Z]/g, '').slice(0, 4),
    name.replace(/[^a-zA-Z]/g, '').slice(0, 5),
  ].map((k) => k.toUpperCase() || 'P')
  for (const k of cands) if (!taken.includes(k)) return k
  let i = 2
  while (taken.includes(`${cands[0]}${i}`)) i++
  return `${cands[0]}${i}`
}

export class Places {
  /** Historical task prefixes, including deleted tasks; supplied by the owning engine. */
  takenTaskKeys?: () => string[]
  /** what was kept under the coordinators that are gone moves to the home spaces first, once */
  retired = migrateToSpaces((m) => placeId.home(m), HOME_AGENT).retire
  spaces: Space[] = readJson('spaces.json', [])
  // kept under their old names too (manual and ping, suggest, auto): read as today's, saved so on the next save
  modes: Record<string, Mode> = Object.fromEntries(Object.entries(readJson<Record<string, unknown>>('modes.json', {})).flatMap(([id, m]) => (asMode(m) ? [[id, asMode(m)!]] : [])))
  /** the sidebar's order the human dragged, of every machine's places (see shared/order.ts) */
  order: PlaceOrder = readJson('order.json', {})
  /** the names the human gave machines and spaces in the sidebar, by id, every machine's: only what shows changes,
   *  never a folder, repo, herdr space or task id */
  names: Record<string, string> = readJson('names.json', {})

  /** the names given: on another machine's worker, the main machine's (it keeps them), read as they are now, so its
   *  commands take the names the sidebar shows */
  given(): Record<string, string> {
    const main = process.env.SHEPHERDR_SHARED_HOME
    if (!main) return this.names
    try {
      return JSON.parse(readFileSync(join(main, 'names.json'), 'utf8')) as Record<string, string>
    } catch {
      return this.names
    }
  }

  /** the emoji the human gave machines, by id, kept by the main machine */
  emojis: Record<string, string> = readJson('emojis.json', {})

  /** a place's new name; '' gives it back its own */
  rename(id: string, name: string) {
    if (name) this.names[id] = name
    else delete this.names[id]
    writeJson('names.json', this.names)
  }

  /** a machine's emoji; '' takes it away */
  setEmoji(id: string, emoji: string) {
    if (emoji) this.emojis[id] = emoji
    else delete this.emojis[id]
    writeJson('emojis.json', this.emojis)
  }

  setOrder(under: string, ids: string[]) {
    setOrder(this.order, under, ids)
    writeJson('order.json', this.order)
  }

  /** the sections the human made in the sidebar, by id (shared/order.ts): what is in them is kept with the order */
  sections: SectionDefs = readJson('sections.json', {})

  /** the sections and the order, after a section changed or a space moved */
  saveSections() {
    writeJson('sections.json', this.sections)
    writeJson('order.json', this.order)
  }

  save() {
    writeJson('spaces.json', this.spaces)
    writeJson('modes.json', this.modes)
  }

  /** the name each of this machine's spaces shows, the home space first: as given, by the space rules, each its own */
  shown(names: Record<string, string> = this.given()): { id: string; name: string; space?: Space }[] {
    const taken: string[] = []
    const own = (want: string) => {
      let n = slugify(want) || 'space'
      for (let i = 2; taken.includes(n); i++) n = `${slugify(want) || 'space'}-${i}`
      taken.push(n)
      return n
    }
    const home = placeId.home()
    return [{ id: home, name: own(names[home] ?? 'home') }, ...this.spaces.map((p) => ({ id: placeId.space(p.name), name: own(names[placeId.space(p.name)] ?? p.name), space: p }))]
  }

  /** the name a space shows */
  nameOf(id: string) {
    return this.shown().find((x) => x.id === id)?.name
  }

  /** a space of this machine by the name it shows, or its id */
  find(name: string) {
    return this.shown().find((x) => x.name === name || x.id === name)
  }

  /** why a space of this machine can't be called so (other than the one renamed), or undefined */
  nameProblem(name: string, except?: string) {
    return nameProblem(name, this.shown().filter((x) => x.id !== except).map((x) => x.name))
  }

  /** a space's GitHub repo, read again (it may have gained or lost its remote since it was added) */
  async recheckGithub(p: Space): Promise<GitHub | null> {
    const g = await github(p.path)
    if ((g?.repo ?? null) !== p.github) {
      p.github = g?.repo ?? null
      this.save()
    }
    return g
  }

  mode(id: string): Mode {
    // every space starts regular: it wakes on news and does what the human asked
    return asMode(this.modes[id]) ?? 'regular'
  }

  setMode(id: string, m: Mode) {
    this.modes[id] = m
    this.save()
  }

  /** a repo space by its id (the home space is none) */
  space(id: string) {
    return this.spaces.find((p) => placeId.space(p.name) === id)
  }

  byKey(key: string) {
    return this.spaces.find((p) => p.key === key.toUpperCase())
  }

  /** a repo space by the name it shows, or the one it was added under */
  byName(name: string) {
    return this.find(name)?.space ?? this.spaces.find((p) => p.name === name)
  }

  /** the repo space a path belongs to */
  forPath(path: string) {
    const abs = resolve(path)
    return this.spaces.find((p) => abs === p.path || abs.startsWith(`${p.path}/`))
  }

  async add(path: string, name?: string): Promise<Space> {
    const abs = resolve(path.replace(/^~(?=$|\/)/, boxHome()))
    if (abs === resolve(boxHome())) throw new Error('Your home folder is the home space already: pick a folder in it.')
    if (!(await isRepo(abs))) throw new Error(`${abs} is not a git repository`)
    const top = (await topLevel(abs)) ?? abs
    if (top === resolve(boxHome())) throw new Error('Your home folder is the home space already: pick a folder in it.')
    const existing = this.spaces.find((p) => p.path === top)
    if (existing) return existing
    const nm = name ?? (slugify(basename(top)) || 'space')
    const bad = this.nameProblem(nm)
    if (bad) throw new Error(name === undefined ? `${bad} Give it another with --name.` : bad)
    // one removed before under the same name comes back with its tasks and chat; the name one renamed away from stays
    // that one's key, so the new one gets a key of its own and shows the name asked for
    let key = nm
    for (let i = 2; this.spaces.some((x) => x.name === key); i++) key = `${nm}-${i}`
    const p: Space = { name: key, path: top, key: keyFor(nm, [...this.spaces.map((x) => x.key), ...(this.takenTaskKeys?.() ?? [])]), github: await githubRemote(top), base: await defaultBranch(top), added: Date.now() }
    this.spaces.push(p)
    if (key !== nm) this.rename(placeId.space(key), nm)
    this.save()
    return p
  }

  async setBase(name: string, base: string) {
    const p = this.byName(name) ?? this.space(name)
    if (!p) throw new Error('Choose a repository space first.')
    await validateBase(p.path, base)
    p.base = base
    this.save()
    return p
  }

  remove(name: string) {
    this.spaces = this.spaces.filter((p) => p.name !== name)
    this.save()
  }

  /** the herdr agent each space's coordinator is */
  coordinator(id: string) {
    if (id === placeId.home()) return { key: id, name: HOME_AGENT, cwd: boxHome(), workspace: HOME_SPACE.label, was: HOME_SPACE.was, tab: 'coordinator', home: true, label: this.nameOf(id) ?? 'home', path: boxHome() }
    const p = this.space(id)
    if (!p) return null
    return { key: id, name: Agents.nameFor(`shep-${p.name}`), cwd: p.path, workspace: p.name, tab: 'coordinator', home: false, label: this.nameOf(id) ?? p.name, path: p.path }
  }
}

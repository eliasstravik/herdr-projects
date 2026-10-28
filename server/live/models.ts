// What each CLI's models and effort levels really are, so the + menu, Settings and task add offer only what it takes.
// Each CLI is asked itself where it can tell: Claude Code's own model catalog in its install (which models it knows
// and which take xhigh and max) and its --help, `codex debug models`, `pi --list-models` and pi's --help. What they
// said is kept in cli-models.json and asked again by version (claude) or every few hours; until then, or where a CLI
// can't tell, the table below holds. A level a model lacks becomes the nearest it has (nearest), never an error at start.
import { realpathSync, statSync } from 'node:fs'
import { readJson, writeJson } from '../store.ts'

export interface Model {
  /** what the CLI takes: claude-opus-5-5, gpt-6-sol, exe-dev-openai/gpt-4.1@llm */
  id: string
  /** what the screen says: Opus 5.5 */
  label: string
  /** the effort levels it takes, lowest first; none: it has no effort to set */
  efforts: string[]
  /** its own default level */
  effort?: string
}

export interface CliModels {
  /** what the menus offer, in the CLI's own order */
  models: Model[]
  /** older ones it still takes by id (--model claude-opus-4-8), offered only once one is in use */
  others?: Model[]
  /** every level the CLI takes, for its own default model */
  efforts: string[]
  /** the model it starts on by itself, by id: what the + menu shows until one is picked */
  default?: string
  /** what it was read from (a version, a binary), and when */
  from?: string
  at?: number
}

/** every level any CLI takes, lowest first; off and none are the same, lowest is the old name for the lowest level */
const RANK: Record<string, number> = { off: 0, none: 0, lowest: 1, minimal: 1, low: 2, medium: 3, high: 4, xhigh: 5, max: 6, ultra: 7 }
export const LEVELS = Object.keys(RANK)

const CLAUDE_LEVELS = ['low', 'medium', 'high', 'xhigh', 'max']
const PI_LEVELS = ['off', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max']
const claude = (id: string, label: string, effort: string): Model => ({ id, label, efforts: CLAUDE_LEVELS, effort })
const codex = (id: string, label: string, effort: string, ultra = true): Model => ({ id, label, efforts: [...CLAUDE_LEVELS, ...(ultra ? ['ultra'] : [])], effort })

/** what each CLI takes when it can't be asked (kept up to date by hand; the CLI's own answer wins) */
export const TABLE: Record<string, CliModels> = {
  claude: {
    models: [claude('claude-fable-5-1', 'Fable 5.1', 'high'), claude('claude-opus-5-5', 'Opus 5.5', 'medium'), claude('claude-sonnet-5-5', 'Sonnet 5.5', 'medium'), claude('claude-haiku-5-5', 'Haiku 5.5', 'medium')],
    efforts: CLAUDE_LEVELS,
    default: 'claude-opus-5-5',
  },
  codex: {
    models: [codex('gpt-6.1-sol', 'GPT-6.1-Sol', 'low'), codex('gpt-6-astra', 'GPT-6-Astra', 'medium'), codex('gpt-6-sol', 'GPT-6-Sol', 'medium'), codex('gpt-6-luna', 'GPT-6-Luna', 'medium', false), codex('gpt-5.6-sol', 'GPT-5.6-Sol', 'low'), codex('gpt-5.6-terra', 'GPT-5.6-Terra', 'medium'), codex('gpt-5.6-luna', 'GPT-5.6-Luna', 'medium', false)],
    efforts: [...CLAUDE_LEVELS, 'ultra'],
  },
  // pi reaches many providers' models: only pi itself can list them
  pi: { models: [], efforts: PI_LEVELS },
}

/** Claude Code's families as its /model menu offers them, newest of each; its aliases (opus, haiku) name these too */
const FAMILIES = ['fable', 'opus', 'sonnet', 'haiku']
const STALE = 6 * 3_600_000
const TEST = process.env.NODE_ENV === 'test'

let cache: Record<string, CliModels> = TEST ? {} : readJson('cli-models.json', {})
const asking = new Set<string>()
const looked = new Map<string, number>()

/** what this CLI takes; nothing for a CLI Shepherdr passes no model or effort to */
export function modelsOf(kind: string): CliModels | undefined {
  if (!TABLE[kind]) return undefined
  if (!TEST) ask(kind)
  const c = cache[kind]
  return c?.models.length || (c && !TABLE[kind].models.length) ? c : TABLE[kind]
}

/** a model by its id, its name on screen or an alias (opus, sonnet:high's sonnet), any case; nothing when this CLI
 *  doesn't have it, or can't tell (pi before it was asked) */
export function findModel(kind: string, value?: string): Model | undefined {
  const v = value?.trim().toLowerCase()
  const c = modelsOf(kind)
  if (!v || v === 'default' || !c) return undefined
  const all = [...c.models, ...(c.others ?? [])]
  return (
    all.find((m) => m.id.toLowerCase() === v || m.label.toLowerCase() === v) ??
    (kind === 'claude' ? c.models.find((m) => m.id.startsWith(`claude-${v}-`) || m.id === `claude-${v}`) : undefined) ??
    // pi: a model without its provider
    (kind === 'pi' ? all.find((m) => m.id.toLowerCase().endsWith(`/${v}`)) : undefined)
  )
}

/** whether this CLI can say which models it has: else any model id goes */
export const knowsModels = (kind: string) => !!modelsOf(kind)?.models.length

/** the model as the CLI takes it: its id, or what was written when the CLI can't tell */
export function modelArg(kind: string, value?: string): string | undefined {
  if (!value || value === 'default') return undefined
  return findModel(kind, value)?.id ?? value
}

/** the levels this model takes (the CLI's own, for its default model or one it can't tell about) */
export function effortsOf(kind: string, model?: string): string[] {
  const c = modelsOf(kind)
  if (!c) return []
  return findModel(kind, model ?? c.default)?.efforts ?? c.efforts
}

/** the level nearest to this one among these, a tie going to the lower; nothing for the CLI's own or an unknown word */
export function nearest(levels: string[], effort?: string): string | undefined {
  const e = effort?.trim().toLowerCase()
  if (!e || e === 'default' || !levels.length) return undefined
  if (levels.includes(e)) return e
  const r = RANK[e]
  if (r === undefined) return undefined
  return levels.reduce((best, l) => (Math.abs(RANK[l] - r) < Math.abs(RANK[best] - r) ? l : best))
}

/** an effort as this CLI and model take it: the nearest level it has, or nothing for its own default */
export const effortFor = (kind: string, effort?: string, model?: string) => nearest(effortsOf(kind, model), effort)

/* ---------- asking the CLIs ---------- */

function ask(kind: string) {
  // looked at most once a minute: the + menu asks on every chat it draws
  if (Date.now() - (looked.get(kind) ?? 0) < 60_000) return
  looked.set(kind, Date.now())
  const c = cache[kind]
  const from = kind === 'claude' ? claudeBinary() : undefined
  if (c && (kind === 'claude' ? c.from === from : Date.now() - (c.at ?? 0) < STALE)) return
  if (asking.has(kind)) return
  asking.add(kind)
  read(kind, from)
    .then((got) => {
      if (!got?.models.length) return
      cache = { ...cache, [kind]: { ...got, from, at: Date.now() } }
      writeJson('cli-models.json', cache)
    })
    .catch(() => {})
    .finally(() => asking.delete(kind))
}

/** the Claude Code install that runs, by its path and when it changed: a new version is read again */
function claudeBinary() {
  try {
    const p = realpathSync(Bun.which('claude') ?? '')
    return `${p}@${statSync(p).mtimeMs}`
  } catch {
    return undefined
  }
}

async function run(cmd: string[]) {
  const p = Bun.spawn(cmd, { stdout: 'pipe', stderr: 'pipe', stdin: 'ignore' })
  const t = setTimeout(() => p.kill(), 20_000)
  const [out, err] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text()])
  clearTimeout(t)
  return `${out}${err}`
}

async function read(kind: string, from?: string): Promise<CliModels | undefined> {
  if (kind === 'claude' && from) {
    const bin = from.slice(0, from.lastIndexOf('@'))
    const [catalog, help] = await Promise.all([run(['grep', '-aoE', '\\{id:"claude-[a-z0-9-]+",family:"[a-z]+",display_name:"[^"]+"|capabilities:\\[[^]]*\\]|default_effort:"[a-z]+"', bin]), run([bin, '--help'])])
    return parseClaude(catalog, help)
  }
  if (kind === 'codex') return parseCodex(await run(['codex', 'debug', 'models']))
  if (kind === 'pi') {
    const [list, help] = await Promise.all([run(['pi', '--list-models']), run(['pi', '--help'])])
    return parsePi(list, help)
  }
  return undefined
}

/** the levels a --help lists for this flag: "--effort <level> … (low, medium, high, xhigh, max)" */
export function helpLevels(help: string, flag: string): string[] {
  const m = help.replace(/\s+/g, ' ').match(new RegExp(`${flag} <\\w+>[^-]*?[(:] ?((?:[a-z]+, )+[a-z]+)`))
  return m ? m[1].split(', ').filter((l) => l in RANK) : []
}

/** Claude Code's model catalog as grep finds it in its install: each model's id, family and name, then its
 *  capabilities (effort, xhigh_effort, max_effort) and default level */
export function parseClaude(catalog: string, help: string): CliModels | undefined {
  const levels = helpLevels(help, '--effort')
  const all: (Model & { family: string })[] = []
  for (const line of catalog.split('\n')) {
    const id = line.match(/^\{id:"([^"]+)",family:"([^"]+)",display_name:"([^"]+)"/)
    const last = all.at(-1) as (Model & { family: string; caps?: boolean; dflt?: boolean }) | undefined
    if (id) all.push({ id: id[1], family: id[2], label: id[3], efforts: [] })
    else if (last && !last.caps && line.startsWith('capabilities:')) {
      last.caps = true
      const caps = line.match(/"([a-z_]+)"/g)?.map((x) => x.slice(1, -1)) ?? []
      const has = caps.includes('effort') ? ['low', 'medium', 'high', ...(caps.includes('xhigh_effort') ? ['xhigh'] : []), ...(caps.includes('max_effort') ? ['max'] : [])] : []
      last.efforts = levels.length ? has.filter((l) => levels.includes(l)) : has
    } else if (last && !last.dflt && last.caps && line.startsWith('default_effort:')) {
      last.dflt = true
      last.effort = line.match(/"([a-z]+)"/)?.[1]
    }
  }
  const tidy = ({ id, label, efforts, effort }: Model): Model => ({ id, label, efforts, ...(effort && efforts.includes(effort) ? { effort } : {}) })
  const models = FAMILIES.map((f) => all.findLast((m) => m.family === f)).filter((m): m is Model & { family: string } => !!m)
  if (!models.length) return undefined
  return {
    models: models.map(tidy),
    others: all.filter((m) => !models.includes(m)).map(tidy),
    efforts: levels.length ? levels : CLAUDE_LEVELS,
    default: models.find((m) => m.family === 'opus')?.id ?? models[0].id,
  }
}

/** `codex debug models`: the ones it lists in its own picker, and the hidden ones it still takes */
export function parseCodex(json: string): CliModels | undefined {
  type Raw = { slug?: string; display_name?: string; visibility?: string; default_reasoning_level?: string; supported_reasoning_levels?: { effort?: string }[] }
  let raw: Raw[]
  try {
    const d = JSON.parse(json.slice(json.indexOf('{')))
    raw = Array.isArray(d) ? d : d.models
  } catch {
    return undefined
  }
  if (!Array.isArray(raw)) return undefined
  const model = (m: Raw): Model => {
    const efforts = (m.supported_reasoning_levels ?? []).map((l) => l.effort ?? '').filter((l) => l in RANK).sort((a, b) => RANK[a] - RANK[b])
    return { id: m.slug!, label: m.display_name || m.slug!, efforts, ...(m.default_reasoning_level && efforts.includes(m.default_reasoning_level) ? { effort: m.default_reasoning_level } : {}) }
  }
  const ok = raw.filter((m) => m.slug)
  const models = ok.filter((m) => m.visibility !== 'hide').map(model)
  if (!models.length) return undefined
  const efforts = [...new Set(models.flatMap((m) => m.efforts))].sort((a, b) => RANK[a] - RANK[b])
  return { models, others: ok.filter((m) => m.visibility === 'hide').map(model), efforts }
}

/** `pi --list-models`: a table of provider, model, …, thinking (yes or no); the levels from its --help */
export function parsePi(list: string, help: string): CliModels | undefined {
  const levels = helpLevels(help, '--thinking')
  const efforts = levels.length ? levels : PI_LEVELS
  const rows = list.split('\n').map((l) => l.trim().split(/\s+/))
  const head = rows.findIndex((r) => r[0] === 'provider' && r[1] === 'model')
  if (head < 0) return undefined
  const think = rows[head].indexOf('thinking')
  const found = rows.slice(head + 1).filter((r) => r.length >= 2 && r[0])
  const names = found.map((r) => r[1])
  const models = found.map((r) => ({
    id: `${r[0]}/${r[1]}`,
    // a model's own name, unless another provider has one so named too
    label: names.indexOf(r[1]) === names.lastIndexOf(r[1]) ? r[1] : `${r[0]}/${r[1]}`,
    efforts: think < 0 || r[think] === 'yes' ? efforts : [],
  }))
  return models.length ? { models, efforts } : undefined
}

/** for tests: what the CLIs said */
export function setModels(kind: string, c: CliModels | undefined) {
  cache = { ...cache }
  if (c) cache[kind] = c
  else delete cache[kind]
}

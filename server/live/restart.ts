// Restarting an agent on a CLI, model and effort: a coordinator, or a task's agent. The same CLI resumes its session;
// another one takes over from it with the role's prompt and what was said and done so far (prompts.ts restartLine).
// What the restart sheet offers (restartInfo) and the calm hint a chat shows once Settings say something else than
// what its agent runs on (restartHint) come from here.
import type { Message, RestartCli, RestartHint, RestartInfo } from '../../shared/types.ts'
import { effortFor, effortsOf, findModel, modelsOf } from './models.ts'
import { CLI_NAME } from './settings.ts'
import type { Harness } from './engine/engine.ts'

export type { Harness }

const name = (kind: string) => CLI_NAME[kind] ?? kind

/** a model as compared: its id, '' for the CLI's own */
const modelOf = (h: Harness) => (!h.model || h.model === 'default' ? '' : (findModel(h.kind, h.model)?.id ?? h.model))
/** an effort as compared: the level its model takes, '' for the CLI's own */
const effortOf = (h: Harness) => (!h.effort || h.effort === 'default' ? '' : (effortFor(h.kind, h.effort, modelOf(h) || undefined) ?? h.effort.toLowerCase()))

export const sameHarness = (a: Harness, b: Harness) => a.kind === b.kind && modelOf(a) === modelOf(b) && effortOf(a) === effortOf(b)

/** a harness as Shepherdr passes it: a model by its id, a level its model takes; nothing for the CLI's own */
export function fitHarness(h: Harness): Harness {
  const model = modelOf(h) || undefined
  const effort = effortOf(h) || undefined
  return { kind: h.kind, ...(model ? { model } : {}), ...(effort ? { effort } : {}) }
}

/** "Codex (GPT-6-Sol, high)", "Claude Code" */
export function harnessWords(h: Harness) {
  const m = modelOf(h)
  const bits = [m ? (findModel(h.kind, m)?.label ?? m) : '', effortOf(h)].filter(Boolean)
  return bits.length ? `${name(h.kind)} (${bits.join(', ')})` : name(h.kind)
}

/**
 * What a chat's hint says, by the first thing that differs: its agent runs on `on`, Settings now say `want`. Nothing
 * while they agree, or while Settings say what they said when it last started on them (`set`): a pick of its own (task
 * add --agent, the + menu's model) is no change of Settings.
 */
export function restartHint(on: Harness, set: Harness | undefined, want: Harness): RestartHint | undefined {
  if (sameHarness(on, want) || (set && sameHarness(set, want))) return undefined
  if (on.kind !== want.kind) return { on: name(on.kind), want: name(want.kind) }
  const label = (h: Harness) => (modelOf(h) ? (findModel(h.kind, modelOf(h))?.label ?? modelOf(h)) : `${name(h.kind)}'s default model`)
  if (modelOf(on) !== modelOf(want)) return { on: label(on), want: label(want) }
  return { on: `${effortOf(on) || 'default'} effort`, want: `${effortOf(want) || 'default'} effort` }
}

/** each CLI the sheet offers: its models (with the levels each takes) where Shepherdr passes them */
export function restartClis(kinds: string[]): RestartCli[] {
  return kinds.map((kind) => {
    const c = modelsOf(kind)
    return {
      kind,
      name: name(kind),
      ...(c ? { models: c.models.map((m) => ({ id: m.id, label: m.label, efforts: m.efforts })), efforts: effortsOf(kind) } : {}),
    }
  })
}

/** what the restart sheet shows: what it runs on now, what Settings say (what it restarts with unless changed), the
 *  CLIs it may restart on */
export function restartInfo(o: { role: 'task' | 'coordinator'; space?: string; on: Harness; want: Harness; kinds: string[]; working: boolean }): RestartInfo {
  const kinds = o.kinds.includes(o.on.kind) ? o.kinds : [o.on.kind, ...o.kinds]
  return { role: o.role, space: o.space, on: fitHarness(o.on), onWords: harnessWords(o.on), want: fitHarness(o.want), clis: restartClis(kinds), working: o.working }
}

/** a chat's last words, latest last, for an agent taking over: yours and its agent's (as `them`), each cut short;
 *  cards and the brief (`skip`) left out */
export function recentChat(messages: Message[], them: string, skip?: string) {
  return messages
    .filter((m) => m.text && !m.card && m.text.trim() !== skip)
    .slice(-12)
    .map((m) => `${m.who === 'you' ? 'Human' : them}: ${m.text!.length > 400 ? `${m.text!.slice(0, 400)}…` : m.text}`)
    .join('\n\n')
}

import { Refusal } from './client.ts'
import type { Action, Question } from '../../shared/types.ts'

/** What a PermissionRequest hook hands back to its CLI. Extra fields go into the hook's decision as they are. */
export interface PermissionAnswer { allow: boolean; message?: string; updatedInput?: Record<string, unknown>; updatedPermissions?: unknown[] }
/** Claude's own question and plan tools arrive as a PermissionRequest but are the human's to answer, not to allow.
 *  Each question of a tool call is its own card; head is the first card, which holds the call. */
export interface NativeAsk { head: string; tool: string; input: Record<string, unknown>; stop?: string; answer?: string; reply?: PermissionAnswer }

const text = (value: unknown) => typeof value === 'string' ? value : ''
const record = (value: unknown) => value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : undefined

/** The mode "stop asking" returns to: what the agent ran on before planning, or edits without asking. */
const stopMode = (mode?: string) => mode === 'auto' || mode === 'bypassPermissions' ? mode : 'acceptEdits'
const STOP: Record<string, { label: string; description: string }> = {
  auto: { label: 'Approve, stop asking', description: 'Claude goes on in auto mode' },
  bypassPermissions: { label: 'Approve, stop asking', description: 'Claude goes on without any prompts' },
  acceptEdits: { label: 'Approve, stop asking about edits', description: 'Claude edits files without asking' },
}

/** The cards of a tool call, or undefined for any other tool: an ordinary permission. An unreadable question gives none. */
export function nativeCards(tool: string | undefined, input: unknown, id: string, mode?: string): { questions: Question[]; native: Omit<NativeAsk, 'head'> } | undefined {
  const value = record(input) ?? {}
  if (tool === 'ExitPlanMode') {
    const stop = stopMode(mode)
    return { native: { tool, input: value, stop }, questions: [{ id, title: 'Plan to approve', kind: 'Plan', plan: text(value.plan), to: 'Claude', noteRequiredFor: 2,
      choices: [STOP[stop], { label: 'Approve, keep asking', description: 'Claude asks before each edit' }, { label: 'Change the plan', description: 'Claude plans again from your note' }] }] }
  }
  if (tool !== 'AskUserQuestion') return
  const asked = Array.isArray(value.questions) ? value.questions.map(record) : []
  const questions = asked.flatMap((item, index): Question[] => {
    const options = Array.isArray(item?.options) ? item.options.map(record).filter(option => text(option?.label)) : []
    if (!item || !text(item.question)) return []
    return [{ id: index ? `${id}-${index}` : id, title: text(item.question), kind: 'Question', multi: item.multiSelect === true || undefined, other: true, to: 'Claude',
      choices: options.map(option => ({ label: text(option!.label), description: [text(option!.description), text(option!.preview)].filter(Boolean).join('\n') || undefined })) }]
  })
  return { native: { tool, input: value }, questions: questions.length === asked.length ? questions : [] }
}

/** Checks a card's answer. Throws for one the card did not offer. */
export function nativeAnswer(question: Question, action: Extract<Action, { type: 'answer' }>): { allow: boolean; sent: number; said: string } {
  const other = action.other?.trim(), selected = action.choices ?? [action.choice]
  if (other && question.other) return { allow: true, sent: -1, said: other }
  if (!selected.length || selected.some(index => !Number.isInteger(index) || index < 0 || index >= question.choices.length) || !question.multi && selected.length !== 1) throw new Refusal('Choose one of the card’s answers')
  const note = action.note?.trim()
  if (question.noteRequiredFor !== undefined && selected.includes(question.noteRequiredFor)) {
    if (!note) throw new Refusal('Add the note this choice asks for')
    return { allow: false, sent: selected[0], said: note }
  }
  return { allow: true, sent: selected[0], said: selected.map(index => question.choices[index].label).join(', ') }
}

/** The hook's reply once every card of a call is answered. Claude keeps its own prompt up unless an allow carries the input. */
export function nativeReply(native: NativeAsk, cards: { question: Question; native?: NativeAsk }[], sent: number, note: string): PermissionAnswer {
  if (native.tool === 'AskUserQuestion') {
    return { allow: true, updatedInput: { ...native.input, answers: Object.fromEntries(cards.map(card => [card.question.title, card.native?.answer ?? ''])) } }
  }
  if (sent === 2) return { allow: false, message: `The human read your plan and wants it changed. Stay in plan mode, change the plan as they say, then present it again. Their words:\n\n${note}` }
  return { allow: true, updatedInput: native.input, updatedPermissions: [{ type: 'setMode', mode: sent === 0 ? native.stop : 'default', destination: 'session' }] }
}

export const nativeClosed = (reason: string): PermissionAnswer => ({ allow: false, message: `Shepherdr closed this without an answer: ${reason}. Nothing was approved or chosen. Ask again if you still need it.` })
export const NATIVE_UNREADABLE: PermissionAnswer = { allow: false, message: 'Shepherdr could not read this question. Ask it with shepherdr ask instead.' }

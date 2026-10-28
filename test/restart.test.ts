// What a restart says and offers: the hint once Settings say other than what an agent runs on, a harness in words,
// and what an agent taking over on another CLI is told.
import { describe, expect, test } from 'bun:test'
import { fitHarness, harnessWords, recentChat, restartHint, sameHarness } from '../server/live/restart.ts'
import { restartLine, resumedLine } from '../server/live/prompts.ts'

describe('the restart hint', () => {
  test('says the first that differs: the CLI, the model, the effort', () => {
    expect(restartHint({ kind: 'claude' }, undefined, { kind: 'codex' })).toEqual({ on: 'Claude Code', want: 'Codex' })
    expect(restartHint({ kind: 'claude', model: 'Opus 5.5' }, undefined, { kind: 'claude', model: 'claude-sonnet-5-5' })).toEqual({ on: 'Opus 5.5', want: 'Sonnet 5.5' })
    expect(restartHint({ kind: 'claude', effort: 'high' }, undefined, { kind: 'claude' })).toEqual({ on: 'high effort', want: 'default effort' })
    expect(restartHint({ kind: 'claude', model: '' }, undefined, { kind: 'claude', model: 'claude-opus-5-5' })).toEqual({ on: "Claude Code's default model", want: 'Opus 5.5' })
  })

  test('none while they agree, a model by its name or id, a level as its model takes it', () => {
    expect(restartHint({ kind: 'claude', model: 'Opus 5.5', effort: 'high' }, undefined, { kind: 'claude', model: 'claude-opus-5-5', effort: 'high' })).toBeUndefined()
    expect(restartHint({ kind: 'claude', model: '', effort: '' }, undefined, { kind: 'claude' })).toBeUndefined()
    expect(sameHarness({ kind: 'claude', effort: 'ultra' }, { kind: 'claude', effort: 'max' })).toBe(true)
  })

  test('none while Settings say what they said when it started: a pick of its own is no change of Settings', () => {
    // started on Settings' Opus, then switched to Sonnet in the chat's + menu
    expect(restartHint({ kind: 'claude', model: 'Sonnet 5.5' }, { kind: 'claude', model: 'claude-opus-5-5' }, { kind: 'claude', model: 'claude-opus-5-5' })).toBeUndefined()
    // Settings changed since
    expect(restartHint({ kind: 'claude', model: 'Sonnet 5.5' }, { kind: 'claude', model: 'claude-opus-5-5' }, { kind: 'codex' })).toEqual({ on: 'Claude Code', want: 'Codex' })
  })
})

describe('a harness', () => {
  test('in words, and as Shepherdr passes it', () => {
    expect(harnessWords({ kind: 'codex' })).toBe('Codex')
    expect(harnessWords({ kind: 'claude', model: 'sonnet', effort: 'high' })).toBe('Claude Code (Sonnet 5.5, high)')
    expect(harnessWords({ kind: 'claude', effort: 'xhigh' })).toBe('Claude Code (xhigh)')
    expect(fitHarness({ kind: 'claude', model: 'Opus 5.5', effort: 'default' })).toEqual({ kind: 'claude', model: 'claude-opus-5-5' })
    expect(fitHarness({ kind: 'claude', model: 'default', effort: '' })).toEqual({ kind: 'claude' })
  })
})

describe('what the agent is told', () => {
  test('another CLI takes over: who ran it, a task its commits and changes, the chat so far', () => {
    const chat = recentChat(
      [
        { id: '1', who: 'you', text: 'The brief', at: 1 },
        { id: '2', who: 'you', text: 'Use blue', at: 2 },
        { id: '3', who: 'agent', text: 'On it', at: 3 },
        { id: '4', who: 'agent', card: { id: 'c', text: 'Restarted' }, at: 4 },
      ] as never,
      'Claude Code',
      'The brief',
    )
    expect(chat).toBe('Human: Use blue\n\nClaude Code: On it')
    const task = restartLine({ from: 'Claude Code', to: 'Codex (GPT-6-Sol, high)', chat, commits: 'a1 Add x', changes: '' })
    expect(task).toContain('restarted this task on Codex (GPT-6-Sol, high). Claude Code ran it until now')
    expect(task).toContain('same worktree and branch')
    expect(task).toContain('Commits on the branch so far:\na1 Add x')
    expect(task).toContain('Uncommitted changes:\nnone')
    expect(task).toContain('Human: Use blue')
    const coord = restartLine({ from: 'Claude Code', to: 'Codex', chat })
    expect(coord).toContain('restarted this chat on Codex')
    expect(coord).not.toContain('Commits')
  })

  test('its own CLI resumes: carry on, or say nothing', () => {
    expect(resumedLine('Claude Code (Opus 5.5, high)')).toContain('<no-reply>restarted</no-reply>')
  })
})

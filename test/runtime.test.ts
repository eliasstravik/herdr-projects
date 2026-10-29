import { describe, expect, test } from 'bun:test'
import { CodexReader, ClaudeReader, PiReader, LogTail } from '../server/live/readers/index.ts'
import { sessionRuntime } from '../server/live/runtime.ts'
import { codexRuntime } from '../server/live/codex-runtime.ts'
import { HookLogs, pickHook } from '../server/live/readers/hooks.ts'
import { mkdtempSync, writeFileSync, appendFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const at = 1_700_000_000_000
const lines = (...rows: unknown[]) => rows.map((row, offset) => ({ text: JSON.stringify(row), offset }))
const context = (model: string, effort?: string | null) => ({ timestamp: new Date(at).toISOString(), type: 'turn_context', payload: { model, effort } })

describe('CLI runtime observations, independent of launch policy', () => {
  test('Codex defaults and CLI overrides come from turn context, not preferences; later snapshots clear unset effort', () => {
    const reader = new CodexReader()
    const ev = reader.parse(lines(context('custom-config-model', 'high')), 'live')
    expect(sessionRuntime(ev, 'live', 0)).toMatchObject({ model: 'custom-config-model', effort: 'high' })
    ev.push(...reader.parse(lines(context('runtime-changed', null)), 'live'))
    expect(sessionRuntime(ev, 'live', 0)).toMatchObject({ model: 'runtime-changed' })
    expect(sessionRuntime(ev, 'live', 0)?.effort).toBeUndefined()
    expect(sessionRuntime(ev, 'other-session', 0)).toBeUndefined()
    expect(sessionRuntime(ev, 'live', at + 1)).toBeUndefined()
  })

  test('resumed daemon state wins over historical launch settings; null never invents default certainty', () => {
    expect(codexRuntime({ model: 'new-model', reasoningEffort: 'xhigh', status: { type: 'active' } }, at)).toMatchObject({ model: 'new-model', effort: 'xhigh' })
    expect(codexRuntime({ model: 'new-model', reasoningEffort: null, status: { type: 'idle' } }, at)?.effort).toBeUndefined()
    expect(codexRuntime({ model: 'historical', reasoningEffort: 'low', status: { type: 'notLoaded' } }, at)).toBeUndefined()
  })

  test('Pi separately records model and thinking changes, including off', () => {
    const ev = new PiReader().parse(lines(
      { type: 'model_change', timestamp: new Date(at).toISOString(), modelId: 'custom/provider-model' },
      { type: 'thinking_level_change', timestamp: new Date(at).toISOString(), thinkingLevel: 'high' },
      { type: 'thinking_level_change', timestamp: new Date(at).toISOString(), thinkingLevel: 'off' },
    ), 'pi')
    expect(sessionRuntime(ev, 'pi', 0)).toMatchObject({ model: 'custom/provider-model', effort: 'off' })
  })

  test('resumed Pi cannot reuse old thinking state; a fresh assistant reports the actual model', () => {
    const ev = new PiReader().parse(lines(
      { type: 'model_change', timestamp: new Date(at - 10).toISOString(), modelId: 'old-model' },
      { type: 'thinking_level_change', timestamp: new Date(at - 10).toISOString(), thinkingLevel: 'high' },
      { type: 'message', timestamp: new Date(at + 10).toISOString(), message: { role: 'assistant', model: 'resumed-model', content: [] } },
    ), 'pi')
    expect(sessionRuntime(ev, 'pi', at)).toMatchObject({ model: 'resumed-model' })
    expect(sessionRuntime(ev, 'pi', at)?.effort).toBeUndefined()
  })

  test('Claude ignores synthetic error models and sidechains', () => {
    const reader = new ClaudeReader()
    const ev = reader.parse(lines(
      { type: 'assistant', timestamp: new Date(at).toISOString(), message: { model: 'claude-real', content: [] } },
      { type: 'assistant', timestamp: new Date(at).toISOString(), message: { model: '<synthetic>', content: [] } },
      { type: 'assistant', timestamp: new Date(at).toISOString(), isSidechain: true, message: { model: 'subagent-model', content: [] } },
    ), 'claude')
    expect(sessionRuntime(ev, 'claude', 0)?.model).toBe('claude-real')
  })

  test('hooks whitelist runtime fields without secrets, ignore subagent effort and reset at session start', () => {
    const dir = mkdtempSync(join(tmpdir(), 'runtime-hook-'))
    try {
      const file = join(dir, 'hooks')
      const logs = new HookLogs()
      const e = pickHook({ hook_event_name: 'Stop', model: 'claude-real', effort: { level: 'high' }, api_key: 'never-copy' }, at)!
      expect(JSON.stringify(e)).not.toContain('never-copy')
      writeFileSync(file, JSON.stringify(e) + '\n')
      expect(logs.read(file).runtime).toMatchObject({ model: 'claude-real', effort: 'high' })
      appendFileSync(file, JSON.stringify({ at: at + 1, event: 'Stop', agent: 'child', effort: 'low' }) + '\n')
      expect(logs.read(file).runtime?.effort).toBe('high')
      appendFileSync(file, JSON.stringify({ at: at + 2, event: 'SessionStart', session: 'resumed' }) + '\n')
      expect(logs.read(file).runtime).toBeUndefined()
    } finally { rmSync(dir, { recursive: true, force: true }) }
  })

  test('replayed session metadata survives server restart; truncated logs replace old runtime', () => {
    const dir = mkdtempSync(join(tmpdir(), 'runtime-log-'))
    try {
      const file = join(dir, 'log')
      writeFileSync(file, JSON.stringify(context('long-old-model-name', 'xhigh')) + '\n')
      const tail = new LogTail(file, new CodexReader())
      expect(sessionRuntime(tail.read(), file, 0)?.effort).toBe('xhigh')
      writeFileSync(file, JSON.stringify(context('new', 'low')) + '\n')
      expect(sessionRuntime(tail.read(), file, 0)).toMatchObject({ model: 'new', effort: 'low' })
      expect(sessionRuntime(new LogTail(file, new CodexReader()).read(), file, 0)?.model).toBe('new')
    } finally { rmSync(dir, { recursive: true, force: true }) }
  })
})

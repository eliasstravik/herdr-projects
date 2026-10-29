import { describe, expect, test, spyOn } from 'bun:test'
import { mkdtempSync, readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

process.env.SHEPHERDR_HOME = mkdtempSync(join(tmpdir(), 'shep-private-launch-'))
process.env.HERDR_SOCKET_PATH = join(process.env.SHEPHERDR_HOME, 'no-herdr.sock')
const { Agents, hidden } = await import('../server/live/agents.ts')
const { herdr } = await import('../server/live/herdr.ts')
const { writeJson } = await import('../server/store.ts')

describe('private launch prompt delivery', () => {
  test('resumed Codex receives changed or cleared snapshots before the restart message, but no unchanged duplicate', async () => {
    const key = 'PRIVATE-1/Agent'
    const cwd = mkdtempSync(join(tmpdir(), 'shep-private-cwd-'))
    const old = { key, name: 'shep-private-test', kind: 'codex', cwd, workspace: 'test', logs: [], sessionId: 'prior-session', started: 0, system: 'old private snapshot' }
    const start = spyOn(herdr, 'start').mockResolvedValue(undefined as never)
    const timer = spyOn(globalThis, 'setTimeout').mockImplementation((() => 0) as never)
    try {
      for (const system of ['new private snapshot', 'Private instructions for task:\n(none)', old.system]) {
        writeJson('agents.json', { [key]: old })
        const a = new Agents()
        const queued: string[][] = []
        // Observe the real launch's queue boundary, without a live terminal, routes or delayed delivery.
        Object.assign(a, {
          refresh: async () => undefined,
          bindRoute: async () => undefined,
          queue: (_key: string, texts: string[]) => queued.push(texts),
        })
        try {
          await a.ensure({ ...old, system, first: 'Resume the work.', pane: async () => 'fake-pane' })
          expect(queued).toEqual([system === old.system ? ['Resume the work.'] : [hidden(system), 'Resume the work.']])
          expect(a.record(key)?.system).toBe(system)
        } finally { a.stop() }
      }
      expect(start).toHaveBeenCalledTimes(3)
    } finally {
      start.mockRestore()
      timer.mockRestore()
    }
  })

  test('Claude and pi append a private prompt file outside the repository, on fresh and resumed starts', () => {
    const a = new Agents()
    try {
      for (const kind of ['claude', 'pi']) for (const resume of [false, true]) {
        const r = { key: 'PRIVATE-2/Agent', name: `private-test-${kind}`, kind, cwd: '/unused-repo', workspace: 'test', logs: [], started: 0, sessionId: 'old-session', system: 'Synthetic personal rule.' }
        const args = a.args(r, resume)
        const flag = kind === 'claude' ? '--append-system-prompt-file' : '--append-system-prompt'
        const path = args[args.indexOf(flag) + 1]
        expect(path).not.toStartWith(r.cwd)
        expect(readFileSync(path, 'utf8')).toBe(r.system)
      }
    } finally { a.stop() }
  })
})

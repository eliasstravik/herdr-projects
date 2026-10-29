import { describe, expect, test } from 'bun:test'
import { mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { RequestedNewsMailbox } from '../server/live/requested-news.ts'

describe('requesting coordinator mailbox', () => {
  test('only real coordinator identities in this installation are accepted, including helper worker records', () => {
    const root = mkdtempSync(join(tmpdir(), 'requester-'))
    const other = mkdtempSync(join(tmpdir(), 'other-user-'))
    try {
      const home = join(root, 'machines', 'desktop')
      mkdirSync(home, { recursive: true })
      writeFileSync(join(home, 'agents.json'), JSON.stringify({
        'p:desktop:repo': { key: 'p:desktop:repo' },
        'p:desktop:old': { key: 'p:desktop:old', finished: true },
        'TASK-1/Agent': { key: 'TASK-1/Agent' },
      }))
      const mail = new RequestedNewsMailbox(root)
      expect(mail.known('p:desktop:repo')).toBe('p:desktop:repo')
      for (const chat of [undefined, 'repo', 'p:desktop:invented', 'p:desktop:old', 'TASK-1/Agent']) expect(mail.known(chat)).toBeUndefined()
      expect(new RequestedNewsMailbox(other).known('p:desktop:repo')).toBeUndefined()
    } finally { rmSync(root, { recursive: true, force: true }); rmSync(other, { recursive: true, force: true }) }
  })
  test('restart and offline replay preserve events; successful delivery is consumed once without recursive publishing', async () => {
    const root = mkdtempSync(join(tmpdir(), 'requester-'))
    try {
      const writer = new RequestedNewsMailbox(root)
      writer.post({ to: 'p:small:repo', from: 'p:desktop:repo', text: 'SHE-1 is ready', task: 'SHE-1', event: 'ready' })
      writer.post({ to: 'p:small:repo', from: 'p:desktop:repo', text: 'SHE-1 merged', task: 'SHE-1', event: 'merged' })
      let calls = 0
      const owns = (chat: string) => chat === 'p:small:repo'
      const restarted = new RequestedNewsMailbox(root)
      await restarted.drain(() => false, async () => { calls++ })
      expect(calls).toBe(0)
      await expect(restarted.drain(owns, async () => { throw new Error('offline') })).rejects.toThrow('offline')
      const ids: string[] = []
      const events: string[] = []
      await new RequestedNewsMailbox(root).drain(owns, async (n) => { ids.push(n.id); events.push(n.event); expect(n.task).toBe('SHE-1'); calls++ })
      await restarted.drain(owns, async () => { calls++ })
      expect(calls).toBe(2)
      expect(ids).toHaveLength(2)
      expect(events).toEqual(['ready', 'merged'])
      expect(readdirSync(join(root, 'requested-news'))).toEqual([])
    } finally { rmSync(root, { recursive: true, force: true }) }
  })
})

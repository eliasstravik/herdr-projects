// A watched terminal's screen (agents.ts): herdr tells no change on a pane's screen, so Shepherdr reads it again while
// a screen shows it, at once and often after a key went in (its echo), and tells only a change.
import { afterAll, describe, expect, test } from 'bun:test'
import { mkdtempSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

process.env.SHEPHERDR_HOME = mkdtempSync(join(tmpdir(), 'shep-term-'))
process.env.HERDR_SOCKET_PATH = join(process.env.SHEPHERDR_HOME, 'herdr.sock')
const { Agents } = await import('../server/live/agents.ts')
const { writeJson } = await import('../server/store.ts')

// herdr, standing in: one agent's pane, whose screen is what was typed into it; subscriptions are kept to look at
let typed = ''
let reads = 0
const subscribed: unknown[][] = []
const server = createServer((c) => {
  let buf = ''
  c.on('data', (d) => {
    buf += d
    const i = buf.indexOf('\n')
    if (i < 0) return
    const { id, method, params } = JSON.parse(buf.slice(0, i))
    buf = buf.slice(i + 1)
    if (method === 'events.subscribe') {
      subscribed.push(params.subscriptions)
      // as herdr 0.9 does: an event it doesn't have turns the whole subscription down
      const unknown = params.subscriptions.find((s: { type: string }) => s.type === 'pane.output_changed')
      if (unknown) return void c.end(`${JSON.stringify({ id, error: { code: 'invalid_request', message: 'unknown variant `pane.output_changed`' } })}\n`)
      return void c.write(`${JSON.stringify({ id, result: { type: 'subscription_started' } })}\n`)
    }
    if (method === 'pane.send_text') typed += params.text
    if (method === 'pane.read') reads++
    const agent = { agent: 'claude', agent_status: 'idle', name: 'shep-t-1', pane_id: 'p1', tab_id: 't', workspace_id: 'w', cwd: null, interactive_ready: true }
    const result =
      method === 'agent.list' ? { agents: [agent] }
      : method === 'pane.read' ? { read: { text: `❯ ${typed}` } }
      : method === 'pane.layout' ? { layout: { panes: [{ pane_id: 'p1', rect: { width: 80, height: 24 } }] } }
      : {}
    c.end(`${JSON.stringify({ id, result })}\n`)
  })
}).listen(process.env.HERDR_SOCKET_PATH)
afterAll(() => server.close())
writeJson('agents.json', { 'T-1/Agent': { key: 'T-1/Agent', name: 'shep-t-1', kind: 'claude', cwd: '/w/1', workspace: 'w', logs: [], started: 0 } })

describe('a watched terminal', () => {
  test('what is typed shows at once, though herdr tells no change; an unchanged screen is not told again', async () => {
    const a = new Agents()
    await a.refresh()
    // left alone it is read seldom: what shows soon after a key is its echo, read for it
    a.termSlow = 60_000
    const screens: string[] = []
    a.on('screen', (key, s) => key === 'T-1/Agent' && screens.push(s.ansi))
    a.watchTerminals(['T-1/Agent'])
    await a.text('T-1/Agent', 'h')
    await Bun.sleep(40)
    expect(screens.at(-1)).toBe('❯ h')
    await a.text('T-1/Agent', 'i')
    await Bun.sleep(40)
    expect(screens.at(-1)).toBe('❯ hi')
    // read again and again for a while after it, but told once per change
    const n = reads
    await Bun.sleep(300)
    expect(reads).toBeGreaterThan(n)
    expect(screens).toEqual(['❯ h', '❯ hi'])
    // no longer watched: no more reads
    a.watchTerminals([])
    await Bun.sleep(100)
    const m = reads
    await a.text('T-1/Agent', '!')
    await Bun.sleep(300)
    expect(reads).toBe(m)
    a.stop()
  })

  test('herdr is never asked for an event it does not have: that would turn the whole subscription down', async () => {
    const a = new Agents()
    // its pane known first, then watched: the next look would have asked for its output's changes
    await a.refresh()
    a.watchTerminals(['T-1/Agent'])
    await a.refresh()
    await Bun.sleep(50)
    expect(subscribed.length).toBeGreaterThan(0)
    for (const subs of subscribed) expect(subs.map((s) => (s as { type: string }).type)).not.toContain('pane.output_changed')
    a.stop()
  })
})

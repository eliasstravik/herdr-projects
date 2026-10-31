import { afterEach, beforeEach, expect, test } from 'bun:test'
import { mkdtempSync, rmSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { CoreSource } from '../server/core/source.ts'
import type { CoreAgent, CoreSpace } from '../server/core/model.ts'
import type { CoreTerminal } from '../server/core/protocol.ts'
import type { Message, Question } from '../shared/types.ts'

const TRUST = `Quick safety check: do you trust this folder?\n\n ❯ No, exit\n   Yes, I trust this folder\n\n Enter to confirm\n`
const THEME = `Choose a style\n\n ❯ Dark mode\n   Light mode\n`
const started = 1_000_000

let home: string, source: CoreSource, screen: string, keys: string[][], alerts: unknown[], terminal: CoreTerminal
const agent = { chat: 'space', space: 'space', machine: 'machine', role: 'coordinator', kind: 'claude', launch: 'one', grant: 'grant', hook: '/unused', state: 'starting', started } satisfies CoreAgent
const saved = () => source.store.get<CoreAgent>('agent', 'space')!
const cards = () => source.store.all<Message>('message:space').filter(message => message.question).map(message => message.question!)
const open = (): Question => cards().find(card => card.sent === undefined && !card.closed)!
/** two matching reads, past the first look */
const settle = async (at = started + 10_000) => { await source.screens.tick(at); await source.screens.tick(at + 2000) }

beforeEach(() => {
  home = mkdtempSync(join(tmpdir(), 'shep-screens-'))
  source = new CoreSource({ home, port: 17887, socket: '/unused', session: 'test', binary: 'hc' })
  source.store.set('space', 'space', { id: 'space', name: 'Example', machine: 'machine', folder: '/unused', base: 'main', prefix: 'EX', mode: 'regular', added: 1 } satisfies CoreSpace)
  source.store.set('agent', 'space', agent)
  screen = TRUST; keys = []; alerts = []
  terminal = { address: 'box/t7', labels: {}, argv: [], cwd: '/', state: 'live', created_ms: 1, cols: 120, rows: 40 }
  source.agents.terminal = () => terminal
  source.connected = () => true
  source.machineConnected = () => true
  source.core.call = (async (method: string, params: { keys?: string[] }) => {
    if (method === 'terminal.read') return { text: screen }
    if (method === 'terminal.keys') keys.push(params.keys!)
    return {}
  }) as typeof source.core.call
  source.on('alert', alert => alerts.push(alert))
})
afterEach(async () => { source.stop(); await Promise.resolve(); source.mailbox.close(); source.store.close(); rmSync(home, { recursive: true, force: true }) })

test('no card before the agent has had time to start, nor for one with a session', async () => {
  await settle(started + 2000)
  expect(cards()).toEqual([])
  source.store.set('agent', 'space', { ...agent, state: 'idle' })
  await settle()
  expect(cards()).toEqual([])
})

test('a reviewer has no chat to show a card in', async () => {
  source.store.set('agent', 'space', { ...agent, role: 'reviewer' })
  await settle()
  expect(cards()).toEqual([])
})

test('a screen still moving waits; a blank one never shows', async () => {
  screen = 'Loading the project'
  await source.screens.tick(started + 10_000)
  screen = 'Loading the servers'
  await source.screens.tick(started + 12_000)
  expect(cards()).toEqual([])
  screen = ''
  await settle()
  expect(cards()).toEqual([])
})

test('a stuck agent shows its screen, counts as Needs you and stops reading as working', async () => {
  expect(source.status(saved())).toBe('work')
  await settle()
  const card = open()
  expect(card).toMatchObject({ title: 'Claude Code is waiting on its screen', terminal: true, press: true, noNote: true, to: 'Claude Code' })
  expect(card.screen).toContain('do you trust this folder?')
  expect(card.choices.map(choice => choice.label)).toEqual(['No, exit', 'Yes, I trust this folder'])
  expect(source.status(saved())).toBe('need')
  expect(source.snapshot().attentionRequests?.space).toEqual([card.id])
  expect(source.chat('space')?.working).toBe(false)
  expect(alerts).toHaveLength(1)
  await settle(started + 20_000)
  expect(cards()).toHaveLength(1) // one screen, one card
})

test('a tap presses the choice from where the cursor is, once, on the screen the card shows', async () => {
  await settle()
  await source.act({ type: 'answer', chat: 'space', question: open().id, choice: 1 })
  expect(keys).toEqual([['down', 'enter']])
  expect(cards()[0].sent).toBe(1)
  await expect(source.act({ type: 'answer', chat: 'space', question: cards()[0].id, choice: 1 })).rejects.toThrow('no longer waiting')
  expect(keys).toHaveLength(1)
})

test('nothing is pressed once the screen has changed under the card', async () => {
  await settle()
  const card = open()
  screen = THEME
  await expect(source.act({ type: 'answer', chat: 'space', question: card.id, choice: 1 })).rejects.toThrow('changed')
  expect(keys).toEqual([])
  expect(open().id).toBe(card.id)
})

test('the next screen is the next card, and the one before it says it went in the terminal', async () => {
  await settle()
  const first = open().id
  screen = THEME
  await settle(started + 20_000)
  expect(cards().find(card => card.id === first)).toMatchObject({ sent: -1, via: 'terminal' })
  expect(open().choices.map(choice => choice.label)).toEqual(['Dark mode', 'Light mode'])
  expect(cards()).toHaveLength(2)
})

test('a press that changed nothing is asked again, after a while', async () => {
  await settle()
  await source.act({ type: 'answer', chat: 'space', question: open().id, choice: 1 })
  await settle(Date.now())
  expect(cards()).toHaveLength(1)
  await settle(Date.now() + 20_000)
  expect(cards()).toHaveLength(2)
})

test('the session starting closes the card, and messages wait for it meanwhile', async () => {
  await settle()
  const id = open().id
  expect(source.status(saved())).toBe('need')
  source.store.set('agent', 'space', { ...agent, state: 'idle' })
  source.screens.reconcile('space')
  expect(cards().find(card => card.id === id)).toMatchObject({ sent: -1, via: 'terminal' })
  expect(source.screens.open('space')).toEqual([])
})

test('an agent gone, or replaced, ends its card honestly', async () => {
  await settle()
  const stopped = open().id
  terminal = { ...terminal, state: 'finished' }
  source.screens.reconcile('space')
  expect(cards().find(card => card.id === stopped)?.closed).toBe('The agent stopped')
  terminal = { ...terminal, state: 'live' }
  screen = THEME
  await settle(started + 40_000)
  const replaced = open().id
  source.store.set('agent', 'space', { ...agent, launch: 'two' })
  source.screens.reconcile('space')
  expect(cards().find(card => card.id === replaced)?.closed).toBe('Session replaced')
  await expect(source.act({ type: 'answer', chat: 'space', question: replaced, choice: 0 })).rejects.toThrow('no longer waiting')
})

test('with no choices to read it is still a card, for the terminal', async () => {
  screen = 'Paste code here if prompted >\n'
  await settle()
  expect(open()).toMatchObject({ choices: [], terminal: true })
  await expect(source.act({ type: 'answer', chat: 'space', question: open().id, choice: 0 })).rejects.toThrow('answers')
})

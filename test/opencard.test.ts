// A card Shepherdr posted for an agent (its plan, a choice card) is why it waits on you only while it is unanswered
// and nothing was written to it since.
import { expect, test } from 'bun:test'
import { mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

process.env.SHEPHERDR_HOME ??= mkdtempSync(join(tmpdir(), 'shep-card-'))
const { Chats } = await import('../server/live/chats.ts')

test('an open plan or choice card, until it is answered or you write', async () => {
  const chats = new Chats({ record: () => undefined } as never, (k) => k, () => 'You')
  const key = `T-${Date.now()}/Agent`
  const question = (id: string) => ({ id, title: 'x', kind: 'Plan' as const, choices: [{ label: 'Yes' }], to: 'T agent' })
  expect(chats.openCard(key)).toBeUndefined()
  chats.post(key, { id: 'plan:1', question: question('plan:T:1') })
  expect(chats.openCard(key)).toBe('plan')
  chats.answer(key, 'plan:T:1', 0)
  expect(chats.openCard(key)).toBeUndefined()
  await Bun.sleep(2)
  chats.post(key, { id: 'ask:1', question: question('ask:1') })
  expect(chats.openCard(key)).toBe('ask')
  await Bun.sleep(2)
  chats.post(key, { id: 'you:1', who: 'you', text: 'Something else' })
  expect(chats.openCard(key)).toBeUndefined()
})

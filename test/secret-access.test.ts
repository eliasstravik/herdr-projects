import { expect, test } from 'bun:test'
import { SecretAccess } from '../server/live/secret-access.ts'

test('capabilities are scoped, rotated on process or scope changes, and revoked independently per worker', () => {
  const a = new SecretAccess()
  const b = new SecretAccess()
  const chain = ['p:one:api', 'm:one', 'all']
  const first = a.issue('TASK-1/Agent', 'pid:start', chain)
  expect(first).toMatch(/^[a-f0-9]{64}$/)
  expect(a.issue('TASK-1/Agent', 'pid:start', chain)).toBe(first)
  expect(a.authenticate(first)).toEqual({ chat: 'TASK-1/Agent', chain })
  expect(b.authenticate(first)).toBeUndefined()
  expect(a.authenticate()).toBeUndefined()
  expect(a.authenticate('invalid')).toBeUndefined()
  const restarted = a.issue('TASK-1/Agent', 'pid:new-start', chain)
  expect(restarted).not.toBe(first)
  expect(a.authenticate(first)).toBeUndefined()
  const rescoped = a.issue('TASK-1/Agent', 'pid:new-start', ['p:one:other', 'm:one', 'all'])
  expect(a.authenticate(restarted)).toBeUndefined()
  a.revoke('TASK-1/Agent')
  expect(a.authenticate(rescoped)).toBeUndefined()
})

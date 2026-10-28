// The tests never reach the data of the Shepherdr running on this machine, however they were started: under bun test the
// store is a fresh folder (server/store.ts, and test/preload.ts gives each test process its own).
import { expect, test } from 'bun:test'
import { homedir } from 'node:os'
import { join } from 'node:path'
import { HOME } from '../server/store.ts'

test("the store's folder is a fresh one for the tests, never ~/.shepherdr", () => {
  expect(HOME).not.toBe(join(homedir(), '.shepherdr'))
  expect(HOME).toContain('shep-test-home-')
})

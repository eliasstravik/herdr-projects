// A turn cut off by the model's provider: each CLI's log says so (on recorded endings in samples/endings, from
// herdr-projects' tests: the "-made" ones are written by hand in a recorded one's shape), which errors pass by
// themselves, and the waits and tries before the human hears.
import { describe, expect, test } from 'bun:test'
import { mkdtempSync, readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

process.env.SHEPHERDR_HOME = mkdtempSync(join(tmpdir(), 'shep-retry-'))
const { readerFor } = await import('../server/live/readers/index.ts')
const { carryOn, providerError, Retries } = await import('../server/live/retry.ts')

const ending = (name: string) => {
  const cli = name.split('-')[0]
  const lines = readFileSync(join(import.meta.dir, 'samples', 'endings', `${name}.jsonl`), 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((text, i) => ({ text, offset: i * 1000 }))
  const ev = readerFor(cli)!.parse(lines, `/logs/${name}.jsonl`)
  const e = ev.find((x) => x.kind === 'error')
  if (e?.kind !== 'error') throw new Error(`${name}: no error read`)
  // the turn is over with it
  expect(ev.findLastIndex((x) => x.kind === 'turn-end')).toBeGreaterThan(ev.indexOf(e))
  return providerError(e.text, e.code, e.status)
}

describe("each CLI's log says what cut its turn off", () => {
  test('errors that pass by themselves are waited out', () => {
    expect(ending('claude-overloaded-made')).toEqual({ passing: true, reason: 'Provider overloaded' })
    expect(ending('claude-rate-limit-made')).toEqual({ passing: true, reason: 'Rate limited' })
    expect(ending('codex-at-capacity')).toEqual({ passing: true, reason: 'Model at capacity' })
    expect(ending('pi-socket-closed')).toEqual({ passing: true, reason: 'Connection dropped' })
    expect(ending('pi-overloaded')).toEqual({ passing: true, reason: 'Provider overloaded' })
    expect(ending('pi-gateway-502').passing).toBe(true)
  })

  test('errors waiting cannot fix go on at once, in a few words', () => {
    expect(ending('claude-usage-limit-made')).toEqual({ passing: false, reason: 'Out of allowance until 5pm' })
    expect(ending('claude-model-not-found')).toEqual({ passing: false, reason: 'Model not available' })
    expect(ending('codex-unauthorized')).toEqual({ passing: false, reason: 'Signed out' })
    expect(ending('codex-model-not-supported')).toEqual({ passing: false, reason: 'Model not available' })
    expect(ending('pi-unknown-provider').passing).toBe(false)
    expect(providerError('Your input exceeds the context window of this model.', 'context_window_exceeded').reason).toBe('Context window full')
    expect(providerError("You've hit your limit · resets 5pm (Europe/Oslo)").reason).toBe('Out of allowance until 5pm')
    expect(providerError('Login expired · Please run /login', 'authentication_failed')).toEqual({ passing: false, reason: 'Signed out' })
    // anything not recognised is not waited out
    expect(providerError('Something odd happened')).toEqual({ passing: false, reason: 'Provider error' })
  })
})

describe('waits and tries', () => {
  const cut = (id: string, text = 'API Error: 529 Overloaded', status = 529) => ({ id, at: Date.now(), text, status })
  const world = (waits = [30, 60, 90, 120]) => {
    const nudged: string[] = []
    const over: { key: string; reason: string }[] = []
    const r = new Retries({ nudge: (key, text) => nudged.push(`${key} ${text}`), over: (key, x) => over.push({ key, reason: x.reason }), changed: () => undefined }, waits)
    return { r, nudged, over }
  }
  /** a wait is over: however slow the machine, its timer has fired */
  const until = async (f: () => boolean) => {
    for (const t0 = Date.now(); !f(); await Bun.sleep(5)) if (Date.now() - t0 > 8000) throw new Error('timed out')
  }

  test('told to carry on after each wait; still cut off after the last try, it gives up', async () => {
    const w = world()
    w.r.seen('A', cut('e1'))
    expect(w.r.get('A')).toMatchObject({ tries: 1, reason: 'Provider overloaded' })
    // the same cut-off turn seen again changes nothing
    w.r.seen('A', cut('e1'))
    expect(w.r.get('A')!.tries).toBe(1)
    await until(() => w.nudged.length > 0)
    expect(w.nudged).toEqual([`A ${carryOn('Provider overloaded')}`])
    expect(w.nudged[0]).toContain('Carry on where you left off.')
    for (const id of ['e2', 'e3', 'e4']) w.r.seen('A', cut(id))
    expect(w.r.get('A')).toMatchObject({ tries: 4 })
    expect(w.over).toEqual([])
    w.r.seen('A', cut('e5'))
    expect(w.over).toEqual([{ key: 'A', reason: 'Provider overloaded, still after 5 tries' }])
    expect(w.r.get('A')).toMatchObject({ over: true })
    expect(w.r.get('A')!.due).toBeUndefined()
  })

  test('a turn that ends well, or a message that goes in, starts the count again', async () => {
    const w = world()
    w.r.seen('B', cut('e1'))
    await until(() => w.nudged.length > 0)
    // the turn it was told to carry on in ended well
    w.r.seen('B', undefined)
    expect(w.r.get('B')).toBeUndefined()
    // waiting, a message of yours goes in: no try is due any more
    w.r.seen('B', cut('e2'))
    w.r.clear('B')
    await Bun.sleep(50)
    expect(w.nudged).toHaveLength(1)
    expect(w.r.get('B')).toBeUndefined()
    // waiting, it went on by itself and that turn ended well (a job woke it, you typed in its terminal): never told
    w.r.seen('B', cut('e3'))
    w.r.seen('B', undefined)
    await Bun.sleep(50)
    expect(w.nudged).toHaveLength(1)
    expect(w.r.get('B')).toBeUndefined()
  })

  test('an error waiting cannot fix gives up at once', () => {
    const w = world()
    w.r.seen('C', cut('e1', "You've hit your session limit · resets 5pm", 429))
    expect(w.over).toEqual([{ key: 'C', reason: 'Out of allowance until 5pm' }])
  })
})

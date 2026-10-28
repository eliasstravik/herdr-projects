// An agent that stopped, or runs on files Shepherdr no longer starts it with, is started again: what counts as wrong,
// and that tries are spaced out and end, so a start that keeps failing never loops.
import { describe, expect, test } from 'bun:test'
import { paneWrong, Revivals } from '../server/live/revive.ts'

const want = { '--settings': '/h/.shepherdr/helper/hooks/shep-x.settings.json', '--append-system-prompt-file': '/h/.shepherdr/helper/prompts/shep-x.md' }
const running = (argv: string[]) => ({ shell_pid: 10, foreground_process_group_id: 20, foreground_processes: [{ argv, pid: 20 }] })

describe('what is wrong with a pane', () => {
  test('its CLI on the files it would start with now is fine', () => {
    expect(paneWrong('claude', running(['claude', '--resume', 'abc', '--settings', want['--settings'], '--append-system-prompt-file', want['--append-system-prompt-file']]), want)).toBeUndefined()
  })

  test('the command herdr restored from before the helper moved is stale', () => {
    const old = ['claude', '--resume', 'abc', '--settings', '/h/.shepherdr/hooks/shep-x.settings.json', '--append-system-prompt-file', '/h/.shepherdr/prompts/shep-x.md']
    expect(paneWrong('claude', running(old), want)).toBe('stale')
    // no hooks at all
    expect(paneWrong('claude', running(['claude', '--resume', 'abc']), want)).toBe('stale')
  })

  test('the shell at its prompt: the CLI exited', () => {
    expect(paneWrong('claude', { shell_pid: 10, foreground_process_group_id: 10, foreground_processes: [{ argv: ['bash'], pid: 10 }] }, want)).toBe('exited')
  })

  test('something else in the foreground, or a CLI with no files to check, is left alone', () => {
    expect(paneWrong('claude', running(['vim', 'x.ts']), want)).toBeUndefined()
    expect(paneWrong('codex', running(['node', '/usr/lib/bin/codex', 'resume', 'id']), {})).toBeUndefined()
  })
})

describe('tries at starting it again', () => {
  const times = { grace: 20, gaps: [0, 60, 300], steady: 600 }

  test('only once it was seen wrong a while, then further apart, then given up once', () => {
    const r = new Revivals(times)
    expect(r.seen('a', 0)).toBeUndefined()
    expect(r.seen('a', 20)).toBe('go')
    // judged again only after its grace, and the next try after its gap
    expect(r.seen('a', 30)).toBeUndefined()
    expect(r.seen('a', 50)).toBeUndefined()
    expect(r.seen('a', 80)).toBe('go')
    expect(r.seen('a', 100)).toBeUndefined()
    expect(r.seen('a', 200)).toBeUndefined()
    expect(r.seen('a', 380)).toBe('go')
    expect(r.seen('a', 400)).toBeUndefined()
    expect(r.seen('a', 420)).toBe('give-up')
    for (const t of [500, 1000, 5000]) expect(r.seen('a', t)).toBeUndefined()
  })

  test('running well a while without a break counts from none again; a moment of it does not', () => {
    const r = new Revivals(times)
    r.seen('a', 0)
    expect(r.seen('a', 20)).toBe('go')
    // up for a moment after its start, then down again: still its second try, after the gap
    r.fine('a', 25)
    r.seen('a', 30)
    expect(r.seen('a', 50)).toBeUndefined()
    expect(r.seen('a', 80)).toBe('go')
    // well for a while, but with a break: still its third try, after the gap
    r.fine('a', 400)
    r.seen('a', 900)
    r.fine('a', 920)
    r.fine('a', 1500)
    r.seen('a', 1510)
    expect(r.seen('a', 1530)).toBe('go')
    // well without a break as long as steady: from none again
    r.fine('a', 1540)
    r.fine('a', 2140)
    r.seen('a', 2150)
    expect(r.seen('a', 2170)).toBe('go')
    r.seen('a', 2180)
    expect(r.seen('a', 2200)).toBeUndefined()
  })

  test('given up, it starts again only once it ran well for a while (the human restarted it)', () => {
    const r = new Revivals({ grace: 0, gaps: [0], steady: 100 })
    expect(r.seen('a', 0)).toBe('go')
    expect(r.seen('a', 1)).toBe('give-up')
    r.fine('a', 50)
    expect(r.seen('a', 60)).toBeUndefined()
    r.fine('a', 100)
    r.fine('a', 200)
    expect(r.seen('a', 210)).toBe('go')
  })

  test('remembers whether it was at work when it stopped', () => {
    const r = new Revivals(times)
    r.status('a', 'working')
    r.status('a', 'gone')
    expect(r.wasWorking('a')).toBe(true)
    r.status('a', 'idle')
    expect(r.wasWorking('a')).toBe(false)
  })
})

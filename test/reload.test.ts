// A screen on an old build reloads itself, but never loses what you typed and never cuts a call short.
import { describe, expect, test } from 'bun:test'
import { buildId } from '../shared/build.ts'
import type { UI } from '../src/kernel.ts'
import { keptOf, mayReload } from '../src/reload.ts'

const ui = (p: Partial<UI> = {}) => ({ call: null, dict: null, files: {}, reply: {}, focus: null, viewer: null, settings: null, term: null, ...p }) as UI
/** a focused element: a text box (kept or not) or something else */
const el = (box: 'kept' | 'plain' | null) => ({ closest: () => (box ? { hasAttribute: (a: string) => a === 'data-kept' && box === 'kept' } : null) }) as unknown as Element
const quiet = { holds: 0, sinceTyped: 60_000, focused: null }

describe('which build', () => {
  test('the hashed files the page loads, in any order; the dev server has none', () => {
    const a = '<script type="module" crossorigin src="./assets/index-UnMxSHig.js"></script><link rel="stylesheet" href="./assets/index-Dw1te9LQ.css">'
    const b = '<link rel="stylesheet" href="./assets/index-Dw1te9LQ.css"><script type="module" src="./assets/index-UnMxSHig.js"></script>'
    expect(buildId(a)).toBe(buildId(b))
    expect(buildId(a)).not.toBe(buildId(a.replace('UnMxSHig', 'Q1w2E3r4')))
    expect(buildId('<script type="module" src="/src/main.tsx"></script>')).toBe('')
  })
})

describe('when the screen reloads', () => {
  test('when nothing is going on', () => {
    expect(mayReload(ui(), quiet)).toBe(true)
  })
  test('not on a voice call, nor while talking to type', () => {
    expect(mayReload(ui({ call: 'gtm' }), quiet)).toBe(false)
    expect(mayReload(ui({ dict: 'gtm' }), quiet)).toBe(false)
  })
  test('not mid-word, nor while a file is on its way', () => {
    expect(mayReload(ui(), { ...quiet, sinceTyped: 500 })).toBe(false)
    expect(mayReload(ui(), { ...quiet, holds: 1 })).toBe(false)
  })
  test('in a message box (its text is kept), but not in a box whose text would be lost', () => {
    expect(mayReload(ui(), { ...quiet, focused: el('kept') })).toBe(true)
    expect(mayReload(ui(), { ...quiet, focused: el('plain') })).toBe(false)
    expect(mayReload(ui(), { ...quiet, focused: el(null) })).toBe(true)
  })
})

describe('what comes back after it', () => {
  test('half-typed messages, attached files, the focus and what was open', () => {
    const k = keptOf(
      ui({
        files: { gtm: [{ name: 'a.png', path: '/inbox/a.png' }, { name: 'b.png' }], x: [{ name: 'c.png' }] },
        focus: 'gtm',
        term: 'GTM-1/Agent',
        reply: { gtm: { id: 'm1', who: 'agent', text: 'Tests pass' } },
      }),
      { gtm: 'half a thought', 'GTM-1/Agent': '' },
    )
    expect(k.reply).toEqual({ gtm: { id: 'm1', who: 'agent', text: 'Tests pass' } })
    expect(k.text).toEqual({ gtm: 'half a thought' })
    expect(k.files).toEqual({ gtm: [{ name: 'a.png', path: '/inbox/a.png' }] })
    expect(k.focus).toBe('gtm')
    expect(k.term).toBe('GTM-1/Agent')
    expect(JSON.parse(JSON.stringify(k))).toEqual(k)
  })
})

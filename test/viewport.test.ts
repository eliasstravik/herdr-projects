// The on-screen keyboard: the app sizes itself to what's visible and knows when the keyboard is up.
import { describe, expect, test } from 'bun:test'
import { keyboardWatch } from '../src/viewport.ts'

describe('the on-screen keyboard', () => {
  test('opening it shrinks the visible area a lot; closing it brings the full height back', () => {
    const up = keyboardWatch()
    expect(up(390, 844)).toBe(false)
    expect(up(390, 508)).toBe(true)
    expect(up(390, 844)).toBe(false)
  })
  test("a browser bar coming and going isn't the keyboard", () => {
    const up = keyboardWatch()
    expect(up(390, 664)).toBe(false)
    expect(up(390, 745)).toBe(false)
    expect(up(390, 664)).toBe(false)
  })
  test('turning the phone starts over at the new width', () => {
    const up = keyboardWatch()
    expect(up(390, 844)).toBe(false)
    expect(up(844, 390)).toBe(false)
    expect(up(844, 200)).toBe(true)
    expect(up(390, 844)).toBe(false)
  })
  test('turning the phone with the keyboard up keeps it up, both ways, until it closes', () => {
    const up = keyboardWatch()
    expect(up(390, 844)).toBe(false)
    expect(up(390, 508)).toBe(true)
    expect(up(844, 200)).toBe(true)
    expect(up(390, 508)).toBe(true)
    expect(up(844, 200)).toBe(true)
    expect(up(844, 390)).toBe(false)
    expect(up(844, 200)).toBe(true)
    expect(up(390, 844)).toBe(false)
  })
})

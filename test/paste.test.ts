// Pasting and dropping files into a chat: a screenshot attaches with a sensible name, text stays a paste of text.
import { describe, expect, test } from 'bun:test'
import { dropped, named, pasted, stamp } from '../plugins/chat/paste.ts'

const at = new Date(2026, 9, 25, 18, 46, 5)
const png = (name = 'image.png') => new File([new Uint8Array([137, 80, 78, 71])], name, { type: 'image/png' })
const clip = (files: File[], data: Record<string, string> = {}) => ({
  files: files as unknown as FileList,
  types: [...Object.keys(data), ...(files.length ? ['Files'] : [])],
  getData: (k: string) => data[k] ?? '',
})

describe('paste', () => {
  test('a screenshot from the clipboard gets a name with the time', () => {
    expect(stamp(at)).toBe('2026-10-25-184605')
    expect(pasted(clip([png()]), at).map((f) => f.name)).toEqual(['screenshot-2026-10-25-184605.png'])
    // no name at all (some browsers), and a JPEG
    expect(pasted(clip([new File(['x'], '', { type: 'image/jpeg' })]), at)[0].name).toBe('screenshot-2026-10-25-184605.jpg')
  })

  test('several at once each attach, nameless ones counted, named ones kept', () => {
    const got = pasted(clip([png(), png('plan.png'), png()]), at)
    expect(got.map((f) => f.name)).toEqual(['screenshot-2026-10-25-184605-1.png', 'plan.png', 'screenshot-2026-10-25-184605-2.png'])
    expect(got[0].type).toBe('image/png')
    expect(got[0].size).toBe(4)
  })

  test('text stays text, and a rich copy with a picture of itself pastes its text', () => {
    expect(pasted(clip([], { 'text/plain': 'hello' }), at)).toEqual([])
    expect(pasted(clip([png()], { 'text/html': '<table>…</table>', 'text/plain': 'a\tb' }), at)).toEqual([])
    // a picture copied in the browser brings an <img> of itself and no text: it is still a picture
    expect(pasted(clip([png()], { 'text/html': '<img src="x.png">' }), at)).toHaveLength(1)
    expect(pasted(null, at)).toEqual([])
  })

  test('a dropped file keeps its own name', () => {
    expect(dropped({ files: [png(), png('image.png')] as unknown as FileList }, at).map((f) => f.name)).toEqual(['image.png', 'image.png'])
    expect(named(png('notes.png'), at).name).toBe('notes.png')
  })
})

// Emoji for reactions: one spelling per emoji, skin tones, a search that finds what Slack would, and the most used kept
// on this device, scored like Slack's frequently used.
import { afterEach, describe, expect, test } from 'bun:test'
import data from '../plugins/reactions/emoji-data.ts'
import { EMOJI, fq, frequent, loadEmoji, quickRow, remember, score, search, setTone, tone, withTone } from '../plugins/reactions/emoji.ts'

const store = new Map<string, string>()
const fake = { getItem: (k: string) => store.get(k) ?? null, setItem: (k: string, v: string) => void store.set(k, v) }
const g = globalThis as { localStorage?: unknown }

afterEach(() => {
  store.clear()
  delete g.localStorage
})

describe('emoji', () => {
  test('the picker and the quick row spell an emoji the same way', () => {
    expect(fq('👍️')).toBe('👍')
    expect(fq('✅️')).toBe('✅')
    // a heart needs its mark to show as an emoji
    expect(fq('❤️')).toBe('❤️')
    for (const e of EMOJI) expect(fq(e)).toBe(e)
  })
  test('a skin tone goes on each person or hand', () => {
    expect(withTone('👍', 3)).toBe('👍🏽')
    expect(withTone('👍', 0)).toBe('👍')
    expect(withTone('🕵️‍♀️', 1)).toBe('🕵🏻‍♀️')
    expect(withTone('🧑‍💻', 5)).toBe('🧑🏿‍💻')
  })
  test('the bundled set has every group, most emoji, and nothing it would fetch', async () => {
    const groups = await loadEmoji()
    expect(groups.map((x) => x.name)).toEqual(['Smileys & people', 'Animals & nature', 'Food & drink', 'Activities', 'Travel & places', 'Objects', 'Symbols', 'Flags'])
    const all = groups.flatMap((x) => x.list)
    expect(all.length).toBeGreaterThan(1800)
    for (const e of EMOJI) expect(all.some((x) => x.e === e)).toBe(true)
    expect(all.find((x) => x.e === '👍')?.tone).toBe(true)
    expect(all.find((x) => x.e === '🎉')?.tone).toBe(false)
    expect(JSON.stringify(data)).not.toMatch(/https?:/)
  })
  test('search finds by name, Slack shortcode or tag, best first', async () => {
    const groups = await loadEmoji()
    const top = (q: string) => search(groups, q)[0]?.e
    expect(top('rocket')).toBe('🚀')
    expect(top('heart')).toBe('❤️')
    expect(top('+1')).toBe('👍')
    expect(top(':thumbsup:')).toBe('👍')
    expect(top('tada')).toBe('🎉')
    expect(top('pray')).toBe('🙏')
    expect(top('white check')).toBe('✅')
    expect(search(groups, 'zzqx')).toEqual([])
    expect(search(groups, '  ')).toEqual([])
  })
  test('used emoji and the skin tone are kept on this device', () => {
    // no storage (a private window): nothing kept, nothing breaks, the favourites show
    remember('🚀')
    expect(frequent()).toEqual([])
    expect(quickRow(0)).toEqual(EMOJI)
    expect(tone()).toBe(0)
    g.localStorage = fake
    for (let i = 0; i < 80; i++) remember(String(i))
    expect(frequent().length).toBe(24)
    expect(Object.keys(JSON.parse(store.get('shepherdr:emoji-uses')!)).length).toBe(60)
    setTone(4)
    expect(tone()).toBe(4)
    store.set('shepherdr:emoji-tone', 'nonsense')
    expect(tone()).toBe(0)
    store.set('shepherdr:emoji-uses', '{"👍":{"n":"x"}}')
    expect(frequent()).toEqual([])
  })
  test('frequently used is frecency: recent and often beats once or long ago, like Slack', () => {
    g.localStorage = fake
    const now = Date.UTC(2026, 9, 25)
    const H = 3600_000
    // a habit from last month, a one-off today, a burst this morning
    for (let i = 0; i < 6; i++) remember('🙏', now - 20 * 24 * H - i)
    remember('🚀', now - H)
    for (let i = 0; i < 3; i++) remember('🔥', now - 2 * H - i)
    expect(frequent(now)).toEqual(['🔥', '🙏', '🚀'])
    // three months on, the old habit has faded out
    expect(frequent(now + 90 * 24 * H)).toEqual([])
    expect(score({ n: 1, t: [now] }, now)).toBe(100)
    expect(score({ n: 0, t: [] }, now)).toBe(0)
  })
  test('the quick row: your most used first, the favourites fill the rest in your skin tone', () => {
    g.localStorage = fake
    expect(quickRow(3)).toEqual(['👍🏽', '❤️', '😂', '🙌🏽', '👀', '✅'])
    remember('🚀')
    remember('🚀')
    remember('🙌')
    expect(quickRow(0)).toEqual(['🚀', '🙌', '👍', '❤️', '😂', '👀'])
    // a toned thumbs up from the picker isn't shown twice
    remember('👍🏽')
    expect(quickRow(3).filter((e) => e.startsWith('👍'))).toEqual(['👍🏽'])
    expect(quickRow(3).length).toBe(6)
  })
  test('recents from before are kept as a start', () => {
    g.localStorage = fake
    store.set('shepherdr:emoji-recent', '["🚀","👍"]')
    expect(frequent()).toEqual(['🚀', '👍'])
  })
})

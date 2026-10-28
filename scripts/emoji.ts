// Packs emojibase's English data into plugins/reactions/emoji-data.ts: every emoji the picker offers, with the words a
// search finds it by, grouped like Slack's picker. Run it after updating emojibase-data: bun scripts/emoji.ts
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import data from 'emojibase-data/en/data.json'
import slack from 'emojibase-data/en/shortcodes/iamcal.json'
import { fq, withTone } from '../plugins/reactions/emoji.ts'

/** newer emoji draw as empty boxes on phones a few years old */
const NEWEST = 15

const GROUPS: [name: string, icon: string, groups: number[]][] = [
  ['Smileys & people', '😀', [0, 1]],
  ['Animals & nature', '🐻', [3]],
  ['Food & drink', '🍔', [4]],
  ['Activities', '⚽', [6]],
  ['Travel & places', '🚗', [5]],
  ['Objects', '💡', [7]],
  ['Symbols', '🔣', [8]],
  ['Flags', '🏁', [9]],
]

type E = (typeof data)[number] & { skins?: { emoji: string; tone?: number | number[]; version: number }[] }

/** Slack's shortcodes for it (thumbsup, +1, heart) */
const codes = (e: E) => [(slack[e.hexcode as keyof typeof slack] as string | string[] | undefined) ?? []].flat()

/** its tags, less the words its name or shortcodes already have */
const words = (e: E) => {
  const have = new Set([e.label, ...codes(e)].join(' ').toLowerCase().split(/[\s_]+/))
  return [...new Set((e.tags ?? []).join(' ').toLowerCase().split(/\s+/))].filter((w) => !have.has(w)).join(' ')
}

/** a skin tone can be put on it by the rule in emoji.ts, for all five tones */
const toned = (e: E) => {
  const same = (e.skins ?? []).filter((s) => [s.tone].flat().every((t) => t === [s.tone].flat()[0]) && s.version <= NEWEST)
  return same.length === 5 && same.every((s) => withTone(e.emoji, [s.tone].flat()[0]!) === fq(s.emoji))
}

const out = GROUPS.map(([name, icon, groups]) => {
  const list = (data as E[])
    .filter((e) => groups.includes(e.group!) && e.version <= NEWEST)
    .sort((a, b) => a.order! - b.order!)
    .map((e) => `${fq(e.emoji)}${toned(e) ? '~' : ''}|${e.label}|${codes(e).join(' ')}|${words(e)}`)
  return [name, icon, list.join('\n')]
})

writeFileSync(
  join(import.meta.dir, '..', 'plugins', 'reactions', 'emoji-data.ts'),
  `// Written by scripts/emoji.ts from emojibase-data. Do not edit.\n// Per group: its name, its tab's emoji, and one line per emoji: the emoji (~ when it takes a skin tone) | its name | its Slack shortcodes | more words it's found by\nexport default ${JSON.stringify(out, null, 0)} as [string, string, string][]\n`,
)

// The live source's own rules: chat lines, terminal menus, triggers, settings, project keys.
import { beforeEach, describe, expect, test } from 'bun:test'
import { existsSync, mkdtempSync, readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

process.env.SHEPHERDR_HOME = mkdtempSync(join(tmpdir(), 'shep-test-'))

const { agentLines, excerpt, findQuoted, line, matches, parseReply, parseUserLine, userLines } = await import('../server/live/chatlines.ts')
const { parseMenu } = await import('../server/live/chats.ts')
const { Automations } = await import('../server/live/automations.ts')
const T = await import('../server/live/trigger.ts')
const { CATALOG, Settings, effortFor, setForTask } = await import('../server/live/settings.ts')
const { readBrief } = await import('../shared/brief.ts')
const { Heartbeats, heartbeatFile } = await import('../server/live/heartbeat.ts')
const { coordinatorPrompt, settingsBlock, splitBrief, taskPrompt } = await import('../server/live/prompts.ts')
const { pastTimes, span } = await import('../server/live/estimates.ts')
const { readJson, writeJson } = await import('../server/store.ts')
const { HOST, keyFor, machineName, placeId } = await import('../server/live/places.ts')
const { stripHidden, hidden } = await import('../server/live/agents.ts')
const { ROOT } = await import('../server/root.ts')

describe('chat lines', () => {
  test('a reaction quotes the start of the message and reads back', () => {
    const l = line.react('Elias', '👍', 'PR #52 is ready to merge')
    expect(l).toBe('<reaction from="Elias" emoji="👍" to="PR #52 is ready to merge"/>')
    expect(parseUserLine(l)).toEqual({ kind: 'react', emoji: '👍', to: 'PR #52 is ready to merge' })
    expect(line.unreact('Elias', '👍', 'x')).toBe('<reaction-removed from="Elias" emoji="👍" to="x"/>')
    expect(parseUserLine(line.unreact('Elias', '👍', 'x'))?.kind).toBe('unreact')
  })
  test('any emoji from the full picker makes the same line, skin tones and joined ones too', () => {
    for (const e of ['🚀', '👍🏽', '🧑🏿\u200D💻', '❤️\u200D🔥', '🏳️\u200D🌈', '1️⃣', '🇸🇪']) {
      expect(parseUserLine(line.react('Elias', e, 'Done'))).toEqual({ kind: 'react', emoji: e, to: 'Done' })
      expect(parseUserLine(line.unreactFile('Elias', e, 'a.png', '/x/a.png'))).toEqual({ kind: 'unreact', emoji: e, to: 'a.png', path: '/x/a.png' })
      expect(agentLines(`<react emoji="${e}" to="Go"/>`).reactions).toEqual([{ emoji: e, to: 'Go' }])
      expect(agentLines(`[react ${e} to "Go"]`).reactions).toEqual([{ emoji: e, to: 'Go' }])
    }
  })
  test('long messages are quoted short, and still match', () => {
    const long = 'Seven things. gtm asks whether to rerun CI on main, GTM-15 is ready to merge, HP-201 needs a call on Pinned'
    const q = excerpt(long)
    expect(q.endsWith('…')).toBe(true)
    expect(matches(long, q)).toBe(true)
    expect(matches('Something else entirely', q)).toBe(false)
  })
  test('answers, attachments, artifact sends, words and notes read back', () => {
    const answer = line.answer('Elias', 'Rerun CI on main?', 'Rerun it')
    expect(answer).toBe('<answer from="Elias" question="Rerun CI on main?">Rerun it</answer>')
    expect(parseUserLine(answer)).toEqual({ kind: 'answer', title: 'Rerun CI on main?', answer: 'Rerun it' })
    const attach = line.attach('Elias', 'screenshot.png', '~/inbox/screenshot.png')
    expect(attach).toBe('<attachment from="Elias" name="screenshot.png" path="~/inbox/screenshot.png"/>')
    expect(parseUserLine(attach)).toEqual({ kind: 'attach', name: 'screenshot.png', path: '~/inbox/screenshot.png' })
    // double quotes in a value turn curly, so the tag stays whole
    const art = line.artifact('Weekly "board"', 'wk3n', 2)
    expect(line.artifactSend('Elias', art, 'Pinned colour: Accent')).toBe('<artifact-sent from="Elias" title="Weekly “board”" artifact="wk3n" version="2">Pinned colour: Accent</artifact-sent>')
    expect(parseUserLine(line.artifactSend('Elias', art, 'Pinned colour: Accent'))).toEqual({ kind: 'artifact-send', title: 'Weekly “board”', text: 'Pinned colour: Accent' })
    expect(parseUserLine(line.artifactSay('Elias', line.artifact('Board', 'ab12', 1), 'Bars in <b>blue</b>'))).toEqual({ kind: 'artifact-say', title: 'Board', text: 'Bars in <b>blue</b>' })
    const note = line.note('Elias', line.artifact('Board', 'ab12', 3), '"Send" (button.primary)', 'Bigger')
    expect(note).toBe('<artifact-note from="Elias" title="Board" artifact="ab12" version="3" on="“Send” (button.primary)">Bigger</artifact-note>')
    expect(parseUserLine(note)).toEqual({ kind: 'note', target: '“Send” (button.primary) in "Board"', note: 'Bigger' })
    expect(parseUserLine(line.note('Elias', line.artifact('Board', 'ab12', 3), undefined, 'Bigger'))).toEqual({ kind: 'note', target: '"Board"', note: 'Bigger' })
    expect(parseUserLine('Just a message')).toBeNull()
    expect(parseUserLine('<b>bold</b>')).toBeNull()
  })
  test('the older [square bracket] lines still read back', () => {
    expect(parseUserLine('[Elias reacted 👍 to "PR #52 is ready to merge"]')).toEqual({ kind: 'react', emoji: '👍', to: 'PR #52 is ready to merge' })
    expect(parseUserLine('[Elias removed 👍 from a.png: /x/a.png]')).toEqual({ kind: 'unreact', emoji: '👍', to: 'a.png', path: '/x/a.png' })
    expect(parseUserLine('[Elias answered "Rerun CI on main?": Rerun it]')).toEqual({ kind: 'answer', title: 'Rerun CI on main?', answer: 'Rerun it' })
    expect(parseUserLine('[Elias attached screenshot.png: ~/inbox/screenshot.png]')).toEqual({ kind: 'attach', name: 'screenshot.png', path: '~/inbox/screenshot.png' })
    expect(parseUserLine('[Elias sent from "Board" (artifact wk3n v2): Pinned colour: Accent]')).toEqual({ kind: 'artifact-send', title: 'Board', text: 'Pinned colour: Accent' })
    expect(parseUserLine('[Elias sent from "Board": Pinned colour: Accent]')).toEqual({ kind: 'artifact-send', title: 'Board', text: 'Pinned colour: Accent' })
    expect(parseUserLine('[Elias wrote about "Board" (artifact ab12 v1): Bars in blue]')).toEqual({ kind: 'artifact-say', title: 'Board', text: 'Bars in blue' })
    expect(parseUserLine('[Elias noted on "Send" (button.primary): Bigger]')?.kind).toBe('note')
    expect(parseReply('[Elias replied to "Go": Yes\n[Elias attached a.png: /inbox/a.png]')).toBeNull()
    expect(parseReply('[Elias replied to "Go": Yes]\n[Elias attached a.png: /inbox/a.png]')).toMatchObject({ to: 'Go', text: 'Yes', rest: ['[Elias attached a.png: /inbox/a.png]'] })
  })
  test('a reaction to a file names its path, so two files of one name stay apart', () => {
    const l = line.reactFile('Elias', '👍', 'screenshot.png', '/inbox/a-screenshot.png')
    expect(l).toBe('<reaction from="Elias" emoji="👍" file="screenshot.png" path="/inbox/a-screenshot.png"/>')
    expect(parseUserLine(l)).toEqual({ kind: 'react', emoji: '👍', to: 'screenshot.png', path: '/inbox/a-screenshot.png' })
    expect(parseUserLine(line.unreactFile('Elias', '👍', 'a.png', '/x/a.png'))).toMatchObject({ kind: 'unreact', path: '/x/a.png' })
    // a quoted message with a colon or quotes in it is still a message
    expect(parseUserLine(line.react('Elias', '👍', 'Note: say "done"'))).toEqual({ kind: 'react', emoji: '👍', to: 'Note: say “done”' })
    expect(agentLines('<react emoji="👀" file="shot.png" path="/inbox/b-shot.png"/>\nOn it.').reactions).toEqual([{ emoji: '👀', to: 'shot.png', path: '/inbox/b-shot.png' }])
    expect(agentLines('[react 👀 to shot.png: /inbox/b-shot.png]\nOn it.').reactions).toEqual([{ emoji: '👀', to: 'shot.png', path: '/inbox/b-shot.png' }])
  })
  test("the agent's own reactions come out of its reply", () => {
    const r = agentLines('<react emoji="👀" to="What needs me today?"/>\nLooking now.')
    expect(r.reactions).toEqual([{ emoji: '👀', to: 'What needs me today?' }])
    expect(r.text).toBe('Looking now.')
    // straight quotes an agent copied into its quote don't break it
    expect(agentLines('<react emoji="👍" to="Say "go" when ready"/>').reactions).toEqual([{ emoji: '👍', to: 'Say "go" when ready' }])
    expect(agentLines('[react 👀 to "What needs me today?"]\nLooking now.')).toEqual(r)
  })
  test('a reply quotes the start of the message it answers, and reads back with its files', () => {
    const l = line.reply('Elias', 'Tests pass on the branch. Want me to merge?', 'Yes, merge it')
    expect(l).toBe('<reply from="Elias" to="Tests pass on the branch. Want me to merge?">Yes, merge it</reply>')
    expect(parseReply(l)).toMatchObject({ to: 'Tests pass on the branch. Want me to merge?', text: 'Yes, merge it', rest: [] })
    // words over several lines, with tags and brackets of their own, and files after them
    const a = line.attach('Elias', 'a.png', '/inbox/a.png')
    const multi = `${line.reply('Elias', 'Say "go" when ready', 'Go.\nAlso check [this] and </reply> that')}\n${a}`
    expect(parseReply(multi)).toMatchObject({ to: 'Say “go” when ready', text: 'Go.\nAlso check [this] and </reply> that', rest: [a] })
    // a reply with only a file
    expect(parseReply(`${line.reply('Elias', 'shot.png', '')}\n${line.attach('Elias', 'b.png', '/inbox/b.png')}`)?.text).toBe('')
    expect(parseReply('Just words')).toBeNull()
    expect(parseReply(line.react('Elias', '👍', 'Go'))).toBeNull()
    // an agent's reply line is no reply of yours
    expect(parseReply('<reply to="Go"/>')).toBeNull()
  })
  test('words of a tag that run over several lines stay whole', () => {
    const say = line.artifactSay('Elias', line.artifact('Board', 'ab12', 1), 'Bars in blue.\nAnd wider.')
    expect(userLines(`${say}\n${line.attach('Elias', 'a.png', '/a.png')}`)).toEqual([say, line.attach('Elias', 'a.png', '/a.png')])
    expect(parseUserLine(userLines(say)[0])).toEqual({ kind: 'artifact-say', title: 'Board', text: 'Bars in blue.\nAnd wider.' })
    expect(userLines('<answer from="Elias" question="x">never closed\nhi')).toEqual(['<answer from="Elias" question="x">never closed', 'hi'])
    // words with a closing tag of their own on a line stay inside
    const own = line.artifactSay('Elias', line.artifact('Board', 'ab12', 1), 'Start\n<artifact-comment>Blue</artifact-comment>\nEnd')
    expect(userLines(`${own}\n${line.attach('Elias', 'a.png', '/a.png')}`)).toEqual([own, line.attach('Elias', 'a.png', '/a.png')])
    expect(parseUserLine(userLines(own)[0])).toMatchObject({ text: 'Start\n<artifact-comment>Blue</artifact-comment>\nEnd' })
  })
  test("the agent's reply line comes out of its reply, the first one counts", () => {
    const r = agentLines('<reply to="What needs me today?"/>\n<react emoji="👀" to="Hi"/>\nThree things.')
    expect(r).toEqual({ text: 'Three things.', reactions: [{ emoji: '👀', to: 'Hi' }], reply: { to: 'What needs me today?' } })
    expect(agentLines('<reply file="shot.png" path="/inbox/b-shot.png"/>\nSeen it.').reply).toEqual({ to: 'shot.png', path: '/inbox/b-shot.png' })
    // its words inside the reply's tags
    expect(agentLines('<reply to="Hi">\nThree things.\n</reply>')).toEqual({ text: 'Three things.', reactions: [], reply: { to: 'Hi' } })
    expect(agentLines('<reply to="Hi">Three things.</reply>')).toEqual({ text: 'Three things.', reactions: [], reply: { to: 'Hi' } })
    expect(agentLines('No reply here.').reply).toBeUndefined()
    // a reply line inside its words stays words
    expect(agentLines('Write <reply to="x"/> on a line of its own.').text).toBe('Write <reply to="x"/> on a line of its own.')
    // the older form
    expect(agentLines('[reply to "What needs me today?"]\n[react 👀 to "Hi"]\nThree things.')).toEqual(r)
    expect(agentLines('[reply to shot.png: /inbox/b-shot.png]\nSeen it.').reply).toEqual({ to: 'shot.png', path: '/inbox/b-shot.png' })
    expect(agentLines('Write [reply to "x"] on a line of its own.').text).toBe('Write [reply to "x"] on a line of its own.')
  })
  test('a reply that is only <no-reply>why</no-reply> says nothing; anything more stays words', () => {
    expect(agentLines('<no-reply>SHE-72 merged, nothing to do</no-reply>').text).toBe('')
    expect(agentLines('\n  <no-reply>\n  SHE-72 merged\n</no-reply>\n').text).toBe('')
    expect(agentLines('<no-reply></no-reply>').text).toBe('')
    expect(agentLines('<no-reply/>').text).toBe('')
    expect(agentLines('`<No-Reply>merged</No-Reply>`').text).toBe('')
    expect(agentLines('<reply to="SHE-9 is ready"/>\n<no-reply>already on it</no-reply>').text).toBe('')
    expect(agentLines('[reply to "SHE-9 is ready"]\n<no-reply>already on it</no-reply>').text).toBe('')
    expect(agentLines('<react emoji="👍" to="SHE-50 is done"/>\n<no-reply>a thumbs up</no-reply>')).toEqual({ text: '', reactions: [{ emoji: '👍', to: 'SHE-50 is done' }] })
    expect(agentLines('<no-reply>merged</no-reply>\nSHE-51 can start now.').text).toBe('<no-reply>merged</no-reply>\nSHE-51 can start now.')
    expect(agentLines('<no-reply>a</no-reply> <no-reply>b</no-reply>').text).toBe('<no-reply>a</no-reply> <no-reply>b</no-reply>')
    expect(agentLines('Write <no-reply>why</no-reply> when news needs nothing.').text).toBe('Write <no-reply>why</no-reply> when news needs nothing.')
    expect(agentLines('<no-reply>unclosed').text).toBe('<no-reply>unclosed')
    expect(parseUserLine('<no-reply>x</no-reply>')).toBeNull()
  })
  test('the older markers still say nothing: [[noted]], any [[...]], a lone [noted]', () => {
    expect(agentLines('[[noted]]').text).toBe('')
    expect(agentLines('\n[[noted]]\n').text).toBe('')
    expect(agentLines('[react 👍 to "SHE-50 is done"]\n[[noted]]')).toEqual({ text: '', reactions: [{ emoji: '👍', to: 'SHE-50 is done' }] })
    expect(agentLines('[[noted]]\nSHE-51 can start now.').text).toBe('[[noted]]\nSHE-51 can start now.')
    expect(agentLines('[Noted]').text).toBe('[Noted]')
    expect(agentLines('[[skip]]').text).toBe('')
    expect(agentLines('[noted]').text).toBe('')
    expect(agentLines('[[noted]] and more').text).toBe('[[noted]] and more')
    expect(agentLines('[Elias noted on "Send test event": Shorter]').text).toBe('[Elias noted on "Send test event": Shorter]')
    expect(parseUserLine('[[noted]]')).toBeNull()
  })
  test('a quote finds its message however it was copied: case, curly quotes, markdown, a shortened start', () => {
    const msgs = [
      { id: 'a', text: 'PR #52 is **ready** to merge. Shall I?' },
      { id: 'b', text: 'Something else' },
      { id: 'c', file: { name: 'shot.png', path: '/inbox/c-shot.png' } },
      { id: 'd', text: 'Something else' },
    ]
    expect(findQuoted(msgs, 'PR #52 is ready to merge')?.id).toBe('a')
    expect(findQuoted(msgs, 'pr #52 is ready…')?.id).toBe('a')
    expect(findQuoted(msgs, 'ready to merge. Shall I?')?.id).toBe('a')
    // the latest of two the same
    expect(findQuoted(msgs, 'Something else')?.id).toBe('d')
    expect(findQuoted(msgs, 'shot.png')?.id).toBe('c')
    expect(findQuoted(msgs, 'x', '/inbox/c-shot.png')?.id).toBe('c')
    // a whole reply line quoted back: its words count
    expect(findQuoted(msgs, '[Elias replied to "Go": Something else]')?.id).toBe('d')
    expect(findQuoted(msgs, '<reply from="Elias" to="Go">Something else</reply>')?.id).toBe('d')
    expect(findQuoted(msgs, 'Nothing like it')).toBeUndefined()
    expect(matches('Hello there', 'there')).toBe(false)
  })
  test("Shepherdr's own words never show in the chat", () => {
    expect(hidden('GTM-15 was merged', 'news', { task: 'GTM-15', event: 'merged' })).toBe('<shepherdr><news task="GTM-15" event="merged">GTM-15 was merged</news></shepherdr>')
    expect(hidden('Go on', 'automation', { name: 'Fresh "board"' })).toBe('<shepherdr><automation name="Fresh “board”">Go on</automation></shepherdr>')
    expect(hidden('only instructions')).toBe('<shepherdr>only instructions</shepherdr>')
    expect(stripHidden(`${hidden('GTM-15 is ready', 'news')}\nhello`)).toBe('hello')
    expect(stripHidden(`${hidden('a > b', 'automation', { name: 'x > y' })}hello`)).toBe('hello')
    expect(stripHidden(`<shepherdr>[news] GTM-15 is ready</shepherdr>\nhello`)).toBe('hello')
    expect(stripHidden(hidden('only instructions'))).toBe('')
  })
})

describe('terminal menus', () => {
  test("Claude Code's permission menu becomes a question card", () => {
    const screen = [
      '╭──────────────────────────────────────────╮',
      '│ Bash command                             │',
      '│                                          │',
      '│   rm -rf template/skills/old             │',
      '│   Remove the old skills                  │',
      '│                                          │',
      '│ Do you want to proceed?                  │',
      '│ ❯ 1. Yes                                 │',
      "│   2. Yes, and don't ask again for rm     │",
      '│   3. No, and tell Claude what to do (esc)│',
      '╰──────────────────────────────────────────╯',
    ].join('\n')
    const m = parseMenu(screen)!
    expect(m.choices).toEqual(['Yes', "Yes, and don't ask again for rm", 'No, and tell Claude what to do'])
    expect(m.cursor).toBe(0)
    expect(m.title.startsWith('Allow: ')).toBe(true)
  })
  test("Codex's approval with a wrapped choice and the command", () => {
    const screen = [
      '  Would you like to run the following command?',
      '  Reason: May I run the tests outside the sandbox?',
      '  $ npm test',
      '› 1. Yes, proceed (y)',
      "  2. Yes, and don't ask again for commands that start with `npm",
      "     test` (p)",
      '  3. No, and tell Codex what to do differently (esc)',
      '  Press enter to confirm or esc to cancel',
    ].join('\n')
    const m = parseMenu(screen)!
    expect(m.title).toBe('Allow: npm test?')
    expect(m.choices).toHaveLength(3)
    expect(m.choices[1]).toBe("Yes, and don't ask again for commands that start with `npm test`")
  })
  test("Claude Code's own question: the whole question, and each choice's description apart from its label", () => {
    const screen = [
      '────────────────────────────────────────────────────────────────────────────────',
      ' ☐ Scope',
      '',
      '│ Should the rebuilt workflow also read the founders own LinkedIn profiles and',
      '│ posts, or only the company page as before?',
      '',
      '❯ 1. Company page only',
      '     Same as before: just the company page, nothing about the founders',
      '     themselves',
      '  2. Company page and founders',
      '     Also read each founder profile and recent posts',
      '  3. Type something.',
      '────────────────────────────────────────────────────────────────────────────────',
      '  4. Chat about this',
      '',
      'Enter to select · ↑/↓ to navigate · Esc to cancel',
    ].join('\n')
    const m = parseMenu(screen)!
    expect(m.title).toBe('Should the rebuilt workflow also read the founders own LinkedIn profiles and posts, or only the company page as before?')
    expect(m.choices).toEqual(['Company page only', 'Company page and founders', 'Type something.'])
    expect(m.descriptions).toEqual(['Same as before: just the company page, nothing about the founders themselves', 'Also read each founder profile and recent posts', undefined])
  })
  test('a screen without a menu has none', () => {
    expect(parseMenu('❯ Try "fix lint errors"\n  ⏵⏵ bypass permissions on')).toBeNull()
  })
})

describe('triggers', () => {
  const Z = 'Europe/Stockholm'
  // Wednesday 28 Oct 2026, 10:40 in Stockholm (CET, UTC+1)
  const now = Date.UTC(2026, 9, 28, 9, 40)
  const rule = (x: string) => T.timeRule(x)
  test('a time rule is kept on one line, counting from 2026-01-01 without its own DTSTART; nonsense says how to write one', () => {
    expect(rule('FREQ=MINUTELY;INTERVAL=30')).toBe('DTSTART:20260101T000000 RRULE:FREQ=MINUTELY;INTERVAL=30')
    expect(rule('RRULE:FREQ=DAILY;BYHOUR=9\nEXDATE:20261224T090000')).toBe('DTSTART:20260101T000000 RRULE:FREQ=DAILY;BYHOUR=9 EXDATE:20261224T090000')
    expect(rule('once 2026-11-02 9:00')).toBe('once 2026-11-02 09:00')
    expect(rule('off')).toBe('off')
    expect(() => rule('sometimes')).toThrow('iCalendar RRULE')
    // plain words are the coordinator's to turn into a rule: rrule's reader would drop "from 8 to 18" without a word
    expect(() => rule('every 30 minutes from 8 to 18')).toThrow('turn the human')
    expect(() => rule('FREQ=SECONDLY')).toThrow('Once a minute at most')
  })
  test("in words, in the time zone's own times, with the next runs", () => {
    const words = (x: string) => T.timeWords(rule(x), Z, now)
    expect(words('FREQ=MINUTELY;INTERVAL=30;BYHOUR=8,9,10,11,12,13,14,15,16,17,18,19,20,21;BYDAY=MO,TU,WE,TH,FR')).toBe('Every 30 minutes, 8:00-22:00, Monday to Friday (Stockholm)')
    expect(words('FREQ=MINUTELY;INTERVAL=90')).toBe('Every 90 minutes (Stockholm)')
    expect(words('FREQ=WEEKLY;INTERVAL=2;BYDAY=FR;BYHOUR=16;BYMINUTE=0')).toBe('Every other week, Friday at 16:00 (Stockholm)')
    expect(words('FREQ=MONTHLY;BYDAY=-1FR;BYHOUR=9;BYMINUTE=0')).toBe('The last Friday of the month at 9:00 (Stockholm)')
    expect(words('FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1;BYHOUR=17;BYMINUTE=0')).toBe('The last weekday of the month at 17:00 (Stockholm)')
    expect(words('DTSTART:20261201T000000 RRULE:FREQ=DAILY;BYHOUR=9;BYMINUTE=0;UNTIL=20261231T000000 EXDATE:20261224T090000')).toBe('Every day at 9:00, from 1 Dec 2026, until 31 Dec 2026, except 24 Dec (Stockholm)')
    expect(words('once 2026-11-02 9:00')).toBe('Once, Monday 2 Nov 2026 at 9:00 (Stockholm)')
    // 10:40 in Stockholm: the next half hour is 11:00 there, not UTC's
    expect(T.nextWords(rule('FREQ=MINUTELY;INTERVAL=30'), Z, now)).toEqual(['Today 11:00', 'Today 11:30', 'Today 12:00'])
    expect(T.nextRun(rule('FREQ=MINUTELY;INTERVAL=30'), Z, now)).toBe(Date.UTC(2026, 9, 28, 10, 0))
    expect(T.nextWords(rule('FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR;BYHOUR=9;BYMINUTE=0'), Z, now)).toEqual(['Tomorrow 9:00', 'Fri 30 Oct 9:00', 'Mon 2 Nov 9:00'])
    expect(T.nextWords(rule('FREQ=MONTHLY;BYDAY=-1FR;BYHOUR=9;BYMINUTE=0'), Z, now)).toEqual(['Fri 30 Oct 9:00', 'Fri 27 Nov 9:00', 'Fri 25 Dec 9:00'])
    // a once in the past is over
    expect(T.nextRuns(rule('once 2026-10-01 9:00'), Z, now)).toEqual([])
    // every so many minutes on some days only: the days are kept here, as rrule gets them wrong or loops for ever
    const workdays = rule('FREQ=MINUTELY;INTERVAL=30;BYHOUR=8,9,10,11,12,13,14,15,16,17;BYDAY=MO,TU,WE,TH,FR')
    expect(T.nextRuns(workdays, Z, Date.UTC(2026, 9, 26, 16, 0), 3)).toEqual([Date.UTC(2026, 9, 26, 16, 30), Date.UTC(2026, 9, 27, 7, 0), Date.UTC(2026, 9, 27, 7, 30)])
    expect(T.nextRuns(workdays, Z, Date.UTC(2026, 9, 30, 17, 0), 1)).toEqual([Date.UTC(2026, 10, 2, 7, 0)]) // Friday evening: Monday 8:00
    expect(T.nextRuns(rule('FREQ=MINUTELY;INTERVAL=30;BYHOUR=10,11;BYDAY=SA,SU'), Z, now, 1)).toEqual([Date.UTC(2026, 9, 31, 9, 0)])
    expect(T.nextRuns(rule('DTSTART:20261025T000000 RRULE:FREQ=MINUTELY;INTERVAL=30;BYHOUR=8;BYDAY=MO'), Z, now, 1)).toEqual([Date.UTC(2026, 10, 2, 7, 0)])
    expect(T.nextRuns(rule('FREQ=MINUTELY;INTERVAL=30;BYHOUR=8;BYMONTHDAY=15'), Z, now, 3)).toEqual([Date.UTC(2026, 10, 15, 7, 0), Date.UTC(2026, 10, 15, 7, 30), Date.UTC(2026, 11, 15, 7, 0)])
    expect(() => rule('FREQ=MINUTELY;INTERVAL=30;BYDAY=MO;COUNT=5')).toThrow('UNTIL')
    // what rrule's own words leave out is said too
    expect(words('FREQ=HOURLY;BYMINUTE=0,30;BYSETPOS=-1')).toContain("only the last of each hour's times")
  })
  test('event rules: all news, none, or events with filters; matched by space, branch and title', () => {
    expect(T.eventRules('all news')).toEqual([{ event: 'news' }])
    expect(T.eventRules('none')).toEqual([])
    const r = T.eventRules('ready, merged branch=release/* title="fix it"')
    expect(r).toEqual([{ event: 'ready' }, { event: 'merged', branch: 'release/*', title: 'fix it' }])
    expect(T.eventsText(r)).toBe('ready, merged branch=release/* title="fix it"')
    expect(T.eventWords(T.eventRules('needs you or main red space=gtm'))).toBe('When a task needs you or main goes red in gtm')
    // a quoted title keeps its commas and its "and"
    expect(T.eventRules('merged title="salt and pepper, please"')).toEqual([{ event: 'merged', title: 'salt and pepper, please' }])
    expect(T.eventRules(T.eventsText(T.eventRules('merged title="salt,pepper"')))).toEqual([{ event: 'merged', title: 'salt,pepper' }])
    expect(() => T.eventRules('lunch')).toThrow('No event')
    expect(T.matches(r, { event: 'merged', branch: 'release/2.1', title: 'Fix it now' })).toBe(true)
    expect(T.matches(r, { event: 'merged', branch: 'work/1-x', title: 'Fix it now' })).toBe(false)
    expect(T.matches(T.eventRules('all news'), { event: 'health' })).toBe(true)
    expect(T.matches([], { event: 'ready' })).toBe(false)
  })
  test('old schedules become time rules', () => {
    expect(T.fromSchedule('every 30m')).toBe(rule('FREQ=MINUTELY;INTERVAL=30'))
    expect(T.fromSchedule('weekdays 9:00')).toBe(rule('FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR;BYHOUR=9;BYMINUTE=0'))
    expect(T.fromSchedule('mon,fri 16:00')).toBe(rule('FREQ=WEEKLY;BYDAY=MO,FR;BYHOUR=16;BYMINUTE=0'))
    expect(T.fromSchedule('cron "0 9 * * 1-5"')).toBe(rule('FREQ=DAILY;BYMINUTE=0;BYHOUR=9;BYDAY=MO,TU,WE,TH,FR'))
    expect(T.fromSchedule('once 2026-11-02 9:00')).toBe('once 2026-11-02 09:00')
    expect(T.fromSchedule('whenever')).toBeUndefined()
  })
})

describe('automations', () => {
  const Z = 'Europe/Stockholm'
  const at = (s: string) => new Date(s).getTime()
  // each test starts with none saved: one that failed leaves none behind for the next
  beforeEach(() => writeJson('automations.json', []))
  test('due on its time rule, in the time zone: once per run', () => {
    const a = new Automations()
    const x = a.add({ place: 'p:x:due', name: 'Morning', trigger: { time: T.timeRule('FREQ=DAILY;BYHOUR=9;BYMINUTE=0') }, task: { title: 'Look around' } }, at('2026-10-26T06:00:00Z'))
    // 9:00 in Stockholm is 8:00 UTC
    expect(a.dueNow(Z, at('2026-10-26T07:59:00Z'))).toEqual([])
    expect(a.dueNow(Z, at('2026-10-26T08:00:00Z'))).toEqual([x])
    a.fired(x, { task: 'G-1' }, at('2026-10-26T08:00:00Z'), true)
    expect(a.dueNow(Z, at('2026-10-26T08:00:30Z'))).toEqual([])
    expect(a.dueNow(Z, at('2026-10-27T08:00:00Z'))).toEqual([x])
    a.remove('p:x:due', 'Morning')
    // a run on an event just after its time takes no run of its time rule's place
    const y = a.add({ place: 'p:x:due', name: 'Both', trigger: { time: T.timeRule('once 2026-10-28 9:00'), on: [{ event: 'merged' }] }, task: { title: 'Both' } }, at('2026-10-27T00:00:00Z'))
    a.fired(y, { task: 'G-2' }, at('2026-10-28T08:00:01Z'))
    expect(a.dueNow(Z, at('2026-10-28T08:00:30Z'))).toEqual([y])
    a.remove('p:x:due', 'Both')
  })
  test('what Tasks lists at the bottom: its trigger in words and next runs, paused ones too', () => {
    const a = new Automations()
    a.add({ place: 'p:x:gtm', name: 'Flaky tests', trigger: { time: T.timeRule('FREQ=DAILY;BYHOUR=7;BYMINUTE=0') }, task: { title: 'Rerun the flaky test suite', desc: 'Three times.' } })
    a.add({ place: 'p:x:gtm', name: 'Changelog', trigger: { on: [{ event: 'merged' }] }, task: { title: 'Update the changelog' } })
    a.pause('p:x:gtm', 'Changelog', false)
    const now = at('2026-10-28T09:40:00Z')
    expect(a.view('p:x:gtm', Z, now)).toEqual([
      { name: 'Flaky tests', when: 'Every day at 7:00 (Stockholm)', what: 'Rerun the flaky test suite', brief: 'Three times.', on: true, time: 'DTSTART:20260101T000000 RRULE:FREQ=DAILY;BYHOUR=7;BYMINUTE=0', next: ['Tomorrow 7:00', 'Fri 30 Oct 7:00', 'Sat 31 Oct 7:00'], runs: [] },
      { name: 'Changelog', when: 'When a task is merged', what: 'Update the changelog', on: false, events: 'merged', next: [], runs: [] },
    ])
    expect(() => a.add({ place: 'p:x:gtm', name: 'x', trigger: {}, task: { title: 'y' } })).toThrow('--when')
    expect(() => a.add({ place: 'p:x:gtm', name: 'x', trigger: { time: T.timeRule('FREQ=DAILY') }, task: { title: '' } })).toThrow('--task')
  })
  test('events: its own space unless a rule names another, with filters', () => {
    const a = new Automations()
    const x = a.add({ place: 'p:x:shop', name: 'Release notes', trigger: { on: T.eventRules('merged branch=release/*') }, task: { title: 'Write release notes' } })
    expect(a.on({ event: 'merged', branch: 'release/2' }, 'p:x:shop')).toEqual([x])
    expect(a.on({ event: 'merged', branch: 'work/2-x' }, 'p:x:shop')).toEqual([])
    expect(a.on({ event: 'merged', branch: 'release/2' }, 'p:x:other')).toEqual([])
    const y = a.add({ place: 'p:x:shop', name: 'Watch gtm', trigger: { on: T.eventRules('failed space=gtm') }, task: { title: 'Look at gtm' } })
    expect(a.on({ event: 'failed' }, 'p:x:gtm')).toEqual([y])
    // the machine struggling: whatever space it is told in
    const z = a.add({ place: 'p:x:shop', name: 'Clean up', trigger: { on: T.eventRules('health') }, task: { title: 'Free some disk' } })
    expect(a.on({ event: 'health' }, 'p:x:')).toEqual([z])
  })
  test('once runs at its time, then turns itself off; its runs are kept, newest first', () => {
    const a = new Automations()
    const x = a.add({ place: 'p:x:once', name: 'Once', trigger: { time: T.timeRule('once 2026-11-02 9:00') }, task: { title: 'One go' } }, at('2026-10-28T00:00:00Z'))
    expect(a.dueNow(Z, at('2026-11-02T07:59:00Z'))).toEqual([])
    expect(a.dueNow(Z, at('2026-11-02T10:00:00Z'))).toEqual([x]) // missed while down: it runs once it can
    a.fired(x, { task: 'G-1' }, Date.now(), true)
    expect(x.on).toBe(false)
    a.fired(x, { error: 'No project' })
    expect(a.view('p:x:once', Z)[0].runs.map((r) => r.task ?? r.error)).toEqual(['No project', 'G-1'])
    // made again under its name: its runs stay
    a.add({ place: 'p:x:once', name: 'Once', trigger: { time: T.timeRule('FREQ=DAILY;BYHOUR=9') }, task: { title: 'Every day' } })
    expect(a.view('p:x:once', Z)[0].runs).toHaveLength(2)
  })
  test('automations from before triggers become time and event rules once; unreadable ones are dropped', () => {
    writeJson('automations.json', [
      { id: 'a', place: 'p:x:old', name: 'Daily', schedule: 'daily 9:00', task: { title: 'Daily' }, on: true },
      { id: 'b', place: 'p:x:old', name: 'On merge', when: 'merged', task: { title: 'Changelog' }, on: true },
      { id: 'c', place: 'p:x:old', name: 'Odd', schedule: 'sometimes', task: { title: 'Odd' }, on: true },
      { id: 'd', place: 'p:x:old', name: 'Missed', schedule: 'once 2026-10-27 9:00', task: { title: 'Missed' }, on: true },
    ])
    const a = new Automations()
    expect(a.list.map((x) => [x.name, x.trigger])).toEqual([
      ['Daily', { time: T.timeRule('FREQ=DAILY;BYHOUR=9;BYMINUTE=0') }],
      ['On merge', { on: [{ event: 'merged' }] }],
      ['Missed', { time: 'once 2026-10-27 09:00' }],
    ])
    // a once missed while Shepherdr was down still runs; a daily one counts from now
    expect(a.dueNow(Z, at('2026-10-28T10:00:00Z')).map((x) => x.name)).toEqual(['Missed'])
    expect(readJson<unknown[]>('automations.json', []).some((x) => 'schedule' in (x as object))).toBe(false)
    writeJson('automations.json', [])
  })
})

describe('heartbeat', () => {
  const min = 60_000
  const Z = 'Europe/Stockholm'
  const every30 = T.timeRule('FREQ=MINUTELY;INTERVAL=30')
  test('on its time rule after Autopilot went on, in the time zone; off: none; outside its hours: none', () => {
    const h = new Heartbeats()
    const t0 = Date.UTC(2026, 9, 26, 11, 0) // 12:00 in Stockholm, a Monday
    h.switched('p:x:hb', true, t0)
    expect(h.due('p:x:hb', every30, Z, t0 + 29 * min)).toBe(false)
    expect(h.due('p:x:hb', every30, Z, t0 + 30 * min)).toBe(true)
    expect(h.due('p:x:hb', 'off', Z, t0 + 90 * min)).toBe(false)
    // only 8:00-10:00: the next is tomorrow morning
    expect(h.due('p:x:hb', T.timeRule('FREQ=MINUTELY;INTERVAL=30;BYHOUR=8,9'), Z, t0 + 30 * min)).toBe(false)
    expect(h.next('p:x:hb', T.timeRule('FREQ=MINUTELY;INTERVAL=30;BYHOUR=8,9'), Z)).toBe(Date.UTC(2026, 9, 27, 7, 0))
    h.beat('p:x:hb', 'space', t0 + 30 * min)
    expect(h.due('p:x:hb', every30, Z, t0 + 59 * min)).toBe(false)
    expect(h.due('p:x:hb', every30, Z, t0 + 60 * min)).toBe(true)
    // one first seen (on Autopilot before a restart) starts counting then
    expect(h.due('p:x:new', every30, Z, t0)).toBe(false)
    expect(h.due('p:x:new', every30, Z, t0 + 30 * min)).toBe(true)
  })
  test('the prompt: its HEARTBEAT.md and the quiet marker; an empty checklist is skipped', () => {
    const h = new Heartbeats()
    const text = h.beat('p:x:news', 'space')!
    // the checklist itself, not its path: the coordinator may run on another machine
    expect(text).toContain('Your HEARTBEAT.md:\n# Heartbeat')
    expect(text).toContain('- Tasks stuck or stalled')
    expect(text).not.toContain(heartbeatFile('p:x:news'))
    expect(text).not.toContain('News')
    expect(text).toEndWith('reply with only <no-reply>heartbeat: nothing to do</no-reply>.')
    // the first beat wrote the starting checklist, for the coordinator to read and edit
    expect(readFileSync(heartbeatFile('p:x:news'), 'utf8')).toContain('Tasks stuck or stalled')
    h.write('p:x:news', '')
    expect(h.beat('p:x:news', 'space')).toBeUndefined()
  })
  test('Settings: an idle browser closes after 10 min; a space or machine sets its own, or off', () => {
    const s = new Settings()
    expect(s.get('browser.idleClose')).toBe('10 min')
    expect(s.idleClose()).toBe(600_000)
    expect(s.set('browser.idleClose', '30').ok).toBe(true)
    expect(s.get('browser.idleClose')).toBe('30 min')
    expect(s.set('browser.idleClose', 'off', 'machine').ok).toBe(true)
    expect(s.idleClose()).toBeNull()
    expect(s.idleClose('all')).toBe(1_800_000)
    expect(s.set('browser.idleClose', '2 min', { space: 'games' }).ok).toBe(true)
    expect(s.idleClose({ space: 'games' })).toBe(120_000)
    expect(s.set('browser.idleClose', '0').ok).toBe(false)
    expect(s.set('browser.idleClose', 'soon').error).toBe('Write a number from 1 to 1440 (whole), in min, or off.')
    s.set('browser.idleClose', null, { space: 'games' })
    s.set('browser.idleClose', null, 'machine')
    s.set('browser.idleClose', null)
  })

  test('news its event rules let wait comes with the next heartbeat, once', () => {
    const h = new Heartbeats()
    h.hold('p:x:held', 'SHE-1 is ready to merge')
    expect(h.held('p:x:held')).toEqual(['SHE-1 is ready to merge'])
    const text = h.beat('p:x:held', 'space')!
    expect(text).toContain('News since the last heartbeat')
    expect(text).toContain('- SHE-1 is ready to merge')
    expect(h.held('p:x:held')).toEqual([])
    // an empty checklist still brings the news
    h.write('p:x:held', '')
    h.hold('p:x:held', 'SHE-2 needs you')
    expect(h.beat('p:x:held', 'space')).toContain('- SHE-2 needs you')
  })
  test('Settings: every 30 minutes, all news; a rule or events it can not read are refused', () => {
    const s = new Settings()
    expect(s.heartbeat()).toEqual({ time: every30, on: [{ event: 'news' }], zone: T.OWN_ZONE })
    expect(s.set('heartbeat.when', 'FREQ=MINUTELY;INTERVAL=30;BYHOUR=8,9,10;BYDAY=MO,TU,WE,TH,FR').ok).toBe(true)
    expect(s.get('heartbeat.when')).toBe('DTSTART:20260101T000000 RRULE:FREQ=MINUTELY;INTERVAL=30;BYHOUR=8,9,10;BYDAY=MO,TU,WE,TH,FR')
    expect(s.set('heartbeat.when', 'nights').error).toContain('iCalendar RRULE')
    expect(s.set('heartbeat.on', 'needs you, failed').ok).toBe(true)
    expect(s.get('heartbeat.on')).toBe('needs-you, failed')
    expect(s.set('heartbeat.on', 'lunch').error).toContain('No event')
    expect(s.set('time.zone', 'Europe/Stockholm').ok).toBe(true)
    expect(s.heartbeat().zone).toBe('Europe/Stockholm')
    expect(s.set('time.zone', 'Mars/Olympus').ok).toBe(false)
    // the old interval and quiet hours are gone
    for (const k of ['heartbeat.every', 'heartbeat.quiet']) expect(s.set(k, '30').ok).toBe(false)
    for (const k of ['heartbeat.when', 'heartbeat.on', 'time.zone']) s.set(k, null)
  })
  test("Settings → Autopilot shows the rule in words, its next runs and presets, never the rule; shepherdr show too", () => {
    const s = new Settings()
    s.set('time.zone', 'Europe/Stockholm')
    const page = s.sections([], [], null).find((x) => x.name === 'Autopilot' && x.level === 'all')!
    const when = page.rows.find((r) => r.key === 'heartbeat.when')!
    expect(when.trigger!.words).toBe('Every 30 minutes (Stockholm)')
    expect(when.trigger!.next).toHaveLength(3)
    expect(when.trigger!.presets.map((p) => p.label)).toEqual(['15 min', '30 min', '1 hour', 'Workdays'])
    expect(page.rows.find((r) => r.key === 'heartbeat.on')!.trigger!.words).toBe('All news')
    expect(s.lines()).toContain('Every 30 minutes (Stockholm); next ')
    s.set('time.zone', null)
  })
})

describe('settings', () => {
  test('only app settings are left, set Everywhere; none belongs to a task', () => {
    const s = new Settings()
    expect(s.get('task.agent')).toBe('Claude Code')
    expect(s.get('coordinator.agent')).toBe('Claude Code')
    expect(s.get('brief.review')).toBe('on')
    expect(s.get('brief.merge')).toBe('on its own')
    // Schedule, optional agent overrides and the shared runtime permission/usage boundary.
    const pilot = CATALOG.find((x) => x.name === 'Autopilot')!.rows.map((r) => r.key)
    for (const key of ['heartbeat.when', 'heartbeat.on', 'autopilot.agent', 'autopilot.access', 'autopilot.traffic', 'autopilot.tokens', 'autopilot.runtime', 'autopilot.parallel']) expect(pilot).toContain(key)
    // what the model judges for itself is no setting: tasks a day, retries, time per task, when to hand back, tasks at once
    for (const k of ['autopilot.tasksPerDay', 'autopilot.hours', 'autopilot.when', 'autopilot.atOnce', 'autopilot.retries', 'autopilot.pauseAfter', 'autopilot.brakeAfter', 'brief.waitChecks', 'machine.limitBy']) expect(s.set(k, 'any').ok).toBe(false)
    for (const k of ['check.command', 'merge.by', 'plan.approve', 'worker.agent', 'limit.tokens', 'agent']) expect(s.set(k, 'x').ok).toBe(false)
  })
  test('a value that makes no sense is refused with the reason', () => {
    const s = new Settings()
    expect(s.set('tasks.archiveAfter', 'lots').error).toBe('Write a number from 1 to 3650 (whole), in days, or off.')
    expect(s.set('task.agent', 'Nonsense').ok).toBe(false)
    // review is on or off; its rounds and what must be fixed are settings of their own
    expect(s.set('brief.review', '2 rounds').ok).toBe(false)
    expect(s.set('review.mustFix', 'serious').ok).toBe(false)
    expect(s.set('review.rounds', '0').ok).toBe(false)
    expect(s.set('review.rounds', '5').ok).toBe(true)
    expect(s.get('review.rounds')).toBe('5 rounds')
    s.set('review.rounds', null)
    expect(s.set('brief.review', 'off').ok).toBe(true)
    expect(s.get('brief.review')).toBe('off')
    s.set('brief.review', null)
    // counts take any number in range, kept with their unit
    expect(s.set('tasks.archiveAfter', '45').ok).toBe(true)
    expect(s.get('tasks.archiveAfter')).toBe('45 days')
    s.set('tasks.archiveAfter', null)
    expect(s.set('task.effort', 'extreme').ok).toBe(false)
    expect(s.set('watcher.agent', 'Claude Code').error).toBe('No setting watcher.agent')
    // a reviewer needs a headless run review.sh knows
    expect(s.set('reviewer.agent', 'Qwen').error).toBe('Pick one of: Codex · Claude Code · pi.')
    // a model or effort level the CLI (and its model) doesn't take, with what it does
    expect(s.set('coordinator.model', 'opus 9').error).toBe('Claude Code has no model opus 9. Pick one of: default · Fable 5.1 · Opus 5.5 · Sonnet 5.5 · Haiku 5.5.')
    expect(s.set('coordinator.model', 'Opus 5.5').ok).toBe(true)
    expect(s.set('coordinator.effort', 'ultra').error).toBe('Opus 5.5 takes default · low · medium · high · xhigh · max.')
    expect(s.set('reviewer.effort', 'ultra').ok).toBe(true)
    expect(s.set('reviewer.model', 'claude-opus-5-5').ok).toBe(false)
    for (const k of ['coordinator.model', 'reviewer.effort']) s.set(k, null)
    // any agent herdr can start may be chosen, by its name or its kind; it is kept by its name
    expect(s.set('task.agent', 'Qwen').ok).toBe(true)
    expect(s.set('coordinator.agent', 'codex').ok).toBe(true)
    expect(s.get('coordinator.agent')).toBe('Codex')
    s.set('task.agent', null)
    s.set('coordinator.agent', null)
  })
  test('tasks and coordinators each start on their own CLI, model and effort; default is the CLI\'s own', () => {
    const s = new Settings()
    expect(s.harness('task')).toEqual({ kind: 'claude', model: undefined, effort: undefined })
    s.set('coordinator.agent', 'pi')
    s.set('coordinator.model', 'openai/gpt-5.5')
    s.set('coordinator.effort', 'high')
    expect(s.harness('coordinator')).toEqual({ kind: 'pi', model: 'openai/gpt-5.5', effort: 'high' })
    expect(s.harness('task').kind).toBe('claude')
    for (const k of ['coordinator.agent', 'coordinator.model', 'coordinator.effort']) s.set(k, null)
  })
  test("a task's change is reviewed by Codex on its own model and effort, unless Settings say another", () => {
    const s = new Settings()
    expect(s.harness('reviewer')).toEqual({ kind: 'codex', model: undefined, effort: undefined })
    expect(s.set('reviewer.agent', 'claude code').ok).toBe(true)
    expect(s.get('reviewer.agent')).toBe('Claude Code')
    s.set('reviewer.effort', 'high')
    expect(s.harness('reviewer')).toEqual({ kind: 'claude', model: undefined, effort: 'high' })
    expect(s.sections([], [], null).find((x) => x.name === 'How tasks work')!.note).toContain("- Reviewer: a headless Claude Code or a Claude Code subagent (its default model, high effort). (your setting for every machine)")
    for (const k of ['reviewer.agent', 'reviewer.effort']) s.set(k, null)
  })
  test("each CLI and model takes its own levels: one it lacks goes in as the nearest it has", () => {
    expect(effortFor('claude', 'lowest')).toBe('low')
    expect(effortFor('claude', 'Max')).toBe('max')
    expect(effortFor('claude', 'ultra', 'Opus 5.5')).toBe('max')
    expect(effortFor('codex', 'ultra', 'gpt-6-sol')).toBe('ultra')
    expect(effortFor('codex', 'ultra', 'gpt-6-luna')).toBe('max')
    expect(effortFor('codex', 'minimal')).toBe('low')
    expect(effortFor('pi', 'lowest')).toBe('minimal')
    expect(effortFor('pi', 'default')).toBeUndefined()
    expect(effortFor('gemini', 'high')).toBeUndefined()
  })
  test('Settings offer each role the models and levels of its own CLI, showing a stored one that no longer fits as what it starts with', () => {
    const s = new Settings()
    // the Reviewer's rows sit in How tasks work (its Settings block), the rest in Agents
    const rows = () => s.sections([], [], null).filter((x) => x.name === 'Agents' || x.name === 'How tasks work').flatMap((x) => x.rows)
    const row = (group: string, label: string) => rows().find((r) => r.group === group && r.label === label)!
    expect(row('Tasks', 'Model').options).toEqual(['default', 'Fable 5.1', 'Opus 5.5', 'Sonnet 5.5', 'Haiku 5.5'])
    expect(row('Tasks', 'Effort').options).toEqual(['default', 'low', 'medium', 'high', 'xhigh', 'max'])
    expect(row('Reviewer', 'Effort').options).toContain('ultra')
    // Opus on Codex: Codex has no such model, so it starts on its own; ultra on Claude Code is its max
    s.set('task.model', 'Opus 5.5')
    s.set('task.agent', 'Codex')
    expect(row('Tasks', 'Model').value).toBe('default')
    s.set('task.effort', 'ultra')
    s.set('task.agent', 'Claude Code')
    expect(row('Tasks', 'Model').value).toBe('Opus 5.5')
    expect(row('Tasks', 'Effort').value).toBe('max')
    expect(s.harness('task')).toEqual({ kind: 'claude', model: 'claude-opus-5-5', effort: 'max' })
    // a CLI Shepherdr passes neither to picks its own; the backup has its own model and effort
    s.set('task.agent', 'Qwen')
    expect(row('Tasks', 'Model')).toMatchObject({ value: "Qwen's own", fixed: true })
    expect(row('Backup', 'Model').options![1]).toBe('GPT-6.1-Sol')
    for (const k of ['task.agent', 'task.model', 'task.effort']) s.set(k, null)
  })
  test("Autopilot's old limits and wake picks leave the saved file as soon as it is read", () => {
    writeJson('settings.json', { everywhere: { 'autopilot.tasksPerDay': '3', 'wake.merged': 'now', 'heartbeat.every': '45 min' }, machine: { 'wake.health': 'next heartbeat' }, projects: { gtm: { 'wake.added': 'now', 'heartbeat.quiet': '22:00-07:00' } } })
    const s = new Settings()
    // the old interval and quiet hours are a time rule now: every 45 minutes; in gtm, Everywhere's 45 outside 22:00-07:00
    expect(readJson('settings.json', {})).toEqual({
      everywhere: { 'heartbeat.when': T.timeRule('FREQ=MINUTELY;INTERVAL=45') },
      machine: {},
      spaces: { gtm: { 'heartbeat.when': T.timeRule('FREQ=MINUTELY;INTERVAL=45;BYHOUR=7,8,9,10,11,12,13,14,15,16,17,18,19,20,21') } },
    })
    s.set('heartbeat.when', null)
    s.set('heartbeat.when', null, { space: 'gtm' })
  })
  test('an older settings file keeps its app settings; its one CLI becomes the tasks\' and the coordinators\'', () => {
    writeJson('settings.json', { all: { 'coordinator.agent': 'Codex', 'check.command': 'npm test', 'autopilot.hours': '9-18', 'autopilot.tasksPerDay': '3', 'wake.merged': 'now', 'heartbeat.every': '45 min' }, 'task:X-1': { 'merge.by': 'by itself' } })
    let s = new Settings()
    expect(s.get('task.agent')).toBe('Codex')
    expect(s.get('coordinator.agent')).toBe('Codex')
    // Autopilot's old limits and wake picks are dropped; its heartbeat stays, as a time rule
    expect(s.get('heartbeat.when')).toBe(T.timeRule('FREQ=MINUTELY;INTERVAL=45'))
    s.set('heartbeat.when', null)
    expect(readJson('settings.json', {})).toEqual({ everywhere: { 'task.agent': 'Codex', 'coordinator.agent': 'Codex' }, machine: {}, spaces: {} })
    // the Watcher's settings, from before agents reported their own progress, are dropped
    writeJson('settings.json', { agent: 'pi', 'watcher.agent': 'Claude Code', 'watcher.effort': 'lowest' })
    s = new Settings()
    expect(s.get('task.agent')).toBe('pi')
    expect(s.get('watcher.effort')).toBe('')
    expect(s.sections([], [], null).some((x) => x.rows.some((r) => r.key.startsWith('watcher.')))).toBe(false)
    s.set('task.agent', null)
    s.set('coordinator.agent', null)
  })
  test('a Merge setting kept under its old words reads as today\'s', () => {
    writeJson('settings.json', { 'brief.merge': 'after your yes' })
    expect(new Settings().get('brief.merge')).toBe('ask me first')
    writeJson('settings.json', { 'brief.merge': 'by itself' })
    expect(new Settings().get('brief.merge')).toBe('on its own')
  })
  test('review rounds read as on, and waiting for GitHub checks as a way to merge', () => {
    writeJson('settings.json', { everywhere: { 'brief.review': '2 rounds', 'brief.waitChecks': 'on' }, machine: {}, spaces: { web: { 'brief.review': 'off', 'brief.merge': 'ask me first', 'brief.waitChecks': 'on' } } })
    const s = new Settings()
    expect(s.get('brief.review')).toBe('on')
    expect(s.get('brief.merge')).toBe('after GitHub checks')
    expect(s.ways()).toEqual({ review: true, mustFix: 'major', rounds: 3, merge: 'on its own', waitChecks: true })
    expect(s.ways('web')).toEqual({ review: false, mustFix: 'major', rounds: 3, merge: 'ask me first', waitChecks: false })
    writeJson('settings.json', {})
  })
  test("defaults at three levels: a space's own win over its machine's, which win over Everywhere's", () => {
    // today's one level becomes All machines; a machine's capacity was each machine's own and stays so
    writeJson('settings.json', { 'brief.review': 'off', 'machine.maxTasks': '8' })
    const s = new Settings()
    expect(readJson('settings.json', {})).toEqual({ 'brief.review': 'off', 'machine.maxTasks': '8' })
    expect(s.pick('brief.review')).toEqual({ value: 'off', from: 'all' })
    expect(s.pick('machine.maxTasks')).toEqual({ value: '8', from: 'machine' })
    expect(s.pick('task.agent')).toEqual({ value: 'Claude Code', from: 'default' })
    expect(s.set('task.agent', 'Codex', 'machine').ok).toBe(true)
    expect(s.set('task.agent', 'pi', { space: 'web' }).ok).toBe(true)
    expect(s.set('brief.review', 'on', { space: 'web' }).ok).toBe(true)
    expect(s.get('task.agent')).toBe('Codex')
    expect(s.get('task.agent', 'web')).toBe('pi')
    expect(s.get('task.agent', 'api')).toBe('Codex')
    expect(s.harness('task', 'web').kind).toBe('pi')
    expect(s.ways('web').review).toBe(true)
    expect(s.ways('api').review).toBe(false)
    // a machine's capacity has no project level; alerts and voice are one for all
    expect(s.set('machine.maxTasks', '2', { space: 'web' }).ok).toBe(false)
    expect(s.set('voice.service', 'ElevenLabs', 'machine').ok).toBe(false)
    expect(readJson<Record<string, unknown>>('settings.json', {})).toEqual({ everywhere: { 'brief.review': 'off' }, machine: { 'machine.maxTasks': '8', 'task.agent': 'Codex' }, spaces: { web: { 'task.agent': 'pi', 'brief.review': 'on' } } })
    // the screen's rows say where an inherited value comes from, and what a reset goes back to
    const rows = (level: string, name: string) => s.sections([], [], null, 'vm', ['web']).find((x) => x.level === level && x.name === name)!.rows
    const agent = (level: string) => rows(level, 'Agents').find((r) => r.key === 'task.agent')!
    expect(agent('p:vm:web')).toMatchObject({ value: 'pi', inherits: { value: 'Codex', from: 'vm' } })
    expect(agent('m:vm')).toMatchObject({ value: 'Codex', inherits: { value: 'Claude Code', from: 'Everywhere' } })
    expect(agent('all')).toMatchObject({ value: 'Claude Code' })
    expect(agent('all').from).toBeUndefined()
    // a machine shown by a name of its own: the rows say that name, its levels keep the key
    const named = s.sections([], [], null, 'vm', ['web'], 'VM box').find((x) => x.level === 'p:vm:web' && x.name === 'Agents')!
    expect(named.rows.find((r) => r.key === 'task.agent')).toMatchObject({ inherits: { value: 'Codex', from: 'VM box' } })
    expect(named.machine).toBe('vm')
    expect(rows('p:vm:web', 'How tasks work').find((r) => r.key === 'brief.merge')).toMatchObject({ value: 'on its own', from: 'Everywhere' })
    expect(s.sections([], [], null, 'vm', ['web']).some((x) => x.level === 'p:vm:web' && x.name === 'Machine')).toBe(false)
    // unset drops a level's own: the one above counts again
    s.set('task.agent', null, { space: 'web' })
    expect(s.pick('task.agent', { space: 'web' })).toEqual({ value: 'Codex', from: 'machine' })
    // the other machines take the main one's All machines
    expect(s.adopt({ 'brief.review': 'on', nonsense: 'x' })).toBe(true)
    expect(s.everywhere()).toEqual({ 'brief.review': 'on' })
    writeJson('settings.json', {})
  })
  test("a task's backup agent is Codex unless set: any CLI herdr starts, or off", () => {
    const s = new Settings()
    expect(s.get('task.fallback')).toBe('Codex')
    expect(s.set('task.fallback', 'pi').ok).toBe(true)
    expect(s.get('task.fallback')).toBe('pi')
    expect(s.set('task.fallback', 'Off').ok).toBe(true)
    expect(s.get('task.fallback')).toBe('off')
    expect(s.set('task.fallback', 'Nonsense').ok).toBe(false)
    s.set('task.fallback', null)
  })
  test("the Settings screen shows what every new brief ends with, as it is now", () => {
    const s = new Settings()
    const note = () => s.sections([], [], null).find((x) => x.name === 'How tasks work')!.note!
    expect(note()).toContain('- Reviewer: a headless Codex or a Codex subagent (its default model and effort).')
    // planning, what to fix and what proof to show are the agent's judgment, not settings
    const section = () => s.sections([], [], null).find((x) => x.name === 'How tasks work')!
    expect(section().rows.map((r) => r.key)).toEqual(['brief.review', 'review.mustFix', 'review.rounds', 'brief.merge', 'reviewer.agent', 'reviewer.model', 'reviewer.effort'])
    // only words in prompts: What agents are told, each row with the line it adds
    expect(section().told).toBe(true)
    expect(s.sections([], [], null).find((x) => x.name === 'Agents')!.told).toBeUndefined()
    expect(section().rows.find((r) => r.key === 'review.rounds')!.line).toBe('Review rounds: 3. Stop when a round finds none to fix, at most 3 rounds.')
    s.set('review.mustFix', 'minor')
    s.set('review.rounds', '1')
    expect(note()).toContain('- Must fix: minor. Findings are tagged critical/major/minor/trivial; fix critical, major and minor. (your setting for every machine)')
    expect(note()).toContain('- Review rounds: 1. One round, no review again. (your setting for every machine)')
    expect(section().rows.find((r) => r.key === 'review.rounds')!.line).toBe('Review rounds: 1. One round, no review again. (your setting for every machine)')
    for (const k of ['review.mustFix', 'review.rounds']) s.set(k, null)
    s.set('brief.review', 'off')
    expect(note()).toContain("- Review: off. No review; the lines on reviewing below don't apply. (your setting for every machine)")
    s.set('brief.review', null)
    expect(s.sections([], [], null).find((x) => x.name === 'Agents')!.rows.map((r) => r.group)).toEqual(['Tasks', 'Tasks', 'Tasks', 'Coordinators', 'Coordinators', 'Coordinators', 'Backup', 'Backup', 'Backup'])
  })
})

describe("this project's own numbers, for a task's time left", () => {
  const min = 60000
  const task = (n: number, title: string, took: number, x: object = {}) => ({ id: `X-${n}`, project: 'p', title, desc: '', created: n * 1e7, doneAt: n * 1e7 + took * min, merged: true, ...x })
  const done = [
    task(1, 'Remove the Watcher', 60),
    task(2, 'Delete a task', 22),
    task(3, 'Archive tasks', 40, { reportedAt: 3e7 + 30 * min }),
    task(4, 'Rename task groups', 6),
    task(5, 'Fix laggy chat input', 39),
    task(6, 'Pin tasks', 77, { project: 'other' }),
    task(7, 'Tighter menu', 13, { merged: false }),
  ]
  test('the median, the middle half, 9 in 10, and the ones that read most like it, from start to its first done', () => {
    expect(pastTimes(done, { id: 'X-9', project: 'p', title: 'Remove a project from sidebar', desc: 'Delete it from the list' })).toBe(
      'How long this space\'s tasks really took, from start to done: the last 5 took 30m (median), half of them 22m to 39m, 9 in 10 under 1h. The ones that read most like this task: X-2 "Delete a task" 22m, X-1 "Remove the Watcher" 1h.',
    )
  })
  test('the latest ones when none reads like it; nothing while too few are done', () => {
    expect(pastTimes(done, { id: 'X-9', project: 'p', title: 'Voice calls', desc: '' })).toContain('The latest ones: X-5 "Fix laggy chat input" 39m, X-4 "Rename task groups" 6m, X-3 "Archive tasks" 30m.')
    expect(pastTimes(done.slice(1), { id: 'X-9', project: 'p', title: 'Voice', desc: '' })).toBe('')
  })
  test('times read short', () => {
    expect([span(0.2), span(25), span(60), span(80), span(130)]).toEqual(['1m', '25m', '1h', '1h20m', '2h10m'])
  })
})

describe('the brief\'s default way of working', () => {
  test("a task's agent is told who it is, its task's facts and which skills to read; the rules are in the skills", () => {
    const p = taskPrompt({ id: 'X-1', branch: 'work/1-x', base: 'main' }, 'How long this space\'s tasks really took: 20m.')
    expect(p).toContain('the agent of task X-1')
    expect(p).toContain('skill/shepherdr/SKILL.md first')
    expect(p).toContain('skill/shepherdr-agent/SKILL.md')
    expect(p).not.toContain('shepherdr-coordinator')
    expect(p).toContain('on branch work/1-x (from main)')
    expect(p).toContain("How long this space's tasks really took: 20m.")
    for (const s of ['shepherdr', 'shepherdr-agent', 'shepherdr-coordinator']) expect(existsSync(join(ROOT, 'skill', s, 'SKILL.md'))).toBe(true)
  })
  const ways = { review: true, mustFix: 'major', rounds: 3, merge: 'on its own', waitChecks: false }
  test("the human's defaults only, one line each: review, the reviewer, the bar, the rounds and merge on its own", () => {
    expect(settingsBlock('X-1', { ways, reviewer: { kind: 'codex' } })).toBe(`Settings: (from shepherdr show; where the brief above says otherwise, it wins)
- Review: on. Have your change reviewed by fresh eyes before it merges.
- Reviewer: a headless Codex or a Codex subagent (its default model and effort).
- Must fix: major. Findings are tagged critical/major/minor/trivial; fix critical and major, then review again.
- Review rounds: 3. Stop when a round finds none to fix, at most 3 rounds.
- Merge: on its own. Merge it yourself once it works (shepherdr merge X-1). Don't wait for GitHub's checks.`)
  })
  test("a brief splits into the coordinator's words and the Settings block, written today or before", () => {
    const how = settingsBlock('X-1', { ways })
    expect(splitBrief(`Fix it.\n\n${how}`)).toEqual(['Fix it.', how])
    expect(splitBrief('Fix it.')).toEqual(['Fix it.', ''])
    for (const old of ["How to work (Shepherdr's defaults; where the brief above says otherwise, it wins):\n- Review: none.", 'How tasks work (from Settings; where the brief above says otherwise, it wins):\n- Review: none.'])
      expect(splitBrief(`Fix it.\n\n${old}`)).toEqual(['Fix it.', old])
  })
  test('each setting changes its line', () => {
    expect(settingsBlock('X-1', { ways: { ...ways, merge: 'ask me first' } })).toContain('- Merge: ask me first. Ask the human first, and merge (shepherdr merge X-1) once they say yes.')
    expect(settingsBlock('X-1', { ways: { ...ways, waitChecks: true } })).toContain("- Merge: after GitHub checks. Merge it yourself once it works and GitHub's checks pass (shepherdr checks X-1, then shepherdr merge X-1).")
    // review off still shows every line, an override among them
    const off = settingsBlock('X-1', { ways: { ...ways, review: false, rounds: 1 }, reviewer: { kind: 'codex' }, by: { review: 'task', rounds: 'task' } })
    expect(off).toContain("- Review: off. No review; the lines on reviewing below don't apply. (set by coordinator for this task)")
    expect(off).toContain('- Reviewer: a headless Codex')
    expect(off).toContain('- Review rounds: 1. One round, no review again. (set by coordinator for this task)')
    expect(settingsBlock('X-1', { ways: { ...ways, mustFix: 'critical', rounds: 5 } })).toContain('- Must fix: critical. Findings are tagged critical/major/minor/trivial; fix critical, then review again.\n- Review rounds: 5. Stop when a round finds none to fix, at most 5 rounds.')
    expect(settingsBlock('X-1', { ways: { ...ways, mustFix: 'trivial' } })).toContain('fix all of them, then review again')
  })
  test('the Reviewer line names the reviewer, its model and effort; the Agent line what it runs on, only so it knows', () => {
    expect(settingsBlock('X-1', { ways, reviewer: { kind: 'claude', model: 'opus', effort: 'high' } })).toContain('- Reviewer: a headless Claude Code or a Claude Code subagent (model opus, high effort).')
    expect(settingsBlock('X-1', { ways, reviewer: { kind: 'pi', model: 'openai/gpt-5.5', effort: '' } })).toContain('- Reviewer: a headless pi or a pi subagent (model openai/gpt-5.5, its default effort).')
    expect(settingsBlock('X-1', { ways, agent: { kind: 'claude', effort: 'high' } })).toContain('- Agent: Claude Code (its default model, high effort). Only so you know what you run on.')
  })
  test('a line that differs from the default says who changed it, and the screen reads it back', () => {
    const block = settingsBlock('X-1', { ways: { ...ways, rounds: 1, merge: 'ask me first' }, by: { rounds: 'task', merge: 'space' } })
    expect(block).toContain('- Review rounds: 1. One round, no review again. (set by coordinator for this task)')
    expect(block).toContain("- Merge: ask me first. Ask the human first, and merge (shepherdr merge X-1) once they say yes. (this space's setting)")
    const read = readBrief(`Fix it.\n\n${block}\n\nQueued: wait.`)!
    expect(read.before).toBe('Fix it.')
    expect(read.after).toBe('Queued: wait.')
    expect(read.lines.map((l) => [l.label, l.by])).toEqual([['Review', undefined], ['Must fix', undefined], ['Review rounds', 'set by coordinator for this task'], ['Merge', "this space's setting"]])
    expect(readBrief('Fix it.')).toBeNull()
  })
  test("Settings say who changed each line: a space's own, a machine's, every machine's, or the coordinator for one task", () => {
    const s = new Settings()
    expect(s.setBy()).toEqual({})
    s.set('review.rounds', '2', 'machine')
    s.set('brief.merge', 'ask me first', { space: 'web' })
    // the same as Shepherdr's own default is no change
    s.set('review.mustFix', 'major')
    expect(s.setBy('web')).toEqual({ rounds: 'machine', merge: 'space' })
    expect(s.setBy('web', { 'review.mustFix': 'critical' })).toEqual({ rounds: 'machine', merge: 'space', mustFix: 'task' })
    expect(s.ways('web', { 'brief.merge': 'after GitHub checks' })).toMatchObject({ merge: 'on its own', waitChecks: true, rounds: 2 })
    for (const [k, l] of [['review.rounds', 'machine'], ['brief.merge', { space: 'web' }], ['review.mustFix', 'all']] as const) s.set(k, null, l)
  })
  test('task add --set takes a How tasks work key or its short name, or stands for a flag; a bad one says why', () => {
    expect(setForTask('review.rounds=1')).toEqual({ key: 'review.rounds', value: '1 round' })
    expect(setForTask('merge=ask me first')).toEqual({ key: 'brief.merge', value: 'ask me first' })
    expect(setForTask("merge='ask me first'")).toEqual({ key: 'brief.merge', value: 'ask me first' })
    expect(setForTask('review=off')).toEqual({ key: 'brief.review', value: 'off' })
    expect(setForTask('agent=codex')).toEqual({ flag: 'agent', value: 'codex' })
    expect(setForTask('reviewer=claude')).toEqual({ flag: 'reviewer', value: 'claude' })
    expect(setForTask('reviewer.effort=high')).toEqual({ flag: 'reviewer-effort', value: 'high' })
    expect(setForTask('merge=whenever')).toMatchObject({ error: expect.stringContaining('Pick one of') })
    expect(setForTask('colour=blue')).toMatchObject({ error: expect.stringContaining('--set takes key=value') })
  })
})

describe("a coordinator's instructions", () => {
  test('who it is, its space and whether Autopilot is on; its rules are in the coordinator skill', () => {
    const place = { role: 'space' as const, name: 'gtm', path: '/x/gtm', machine: 'vm-str' }
    for (const mode of ['regular', 'autopilot'] as const) {
      const p = coordinatorPrompt(place, mode)
      expect(p).toContain('the coordinator of the space gtm on vm-str (/x/gtm)')
      expect(p).not.toContain('home space')
      expect(p).toContain('skill/shepherdr/SKILL.md first')
      expect(p).toContain('skill/shepherdr-coordinator/SKILL.md')
      expect(p).not.toContain('shepherdr-agent')
      expect(p).toContain(`Autopilot is ${mode === 'autopilot' ? 'on' : 'off'} now.`)
    }
    // a home space's also looks after its machine; the main machine's also keeps every machine's overview
    const home = coordinatorPrompt({ role: 'home', name: 'home', path: '/home/x', machine: 'mac' }, 'regular')
    expect(home).toContain("This is mac's home space: you also look after the machine")
    expect(home).not.toContain('every machine')
    const main = coordinatorPrompt({ role: 'main', name: 'home', path: '/home/x', machine: 'vm-str' }, 'regular')
    expect(main).toContain('This is the home space of the main machine, vm-str')
    expect(main).toContain('you keep the overview of every machine')
    expect(main).toContain('which alerts reach the phone')
    const rules = readFileSync(join(ROOT, 'skill', 'shepherdr-coordinator', 'SKILL.md'), 'utf8')
    for (const w of ["Only on the human's plain word", 'standing requests', 'Autopilot', 'force-push', 'home space']) expect(rules).toContain(w)
  })
})

describe('projects', () => {
  test('task keys from project names', () => {
    expect(keyFor('gtm', [])).toBe('GTM')
    expect(keyFor('herdr-projects', [])).toBe('HP')
    expect(keyFor('lilla-varlden', [])).toBe('LV')
    expect(keyFor('herdr-plugins', ['HP'])).toBe('HERD')
  })

  test("this machine's shown name: a Mac's model, as herdr shows it; its HOST stays the key and is the fallback", () => {
    expect(machineName({}, 'darwin', () => 'Mac mini')).toBe('Mac mini')
    // a Mac that doesn't say, or can't be asked (no system_profiler here), and any other machine: its HOST
    expect(machineName({}, 'darwin', () => '')).toBe(HOST)
    expect(machineName({}, 'darwin')).toBe(HOST)
    expect(machineName({}, 'linux', () => 'Mac mini')).toBe(HOST)
    // run over SSH from the main machine: its herdr label
    expect(machineName({ SHEPHERDR_MACHINE: HOST }, 'darwin', () => 'Mac mini')).toBe(HOST)
    // the place, its chat, mode and settings stay under the key
    expect(placeId.machine()).toBe(`m:${HOST}`)
  })
})

describe('interactive chat capabilities', () => {
  test('Settings offers runtime-readable kinds and rejects unsupported selections without changing defaults', async () => {
    const { CHAT_KINDS, readerFor } = await import('../server/live/readers/index.ts')
    const { LOG_FINDERS } = await import('../server/live/readers/find.ts')
    const { KIND, REVIEWERS } = await import('../server/live/settings.ts')
    const settings = new Settings({ everywhere: {}, machine: {}, spaces: {} })
    const legacy = new Settings({ everywhere: { 'task.agent': 'Antigravity' }, machine: {}, spaces: {} })
    expect(legacy.get('task.agent')).toBe('Antigravity')
    for (const key of ['task.agent', 'coordinator.agent', 'task.fallback']) {
      const row = CATALOG.flatMap((s) => s.rows).find((r) => r.key === key)!
      expect(row.options!.filter((v) => v !== 'off').map((v) => KIND[v])).toEqual(CHAT_KINDS)
      const before = settings.get(key)
      for (const value of ['Antigravity', 'Gemini', 'Cursor']) {
        expect(settings.set(key, value)).toMatchObject({ ok: false, error: expect.stringContaining('no chat log reader') })
        expect(settings.get(key)).toBe(before)
      }
    }
    for (const kind of CHAT_KINDS) {
      expect(readerFor(kind)).toBeDefined()
      expect(LOG_FINDERS[kind]).toBeFunction()
    }
    expect(settings.set('task.fallback', 'off').ok).toBe(true)
    expect(REVIEWERS).toEqual(['Codex', 'Claude Code', 'pi'])
  })

  test('unsupported starts and restarts fail before callbacks or agent state changes', async () => {
    const { Agents } = await import('../server/live/agents.ts')
    const agents = new Agents()
    const spec = { key: 'unsupported-chat', name: 'unsupported-chat', kind: 'agy', cwd: '/tmp', workspace: 'test' }
    let stopped = false
    await expect(agents.ensure(spec)).rejects.toThrow('no chat log reader')
    await expect(agents.restart(spec, async () => { stopped = true })).rejects.toThrow('no chat log reader')
    expect(stopped).toBe(false)
    expect(agents.record(spec.key)).toBeUndefined()
    agents.stop()
  })
})

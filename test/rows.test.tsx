// A task row has one line under its title: its agent's status line while it works, else what it asks, waits for or did.
import { describe, expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { tasks } from '../server/demo/data.ts'
import type { Task } from '../shared/types.ts'
import { TaskDot, taskLine, ReportLine } from '../src/ui.tsx'

const ts = tasks()
const line = (t: Task, where?: string[]) => renderToStaticMarkup(<>{taskLine(t, where)}</>)
const text = (html: string) => html.replace(/<[^>]+>/g, '')
const report = { msg: 'Moving the skills into the template', progress: 62, eta: 4 }
const task = (status: Task['status'], over: Partial<Task> = {}): Task => ({ ...ts[0], status, sub: 'Pick an upload limit', report: undefined, pr: undefined, ...over })

describe('a task row', () => {
  test('waiting on you, it says only what it asks: no frozen status line, no %; its dot keeps the % for screen readers, with no tooltip', () => {
    const t = task('need', { report: { ...report, paused: true } })
    expect(text(line(t))).toBe('Pick an upload limit')
    const dot = renderToStaticMarkup(<TaskDot s="need" w={t.report} />)
    expect(dot).toContain('aria-label="Needs you, 62% done"')
    expect(dot).not.toContain('title=')
  })
  test('working, it says only what its agent reported; frozen, its pie keeps the %, with no pause mark or word', () => {
    expect(text(line(task('work', { sub: 'Working', report })))).toBe(report.msg)
    const frozen = line(task('work', { sub: 'Working', report: { ...report, paused: true } }))
    expect(text(frozen)).toBe(report.msg)
    expect(frozen).not.toContain('<svg')
    expect(renderToStaticMarkup(<TaskDot s="work" w={{ ...report, paused: true }} />)).toContain('aria-label="Working, 62% done"')
  })
  test('ready, done or to do, it says its summary', () => {
    for (const s of ['ready', 'done', 'todo'] as const) expect(text(line(task(s, { sub: 'It now checks the raw body' })))).toBe('It now checks the raw body')
  })
  test('where the list does not say where it is, the line starts with that: its machine, its project', () => {
    expect(text(line(task('need', { report: { ...report, paused: true } }), ['gtm']))).toBe('gtm · Pick an upload limit')
    expect(text(line(task('work', { report }), ['vm-str', 'gtm']))).toBe(`vm-str · gtm · ${report.msg}`)
  })
  test('every demo task has one line at most: its summary or its status line, never both, nothing stacked', () => {
    for (const t of ts) {
      const html = line(t)
      const pr = t.pr ? `PR #${t.pr.number} · ` : ''
      expect(text(html)).toBe(pr + (t.status === 'work' && t.report ? t.report.msg : (t.sub ?? '')))
      expect(html).not.toMatch(/\bblock\b/)
    }
  })
  test('its pull request, once it has one, leads the line, unless where it is does: one line on a phone', () => {
    const pr = { number: 48, url: 'https://github.com/a/b/pull/48' }
    expect(text(line(task('ready', { sub: 'It now checks the raw body', pr })))).toBe('PR #48 · It now checks the raw body')
    expect(text(line(task('done', { sub: 'Merged', pr }), ['vm-str', 'gtm']))).toBe('vm-str · gtm · Merged')
  })
  test("a chat's header still shows the % an agent waiting on you stopped at", () => {
    expect(text(renderToStaticMarkup(<ReportLine w={{ ...report, paused: true }} head />))).toContain('62%')
  })
})

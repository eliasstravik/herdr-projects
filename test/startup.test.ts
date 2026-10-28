// Captured CLI screens: startup trust, MCP consent and approvals must work before a session log exists.
import { describe, expect, test } from 'bun:test'
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { menuId, menuKeys, parseMenu } from '../server/live/chats.ts'
import { codexTrust, trustWorktree } from '../server/live/trust.ts'
const screen = (name: string) => readFileSync(join(import.meta.dir, 'fixtures/startup', name + '.txt'), 'utf8')

describe('recorded startup prompts', () => {
  test('Claude folder trust: real order, cursor, and keys', () => {
    const m = parseMenu(screen('claude-trust'))!
    expect(m.choices).toEqual(['No, exit', 'Yes, I trust this folder'])
    expect(m.cursor).toBe(0)
    expect(menuKeys(m, 1)).toEqual(['down', 'enter'])
    expect(m.screen).toContain('/scratchpad/fresh3')
  })
  test('Claude MCP approval: no question mark and a safe default', () => {
    const m = parseMenu(screen('claude-mcp'))!
    expect(m.title).toContain('New MCP server found in this project: probe')
    expect(m.choices).toEqual(['Use this MCP server', 'Use this and all future MCP servers in this project', 'Continue without using this MCP server'])
    expect(m.cursor).toBe(2)
    expect(menuKeys(m, 0)).toEqual(['up', 'up', 'enter'])
  })
  test('Claude multi-server MCP consent toggles precisely the chosen servers, then submits', () => {
    const m = parseMenu(screen('claude-mcp-multi'))!
    expect(m.multi).toBe(true)
    expect(m.choices).toEqual(['proof-one', 'proof-two'])
    expect(m.selected).toEqual([0, 1])
    const toggled = parseMenu(screen('claude-mcp-multi').replace('[✔] proof-one', '[ ] proof-one'))!
    expect(menuId('c', toggled)).toBe(menuId('c', m))
    expect(menuKeys(m, 1, [1])).toEqual(['space', 'down', 'down', 'enter'])
    expect(menuKeys(m, -1, [])).toEqual(['escape'])
    expect(menuKeys(m, 9, [9])).toBeUndefined()
  })
  test('Codex folder trust', () => {
    const m = parseMenu(screen('codex-trust'))!
    expect(m.title).toBe('Trust this folder?')
    expect(m.choices).toEqual(['Trust and continue', 'Back to Agent Command Center'])
    expect(menuKeys(m, 0)).toEqual(['enter'])
  })
  test('generic arrow and y/n menus; ordinary input is not a menu', () => {
    expect(parseMenu('Choose provider\n❯ Local\n  Hosted\nEnter to select · ↑/↓ to navigate')?.choices).toEqual(['Local', 'Hosted'])
    expect(parseMenu('Choose\n❯ Allow once\n  Allow always\n  Cancel\nEnter to confirm')?.choices).toEqual(['Allow once', 'Allow always', 'Cancel'])
    const yn = parseMenu('Allow this tool? [y/N]')!
    expect(menuKeys(yn, 1)).toEqual(['n', 'enter'])
    expect(parseMenu('❯ Try fixing a bug\n  permissions on')).toBeNull()
    expect(menuKeys(yn, -1)).toBeUndefined()
    expect(menuKeys(yn, 12)).toBeUndefined()
  })
  test('a changed permission gets a new identity; a cursor move does not', () => {
    const m = parseMenu('Allow? [y/n]')!
    expect(menuId('c', m)).toBe(menuId('c', { ...m, cursor: 1 }))
    expect(menuId('c', m)).not.toBe(menuId('c', { ...m, screen: 'Delete files? [y/n]' }))
  })
})

describe('trust only the task folder, preserving settings', () => {
  test('Codex retains comments, other projects and preferences', () => {
    const original = '# keep\nmodel = "example"\n[projects."/other"]\ntrust_level = "untrusted"\n'
    const once = codexTrust(original, '/task')
    expect(codexTrust(once, '/task')).toBe(once)
    expect(once).toContain(original.trim())
    expect((Bun.TOML.parse(once) as Record<string, unknown>).projects).toEqual({ '/other': { trust_level: 'untrusted' }, '/task': { trust_level: 'trusted' } })
    expect((Bun.TOML.parse(codexTrust("[projects.'/task']\ntrust_level = 'untrusted'\n[other]\nvalue=1\n", '/task')) as Record<string, unknown>).other).toEqual({ value: 1 })
    expect(() => codexTrust('bad = [', '/task')).toThrow()
    expect(() => codexTrust('projects = { "/other" = { trust_level = "untrusted" } }', '/task')).toThrow()
  })
  test('Claude retains project permissions and never approves MCP servers', () => {
    const dir = mkdtempSync(join(tmpdir(), 'shep-startup-'))
    writeFileSync(join(dir, '.claude.json'), JSON.stringify({ preference: true, projects: { [dir]: { allowedTools: ['Read'], hasTrustDialogAccepted: false }, '/other': { hasTrustDialogAccepted: false } } }))
    trustWorktree(dir, 'claude', { CLAUDE_CONFIG_DIR: dir })
    const config = JSON.parse(readFileSync(join(dir, '.claude.json'), 'utf8'))
    expect(config.preference).toBe(true)
    expect(config.projects[dir]).toEqual({ allowedTools: ['Read'], hasTrustDialogAccepted: true })
    expect(config.projects['/other'].hasTrustDialogAccepted).toBe(false)
  })
})

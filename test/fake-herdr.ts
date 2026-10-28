// herdr's worktree API, standing in for herdr on real git: a linked worktree's workspace nests under its repo's own,
// which it opens when none is open; removing one is git worktree remove, never forced.
import { execSync } from 'node:child_process'
import { mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { basename, join } from 'node:path'

const sh = (cmd: string, cwd: string) => execSync(cmd, { cwd, shell: '/bin/sh', encoding: 'utf8' }).trim()

type Space = { workspace_id: string; label: string; tab_count: number; focused?: boolean; worktree?: { checkout_path: string; is_linked_worktree: boolean; repo_root: string }; panes: string[] }

export function fakeHerdr() {
  let n = 0
  const spaces: Space[] = []
  const cwd = new Map<string, string>()
  const agentPanes = new Set<string>()
  const busy = new Set<string>()
  let down = false
  const root = mkdtempSync(join(tmpdir(), 'shep-herdr-'))
  const find = (id: string) => spaces.find((w) => w.workspace_id === id)
  const drop = (id: string) => {
    const i = spaces.findIndex((w) => w.workspace_id === id)
    if (i >= 0) spaces.splice(i, 1)
  }
  const pane = (ws: Space, dir: string) => {
    const id = `${ws.workspace_id}:p${++n}`
    ws.panes.push(id)
    cwd.set(id, dir)
    return id
  }
  const space = (label: string, dir: string, worktree?: Space['worktree']) => {
    const ws: Space = { workspace_id: `w${++n}`, label, tab_count: 1, worktree, panes: [] }
    spaces.push(ws)
    pane(ws, dir)
    return ws
  }
  /** the repo space a worktree nests under: the workspace named, or the one herdr knows (or finds by its folder, or
   *  opens) for the folder; it learns that it is the repo's */
  const source = (from: { workspace_id?: string; cwd?: string }) => {
    const named = from.workspace_id ? find(from.workspace_id) : undefined
    if (from.workspace_id && !named) throw new Error('no such workspace')
    const repo = named ? (named.worktree?.checkout_path ?? cwd.get(named.panes[0])!) : from.cwd!
    const ws =
      named ??
      spaces.find((w) => w.worktree && !w.worktree.is_linked_worktree && w.worktree.checkout_path === repo) ??
      spaces.find((w) => !w.worktree && cwd.get(w.panes[0]) === repo) ??
      space(basename(repo), repo)
    ws.worktree ??= { checkout_path: repo, is_linked_worktree: false, repo_root: repo }
    return repo
  }
  const opened = (repo: string, path: string, label: string) => {
    const ws = space(label, path, { checkout_path: path, is_linked_worktree: true, repo_root: repo })
    return { workspace: { ...ws }, root_pane: { pane_id: ws.panes[0] }, worktree: { path } }
  }
  const up = () => {
    if (down) throw new Error('herdr does not answer')
  }
  const h = {
    workspaces: async () => (up(), spaces.map(({ panes, ...w }) => ({ ...w, pane_count: panes.length }))),
    agents: async () => (up(), [...agentPanes].map((pane_id) => ({ pane_id }))),
    panes: async (id: string) => (up(), (find(id)?.panes ?? []).map((pane_id) => ({ pane_id, workspace_id: id, cwd: cwd.get(pane_id) }))),
    paneIdle: async (id: string) => !busy.has(id),
    worktreeCreate: async (from: { workspace_id?: string; cwd?: string }, branch: string, base: string, label: string, at?: string) => {
      up()
      const repo = source(from)
      const path = at ?? join(root, basename(repo), branch.replace(/\//g, '-'))
      const has = execSync(`git branch --list ${branch}`, { cwd: repo, encoding: 'utf8' }).trim()
      sh(has ? `git worktree add -q ${path} ${branch}` : `git worktree add -q -b ${branch} ${path} ${base}`, repo)
      return opened(repo, path, label)
    },
    worktreeOpen: async (from: { workspace_id?: string; cwd?: string }, path: string, label: string) => {
      up()
      const repo = source(from)
      const open = spaces.find((w) => w.worktree?.checkout_path === path)
      if (open) return { workspace: { ...open }, root_pane: { pane_id: open.panes[0] }, already_open: true }
      return opened(repo, path, label)
    },
    worktreeRemove: async (id: string, force = false) => {
      const ws = find(id)!
      sh(`git worktree remove ${force ? '--force ' : ''}${ws.worktree!.checkout_path}`, ws.worktree!.repo_root)
      for (const p of ws.panes) agentPanes.delete(p)
      drop(id)
    },
    createTab: async (id: string, _label: string, dir: string) => {
      const ws = find(id)!
      ws.tab_count++
      return { root_pane: { pane_id: pane(ws, dir) } }
    },
    closePane: async (id: string) => {
      const ws = spaces.find((w) => w.panes.includes(id))
      agentPanes.delete(id)
      if (!ws) return
      ws.panes = ws.panes.filter((p) => p !== id)
      ws.tab_count--
      if (!ws.panes.length) drop(ws.workspace_id)
    },
    closeWorkspace: async (id: string) => {
      drop(id)
    },
  }
  return { h, spaces, agentPanes, busy, cwd, down: (on: boolean) => (down = on) }
}


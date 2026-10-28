// A task's place in herdr, as herdr-projects keeps it: a worktree workspace nested under its project's own space in
// herdr's sidebar, made, opened again and removed through herdr's worktree API. Putting it away never loses work: a
// checkout with uncommitted changes, or a branch with work main lacks, is kept and said so, and nothing is forced. A
// repo space herdr opened only because a task asked for a worktree is closed again with the repo's last task space,
// unless you have started using it.
import { boxHome } from '../box.ts'
import { existsSync, realpathSync } from '../mfs.ts'
import { readJson, writeJson } from '../../store.ts'
import { herdr, type WorkspaceInfo, type WorktreeFrom, type WorktreeSpace } from '../herdr.ts'
import * as G from './git.ts'

/** what this asks of herdr; tests give a stand-in */
export type HerdrSpaces = Pick<
  typeof herdr,
  'workspaces' | 'agents' | 'panes' | 'paneIdle' | 'worktreeCreate' | 'worktreeOpen' | 'worktreeRemove' | 'createTab' | 'closePane' | 'closeWorkspace'
>

/** a worktree herdr just made (or opened): its folder, its workspace and the shell herdr opened in a new one */
export interface Made {
  path: string
  workspace: string
  root?: string
}

/** where a task's worktree belongs: its project's repo, and the name of the project's own space (its coordinator's) */
export interface Home {
  repo: string
  name: string
}

export interface PutAway {
  /** what was kept, and why, in words: "~/x (uncommitted changes)" */
  kept: string[]
  /** something still runs in its space (or herdr didn't answer): nothing was touched, try again later */
  busy?: boolean
}

const real = (p: string) => {
  try {
    return realpathSync(p)
  } catch {
    return p
  }
}

const tilde = (p: string) => (p.startsWith(boxHome()) ? `~${p.slice(boxHome().length)}` : p)

/** the herdr spaces opened for repos, to close with their last task (spaces.json before spaces came: migrate.ts) */
const SPACES = 'repo-spaces.json'

export class Worktrees {
  constructor(private h: HerdrSpaces = herdr) {}

  private repoSpaces(list: WorkspaceInfo[], repo: string) {
    return list.filter((w) => w.worktree && !w.worktree.is_linked_worktree && real(w.worktree.checkout_path) === real(repo))
  }

  /**
   * A herdr worktree create or open, nested under the project's own space (its coordinator's, which herdr then knows
   * as the repo's), else the repo's space herdr knows, else the repo's folder: herdr opens a space for it, which is
   * noted so it can be closed again with the repo's last task space.
   */
  private async opening(home: Home, f: (from: WorktreeFrom) => Promise<WorktreeSpace>): Promise<WorktreeSpace> {
    const list = await this.h.workspaces().catch(() => null)
    const own = list?.find((w) => w.label === home.name && (!w.worktree || (!w.worktree.is_linked_worktree && real(w.worktree.checkout_path) === real(home.repo))))
    const known = own ?? (list ? this.repoSpaces(list, home.repo)[0] : undefined)
    if (known) {
      try {
        return await f({ workspace_id: known.workspace_id })
      } catch {
        /* not one herdr takes as the repo's (another folder of the same name): the folder it is */
      }
    }
    const r = await f({ cwd: home.repo })
    if (!list) return r
    const before = new Set(this.repoSpaces(list, home.repo).map((w) => w.workspace_id))
    const made = this.repoSpaces(await this.h.workspaces().catch(() => []), home.repo).find((w) => !before.has(w.workspace_id))
    if (made) writeJson(SPACES, { ...readJson<Record<string, string>>(SPACES, {}), [real(home.repo)]: made.workspace_id })
    return r
  }

  private made(r: WorktreeSpace, path?: string): Made {
    const at = path ?? r.worktree?.path ?? r.workspace.worktree?.checkout_path
    if (!at) throw new Error('herdr made no worktree')
    return { path: at, workspace: r.workspace.workspace_id, root: r.already_open ? undefined : r.root_pane.pane_id }
  }

  /**
   * A new worktree for a task on its branch from `base`, .worktreeinclude's files copied in; path: where (herdr picks
   * when left out). One its branch is checked out in already (a start herdr finished but whose answer was lost) is
   * opened instead.
   */
  async create(home: Home, branch: string, base: string, label: string, path?: string): Promise<Made> {
    const open = async () => {
      const at = await G.worktreeOf(home.repo, branch)
      return at && existsSync(at) ? this.made(await this.opening(home, (from) => this.h.worktreeOpen(from, at, label)), at) : undefined
    }
    const was = await open()
    if (was) return was
    const from = await G.startPoint(home.repo, base)
    let r: WorktreeSpace
    try {
      r = await this.opening(home, (src) => this.h.worktreeCreate(src, branch, from, label, path))
    } catch (e) {
      const late = await open()
      if (late) return late
      throw e
    }
    const made = this.made(r)
    await G.copyIncludes(home.repo, made.path)
    return made
  }

  /** the task's space: its herdr worktree workspace, or the plain one by its label (a task started before) */
  async spaceOf(wt: string | undefined, label: string, list?: WorkspaceInfo[]) {
    const all = list ?? (await this.h.workspaces())
    return (wt ? all.find((w) => w.worktree?.is_linked_worktree && real(w.worktree.checkout_path) === real(wt)) : undefined) ?? all.find((w) => !w.worktree && w.label === label)
  }

  /**
   * A pane for the task's agent, in a tab of its own that has the agent's environment (herdr's worktree commands take
   * none): in the space just made, in its space as it is, or in its space opened again from the worktree through
   * herdr. The shell herdr opened a new space with is closed.
   */
  async agentPane(o: { home: Home; wt: string; label: string; tab: string; env: Record<string, string>; made?: Made }) {
    let ws: string
    let root: string | undefined
    if (o.made) ({ workspace: ws, root } = o.made)
    else {
      const open = await this.spaceOf(o.wt, o.label)
      if (open) ws = open.workspace_id
      else ({ workspace: ws, root } = this.made(await this.opening(o.home, (from) => this.h.worktreeOpen(from, o.wt, o.label)), o.wt))
    }
    const pane = (await this.h.createTab(ws, o.tab, o.wt, o.env)).root_pane.pane_id
    if (root) await this.h.closePane(root).catch(() => undefined)
    return pane
  }

  /** something other than the task's agent runs in the space: an agent, or a command at work in a shell */
  private async busy(ws: string, agentPane?: string) {
    try {
      const [panes, agents] = await Promise.all([this.h.panes(ws), this.h.agents()])
      for (const p of panes) {
        if (p.pane_id === agentPane) continue
        if (agents.some((a) => a.pane_id === p.pane_id) || !(await this.h.paneIdle(p.pane_id))) return true
      }
      return false
    } catch {
      return true
    }
  }

  /**
   * Put a task's place away: its worktree removed through herdr (which closes its space and the agent's pane with
   * it), its agent closed and its branch deleted, each only when that loses nothing. A checkout with uncommitted
   * changes, or a branch with work `base` lacks, is kept and said so; its space and agent close all the same, and
   * writing to the task opens it again. A space where something else still runs, or a herdr that doesn't answer,
   * leaves everything as it is for a later try. squashes: a merged task's squash merges into `base`, which a branch
   * with nothing after them is in, however `base` moved on since.
   */
  async putAway(o: { repo: string; wt?: string; branch?: string; base: string; squashes?: string[]; label: string; agentPane?: string; closeAgent: () => Promise<unknown> }): Promise<PutAway> {
    const where = o.wt ? tilde(o.wt) : `${o.label}'s space`
    let space: WorkspaceInfo | undefined
    try {
      space = await this.spaceOf(o.wt, o.label)
    } catch {
      return { kept: [`${where} (herdr does not answer)`], busy: true }
    }
    if (space && (await this.busy(space.workspace_id, o.agentPane))) return { kept: [`${where} (something still runs in its herdr space)`], busy: true }
    const there = !!o.wt && existsSync(o.wt)
    const branch = !!o.branch && (await G.branchExists(o.repo, o.branch))
    const kept = await this.holds(o)
    if (there && !kept.length) {
      if (space?.worktree) await this.h.worktreeRemove(space.workspace_id).catch(() => undefined)
      // herdr didn't (or it has no herdr space open): git removes it, never forced
      if (existsSync(o.wt!)) {
        const r = await G.removeWorktree(o.repo, o.wt!)
        if (!r.ok) kept.push(`${where} (${(r.err || r.out).split('\n')[0] || 'git could not remove it'})`)
      }
    }
    await o.closeAgent()
    space = await this.spaceOf(o.wt, o.label).catch(() => undefined)
    if (space) await this.h.closeWorkspace(space.workspace_id).catch(() => undefined)
    await G.git(o.repo, 'worktree', 'prune')
    if (branch && !kept.length) {
      const d = await G.git(o.repo, 'branch', '-D', o.branch!)
      if (!d.ok) kept.push(`branch ${o.branch} (${d.err.split('\n')[0]})`)
    }
    await this.closeRepoSpace(o.repo)
    return { kept }
  }

  /** what a task's place holds that main lacks, in words ("~/x (uncommitted changes)"); none: it loses nothing */
  async holds(o: { repo: string; wt?: string; branch?: string; base: string; squashes?: string[] }): Promise<string[]> {
    const there = !!o.wt && existsSync(o.wt)
    const branch = !!o.branch && (await G.branchExists(o.repo, o.branch))
    const unsaved = there && (await G.dirty(o.wt!))
    const unmerged = branch && !(await G.landed(o.repo, o.branch!, o.base, o.squashes))
    if (!unsaved && !unmerged) return []
    const why = [unsaved && 'uncommitted changes', unmerged && `work on ${o.branch} that ${o.base} lacks`].filter(Boolean).join(', ')
    return [there ? `${tilde(o.wt!)} (${why})` : `branch ${o.branch} (${why})`]
  }

  /**
   * Remove a deleted task's place for good, its agent closed already: its worktree removed (forced: whatever it held
   * was checked first, or you said --force), its space closed and its branch deleted. Whatever else runs in its space
   * goes with it. Returns what could not be removed.
   */
  async remove(o: { repo: string; wt?: string; branch?: string; label: string }): Promise<string[]> {
    const left: string[] = []
    const space = await this.spaceOf(o.wt, o.label).catch(() => undefined)
    if (o.wt && existsSync(o.wt)) {
      if (space?.worktree) await this.h.worktreeRemove(space.workspace_id, true).catch(() => undefined)
      if (existsSync(o.wt)) {
        const r = await G.removeWorktree(o.repo, o.wt, true)
        if (!r.ok && existsSync(o.wt)) left.push(`${tilde(o.wt)} (${(r.err || r.out).split('\n')[0] || 'git could not remove it'})`)
      }
    }
    const still = await this.spaceOf(o.wt, o.label).catch(() => undefined)
    if (still) await this.h.closeWorkspace(still.workspace_id).catch(() => undefined)
    await G.git(o.repo, 'worktree', 'prune')
    if (o.branch && (await G.branchExists(o.repo, o.branch))) {
      const d = await G.git(o.repo, 'branch', '-D', o.branch)
      if (!d.ok) left.push(`branch ${o.branch} (${d.err.split('\n')[0]})`)
    }
    await this.closeRepoSpace(o.repo)
    return left
  }

  /**
   * A repo space herdr opened for a task's worktree closes once no worktree space of the repo is open: unless you are
   * looking at it or have used it (another tab or pane, an agent, a command at work, another folder).
   */
  private async closeRepoSpace(repo: string) {
    const all = readJson<Record<string, string>>(SPACES, {})
    const id = all[real(repo)]
    if (!id) return
    const forget = () => {
      const now = readJson<Record<string, string>>(SPACES, {})
      delete now[real(repo)]
      writeJson(SPACES, now)
    }
    const list = await this.h.workspaces().catch(() => null)
    if (!list) return
    const w = list.find((x) => x.workspace_id === id)
    if (!w?.worktree || w.worktree.is_linked_worktree || real(w.worktree.checkout_path) !== real(repo)) return forget()
    if (list.some((x) => x.worktree?.is_linked_worktree && real(x.worktree.repo_root) === real(repo))) return
    if (w.focused) return
    const [panes, agents] = await Promise.all([this.h.panes(id), this.h.agents()]).catch(() => [null, null])
    if (!panes || !agents) return
    const one = panes.length === 1 ? panes[0] : undefined
    const used = !one || (w.tab_count ?? 1) > 1 || agents.some((a) => a.pane_id === one.pane_id) || real(one.foreground_cwd ?? one.cwd ?? '') !== real(repo) || !(await this.h.paneIdle(one.pane_id).catch(() => false))
    if (!used) await this.h.closeWorkspace(id).catch(() => undefined)
    forget()
  }

  /** the sweep looks again at repo spaces that could not be closed when their last task ended */
  async closeRepoSpaces() {
    for (const repo of Object.keys(readJson<Record<string, string>>(SPACES, {}))) await this.closeRepoSpace(repo)
  }

  workspaces() {
    return this.h.workspaces()
  }
}

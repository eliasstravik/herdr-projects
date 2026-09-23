//! `sweep`: what a project left behind and nothing uses any more. Worktrees
//! on `hp/<slug>/` branches with no open thread, local branches of resolved
//! threads whose pull request merged, tabs of resolved threads, working
//! folders of long-resolved tab threads, handled inbox items older than 30
//! days. `--dry-run` lists; otherwise each is removed after a confirmation.
//! Remote machines are left to `thread resolve`.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Result, bail};

use crate::paths::Ctx;
use crate::project::Project;
use crate::runner::Cmd;
use crate::thread::{self, Kind, Status, Thread};
use crate::threads;

#[derive(Debug, Clone, PartialEq)]
pub enum Orphan {
    /// `thread` is the resolved thread that still owns it, if any: its files
    /// are copied home again, completely, before the worktree may go.
    Worktree { repo: String, path: String, workspace: Option<String>, branch: String, thread: Option<String> },
    Branch { repo: String, branch: String },
    Tab { id: String, tab: String },
    Folder { id: String, path: PathBuf },
    DoneItems { count: usize },
}

impl Orphan {
    pub fn describe(&self) -> String {
        match self {
            Orphan::Worktree { path, branch, workspace, .. } => format!("worktree {path} ({branch}){} with no open thread", if workspace.is_some() { ", workspace open" } else { "" }),
            Orphan::Branch { branch, .. } => format!("branch {branch}: its thread is resolved and its pull request merged"),
            Orphan::Tab { id, tab } => format!("tab {tab} of resolved thread {id}"),
            Orphan::Folder { id, path } => format!("working folder {} of {id}, resolved long ago (its report is home)", path.display()),
            Orphan::DoneItems { count } => format!("{count} handled inbox item(s) older than 30 days"),
        }
    }
}

fn git(ctx: &Ctx, repo: &str, args: &[&str]) -> Option<String> {
    let out = ctx.runner.run(&Cmd::new("git", Duration::from_secs(10)).args(["-C", repo]).args(args.iter().copied())).ok()?;
    out.success().then_some(out.stdout)
}

/// `git worktree list --porcelain`: (path, branch without refs/heads/).
pub fn parse_worktrees(text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut path = String::new();
    for line in text.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            path = p.to_string();
        } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
            found.push((path.clone(), b.to_string()));
        }
    }
    found
}

fn older_than(stamp: &str, days: u32, now: jiff::Timestamp) -> bool {
    days > 0 && thread::seconds_since(stamp, now) > i64::from(days) * 86_400
}

pub fn find(ctx: &Ctx, project: &Project) -> Vec<Orphan> {
    let slug = &project.slug;
    let prefix = format!("hp/{slug}/");
    let threads = thread::list(project);
    let settings = project.read_project_md().map(|(s, _)| s).unwrap_or_default();
    let view = threads::session_view(ctx, project);
    let now = jiff::Timestamp::now();
    let mut orphans = Vec::new();

    // Local repositories this project works in.
    let mut repos: Vec<String> = settings.repos.iter().filter(|r| r.machine.is_none()).map(|r| r.path.clone()).collect();
    repos.extend(threads.iter().filter(|t| !t.is_remote() && !t.repo.is_empty() && t.kind == Kind::Worktree).map(|t| t.repo.clone()));
    repos.sort();
    repos.dedup();
    let open = |branch: &str, path: &str| threads.iter().any(|t| t.status != Status::Resolved && (t.branch == branch || t.worktree_path == path));
    for repo in &repos {
        let listed = git(ctx, repo, &["worktree", "list", "--porcelain"]).map(|t| parse_worktrees(&t)).unwrap_or_default();
        let mut with_worktree = Vec::new();
        for (path, branch) in listed {
            if !branch.starts_with(&prefix) {
                continue;
            }
            with_worktree.push(branch.clone());
            if open(&branch, &path) {
                continue;
            }
            let owner = threads.iter().find(|t| t.branch == branch || t.worktree_path == path);
            if owner.is_some_and(|t| t.kept_worktree) {
                continue; // resolved with --keep-worktree
            }
            // A workspace whose root is this worktree (herdr opens it there).
            let workspace = view.as_ref().and_then(|v| v.panes.iter().find(|p| p.cwd == path).map(|p| p.workspace_id.clone()));
            orphans.push(Orphan::Worktree { repo: repo.clone(), path, workspace, branch, thread: owner.map(|t| t.id.clone()) });
        }
        let branches = git(ctx, repo, &["for-each-ref", "--format=%(refname:short)", &format!("refs/heads/{prefix}")]).unwrap_or_default();
        for branch in branches.lines().map(str::trim).filter(|b| !b.is_empty()) {
            if with_worktree.iter().any(|b| b == branch) {
                continue;
            }
            let state = crate::steps::load_state(project);
            let merged = threads.iter().any(|t| {
                t.branch == branch && t.status == Status::Resolved && t.pr_state.eq_ignore_ascii_case("merged")
                    // Only when the local tip is what was merged.
                    && state.prs.get(&t.id).is_some_and(|s| !s.head_oid.is_empty() && git(ctx, repo, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).is_some_and(|tip| tip.trim() == s.head_oid))
            });
            if merged {
                orphans.push(Orphan::Branch { repo: repo.clone(), branch: branch.to_string() });
            }
        }
    }

    for t in threads.iter().filter(|t| t.status == Status::Resolved && !t.is_remote()) {
        if matches!(t.kind, Kind::Tab | Kind::Checkout)
            && let Some(view) = &view
            && !t.pane_id.is_empty()
            && thread::live_state(t, &view.agents, &view.panes, now).pane_exists
        {
            orphans.push(Orphan::Tab { id: t.id.clone(), tab: t.tab_id.clone() });
        }
        let folder = project.dir().join("threads").join(&t.id);
        if t.kind == Kind::Tab && folder.is_dir() && older_than(&t.updated, settings.auto_resolve_days.max(1), now) && thread::home_report_path(project, &t.id).is_file() {
            orphans.push(Orphan::Folder { id: t.id.clone(), path: folder });
        }
    }

    let limit = std::time::Duration::from_secs(u64::from(crate::steps::DONE_RETENTION_DAYS as u32) * 86_400);
    let old_done = std::fs::read_dir(project.dir().join("inbox/done"))
        .map(|e| e.flatten().filter(|e| e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age > limit)).count())
        .unwrap_or(0);
    if old_done > 0 {
        orphans.push(Orphan::DoneItems { count: old_done });
    }
    orphans
}

fn remove(ctx: &Ctx, project: &Project, orphan: &Orphan) -> Result<()> {
    match orphan {
        Orphan::Worktree { repo, path, workspace, thread: owner, .. } => {
            if let Some(id) = owner {
                let t = thread::load(project, id)?;
                let copied = threads::final_copy(ctx, project, &Thread { thread_dir: t.thread_dir.clone(), ..t });
                if copied.outcome != thread::CopyOutcome::Complete {
                    bail!("not everything in it could be copied home first");
                }
            }
            if let (Some(workspace), Some(view)) = (workspace, threads::session_view(ctx, project)) {
                return view.herdr.worktree_remove(workspace).map_err(|e| anyhow::anyhow!("{e}"));
            }
            git(ctx, repo, &["worktree", "remove", path]).ok_or_else(|| anyhow::anyhow!("git refused to remove {path} (uncommitted changes?)"))?;
            let _ = git(ctx, repo, &["worktree", "prune"]);
            Ok(())
        }
        Orphan::Branch { repo, branch } => git(ctx, repo, &["branch", "-D", branch]).map(|_| ()).ok_or_else(|| anyhow::anyhow!("git refused to delete {branch}")),
        Orphan::Tab { tab, .. } => {
            let view = threads::session_view(ctx, project).ok_or_else(|| anyhow::anyhow!("the session is not reachable"))?;
            view.herdr.call(&["tab", "close", tab], crate::herdr::CALL_TIMEOUT).map(|_| ()).map_err(|e| anyhow::anyhow!("{e}"))
        }
        Orphan::Folder { path, .. } => Ok(std::fs::remove_dir_all(path)?),
        Orphan::DoneItems { .. } => {
            crate::inbox::prune_done(project, crate::steps::DONE_RETENTION_DAYS);
            Ok(())
        }
    }
}

/// `sweep <slug> [--dry-run] [--yes]`.
pub fn run(ctx: &Ctx, slug: &str, dry_run: bool, yes: bool) -> Result<()> {
    let project = Project::load(&ctx.root, slug)?;
    let orphans = find(ctx, &project);
    if orphans.is_empty() {
        println!("nothing to clean in `{slug}`");
        return Ok(());
    }
    for orphan in &orphans {
        println!("{}", orphan.describe());
    }
    if dry_run {
        return Ok(());
    }
    if !yes {
        use std::io::{BufRead, IsTerminal, Write};
        if !std::io::stdin().is_terminal() {
            bail!("pass --yes to remove these (or --dry-run to only list them)");
        }
        print!("Remove all of this? [y/N] ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        if !matches!(line.trim(), "y" | "Y" | "yes") {
            println!("nothing was removed");
            return Ok(());
        }
    }
    let mut failed = 0;
    for orphan in &orphans {
        match remove(ctx, &project, orphan) {
            Ok(()) => println!("removed: {}", orphan.describe()),
            Err(error) => {
                failed += 1;
                println!("kept: {} ({error:#})", orphan.describe());
            }
        }
    }
    println!("swept `{slug}`: {} removed, {failed} kept", orphans.len() - failed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn porcelain_worktrees_are_parsed() {
        let text = "worktree /repo\nHEAD abc\nbranch refs/heads/main\n\nworktree /wt/x\nHEAD def\nbranch refs/heads/hp/demo/t-0001-x\n\nworktree /wt/detached\nHEAD 123\ndetached\n";
        assert_eq!(parse_worktrees(text), [("/repo".to_string(), "main".to_string()), ("/wt/x".into(), "hp/demo/t-0001-x".into())]);
    }
}

//! Leftovers `doctor` could clean, each with its size and the command that removes it: the worktree of a task that is
//! done or archived (a clean-up keeps one that holds unsaved work), a worktree on a task branch no task has, one git
//! still lists whose folder is gone, and a kept warm build no longer used (scripts/warm-start.sh). Nothing is removed
//! here: you look first.
use super::Found;
use crate::json::*;
use crate::store::marks::tilde;
use crate::store::repos::{git_out, is_repo, known_repos};
use crate::store::settings::lookup::{setting, setting_text};
use crate::store::tasks::{list, ListOpts};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// What it is: the worktree of a finished task, one no task has, one whose folder is gone, an old build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Why { Finished(String), NoTask(String), Gone, OldBuild }
#[derive(Clone, Debug)]
pub struct Left { pub path: PathBuf, pub repo: PathBuf, pub why: Why, pub bytes: u64, pub more: bool }

/// The board's own caches (`${XDG_CACHE_HOME:-~/.cache}/herdr-projects`).
pub fn cache_dir() -> PathBuf {
    std::env::var("XDG_CACHE_HOME").ok().filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| crate::store::files::user_home().join(".cache")).join("herdr-projects")
}
/// The bytes a folder takes on disk, links not followed; `more` when the walk ran out of time first.
fn size(dir: &Path, until: Instant) -> (u64, bool) {
    use std::os::unix::fs::MetadataExt;
    let (mut n, mut todo) = (0u64, vec![dir.to_path_buf()]);
    while let Some(d) = todo.pop() {
        if Instant::now() > until { return (n, true); }
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let Ok(m) = e.metadata() else { continue };
            n += m.blocks() * 512;
            if m.is_dir() { todo.push(e.path()); }
        }
    }
    (n, false)
}
/// A task branch's fixed start (`work/` of `work/{n}-{slug}`): a branch that starts so is the board's.
fn branch_start() -> String { setting_text("workspace.branch", None).split('{').next().unwrap_or("").to_string() }
/// Every leftover, sizes measured for at most a few seconds in all.
pub fn ask(cache: &Path) -> Vec<Left> {
    let mut out: Vec<Left> = vec![];
    let tasks = list(ListOpts { include_archived: true, attempts: true, ..Default::default() });
    let real = |p: &str| std::fs::canonicalize(p).unwrap_or_else(|_| PathBuf::from(p));
    let owned: Vec<PathBuf> = tasks.iter().filter(|t| t.b("worktree")).map(|t| real(t.s("worktree"))).collect();
    for t in &tasks {
        let wt = crate::sync::tree_of(t);
        let finished = t.s("stage") == "done" || t.b("archivedAt");
        // A clean-up still to come (`tidy`), or one you turned off, is no leftover.
        if wt.is_empty() || !finished || t.b("tidy") || setting("workspace.cleanup", Some(t)) != json!(true) { continue; }
        let id = crate::store::keys::say_id(t.s("id"), &crate::store::keys::repo_keys());
        out.push(Left { path: PathBuf::from(&wt), repo: crate::store::repos::folder_of(t.s("id")), why: Why::Finished(format!("{id}, which is {}", if t.b("archivedAt") { "archived" } else { "done" })), bytes: 0, more: false });
    }
    let start = branch_start();
    for (_, f) in known_repos() {
        let repo = PathBuf::from(text(&f));
        if !is_repo(&repo) { continue; }
        let Ok(said) = git_out(&repo, &["worktree", "list", "--porcelain"]) else { continue };
        for (i, w) in said.split("\n\n").enumerate() {
            let get = |k: &str| w.lines().find_map(|l| l.strip_prefix(k)).map(str::to_string);
            let (Some(path), false) = (get("worktree "), i == 0) else { continue };
            let branch = get("branch refs/heads/").unwrap_or_default();
            if w.lines().any(|l| l.starts_with("prunable")) { out.push(Left { path: path.into(), repo: repo.clone(), why: Why::Gone, bytes: 0, more: false }); continue; }
            if start.is_empty() || !branch.starts_with(&start) || owned.contains(&real(&path)) { continue; }
            out.push(Left { path: path.into(), repo: repo.clone(), why: Why::NoTask(branch), bytes: 0, more: false });
        }
    }
    // Kept warm builds no longer `current`, an hour after they last changed (the setup removes them only when it runs).
    for set in std::fs::read_dir(cache.join("warm")).into_iter().flatten().flatten().map(|e| e.path()) {
        let current = std::fs::read_to_string(set.join("current")).unwrap_or_default().trim().to_string();
        for s in std::fs::read_dir(&set).into_iter().flatten().flatten() {
            let (name, old) = (s.file_name().to_string_lossy().to_string(), s.metadata().and_then(|m| m.modified()).map(|t| t.elapsed().unwrap_or_default() > Duration::from_secs(3600)).unwrap_or(false));
            if name.starts_with("seed-") && name != current && old { out.push(Left { path: s.path(), repo: PathBuf::new(), why: Why::OldBuild, bytes: 0, more: false }); }
        }
    }
    out.dedup_by(|a, b| a.path == b.path);
    let until = Instant::now() + Duration::from_secs(3);
    for l in out.iter_mut() { (l.bytes, l.more) = size(&l.path, until); }
    out
}
/// How many are said one by one; the rest in one line.
pub const SHOWN: usize = 8;
pub fn check(left: &[Left]) -> Vec<Found> {
    if left.is_empty() { return vec![Found::ok("No leftover worktrees or build caches to clean")]; }
    let gb = |l: &Left| format!("{}{}", if l.more { "at least " } else { "" }, super::disk::gb(l.bytes));
    let mut out: Vec<Found> = left.iter().take(SHOWN).map(|l| {
        let (p, r) = (tilde(&l.path.display().to_string()), tilde(&l.repo.display().to_string()));
        let remove = format!("Look at it, then: git -C {r} worktree remove {p}  (add --force to drop what is unsaved in it)");
        match &l.why {
            Why::Finished(who) => Found::bad(format!("The worktree of {who}, is still on disk: {p} ({})", gb(l)), remove),
            Why::NoTask(branch) => Found::bad(format!("No task has the worktree on {branch}: {p} ({})", gb(l)), remove),
            Why::Gone => Found::bad(format!("Git still lists the worktree {p}, whose folder is gone"), format!("Run: git -C {r} worktree prune")),
            Why::OldBuild => Found::bad(format!("An old warm build nothing uses: {p} ({})", gb(l)), format!("Run: rm -rf {p}")),
        }
    }).collect();
    if left.len() > SHOWN {
        let rest = &left[SHOWN..];
        out.push(Found::bad(format!("And {} more leftovers ({})", rest.len(), super::disk::gb(rest.iter().map(|l| l.bytes).sum())), "Each the same way as the ones above."));
    }
    out
}

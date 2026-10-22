//! A test merge of main, for a task that waits under `merge.sync approve`: git makes the merge in memory
//! (`merge-tree --write-tree`), so the worktree, the branch and the pull request stay as they are. It only tells
//! whether main would merge cleanly ("clean") or where it conflicts ("conflict", as a real merge says it).
use super::Synced;
use crate::json::*;
use crate::store::repos::git_full;
use crate::store::settings::lookup::setting;
use std::path::Path;

/// Main is merged in only before the review and after your yes, never while the task waits (`merge.sync`).
pub fn only_at_approve(t: &Obj) -> bool { setting("merge.sync", Some(t)) == json!("approve") }
/// What merging `target` into `tip` would give, made in memory: `r` carries what the sync looked at.
pub fn test_merge(tree: &Path, tip: &str, r: Synced) -> Synced {
    let (status, out, err) = git_full(tree, &["merge-tree", "--write-tree", "--name-only", &r.target, tip], 60000);
    if status == 0 { return Synced { status: "clean", ..r }; }
    // 1 is a conflict: the tree, then the files that conflict, a blank line, and git's own words about them.
    let mut parts = out.splitn(2, "\n\n");
    let (head, said) = (parts.next().unwrap_or(""), parts.next().unwrap_or("").trim());
    let mut files: Vec<&str> = vec![];
    for f in head.lines().skip(1).map(str::trim).filter(|f| !f.is_empty()) { if !files.contains(&f) { files.push(f); } }
    if status != 1 || files.is_empty() { return Synced { status: "error", detail: format!("main could not be test-merged.\n\nWhat git said:\n{}", if err.trim().is_empty() { out.trim() } else { err.trim() }), ..r }; }
    Synced { status: "conflict", files: files.join("\n"), restored: true, out: said.to_string(), ..r }
}

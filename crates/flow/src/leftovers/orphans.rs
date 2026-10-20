//! Leftovers no task at work names, found by the sweep: a folder an agent's program kept for a worktree that is gone
//! (`<temp>/claude-<uid>/-home-me--herdr-worktrees-gtm-work-4-…`), and a task's temporary folder whose task is gone.
//! Such a folder is the board's only when that is proven: its name is a worktree folder of a known worktree folder
//! (a parent of the tasks' worktrees, kept in `worktree-roots.json` once seen) and then a name as the task branches
//! start (`work-`); no folder of that name is there; no other folder beside the worktree folder starts so; no task at
//! work has it; and it did not change in the last hour.
use super::guarded;
use crate::agents::scratch::{per_folder, slug, temp_roots};
use crate::json::*;
use crate::store::files::{data_dir, read_json, write_json};
use crate::store::tasks::Task;
use std::path::{Path, PathBuf};

/// A changed folder may be in use by a program just started: an hour's quiet first.
const QUIET_MS: i64 = 3_600_000;
fn roots_file() -> PathBuf { data_dir().join("worktree-roots.json") }
/// The worktree folders the board kept as seen.
pub fn kept_roots() -> Vec<PathBuf> { read_json(&roots_file()).map(|j| strings(&j)).unwrap_or_default().into_iter().map(PathBuf::from).collect() }
fn add_roots(v: &mut Vec<PathBuf>, worktrees: &[PathBuf]) -> bool {
    let before = v.len();
    for up in worktrees.iter().filter_map(|w| w.parent()) { if !up.as_os_str().is_empty() && !v.iter().any(|x| x == up) { v.push(up.to_path_buf()); } }
    if v.len() != before { let _ = write_json(&roots_file(), &json!(v.iter().map(|p| p.display().to_string()).collect::<Vec<_>>())); }
    v.len() != before
}
/// A task's worktree folder, kept as seen, so a folder its programs kept is known as the board's once it is gone.
pub fn remember(worktree: &Path) { let mut v = kept_roots(); add_roots(&mut v, &[worktree.to_path_buf()]); }
/// The worktree folders the board has seen: kept, and those of every task and every git worktree of a known repo.
pub fn worktree_roots(tasks: &[Task]) -> Vec<PathBuf> {
    let mut trees: Vec<PathBuf> = tasks.iter().filter(|t| t.b("worktree")).map(|t| PathBuf::from(t.s("worktree"))).collect();
    for (_, f) in crate::store::repos::known_repos() {
        let Ok(said) = crate::store::repos::git_out(Path::new(&text(&f)), &["worktree", "list", "--porcelain"]) else { continue };
        for w in said.split("\n\n").skip(1) { if let Some(p) = w.lines().find_map(|l| l.strip_prefix("worktree ")) { trees.push(p.into()); } }
    }
    let mut v = kept_roots();
    add_roots(&mut v, &trees);
    v
}
/// How task branches start, as a folder name has it (`work/` is `work-`): of each task's pattern and yours.
pub fn branch_starts(tasks: &[Task]) -> Vec<String> {
    let start = |t: Option<&Task>| crate::store::settings::lookup::setting_text("workspace.branch", t).split('{').next().unwrap_or("").to_string();
    let mut v: Vec<String> = std::iter::once(start(None)).chain(tasks.iter().map(|t| start(Some(t)))).map(|s| slug(Path::new(&s))).filter(|s| s.len() > 1).collect();
    v.sort(); v.dedup();
    v
}
fn names_of(dir: &Path) -> Vec<String> { std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect() }
/// Is `name` the folder `p`, or one inside it?
fn of(name: &str, p: &Path) -> bool { let s = slug(p); name == s || name.starts_with(&format!("{s}-")) }
/// Quiet for an hour by the board's own clock (which a test moves).
fn quiet(p: &Path) -> bool { super::changed_at(p).map(|at| crate::clock::now() - at >= QUIET_MS).unwrap_or(false) }
/// Is the folder free to go: it was quiet, and nothing of the board's environment is in it. What still runs from it
/// (a dev server left behind) is a leftover of a task that ended too, and is stopped as it goes.
fn free_to_go(p: &Path) -> bool { quiet(p) && !guarded(p) }
/// The programs' folders of worktrees that are gone. `working`: the folders tasks at work are in (worktrees and
/// checkouts), which are never a leftover.
pub fn scratch_left(tasks: &[Task], working: &[PathBuf]) -> Vec<PathBuf> {
    let (roots, starts) = (worktree_roots(tasks), branch_starts(tasks));
    if starts.is_empty() { return vec![]; }
    let mut out = vec![];
    for base in temp_roots().iter().flat_map(|r| per_folder(r)) {
        for name in names_of(&base) {
            let Some(root) = roots.iter().find(|r| name.strip_prefix(&format!("{}-", slug(r))).map(|rest| starts.iter().any(|s| rest.starts_with(s.as_str()))).unwrap_or(false)) else { continue };
            if names_of(root).iter().any(|x| of(&name, &root.join(x))) { continue; } // its worktree, or a folder of that name, is there
            let beside = root.parent().map(|up| names_of(up).into_iter().filter(|x| Some(x.as_str()) != root.file_name().and_then(|n| n.to_str())).any(|x| of(&name, &up.join(x)))).unwrap_or(true);
            // A folder at work that holds the worktree folder (your home) says nothing of what is inside it but itself.
            if beside || working.iter().any(|w| name == slug(w) || (!root.starts_with(w) && of(&name, w))) { continue; }
            let p = base.join(&name);
            if p.is_dir() && free_to_go(&p) { out.push(p); }
        }
    }
    out
}
/// The board's task temporary folders no task has any more.
pub fn temp_left(tasks: &[Task]) -> Vec<PathBuf> {
    let base = crate::temp::board_base();
    let named: Vec<PathBuf> = tasks.iter().flat_map(|t| [crate::temp::of(t), PathBuf::from(t.s("tmp"))]).collect();
    names_of(&base).into_iter().map(|n| base.join(n)).filter(|p| p.is_dir() && !named.contains(p) && free_to_go(p)).collect()
}

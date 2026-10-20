//! Old build output in a worktree a done or archived task kept (a clean-up that kept unsaved work, or
//! `workspace.cleanup` off): `target/`, `node_modules/` and `.next/` (at most three folders down), once nothing changed
//! on the task for `board.oldBuilds` days. Only what git ignores goes: never a file the work is made of, and never in a
//! worktree something still runs from but the task's own agent, at rest. The next build makes it again.
use crate::json::*;
use crate::store::repos::git_out;
use crate::store::settings::lookup::setting;
use crate::store::tasks::Task;
use std::path::{Path, PathBuf};

pub const NAMES: [&str; 3] = ["target", "node_modules", ".next"];
const DAY_MS: i64 = 86_400_000;
/// The build folders in a worktree, the ones inside them left out.
pub fn build_dirs(tree: &Path) -> Vec<PathBuf> {
    let (mut out, mut todo) = (vec![], vec![(tree.to_path_buf(), 0)]);
    while let Some((d, depth)) = todo.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if !ft.is_dir() { continue; }
            let name = e.file_name().to_string_lossy().into_owned();
            if NAMES.contains(&name.as_str()) { out.push(e.path()); } else if depth < 2 && name != ".git" { todo.push((e.path(), depth + 1)); }
        }
    }
    out
}
/// Is the task's worktree old enough for its builds to go: ended, quiet for `board.oldBuilds` days (0: never).
pub fn old_enough(t: &Task) -> bool {
    let days = setting("board.oldBuilds", None).as_i64().unwrap_or(7);
    if days <= 0 { return false; }
    let file = crate::store::files::dir_of(t.s("id")).join("task.json");
    let at = super::changed_at(&file).unwrap_or(crate::clock::now()).max(t.i("archivedAt"));
    crate::clock::now() - at >= days * DAY_MS
}
/// The old build output of an ended task's kept worktree that may go. `agent`: the shell of the pane its agent is still
/// open in, at rest: what runs under it does not keep the build.
pub fn old_builds_of(t: &Task, agent: Option<i32>) -> Vec<PathBuf> {
    let tree = PathBuf::from(t.s("worktree"));
    if !t.b("worktree") || !tree.is_dir() || !old_enough(t) || !super::procs::in_folder_but(&tree, agent).is_empty() { return vec![]; }
    if tree == crate::store::repos::folder_of(t.s("id")) { return vec![]; } // the human's own checkout, never
    super::orphans::remember(&tree); // its worktree folder is trusted from here on (leftovers.rs)
    build_dirs(&tree).into_iter().filter(|d| git_out(&tree, &["check-ignore", "-q", &d.display().to_string()]).is_ok()).collect()
}
/// Remove it: the bytes freed.
pub fn free_builds(t: &Task, agent: Option<i32>) -> u64 { old_builds_of(t, agent).iter().map(|d| super::remove(d)).sum() }

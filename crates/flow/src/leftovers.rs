//! What a task leaves behind once it has ended, removed (docs/disk-space.md): its own temporary folder (temp.rs in
//! sw-core) and the folders its agent's and reviewers' programs kept for its worktree (agents/scratch.rs), each once
//! what still runs from it is stopped (leftovers/procs.rs). The sweep (leftovers/sweep.rs) does the same for tasks
//! that ended before, takes old build output out of kept worktrees (leftovers/builds.rs), and a start makes room first
//! on a disk that is nearly full (leftovers/room.rs). Nothing is removed that the board's environment points into.
pub mod builds;
pub mod orphans;
pub mod procs;
pub mod room;
pub mod sweep;

use crate::agents::scratch::scratch_of;
use crate::disk::{gb, size};
use crate::json::*;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A task's own leftovers: its temporary folder, and its programs' folders for its worktree.
pub fn places_of(t: &Obj) -> Vec<PathBuf> {
    let mut v = vec![crate::temp::of(t)];
    if t.b("worktree") { v.extend(scratch_of(Path::new(t.s("worktree")))); }
    v
}
/// Free what a task that ended left: what runs from it stopped, then removed. The bytes freed.
pub fn free(t: &Obj) -> u64 {
    if t.b("worktree") { orphans::remember(Path::new(t.s("worktree"))); }
    places_of(t).iter().map(|p| remove(p)).sum()
}
/// Never removed: a folder this process's environment points into (a PATH entry of yours in an old scratchpad), the
/// home folder, the board's own folders, a known repo, or a folder that holds one of them.
pub fn guarded(p: &Path) -> bool {
    let holds = |x: &Path| x.starts_with(p);
    let home = crate::store::files::user_home();
    if p.parent().is_none() || p == home || p == crate::temp::root() { return true; }
    if std::env::vars_os().any(|(_, v)| std::env::split_paths(&v).any(|x| x.is_absolute() && holds(&x))) { return true; }
    let mine = [crate::store::files::data_dir(), crate::store::files::config_dir(), crate::store::files::reports_dir(), home];
    mine.iter().any(|m| holds(m)) || crate::store::repos::known_repos().values().any(|r| holds(Path::new(&text(r))))
}
/// Stop what runs from a folder, then remove it: the bytes that went.
pub fn remove(p: &Path) -> u64 {
    if !p.is_dir() || guarded(p) { return 0; }
    procs::stop_in(p);
    let took = || size(p, Instant::now() + Duration::from_secs(10)).0;
    let before = took();
    if std::fs::remove_dir_all(p).is_err() { writable(p); let _ = std::fs::remove_dir_all(p); } // a read-only folder in it (Go's module cache)
    before.saturating_sub(if p.exists() { took() } else { 0 })
}
/// Every folder in it writable by its owner, so what is in them can go.
fn writable(p: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut todo = vec![p.to_path_buf()];
    while let Some(d) = todo.pop() {
        let Ok(m) = std::fs::symlink_metadata(&d) else { continue };
        if !m.is_dir() { continue; }
        let _ = std::fs::set_permissions(&d, std::fs::Permissions::from_mode(m.permissions().mode() | 0o700));
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() { todo.push(e.path()); }
    }
}
/// When a file or folder last changed, in the board's milliseconds.
pub fn changed_at(p: &Path) -> Option<i64> { std::fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64) }
/// What the task's line says of it: nothing under a megabyte.
pub fn said(bytes: u64) -> String { if bytes < 1_000_000 { String::new() } else { format!("Freed {} of temporary files.", gb(bytes)) } }
/// `msg` with what was freed after it.
pub fn with_said(msg: &str, bytes: u64) -> String {
    let s = said(bytes);
    if s.is_empty() || msg.contains(&s) { return msg.to_string(); }
    if msg.is_empty() { s } else { format!("{msg}{} {s}", if msg.ends_with('.') { "" } else { "." }) }
}

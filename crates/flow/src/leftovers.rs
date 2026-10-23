//! What a task leaves behind once it has ended, removed (docs/disk-space.md): its own temporary folder (temp.rs in
//! sw-core) and the folders its agent's and reviewers' programs kept for its worktree (agents/scratch.rs), each once
//! what still runs from it is stopped (leftovers/procs.rs; what a stop and a clean-up end first: leftovers/ends.rs).
//! The sweep (leftovers/sweep.rs) does the same for tasks that ended before, takes old build output out of kept
//! worktrees (leftovers/builds.rs), and a start makes room first on a disk that is nearly full (leftovers/room.rs).
//! Nothing is removed that the board's environment points into, and nothing is reached through a link: a folder is
//! removed only where it and each folder above it, up to the temporary, cache or worktree folder it is in, is a real
//! folder (a link in its place is taken away, never followed).
pub mod builds;
pub mod ends;
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
/// The same for a done task whose agent is still open, at rest (`workspace.cleanup` off): everything goes but the
/// folders its program made for itself (Claude Code's `claude-<uid>/<folder>/<session>/scratchpad`), emptied, so the
/// agent goes on working if you talk to it again.
pub fn free_open(t: &Obj) -> u64 {
    let per: Vec<std::ffi::OsString> = crate::agents::scratch::per_folder(Path::new("")).iter().filter_map(|p| p.file_name().map(|n| n.to_os_string())).collect();
    let tmp = crate::temp::of(t);
    let mut n = 0;
    if safe(&tmp) {
        procs::stop_in(&tmp);
        for e in std::fs::read_dir(&tmp).into_iter().flatten().flatten() {
            let (p, dir) = (e.path(), e.file_type().map(|f| f.is_dir()).unwrap_or(false));
            n += if dir && per.contains(&e.file_name()) { thin(&p, 3) } else if dir { remove(&p) } else { gone_file(&p) };
        }
    }
    if t.b("worktree") { for p in scratch_of(Path::new(t.s("worktree"))) { n += thin(&p, 2); } }
    n
}
fn gone_file(p: &Path) -> u64 { use std::os::unix::fs::MetadataExt; let n = std::fs::symlink_metadata(p).map(|m| m.blocks() * 512).unwrap_or(0); if std::fs::remove_file(p).is_ok() { n } else { 0 } }
/// Empty a folder, keeping the folders in it `keep` levels down (emptied too): the bytes that went.
fn thin(p: &Path, keep: usize) -> u64 {
    if !safe(p) { return 0; }
    procs::stop_in(p);
    let mut n = 0;
    for e in std::fs::read_dir(p).into_iter().flatten().flatten() {
        let (q, dir) = (e.path(), e.file_type().map(|f| f.is_dir()).unwrap_or(false));
        n += if dir && keep > 0 { thin(&q, keep - 1) } else if dir { remove(&q) } else { gone_file(&q) };
    }
    n
}
/// The folder it is in that the board trusts: a temporary folder, the build caches, a worktree folder.
fn trusted_top(p: &Path) -> Option<PathBuf> {
    let mut tops = crate::agents::scratch::temp_roots();
    tops.push(crate::doctor::stale::cache_dir());
    tops.extend(orphans::kept_roots());
    tops.into_iter().filter(|t| p.starts_with(t) && p != t).max_by_key(|t| t.components().count())
}
/// Every folder above `p`, up to the trusted folder it is in, is a real folder, not a link: what is named under it is
/// where it is named.
fn above_real(p: &Path) -> bool {
    let Some(top) = trusted_top(p) else { return false };
    !p.ancestors().skip(1).take_while(|a| *a != top).any(|a| std::fs::symlink_metadata(a).map(|m| !m.is_dir() || m.file_type().is_symlink()).unwrap_or(true))
}
/// May the folder be emptied or removed: a real folder all the way down from a trusted folder, and guarded neither as
/// it is named nor where it really is. Nothing is changed in asking.
pub fn removable(p: &Path) -> bool {
    let Ok(m) = std::fs::symlink_metadata(p) else { return false };
    if !m.is_dir() || !above_real(p) { return false; }
    let real = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    !guarded(p) && !guarded(&real)
}
/// `removable`; a link in a temporary folder's place, under real folders only, is taken away (what it points to
/// stays, and nothing runs from it is stopped). A link reached through another link is never touched.
fn safe(p: &Path) -> bool {
    let is_link = std::fs::symlink_metadata(p).map(|m| m.file_type().is_symlink()).unwrap_or(false);
    if is_link && above_real(p) && !guarded(p) { let _ = std::fs::remove_file(p); }
    !is_link && removable(p)
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
    if !safe(p) { return 0; }
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
pub fn with_said(msg: &str, bytes: u64) -> String { then_say(msg, &said(bytes)) }
/// `msg` with the sentences `s` after it, once.
pub fn then_say(msg: &str, s: &str) -> String {
    if s.is_empty() || msg.contains(s) { return msg.to_string(); }
    if msg.is_empty() { s.to_string() } else { format!("{msg}{} {s}", if msg.ends_with('.') { "" } else { "." }) }
}

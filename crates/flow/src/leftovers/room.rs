//! Room on the disk before a task starts (its worktree, its setup, its agent): under `board.diskFree` GB free on a disk the
//! start writes to (the repo's, the worktrees', the board's data, the temporary files, the build caches), the sweep
//! runs first; still under, the task waits in the queue with a plain line instead of failing half-way through a build
//! ("No space left on device"), and starts by itself once there is room again.
use crate::disk::{free_at, gb, GB};
use crate::json::*;
use crate::store::settings::lookup::setting;
use crate::store::tasks::Task;
use std::path::PathBuf;

/// The free space a start needs: `board.diskFree` GB (0: none is looked at).
pub fn needed() -> u64 { setting("board.diskFree", None).as_i64().unwrap_or(5).max(0) as u64 * GB }
/// The disks a start of this task writes to.
fn places(t: &Task) -> Vec<PathBuf> {
    let mut v = vec![crate::store::repos::folder_of(t.s("id")), crate::store::files::data_dir(), crate::temp::root(), crate::doctor::stale::cache_dir()];
    if t.b("worktree") { v.push(PathBuf::from(t.s("worktree"))); }
    v.extend(super::orphans::kept_roots()); // where worktrees are made
    v
}
/// The least free space among them, and where: None while there is room (or nothing is looked at).
pub fn low(t: &Task) -> Option<(u64, PathBuf)> {
    let need = needed();
    if need == 0 { return None; }
    places(t).into_iter().filter_map(|p| free_at(&p).map(|f| (f, p))).filter(|(f, _)| *f < need).min_by_key(|(f, _)| *f)
}
/// This machine's least free space among the disks the board writes to, for `herdr-projects machines`.
pub fn free_here() -> Option<u64> {
    let mut v = vec![crate::store::files::data_dir(), crate::temp::root()];
    v.extend(super::orphans::kept_roots());
    v.extend(crate::store::repos::known_repos().values().map(|r| PathBuf::from(text(r))));
    v.iter().filter_map(|p| free_at(p)).min()
}
/// The task's line while it waits.
pub fn waiting_line(free: u64, at: &std::path::Path) -> String {
    let on = crate::doctor::disk::mount_of(at.ancestors().find(|a| a.exists()).unwrap_or(at));
    format!("Disk nearly full on {}: {} free on {}; this task starts once {} is free (board.diskFree). Free some space: herdr-projects sweep, and herdr-projects doctor lists more that can go.",
        crate::remote::check::hostname(), gb(free), crate::store::marks::tilde(&on.display().to_string()), gb(needed()))
}
/// Before a start: room, or the sweep first (at most once in ten minutes), or the line the task waits with.
pub fn make_room(t: &Task) -> Result<(), String> {
    if low(t).is_none() { return Ok(()); }
    if crate::store::marks::due("sweep-room", 600_000) { super::sweep::sweep(); }
    match low(t) { None => Ok(()), Some((free, at)) => Err(waiting_line(free, &at)) }
}
/// A task that waits for room: still none (the queue leaves it where it is, and starts no runner for it).
pub fn still_waiting(t: &Task) -> bool { t.b("diskWait") && low(t).is_some() }
/// Put back in the queue to wait for room, with its line.
pub fn wait(t: &mut Obj, line: &str) { t.put("stage", "queued"); t.put("msg", line); t.put("diskWait", crate::clock::now()); }

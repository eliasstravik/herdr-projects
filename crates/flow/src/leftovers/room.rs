//! Room on the disk before a task starts (its worktree, its setup, its agent): under `board.diskFree` GB free (never more
//! than half the disk) on a disk the start writes to (the repo's, the worktrees', the board's data, the temporary files,
//! the build caches), the sweep runs first; still under, the task waits in the queue with a plain line instead of failing
//! half-way through a build ("No space left on device"), and starts by itself once there is room again.
use crate::disk::{free_at, gb, total_at, GB};
use crate::json::*;
use crate::store::settings::lookup::setting;
use crate::store::tasks::Task;
use std::path::PathBuf;

/// The free space a start needs: `board.diskFree` GB (0: none is looked at).
pub fn needed() -> u64 { setting("board.diskFree", None).as_i64().unwrap_or(5).max(0) as u64 * GB }
/// What a disk of this whole size must keep free: `board.diskFree`, never more than half of it, so that a small disk (a
/// `/tmp` of 3.4 GB) can have room at all. Its size not known: `board.diskFree`.
pub fn need_on(total: Option<u64>) -> u64 { let n = needed(); total.map_or(n, |t| n.min(t / 2)) }
/// The disks a start of this task writes to.
fn places(t: &Task) -> Vec<PathBuf> {
    let mut v = vec![crate::store::repos::folder_of(t.s("id")), crate::store::files::data_dir(), crate::temp::root(), crate::doctor::stale::cache_dir()];
    if t.b("worktree") { v.push(PathBuf::from(t.s("worktree"))); }
    v.extend(super::orphans::kept_roots()); // where worktrees are made
    v
}
/// A disk without the room a start needs: what it has free, what it needs, its whole size, and a place on it.
#[derive(Clone, Debug)]
pub struct Low { pub free: u64, pub need: u64, pub total: Option<u64>, pub at: PathBuf }
impl Low {
    /// The folder the disk is mounted at.
    pub fn mount(&self) -> PathBuf { crate::doctor::disk::mount_of(self.at.ancestors().find(|a| a.exists()).unwrap_or(&self.at)) }
    /// The task's line while it waits.
    pub fn line(&self) -> String { waiting_line(self) }
}
/// The disk with the least free space among those without room, None while there is room (or nothing is looked at).
pub fn low(t: &Task) -> Option<Low> {
    if needed() == 0 { return None; }
    places(t).into_iter().filter_map(|at| { let free = free_at(&at)?; let total = total_at(&at); Some(Low { free, need: need_on(total), total, at }) })
        .filter(|l| l.free < l.need).min_by_key(|l| l.free)
}
/// This machine's least free space among the disks the board writes to, for `herdr-projects machines`.
pub fn free_here() -> Option<u64> {
    let mut v = vec![crate::store::files::data_dir(), crate::temp::root()];
    v.extend(super::orphans::kept_roots());
    v.extend(crate::store::repos::known_repos().values().map(|r| PathBuf::from(text(r))));
    v.iter().filter_map(|p| free_at(p)).min()
}
/// The task's line while it waits: what it needs, and plainly why when that is less than `board.diskFree`.
fn waiting_line(l: &Low) -> String {
    let on = crate::store::marks::tilde(&l.mount().display().to_string());
    let why = match l.total { Some(total) if l.need < needed() => format!("{on} is only {} in all, so it needs {} free, not the {} of board.diskFree", gb(total), gb(l.need), gb(needed())), _ => "board.diskFree".into() };
    format!("Disk nearly full on {}: {} free on {on}; this task starts once {} is free ({why}). Free some space: herdr-projects sweep, and herdr-projects doctor lists more that can go.",
        crate::remote::check::hostname(), gb(l.free), gb(l.need))
}
/// Before a start: room, or the sweep first (at most once in ten minutes), or the disk the task waits for.
pub fn make_room(t: &Task) -> Result<(), Low> {
    if low(t).is_none() { return Ok(()); }
    if crate::store::marks::due("sweep-room", 600_000) { super::sweep::sweep(); }
    match low(t) { None => Ok(()), Some(l) => Err(l) }
}
/// A task that waits for room: still none (the queue leaves it where it is, and starts no runner for it).
pub fn still_waiting(t: &Task) -> bool { t.b("diskWait") && low(t).is_some() }
/// Put back in the queue to wait for room, with its line, and since when and for which disk (`herdr-projects doctor`
/// names it beside that disk).
pub fn wait(t: &mut Obj, l: &Low) { t.put("stage", "queued"); t.put("msg", l.line()); t.put("diskWait", json!({ "at": crate::clock::now(), "on": l.mount().display().to_string() })); }

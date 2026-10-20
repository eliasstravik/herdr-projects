//! The sweep: what tasks that ended left behind, removed (docs/disk-space.md). The temporary folders of done and
//! archived tasks whose agent is gone, the folders agents' programs kept for worktrees that are gone (orphans.rs),
//! the board's task folders no task has, old build output in kept worktrees (builds.rs) and old warm builds nothing
//! uses. A task at work, its worktree and its folders are never touched. It runs when the board starts (when the last
//! sweep is over an hour old), once a day from the housekeeping pass, before a start on a nearly full disk (room.rs),
//! and as `herdr-projects sweep`; one at a time, and what it freed is kept in `sweep.json`.
use super::{builds, orphans, remove};
use crate::disk::gb;
use crate::herdr::{cli::socket_exists, Herdr};
use crate::json::*;
use crate::store::files::{data_dir, with_lock_wait, write_json};
use crate::store::marks::{due, tilde};
use crate::store::repos::cwd_of;
use crate::store::tasks::{list, ListOpts, Task};
use std::path::PathBuf;

const HOUR_MS: i64 = 3_600_000;
pub fn ended(t: &Task) -> bool { t.s("stage") == "done" || t.b("archivedAt") }
/// Where a task's agent is: gone (no pane, its herdr session not running, so its panes went with it, or herdr says
/// none is there), still open and at rest (with the pid of its pane's shell), or at work. What herdr cannot say is
/// at work.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Agent { Gone, Open(Option<i32>), Busy }
pub fn agent_now(t: &Task) -> Agent {
    let l = t.at("link");
    if !l.b("pane") || !l.b("socket") || !socket_exists(l.s("socket")) { return Agent::Gone; }
    let api = Herdr::new(Some(l.t("socket")));
    match api.agent_get(l.s("pane")) {
        Ok(None) => Agent::Gone,
        Ok(Some(a)) if ["idle", "done"].contains(&a.s("agent_status")) => Agent::Open(api.shell_pid(l.s("pane")).map(|p| p as i32)),
        _ => Agent::Busy,
    }
}
pub fn agent_gone(t: &Task) -> bool { agent_now(t) == Agent::Gone }
/// Free what a task that just ended left, as its agent is: all of it once the agent is gone, all but what the agent's
/// program keeps for itself while it is open at rest, nothing while it works.
pub fn free_ended(t: &Task) -> u64 { match agent_now(t) { Agent::Gone => super::free(t), Agent::Open(_) => super::free_open(t), Agent::Busy => 0 } }
/// An ended task the sweep may tidy: no job of the board's on it, no clean-up still to come.
fn at_rest(t: &Task) -> bool { ended(t) && !crate::runner::runner_alive(t) && !t.at("tidy").is_object() }
/// Nothing changed on the task for an hour: its open agent is not talked to.
fn quiet(t: &Task) -> bool { super::changed_at(&crate::store::files::dir_of(t.s("id")).join("task.json")).map(|at| crate::clock::now() - at >= HOUR_MS).unwrap_or(false) }
/// The folders of tasks at work, and your repos: never a leftover.
fn working(tasks: &[Task]) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = tasks.iter().filter(|t| !ended(t)).flat_map(|t| [cwd_of(t), PathBuf::from(t.s("worktree"))]).filter(|p| !p.as_os_str().is_empty()).collect();
    v.extend(crate::store::repos::known_repos().values().map(|r| PathBuf::from(text(r))));
    v
}
/// Sweep now: what went, each with its size. `{"busy": true}` while another sweep runs.
pub fn sweep() -> Value { sweep_with(false) }
/// `dry`: only what would go, each with its size; nothing is stopped or removed.
pub fn sweep_with(dry: bool) -> Value {
    let remove = |p: &std::path::Path| if dry { if super::removable(p) { crate::disk::size(p, std::time::Instant::now() + std::time::Duration::from_secs(10)).0.max(1) } else { 0 } } else { remove(p) };
    let r = with_lock_wait("sweep", 0, || {
        let tasks = list(ListOpts { include_archived: true, attempts: true, ..Default::default() });
        let mut went: Vec<Value> = vec![];
        let mut add = |what: &str, p: &std::path::Path, n: u64| { if n > 0 { went.push(json!({ "what": what, "path": p.display().to_string(), "bytes": n })); } };
        for t in &tasks {
            let there: Vec<PathBuf> = super::places_of(t).into_iter().filter(|p| p.is_dir()).collect();
            if (there.is_empty() && !std::path::Path::new(t.s("worktree")).is_dir()) || !at_rest(t) { continue; } // herdr is asked only where there is something
            let shell = match agent_now(t) {
                Agent::Gone => { for p in there { let n = remove(&p); add("temporary files of an ended task", &p, n); } None }
                // Open at rest (`workspace.cleanup` off): all but its program's own folders, once nobody talked to it for an hour.
                Agent::Open(shell) if quiet(t) => { let n = if dry { there.iter().map(|p| remove(p)).sum() } else { super::free_open(t) }; add("temporary files of a done task whose agent is open", &crate::temp::of(t), n); shell }
                _ => continue,
            };
            let n = if dry { builds::old_builds_of(t, shell).iter().map(|d| remove(d)).sum() } else { builds::free_builds(t, shell) };
            add("old build output in a kept worktree", std::path::Path::new(t.s("worktree")), n);
        }
        let live = working(&tasks);
        for p in orphans::scratch_left(&tasks, &live) { let n = remove(&p); add("an agent's files for a worktree that is gone", &p, n); }
        for p in orphans::temp_left(&tasks) { let n = remove(&p); add("temporary files of a task that is gone", &p, n); }
        for p in crate::doctor::stale::old_builds(&crate::doctor::stale::cache_dir()) { let n = remove(&p); add("an old warm build", &p, n); }
        let freed: u64 = went.iter().map(|w| w.i("bytes") as u64).sum();
        let out = json!({ "at": crate::clock::now(), "freed": freed, "went": went, "dry": dry });
        if !dry { let _ = write_json(&data_dir().join("sweep.json"), &out); }
        Ok(out)
    });
    r.unwrap_or_else(|_| json!({ "busy": true }))
}
/// What a sweep did, in words.
pub fn say(r: &Value) -> String {
    if r.b("busy") { return "Another sweep is running.".into(); }
    if r.arr("went").is_empty() { return "Nothing to remove: no leftovers of ended tasks.".into(); }
    let mut lines = vec![format!("{} {}:", if r.b("dry") { "Would free" } else { "Freed" }, gb(r.i("freed") as u64))];
    lines.extend(r.arr("went").iter().map(|w| format!("  {}  {} ({})", gb(w.i("bytes") as u64), tilde(w.s("path")), w.s("what"))));
    lines.join("\n")
}
/// A test world sweeps only when it asks to.
fn allowed() -> bool { std::env::var("HERDR_PROJECTS_NO_SWEEP").map(|v| v.is_empty()).unwrap_or(true) }
fn start() { if crate::runner::inline() { crate::runner::JOBS.with(|j| j.borrow_mut().push(("sweep".into(), "-".into(), vec![]))); } else { crate::housekeeping::spawn_loose(&["run", "sweep", "-"]); } }
/// The board opening: a sweep, when the last one is over an hour old.
pub fn at_start() { if allowed() && due("sweep", HOUR_MS) { start(); } }
/// The housekeeping pass: a sweep once a day.
pub fn daily() { if allowed() && due("sweep", 24 * HOUR_MS) { start(); } }

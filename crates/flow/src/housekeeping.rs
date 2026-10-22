//! The housekeeping pass. It rides on every hook run (for any pane, in any task), every board tick and the startup
//! hook, rate-limited by timestamps in the data folder, so nothing important depends on the board being open:
//! start the jobs asked for from inside a sandbox or by a command (runner/outside.rs), start queued tasks when a slot is
//! free (and the ones that wait for others, once those are done: after.rs), reconcile each linked task with herdr, try
//! a start whose runner was lost again (hook/lost.rs), bring waiting tasks up to date when main moved, and poll open
//! pull requests (housekeeping/prs.rs). One pass per herdr session at a time (housekeeping/once.rs).
mod once;
mod prs;
pub use prs::poll_prs;

use crate::actions::start::start_queued;
use crate::clock::now;
use crate::finish::{auto_merge::{auto_merge_pass, green}, line::line_pass};
use crate::flow::{send::resume_cut, spaces::{sweep_closing, sweep_repo_spaces}, task::is_agent_task};
use crate::herdr::{cli::socket_exists, Herdr};
use crate::json::*;
use crate::runner::{inline, runner_alive, spawn_runner, start_reaped, user_env, detach, JOBS};
use crate::store::{files::{bin, data_dir, has_tasks, RUNNER_STAGES}, marks::due, old_suggestions::save_old_suggestions, repos::folder_of, retries::waiting_of, settings::lookup::setting, tasks::{get, list, ListOpts, Task}};
use crate::sync::{fetch_main, known_target, main_branch, resyncable, tree_of, via_remote};
use std::collections::HashMap;

fn key(s: &str) -> String { sha1_smol::Sha1::from(s).digest().to_string()[..8].to_string() }
#[derive(PartialEq, Clone, Copy)]
pub enum Force { No, Yes, Pr }

pub fn housekeeping(socket: &str, force: Force) {
    if !has_tasks() { return; }
    crate::runner::outside::start_asked(); // jobs asked for from inside a plan writer's sandbox, started out here
    if force == Force::No && !due(&format!("housekeep-{}", key(socket)), 2000) { return; }
    once::once(&key(socket), force, |f| pass(socket, f));
}
fn pass(socket: &str, force: Force) {
    // What an older version kept of suggested tasks is written down and taken off the tasks, once (store/old_suggestions.rs).
    let _ = save_old_suggestions();
    // With the attempts of tasks done several times at once: each has an agent of its own to look after (attempts.rs).
    let every = list(ListOpts { attempts: true, ..Default::default() });
    let tasks: Vec<Task> = every.iter().filter(|t| !t.b("attemptOf")).cloned().collect();
    if socket_exists(socket) {
        let linked: Vec<&Task> = every.iter().filter(|t| is_agent_task(t) && t.at("link").s("socket") == socket && !["done", "todo", "progress", "queued", "after"].contains(&t.s("stage"))).collect();
        if !linked.is_empty() {
            let api = Herdr::new(Some(socket.into()));
            if api.workspaces().is_ok() { for t in linked { crate::hook::reconcile(&get(t.s("id")).unwrap_or_else(|| t.clone()), &api); } }
        }
        // A start whose runner was lost before the task had a pane: tried again once, then failed (hook/lost.rs).
        for t in every.iter().filter(|t| !is_agent_task(t) && t.at("link").s("socket") == socket && RUNNER_STAGES.contains(&t.s("stage")) && !t.b("archivedAt")) {
            if let Some(t) = get(t.s("id")) { crate::hook::retry_lost(&t); }
        }
        // A task that waited for others to be done goes on to a slot once they are (after.rs).
        let woke = crate::after::pass::after_pass(&tasks, socket);
        if woke || every.iter().any(|t| t.s("stage") == "queued") { start_queued(socket); }
        // Attempts that are all through: the task fails, goes on with the one that passed, or waits for a pick.
        crate::attempts::settle::settle_pass(&tasks, socket, now());
        if due(&format!("spaces-{}", key(socket)), 30000) { sweep_repo_spaces(socket); sweep_closing(socket); }
        // A clean-up that found the task's space in use is tried again, so a done task leaves the sidebar by itself once
        // the space is free; after a turn in the agent's own terminal only when that has been quiet for a while.
        let quiet: i64 = std::env::var("HERDR_PROJECTS_QUIET_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(600000);
        for t in &tasks {
            if t.s("stage") != "done" || setting("workspace.cleanup", Some(t)) != json!(true) || !t.b("tidy") || t.b("archivedAt") || t.at("link").s("socket") != socket || runner_alive(t) || now() - t.at("tidy").i("quietFrom") < quiet { continue; }
            // Read again: the list is from the pass's start, and a clean-up started since may still run (it can take
            // longer than the half minute between tries).
            if get(t.s("id")).is_none_or(|c| runner_alive(&c)) { continue; }
            if due(&format!("tidy-{}", key(t.s("id"))), 30000) { spawn_runner("cleanup", t.s("id"), if t.at("tidy").s("deleteBranch") == "merged" { &["--merged"] } else { &[] }); }
        }
        crate::notify::notify_pass(&list(ListOpts::default()));
        resync(&tasks, socket);
        // A chat that changed the code, and you are done with it: its one review (finish/chat.rs).
        crate::finish::chat::chat_pass(&tasks, socket);
        // `merge.by` changed under a task that waits: it merges now, or waits for your yes after all.
        auto_merge_pass(&tasks, socket);
        // Tasks that wait their turn to merge: the next one starts when the one before it is through.
        line_pass(&tasks);
        // An agent whose turn a passing provider error cut off is told to carry on once its wait is over.
        for t in &every {
            if let Some(w) = waiting_of(t) { if w.s("who") == "agent" && t.at("link").s("socket") == socket && now() >= w.i("until") && !runner_alive(t) { resume_cut(t.s("id")); } }
        }
    }
    crate::leftovers::sweep::daily(); // what ended tasks left behind, once a day
    let watch = tasks.iter().any(|t| (t.b("pr") && !t.b("prMerged")) || (resyncable(t) && t.b("worktree")));
    if watch && (force == Force::Pr || due("prpoll", 120000)) {
        if inline() { JOBS.with(|j| j.borrow_mut().push(("prpoll".into(), "-".into(), vec![]))); } else { spawn_loose(&["run", "prpoll", "-"]); }
    }
}
/// Main moved (another task merged, or anything else): every task that waits for approval is brought up to date in
/// its own worktree, each by a runner of its own. A task under review is reached the moment its review ends. Only
/// what git already knows locally is read here; the periodic fetch below and every merge keep that current.
fn resync(tasks: &[Task], socket: &str) {
    let mut known = HashMap::new();
    for t in tasks {
        if !resyncable(t) || t.at("link").s("socket") != socket || tree_of(t).is_empty() || runner_alive(t) { continue; }
        // A task that merges by itself is not brought up to date here, with every other one at once: it catches up when
        // its turn to merge comes (finish/line.rs), and so does that only once.
        if setting("merge.by", Some(t)) == json!("itself") && green(t) { continue; }
        let target = known_target(t, &mut known);
        if !target.is_empty() && (target != t.s("syncSeen") || t.b("syncBlocked")) && now() >= t.i("syncRetryAt") { spawn_runner("sync", t.s("id"), &[]); }
    }
}
/// Ask the remote where main is, for every repo with a task waiting: main also moves without the board (a merge on
/// GitHub, your own push). A fetch that fails says nothing about main, so nothing may go on looking ready: each waiting
/// task of that repo gets a sync runner anyway, which finds the same failure and holds it "behind main".
pub fn fetch_mains() {
    let mut done: HashMap<String, bool> = HashMap::new();
    for t in list(ListOpts::default()) {
        if !resyncable(&t) || tree_of(&t).is_empty() || runner_alive(&t) { continue; }
        let root = folder_of(t.s("id"));
        if !via_remote(&root, &t) { continue; }
        let b = main_branch(&root, &t);
        let k = format!("{}|{b}", root.display());
        let ok = *done.entry(k).or_insert_with(|| fetch_main(&root, &b).is_ok());
        if !ok && !t.b("syncBlocked") { spawn_runner("sync", t.s("id"), &["--force"]); }
    }
}
/// A detached job that belongs to no single task.
pub fn spawn_loose(a: &[&str]) -> u32 { start_reaped(&mut loose(a)) }
/// The command `spawn_loose` starts, for a caller that follows what became of it (the keeper's successor): that caller
/// keeps the `Child` and waits for it itself.
pub fn loose(a: &[&str]) -> std::process::Command {
    use std::process::{Command, Stdio};
    user_env();
    let _ = std::fs::create_dir_all(data_dir());
    let mut c = Command::new(bin());
    c.args(a).current_dir(data_dir()).stdin(Stdio::null());
    for k in ["HERDR_PLUGIN_EVENT", "HERDR_PLUGIN_EVENT_JSON", "HERDR_PLUGIN_CONTEXT_JSON", "HERDR_PANE_ID", "HERDR_TAB_ID", "HERDR_WORKSPACE_ID"] { c.env_remove(k); }
    match std::fs::OpenOptions::new().create(true).append(true).open(data_dir().join("jobs.log")) {
        Ok(f) => { let g = f.try_clone().ok(); c.stdout(f); if let Some(g) = g { c.stderr(g); } }
        Err(_) => { c.stdout(Stdio::null()).stderr(Stdio::null()); }
    }
    detach(&mut c);
    c
}

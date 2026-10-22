//! A runner lost before it finished its step: its marker's process is dead, or a start that was asked for never got
//! a runner at all (the command that delegated was ended with everything it started, before the runner took the task:
//! a systemd oneshot ends its whole cgroup, #312). The step is tried again once; the second time the task fails, with
//! what happened and how to start it again. Looked at for every task in a runner's stage, with a pane or not yet.
use super::reconcile::upd;
use crate::board::keys::key_name;
use crate::clock::now;
use crate::json::*;
use crate::runner::spawn_runner;
use crate::store::{failures::failure_patch, files::{marker_alive, RUNNER_STAGES}, tasks::{update_wait, Change, Task}};

/// How long a start may wait for its runner to take the task.
pub const GRACE_MS: i64 = 30000;
/// The stages a start's runner stands in (delegate/jobs.rs).
const START: [&str; 2] = ["starting", "setup"];
/// When its runner was last asked for: put in the queue, taken from it, or tried again.
fn asked_at(t: &Task) -> i64 { [t.i("startAsked"), t.i("queuedAt"), t.i("since")].into_iter().max().unwrap_or(0) }
/// The stage a start is asked for again in, and when.
pub fn ask_start(t: &mut Task) { t.put("stage", "starting"); t.put("startAsked", now()); }
fn lost(t: &Task) -> bool {
    if t.at("runner").is_object() { return !marker_alive(t.at("runner")); }
    START.contains(&t.s("stage")) && now() - asked_at(t) > GRACE_MS
}
/// A lost runner's marker is cleared and its step tried again once; the next time the task fails. Returns whether
/// it was lost (the caller then leaves the task alone for this pass).
pub fn retry_lost(t: &Task) -> bool {
    if !lost(t) { return false; }
    let id = t.s("id");
    let mut was = false;
    let Ok(cur) = update_wait(id, 10000, |t| {
        if !lost(t) { return Ok(Change::Keep); }
        was = true;
        t.put("runner", Value::Null); let r = t.i("retried") + 1; t.put("retried", r);
        Ok(Change::Write)
    }) else { return false };
    if !was { return false; }
    let stage = cur.t("stage");
    let auto = stage == "merging" && cur.b("autoMerge"); // a remembered auto-merge runs nothing
    if cur.b("runner") || !RUNNER_STAGES.contains(&stage.as_str()) || auto { return true; }
    let job = if stage == "merging" { "land" } else if START.contains(&stage.as_str()) { "delegate" } else { "pipeline" };
    if cur.i("retried") <= 1 {
        if job == "delegate" { upd(id, |t| { ask_start(t); true }); }
        if job == "pipeline" { upd(id, |t| { t.put("stage", "checking"); true }); }
        spawn_runner(job, id, &[]);
        return true;
    }
    let f = if job == "delegate" { cut_start() } else {
        json!({ "stage": "workflow", "title": "A workflow step stopped unexpectedly", "detail": format!("The {} step ended without finishing, twice.", t.at("runner").s("job")), "next": "Reply to the agent or delegate again to retry.", "auth": false })
    };
    let msg = if job == "delegate" { "Could not start: its start was cut off, twice".to_string() } else { f.t("title") };
    upd(id, |t| {
        t.put("stage", "failed"); t.put("msg", msg);
        for (k, v) in failure_patch(&obj(f)) { t.insert(k, v); }
        true
    });
    true
}
/// The failure of a start that was cut off twice: what happened, in plain words, and how to start it again.
fn cut_start() -> Value {
    json!({ "stage": "start", "title": "The start was cut off", "agentCanFix": false, "auth": false,
        "detail": "The job that starts this task (its worktree, its space and its agent) ended before it was done, twice, with nothing said: it was ended from outside. Most often that is the command that delegated the task, run from a script or a service that ends everything it started when it ends itself (a systemd oneshot).",
        "next": format!("Delegate again ({}). From a script or a service: while herdr runs on this machine, its keeper does, and the start is handed to it, so it no longer ends with the command.", key_name("delegate")) })
}

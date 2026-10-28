//! The waits' own pass, run by whoever ticks (the keeper, an open board, any command, through housekeeping.rs): a task
//! whose tasks are all done goes on to a slot by itself, one whose task went wrong waits for you with the reason, and
//! the words on its row stay current. Nothing is kept of a reason: once that task is set going again and done after
//! all, the wait goes on by itself.
use super::{handed::hand_over, standing, waits_line, STAGE};
use crate::json::*;
use crate::store::tasks::{update, Change, Task};

/// Every waiting task of this herdr session brought up to date; true when one of them is now queued.
pub fn after_pass(tasks: &[Task], socket: &str) -> bool {
    // The machines that tasks here wait on are asked by a job of its own (after/far_ask.rs), now and then.
    if super::far_ask::waits_far(tasks) && crate::store::marks::due("after-far", super::far_ask::EVERY_MS) {
        if crate::runner::inline() { crate::runner::JOBS.with(|j| j.borrow_mut().push(("afterfar".into(), "-".into(), vec![]))); } else { crate::housekeeping::spawn_loose(&["run", "afterfar", "-"]); }
    }
    let mut woke = false;
    for t in tasks.iter().filter(|t| t.s("stage") == STAGE && crate::herdr::cli::same_socket(t.at("link").s("socket"), socket) && !t.b("archivedAt")) { if refresh(t.s("id")) { woke = true; } }
    woke
}
/// One waiting task brought up to date; true when its wait is over and it is queued.
pub fn refresh(id: &str) -> bool {
    let mut queued = false;
    let _ = update(id, |t| {
        if t.s("stage") != STAGE || t.b("archivedAt") { return Ok(Change::Keep); }
        let (left, broken) = standing(t);
        if left.is_empty() { hand_over(t, false); queued = true; return Ok(Change::Write); }
        let msg = broken.clone().unwrap_or_else(|| waits_line(&left));
        let blocked = broken.map(Value::from).unwrap_or(Value::Null);
        if t.s("msg") == msg && t.at("afterBlocked") == &blocked && t.at("afterLeft") == &json!(left) { return Ok(Change::Keep); }
        if t.at("afterBlocked") != &blocked { t.put("at", crate::clock::now()); } // it waits for you from now, or no more
        t.put("msg", msg); t.put("afterBlocked", blocked); t.put("afterLeft", left);
        Ok(Change::Write)
    });
    queued
}

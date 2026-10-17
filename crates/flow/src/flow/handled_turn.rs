//! A turn that is already handled: the same turn seen again, and a task re-armed for a turn that never came.
use super::task::note_stat;
use crate::agents::installed::read_turn_of;
use crate::json::*;
use crate::store::repos::cwd_of;
use std::path::Path;

/// The saved conversation still ends in the message the finish line already handled.
pub fn same_turn(t: &Obj) -> bool {
    let _account = crate::agents::account::task(t, "agent");
    if !t.at("handled").b("msg") { return false; }
    let file = t.so("transcript").map(Path::new);
    read_turn_of(t.s("agent"), file, t.s("session"), &cwd_of(t), false).map(|turn| turn.id == t.at("handled").s("msg")).unwrap_or(false)
}
fn reopen_ms() -> i64 { std::env::var("HERDR_PROJECTS_REOPEN_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(60000) }
/// A task opened for a turn that never came: nobody is known to have opened the turn, it was armed within seconds of
/// the handled turn, and herdr counts no turn finished since, and the agent has written nothing since.
pub fn reopened(t: &Obj, comp: &Value) -> bool {
    let h = t.at("handled");
    if t.s("stage") != "working" || comp.is_null() || !h.is_object() || comp != h.at("comp") || !t.b("turnOpen") || t.b("turnBy") || t.b("expectedWake") { return false; }
    let gap = t.i("armedAt") - h.i("at");
    if !(0..=15000).contains(&gap) || crate::clock::now() - t.i("armedAt") < reopen_ms() { return false; }
    if h.b("msg") { return same_turn(t); }
    note_stat(t).map(|n| json!(n.0) == *h.at("note") || n.0 == h.f("note")).unwrap_or(false)
}

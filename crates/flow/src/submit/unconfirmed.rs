//! Text that herdr's counters alone call taken: they moved since it was typed, but herdr never saw the agent work and
//! its saved conversation does not hold the words. A program that only redraws its screen moves them too. The board
//! keeps a light mark (`unconfirmed`) that holds nothing back, and confirms it within a few minutes: an agent still at
//! rest whose saved conversation and pane do not show the text never got it, and the task fails as not started.
use super::give_up::{fail, mine, on_screen, screen};
use super::{said_of, took, wait_ms, working, LOOKED};
use crate::board::keys::key_name;
use crate::clock::now;
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::tasks::{update_wait, Change, Task};

/// How long a move of the counters alone waits for a proof that the agent took the text: 3 minutes.
fn bound_ms() -> i64 { super::ms("HERDR_PROJECTS_CONFIRM_MS", 180_000) }
/// Only herdr's counters moved since the text was typed: no work seen, nothing read.
pub fn only_moved(m: &Value, a: &Value) -> bool { !working(a) && took(m, Some(a), None) == Some(true) }
/// What was typed is taken: `sent` goes. Taken on the counters' word alone, a light mark stays, to be confirmed.
pub fn taken(c: &mut Task, a: &Value) {
    let m = c.at("sent").clone();
    c.put("sent", Value::Null);
    if let (Some(mut u), true) = (m.as_object().cloned(), only_moved(&m, a)) { u.put("seen", now()); c.put("unconfirmed", u); }
}
/// One look, on the housekeeping pass. Never waits; the pane is read once, when the bound is reached.
pub fn check(t: &Task, a: Option<&Value>, api: &Herdr) {
    let u = t.at("unconfirmed").clone();
    if !u.is_object() { return; }
    let id = t.s("id");
    let drop = || {
        LOOKED.lock().unwrap().remove(id);
        let _ = update_wait(id, 10000, |c| { if !mine(c, "unconfirmed", &u) { return Ok(Change::Keep); } c.del("unconfirmed"); Ok(Change::Write) });
    };
    if t.s("stage") != "working" { return drop(); } // stopped, or the finish line took a turn that ended
    let Some(a) = a else { return }; // its program ended: the row says so
    if !only_moved(&u, a) || t.i("workedAt") > u.i("at") { return drop(); } // it works or has worked since, or herdr restarted
    let due = now() - u.i("seen") >= bound_ms();
    let last = LOOKED.lock().unwrap().get(id).copied().unwrap_or(0);
    if due || now() - last >= wait_ms(None) { LOOKED.lock().unwrap().insert(id.into(), now()); if said_of(t, &u) == Some(true) { return drop(); } }
    if !due { return; }
    if !["idle", "done"].contains(&a.s("agent_status")) { return drop(); } // herdr cannot say it is at rest: nothing to confirm with
    let shown = screen(api, t.at("link").s("pane"), Some(a.s("agent_status")));
    if on_screen(&shown, u.s("text")) { return drop(); }
    let mins = ((now() - u.i("at")) as f64 / 60000.0).round().max(1.0) as i64;
    fail(id, "unconfirmed", &u, &format!("The board typed the text into the agent's box {mins} minute{} ago. Since then herdr's counters for it moved, but it never worked, and neither its saved conversation nor its screen holds the text: it never reached the agent.", if mins == 1 { "" } else { "s" }),
        &format!("Retry ({}) starts the agent's program again and hands it the text once more.", key_name("yes")), &shown);
    LOOKED.lock().unwrap().remove(id);
}

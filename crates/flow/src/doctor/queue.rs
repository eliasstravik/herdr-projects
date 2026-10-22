//! Messages that wait undelivered, for `doctor`: each coordinator of this herdr session whose queue still holds a
//! message sent a while ago (one sent just now is the pump at work), a problem with how many and what to do.
use super::Found;
use crate::coordinator::{co::Co, state};
use crate::json::*;

/// A message queued longer than this is no longer on its way.
pub const LATE_MS: i64 = 60_000;
/// One coordinator with messages that wait: its name, how many, and when the oldest was sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Waiting { pub name: String, pub count: usize, pub oldest: i64 }
/// Each coordinator of the session `socket` whose queue holds a message older than LATE_MS at `now`.
pub fn ask(socket: &str, now: i64) -> Vec<Waiting> {
    state::all(socket).into_iter().filter_map(|co: Co| {
        let late: Vec<i64> = state::get(&co)?.arr("queue").iter().map(|m| m.i("at")).filter(|at| now - at > LATE_MS).collect();
        let oldest = *late.iter().min()?;
        Some(Waiting { name: co.name(), count: late.len(), oldest })
    }).collect()
}
pub fn check(waiting: &[Waiting], now: i64) -> Vec<Found> {
    if waiting.is_empty() { return vec![Found::ok("No message waits undelivered for a coordinator")]; }
    waiting.iter().map(|w| {
        let mins = (now - w.oldest) / 60_000;
        Found::bad(format!("{} message{} wait{} in the {}'s queue, not delivered (the oldest sent {} ago)", w.count, if w.count == 1 { "" } else { "s" }, if w.count == 1 { "s" } else { "" }, w.name, if mins < 1 { "under a minute".to_string() } else if mins == 1 { "1 minute".to_string() } else { format!("{mins} minutes") }),
            "Open the board in herdr: it delivers them. Sent from a shell that cannot find herdr? Set HERDR_BIN_PATH to herdr's full path there.")
    }).collect()
}

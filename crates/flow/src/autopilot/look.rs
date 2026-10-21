//! The free look: before a timer wakes a coordinator, the board asks itself whether anything changed in its area since
//! its last wake ended, at no cost (no model, no tokens). What it compares: the tasks in its area and their stages, its
//! goal and settings, and whether today's limits have room (a new day with room counts as a change). The snapshot is
//! taken when a wake's turn ends, or when the wake is judged unanswered, so what its own turn did wakes nothing.
use super::care::in_area;
use super::limits::room;
use super::record::{day_of, scope_of};
use crate::coordinator::co::Co;
use crate::json::*;
use crate::store::settings::lookup::lookup;
use crate::store::settings::places::yours;
use crate::store::settings_list::coord;

/// What the board is now, for this coordinator, in one string.
pub fn snapshot(co: &Co, ap: &Obj, board: &[Obj], now: i64) -> String {
    let mut tasks: Vec<String> = board.iter().filter(|t| in_area(co, t)).map(|t| format!("{}={}:{}:{}", t.s("id"), t.s("stage"), t.b("archivedAt"), t.b("mergedAt"))).collect();
    tasks.sort();
    let scope = scope_of(co);
    let mine = yours();
    let settings: Vec<String> = coord().into_iter().filter(|n| n.starts_with("autopilot.")).map(|n| format!("{n}={}", lookup(n, &scope, &mine).map(|f| text(&f.value)).unwrap_or_default())).collect();
    let r = room(co, ap, board, now);
    let day = if r { day_of(now) } else { String::new() };
    hashed(&format!("{}|{}|{r}|{day}", tasks.join(","), settings.join(",")))
}
/// Kept short in the record: two snapshots are only ever compared.
fn hashed(s: &str) -> String { use std::hash::{Hash, Hasher}; let mut h = std::collections::hash_map::DefaultHasher::new(); s.hash(&mut h); format!("{:016x}", h.finish()) }

//! The idle check-in: a coordinator on autopilot with nothing of its own under way and room in today's limits is never
//! left to stall. Its timer wakes it even when the free look (look.rs) finds nothing new, so a turn that once declined
//! (or misread a wake) gets another chance. Wakes whose turn ends with nothing changed are counted in a row
//! (`idleRuns`): from the third on, the timer's gap doubles each time, up to 4 hours, and its row says so. A turn that
//! changes its area sets the count back. Its row also says when it stalled: idle with room, and no wake for more than
//! twice `autopilot.every` (backed off too: then it also says why the timer waits longer).
use super::care::under_way_count;
use super::limits::room;
use super::record::{count, day_of};
use crate::coordinator::co::Co;
use crate::json::*;
use crate::shown::words::age;

/// Answered wakes in a row that changed nothing before the timer backs off.
pub const IN_A_ROW: i64 = 3;
/// The longest a backed-off timer waits (an `autopilot.every` longer than this is kept as it is).
pub const MOST_MS: i64 = 4 * 3_600_000;

/// Nothing of its own under way (working, queued or waiting on you), and room for something new.
pub fn idle(co: &Co, ap: &Obj, board: &[Obj], now: i64) -> bool { under_way_count(ap, board) == 0 && room(co, ap, board, now) }
/// Its wakes in a row whose turn changed nothing.
pub fn runs(ap: &Obj) -> i64 { ap.i("idleRuns") }
pub fn backed_off(ap: &Obj) -> bool { runs(ap) >= IN_A_ROW }
/// The timer's gap now: `every`, doubled for each idle wake in a row from the third on, up to `MOST_MS`.
pub fn timer_gap(every: i64, ap: &Obj) -> i64 {
    if !backed_off(ap) { return every; }
    let n = (runs(ap) - IN_A_ROW + 1).min(16) as u32;
    every.saturating_mul(1 << n).min(MOST_MS.max(every))
}
/// A turn ended with the board as `snap` (the record still holds the look of the turn before): a wake's turn that
/// changed nothing is one more in a row; any turn that changed something sets the count back.
pub fn noted(ap: &mut Obj, snap: &str, wake: bool) {
    let n = if ap.s("snap") != snap { 0 } else if wake { runs(ap) + 1 } else { runs(ap) };
    ap.put("idleRuns", n);
}
/// When it last ran: the end of its last turn, a wake typed in, or turning on.
fn last_ran(ap: &Obj) -> i64 { [ap.i("endAt"), ap.i("since"), ap.at("wake").i("typedAt")].into_iter().max().unwrap_or(0) }
/// Stalled: idle with room, wakes left today, and no wake for more than twice `autopilot.every`, backed off or not.
pub fn stalled(co: &Co, ap: &Obj, board: &[Obj], now: i64, every: i64) -> bool {
    let woke = if ap.s("day") == day_of(now) { ap.i("woke") } else { 0 };
    every > 0 && !ap.at("wake").b("typedAt") && woke < count(co, "autopilot.wakesPerDay") && idle(co, ap, board, now) && now - last_ran(ap) > 2 * every
}
/// What its row says of it, after `Autopilot`: `stalled: no check-in for 3h`, `idle: last 3 wakes added nothing`, both
/// (`stalled: no check-in for 3h, last 4 wakes added nothing`), or "" (it is busy, or waking as it should).
pub fn words(co: &Co, ap: &Obj, board: &[Obj], now: i64, every: i64) -> String {
    let idle_runs = (backed_off(ap) && idle(co, ap, board, now)).then(|| format!("last {} wakes added nothing", runs(ap)));
    match (stalled(co, ap, board, now, every), idle_runs) {
        (true, Some(r)) => format!("stalled: no check-in for {}, {r}", age(now - last_ran(ap))),
        (true, None) => format!("stalled: no check-in for {}", age(now - last_ran(ap))),
        (false, Some(r)) => format!("idle: {r}"),
        (false, None) => String::new(),
    }
}

//! Who fires the automations: the keeper and an open board, by this one pass, once a second, so it works with the
//! board closed. One pass at a time on the machine: whoever finds it taken skips, and what is due is decided and
//! written down under it, so nothing fires twice. A schedule fires when it is due; a tick found more than LATE late is
//! skipped and never caught up. A board event fires the automations that wait for it, those whose condition holds.
use super::condition::mismatch;
use super::events::new_events;
use super::fire::{fire, one_at_a_time, say_cause};
use super::told::{made_by, Cause};
use super::triggers::next_after;
use crate::err::Kind;
use crate::json::*;
use crate::store::automations::{automations, change_automation, drop_events_seen, events_seen, has_automations, note_firing, save_events_seen};
use crate::store::tasks::{list, ListOpts};
use std::sync::atomic::{AtomicI64, Ordering};

/// A tick found later than this was missed, not late.
pub const LATE: i64 = 5 * 60000;
const EVENTS_EVERY: i64 = 2000;
static LOOKED_AT: AtomicI64 = AtomicI64::new(0);
pub fn automations_tick(now: i64) -> Vec<Value> {
    if !has_automations() { return vec![]; }
    let on: Vec<Obj> = automations().into_iter().filter(|a| a.b("on")).collect();
    let events = on.iter().any(|a| a.at("trigger").s("kind") == "event");
    if !events && events_seen().is_some() { drop_events_seen(); } // nobody waits for one: the next automation that does starts from then
    let look = events && (now - LOOKED_AT.load(Ordering::Relaxed)).abs() >= EVENTS_EVERY;
    if !look && !on.iter().any(|a| a.at("trigger").s("kind") == "schedule" && (!a.b("nextAt") || now >= a.i("nextAt"))) { return vec![]; }
    let mut fired = vec![];
    let r = one_at_a_time(0, || {
        for a in automations().into_iter().filter(|x| x.b("on") && x.at("trigger").s("kind") == "schedule") {
            if a.b("nextAt") && now < a.i("nextAt") { continue; }
            let late = if a.b("nextAt") { now - a.i("nextAt") } else { 0 };
            change_automation(a.s("id"), |x| { let since = if x.b("since") { x.i("since") } else { now }; let n = next_after(x.at("trigger"), now, since); x.put("nextAt", n); })?;
            if !a.b("nextAt") { continue; } // never armed (a file edited by hand): it counts from now
            if late > LATE { fired.push(note_firing(a.s("id"), obj(json!({ "at": now, "cause": "schedule", "what": a.at("trigger").at("text"), "skipped": "missed while the machine or the keeper was off: not caught up" })))?); }
            else { fired.push(fire(a.s("id"), Cause::Schedule, now)?); }
        }
        if look { LOOKED_AT.store(now, Ordering::Relaxed); fired.extend(events_pass(now)?); }
        Ok(())
    });
    if let Err(e) = r { if e.kind != Kind::Busy { eprintln!("automations: {}", e.msg); } }
    fired
}
/// What happened on the board since the last look, and the automations it fires.
fn events_pass(now: i64) -> crate::err::Res<Vec<Value>> {
    let mut tasks = list(ListOpts { include_archived: true, ..Default::default() });
    tasks.sort_by(|x, y| x.i("created").cmp(&y.i("created")).then_with(|| x.s("id").cmp(y.s("id")))); // the oldest task first
    let seen = events_seen();
    let (events, next, changed) = new_events(&tasks, seen.as_ref());
    if changed { save_events_seen(&next)?; }
    let mut fired = vec![];
    for e in events {
        for a in automations().into_iter().filter(|x| x.b("on") && x.at("trigger").s("kind") == "event" && x.at("trigger").s("event") == e.event) {
            if !a.at("trigger").b("everyRepo") && e.task.s("repo") != a.s("repo") { continue; }
            if e.at < a.i("since") { continue; } // it happened before this automation began to wait for it
            if let Some(m) = made_by(&e.task) { if m.arr("chain").iter().any(|c| text(c) == a.s("id")) || m.s("id") == a.s("id") { continue; } } // no loops
            let cause = Cause::Event { event: e.event.clone(), task: e.task.clone() };
            // its condition does not hold for this task: a written skip, no task, nothing counted toward a runaway
            if let Some(why) = mismatch(a.at("trigger"), &e.task) { fired.push(note_firing(a.s("id"), obj(json!({ "at": now, "cause": "event", "what": say_cause(&a, &cause), "skipped": why })))?); continue; }
            fired.push(fire(a.s("id"), cause, now)?);
        }
    }
    Ok(fired)
}

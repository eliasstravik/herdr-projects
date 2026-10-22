//! Who wakes the coordinators on autopilot: the keeper (and an open board), beside the automations, once a second, so
//! it works with the board closed. Each tick, for each coordinator on autopilot: the end of its turn is seen (a wake
//! answered); a wake that waited 15 minutes is unanswered; a failed start is said; the board's events and its timer add
//! reasons; and when nothing holds it back the reasons go out as one wake. Nothing holds it back once: it is awake by
//! its hours, past `autopilot.gap` since its last turn ended (but for the wake of turning on), under
//! `autopilot.wakesPerDay`, at rest with no message of yours waiting, and with room or a task that needs looking after.
//! A reason that is only the timer goes only when the free look (look.rs) finds the board changed.
use super::care::needing;
use super::events::{look as events, reason};
use super::limits::{board_tasks, room};
use super::look::snapshot;
use super::record::{change, count, roll, words};
use super::wake::{merged, pull_back, queue, rewrite, text_of, waiting};
use crate::coordinator::co::Co;
use crate::coordinator::state::{all, get, pumping};
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::settings::pilot_values::awake;
use crate::store::settings::values::ms;

/// How long a wake may wait for its turn to end before it counts as unanswered.
pub const ANSWER_MS: i64 = 15 * 60000;
/// Every coordinator of this herdr session that is on autopilot.
pub fn autopilot_tick(socket: &str, now: i64) {
    let on: Vec<Co> = all(socket).into_iter().filter(super::record::is_on).collect();
    if on.is_empty() { return; }
    let board = board_tasks();
    for co in on { tick(&co, &board, now); }
}
/// What herdr says of its pane: at rest, and the count of turns it finished. None: no program there.
fn seen(co: &Co, c: &Obj) -> Option<(bool, Value)> {
    if !c.b("pane") { return None; }
    let a = Herdr::new(Some(co.socket.clone())).agent_get(c.s("pane")).ok().flatten()?;
    Some((["idle", "done"].contains(&a.s("agent_status")), a.at("completion_seq").clone()))
}
fn mine(m: &Value) -> bool { !m.b("autopilot") }
pub fn tick(co: &Co, board: &[Obj], now: i64) {
    let Some(c) = get(co) else { return };
    let look = seen(co, &c);
    let (approve, retries, when) = (words(co, "autopilot.approve") == "yes", count(co, "autopilot.retries"), words(co, "autopilot.when"));
    let (gap, every, per_day, after) = (ms(&words(co, "autopilot.gap")), ms(&words(co, "autopilot.every")), count(co, "autopilot.wakesPerDay"), count(co, "autopilot.needsYouAfter"));
    let hours = words(co, "autopilot.hours");
    // A coordinator that asks before it acts would wait at a permission prompt: it shows `needs you`, and sleeps.
    let ask_first = words(co, "coordinator.access") == "ask";
    let mut kick = false;
    let _ = change(co, |ap, c| {
        if !ap.b("on") { return false; }
        roll(ap, now);
        let ended_at = |ap: &mut Obj, c: &Obj| { ap.put("endAt", now); ap.put("seenId", c.t("typedId")); let s = snapshot(co, ap, board, now); ap.put("snap", s); };
        let judge = |ap: &mut Obj, answered: bool| {
            let mut w = ap.arr("wakes").to_vec();
            if let Some(last) = w.last_mut() { last["answered"] = json!(answered); }
            ap.put("wakes", w);
            let n = if answered { 0 } else { ap.i("unanswered") + 1 };
            ap.put("unanswered", n);
            if n >= after { ap.put("needsYou", true); }
        };
        // The turn the last message typed in started has ended: a wake's is answered.
        if let Some((rest, seq)) = &look {
            let mark = c.at("mark");
            if *rest && !seq.is_null() && mark.i("run") == c.i("startedAt") && *seq != *mark.at("seq") && c.b("typedId") && c.s("typedId") != ap.s("seenId") {
                let typed = ap.at("wake").b("typedAt");
                ended_at(ap, c);
                if typed { judge(ap, true); ap.put("wake", Value::Null); }
            }
        }
        // 15 minutes on: a typed wake whose turn has not ended, or one still held in the queue, is unanswered.
        if ap.at("wake").is_object() && now - ap.at("wake").i(if ap.at("wake").b("typedAt") { "typedAt" } else { "at" }) >= ANSWER_MS {
            if ap.at("wake").b("typedAt") { judge(ap, false); ap.put("wake", Value::Null); ended_at(ap, c); }
            else { pull_back(c, ap, true); judge(ap, false); ap.put("endAt", now); }
        }
        // A start that failed: not a wake. Only turning on stays waiting, and the gap starts as if a turn had ended.
        if waiting(c, ap) && c.b("error") && !pumping(c) {
            let why = c.t("error");
            pull_back(c, ap, true);
            let p: Vec<Value> = ap.arr("pending").iter().filter(|r| r.s("why") == "on").cloned().collect();
            ap.put("pending", p); ap.put("endAt", now); ap.put("failedAt", now); ap.put("couldNot", why);
        }
        // What happened, and the timer.
        // Asking first now: a wake not typed in yet is taken back, its reasons kept for when it may wake again.
        if ask_first && waiting(c, ap) { pull_back(c, ap, true); }
        let happened = events(co, ap, board, &when, approve, retries);
        let mut pending = merged(ap.arr("pending"), &happened);
        // The timer counts from when its last wake ended, or from its last look that found nothing new; missed ticks
        // are not caught up.
        let ended = ap.i("endAt").max(ap.i("since"));
        if every > 0 && now >= ended.max(ap.i("timerAt")) + every && !pending.iter().any(|r| r.s("why") == "timer") { pending.push(reason("timer", "", "")); ap.put("timerAt", now); }
        ap.put("pending", pending.clone());
        if pending.is_empty() { return true; }
        // One wake in the queue at most: one not typed in yet takes the new reasons in.
        if waiting(c, ap) {
            let all = merged(ap.at("wake").arr("reasons"), &pending);
            let text = text_of(&all);
            rewrite(c, ap, &text, &all); ap.put("pending", Vec::<Value>::new());
            return true;
        }
        // What holds it back.
        let on_wake = pending.iter().any(|r| r.s("why") == "on") && now - ap.i("failedAt") >= gap;
        let busy = look.as_ref().map(|(rest, _)| !rest).unwrap_or(false) || c.arr("queue").iter().any(mine) || ap.at("wake").b("typedAt");
        if ap.b("needsYou") || ask_first || !awake(&hours, now) || ap.i("woke") >= per_day || busy { return true; }
        if !on_wake && now < ended + gap { return true; }
        if !room(co, ap, board, now) && needing(ap, board, approve).is_empty() { return true; }
        // A reason that is only the timer goes only when the board changed since its last wake ended.
        if pending.iter().all(|r| r.s("why") == "timer") && ap.s("snap") == snapshot(co, ap, board, now) { ap.put("pending", Vec::<Value>::new()); return true; }
        let text = text_of(&pending);
        queue(c, ap, &text, &pending); kick = true;
        ap.put("pending", Vec::<Value>::new());
        true
    });
    if kick { crate::coordinator::requests::kick(co); }
}

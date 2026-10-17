//! "Finish" on a task whose finish the board missed: still at stage "working", its agent at rest and no runner at
//! work (HSW-218: the board waited for a helper it could not see come back, through every turn after). What the agent
//! said last is the summary, and the finish line runs on the work as it is.
use super::shared::{say, Said};
use crate::agents::installed::read_turn_of;
use crate::clock::now;
use crate::err::{Error, Res};
use crate::flow;
use crate::json::*;
use crate::runner::{runner_alive, spawn_runner};
use crate::store::inbox::pending_text;
use crate::store::repos::cwd_of;
use crate::store::tasks::{update, Change, Task};
use std::path::Path;

fn busy(id: &str) -> Error { Error::busy(format!("{id} is busy. Try again in a moment.")) }
pub fn finish_stuck(cur: &Task) -> Res<Said> {
    let _account = crate::agents::account::task(cur, "agent");
    let id = cur.t("id");
    if runner_alive(cur) { return Err(busy(&id)); }
    if !flow::task::is_agent_task(cur) { crate::fail!("{id} has no agent to finish for."); }
    if !pending_text(&id).is_empty() { crate::fail!("{id} has words waiting for its agent: answer its dialog first."); }
    let a = flow::task::h(cur).agent_get(cur.at("link").s("pane"))?;
    if a.as_ref().map(|a| ["working", "blocked"].contains(&a.s("agent_status"))).unwrap_or(false) { crate::fail!("{id}'s agent is still at work: wait for it to stop."); }
    let turn = read_turn_of(cur.s("agent"), cur.so("transcript").map(Path::new), cur.s("session"), &cwd_of(cur), false).filter(|x| x.finished && x.error.is_empty());
    let note = flow::task::note_stat(cur);
    let said = [turn.as_ref().map(|x| x.said.clone()), note.as_ref().map(|n| n.1.clone()), Some(cur.t("said"))].into_iter().flatten().find(|s| !s.trim().is_empty()).unwrap_or_default();
    if said.is_empty() { crate::fail!("{id} has no closing summary to finish with: tell its agent to write one."); }
    let comp = a.as_ref().map(|a| a.at("completion_seq").clone()).unwrap_or(Value::Null);
    let t = update(&id, |t| {
        if t.s("stage") != "working" || runner_alive(t) { return Err(busy(&id)); }
        // This turn is handled: neither its late turn-end report, nor herdr's count, nor its note starts the finish again.
        let h = t.obj_mut("handled");
        h.put("at", now());
        if !comp.is_null() { h.put("comp", comp.clone()); }
        if let Some(x) = &turn { h.put("msg", x.id.clone()); h.put("pos", x.pos); }
        if let Some(n) = &note { h.put("note", n.0); }
        let seq = t.at("eventSeq").clone(); t.put("eventsHandled", seq); t.put("events", json!([]));
        for k in ["turnBy", "expectedWake", "selfBlocked"] { t.put(k, Value::Null); }
        t.put("turnOpen", false); t.put("said", said.clone());
        t.put("stage", "checking"); t.put("msg", ""); t.put("since", now());
        Ok(Change::Write)
    })?;
    spawn_runner("pipeline", &id, &[]);
    Ok(say(t, format!("{id}: finishing with what its agent said last")))
}

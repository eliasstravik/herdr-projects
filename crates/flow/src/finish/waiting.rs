//! An agent that stopped to wait for the human: the end of that turn is not a finish, so no check, no review, no push
//! and no merge follow it. The agent says that it waits with `report --state blocked`, in the turn it then ends. That end
//! reaches the board twice, in no fixed order (herdr sees the pane at rest; the agent's own turn-end report starts the
//! finish line), so the turn is known by what it is: it began before the agent said that it waits, and ended after.
use crate::agents::installed::read_turn_of;
use crate::agents::read_turn::Turn;
use crate::clock::now;
use crate::json::*;
use crate::research::keep_report;
use crate::store::tasks::{update_wait, Change, Task};

/// When the turn began; where only the end of the file was read and holds no prompt, the whole file is read this once.
fn began(t: &Task, turn: &Turn) -> Option<i64> {
    if turn.began > 0 { return Some(turn.began); }
    let all = read_turn_of(t.s("agent"), turn.file.as_deref(), "", std::path::Path::new("."), true)?;
    (all.id == turn.id).then_some(all.began)
}
fn same_turn(t: &Task, turn: Option<&Turn>) -> bool {
    let Some(turn) = turn else { return false };
    let at = t.at("blocked").i("at");
    if at == 0 || !turn.finished || turn.pos < at { return false; }
    began(t, turn).map(|b| b < at).unwrap_or(false)
}
/// Does this turn's end leave the task waiting for the human? A conversation that cannot be read says nothing: a task
/// that still says it waits then goes on waiting.
pub fn waits(t: &Task, turn: Option<&Turn>) -> bool {
    (t.b("selfBlocked") && t.s("stage") == "asks") || (turn.is_none() && t.at("blocked").b("at") && t.s("stage") == "asks") || same_turn(t, turn)
}
/// The turn is handled and the task waits, with the kind and the words its agent gave. A report that is there already
/// gets its link: an interim report can be read while the task waits.
pub fn stay(id: &str, t: &Task, turn: Option<&Turn>, seen: Obj, session: &str) {
    let report = keep_report(t, || crate::review_page::report_base(t.s("id")));
    let _ = update_wait(id, 10000, |t| {
        let h = t.obj_mut("handled");
        for (k, v) in seen { h.insert(k, v); }
        h.put("at", now());
        if let Some(turn) = turn { h.put("msg", turn.id.clone()); h.put("pos", turn.pos); }
        for k in ["selfBlocked", "turnBy", "expectedWake"] { t.put(k, Value::Null); }
        t.put("turnOpen", false);
        if t.s("stage") != "asks" {
            t.put("stage", "asks");
            t.put("kind", if t.at("blocked").b("kind") { t.at("blocked").t("kind") } else { "question".into() });
            t.put("msg", if t.at("blocked").b("msg") { t.at("blocked").t("msg") } else { "Needs you".into() });
        }
        if let Some(turn) = turn { if !turn.said.is_empty() { t.put("said", turn.said.clone()); } if let Some(f) = &turn.file { t.put("transcript", f.display().to_string()); } }
        if !session.is_empty() { t.put("session", session); }
        if let Some(r) = &report { if !(t.at("report").is_object() && t.at("report").at("file") == r.at("file") && t.at("report").at("link") == r.at("link")) { t.put("report", r.clone()); } }
        // The turn's own words and the waiting line as the human reads them: a report they name is a link. Nothing is
        // added to a question that names no report. An agent started under older rules is covered here.
        for k in ["said", "msg"] { let have = t.t(k); if !have.is_empty() { let linked = crate::research::report_linked(&have, t); t.put(k, linked); } }
        Ok(Change::Write)
    });
}

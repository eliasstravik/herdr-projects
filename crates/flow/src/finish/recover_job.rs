//! A finished turn re-armed by mistake (flow/handled_turn.rs: reopened) would stand in "working" for ever. This runner
//! asks herdr itself, holding the task, and does nothing unless the agent is at rest right now in the newest terminal
//! with no turn finished since the handled one. It never finishes the task: it goes to the human as a turn that
//! stopped without a summary.
use super::shared::last_lines;
use crate::clock::now;
use crate::flow;
use crate::herdr::api::read_screen;
use crate::json::*;
use crate::runner::{claim, release, Guard};
use crate::store::inbox::pending_text;

pub fn recover_job(id: &str) {
    let Ok(Some(t)) = claim(id, "recover", |_| {}) else { return };
    let mut g = Guard::new(id);
    (|| {
        if !flow::task::is_agent_task(&t) || t.b("archivedAt") || t.s("stage") != "working" { return; }
        let api = flow::task::h(&t);
        let Ok(Some(a)) = api.agent_get(t.at("link").s("pane")) else { return };
        if a.s("agent_status") != "idle" && a.s("agent_status") != "done" { return; }
        let terms = t.arr("terms");
        if a.b("terminal_id") && !terms.is_empty() && terms.last().map(text) != Some(a.t("terminal_id")) { return; } // a terminal the hook has not seen yet: it decides first
        if !flow::handled_turn::reopened(&t, a.at("completion_seq")) || !pending_text(id).is_empty() { return; }
        let screen = last_lines(&read_screen(&api, t.at("link").s("pane")), 12);
        g.expect(&t, &[]);
        let _ = g.set(|t| { t.put("stage", "nosummary"); t.put("msg", "Stopped without a summary"); t.put("screen", screen.clone()); t.put("turnOpen", false); t.put("since", now()); });
    })();
    release(id, false);
}

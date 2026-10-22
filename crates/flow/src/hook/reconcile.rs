//! Reconcile compares a linked task with herdr: pane gone, pane alive with no agent, a finished turn nobody handled,
//! a runner lost before its step was done (hook/lost.rs). A task whose pane id no longer matches, or now names
//! another's pane, is linked again.
use super::apply;
use crate::clock::now;
use crate::flow::task::is_agent_task;
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::{repos::cwd_of, tasks::{update_wait, Change, Task}};
use crate::submit::check_sent;
use std::path::Path;

pub fn same_dir(a: &str, b: &Path) -> bool {
    if a.is_empty() || b.as_os_str().is_empty() { return false; }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) { (Ok(x), Ok(y)) => x == y, _ => Path::new(a) == b }
}
pub(super) fn upd(id: &str, f: impl FnOnce(&mut Task) -> bool) { let _ = update_wait(id, 10000, |t| Ok(if f(t) { Change::Write } else { Change::Keep })); }

pub fn reconcile(t: &Task, api: &Herdr) {
    if !is_agent_task(t) || t.b("archivedAt") { return; }
    let id = t.s("id");
    // A runner lost before its step was done: tried again once, then failed; also before the task's agent is there.
    if super::lost::retry_lost(t) { return; }
    let mut pane = api.pane_get(t.at("link").s("pane")).filter(|p| crate::flow::live::owns_pane(t, p)); // its id handed to another: gone
    if pane.is_none() { pane = relink(t, api); }
    let agent = pane.as_ref().and_then(|p| api.agent_get(p.s("pane_id")).ok().flatten());
    if agent.is_none() { check_sent(id, None, api); } // gone with typed text not taken: not started, the text kept
    let Some(_) = pane else {
        return upd(id, |t| { if t.at("last").b("gone") { return false; } let l = t.obj_mut("last"); l.put("gone", true); l.put("status", Value::Null); l.put("at", now()); true });
    };
    let Some(a) = agent else {
        return upd(id, |t| {
            if t.at("last").has("status") == false && t.at("last").is_object() && !t.at("last").b("gone") && t.b("exitedAt") { return false; }
            let l = t.obj_mut("last"); l.put("status", Value::Null); l.put("gone", false); l.put("at", now());
            if !t.b("exitedAt") { t.put("exitedAt", now()); }
            true
        });
    };
    let cwd = cwd_of(t);
    if !same_dir(a.s("cwd"), &cwd) && !same_dir(a.s("foreground_cwd"), &cwd) { return; }
    check_sent(id, Some(&a), api); // text typed in a while ago and not taken yet gets its enter again
    apply(id, &a, api); // covers hooks missed while herdr restarted
}
/// When the stored pane id no longer matches, the task is relinked: first by terminal_id, then by agent name, then
/// to the single agent pane whose folder is the task's worktree. Otherwise its terminal is gone.
fn relink(t: &Task, api: &Herdr) -> Option<Value> {
    let panes = api.panes(None).ok()?;
    let agents = api.agents().unwrap_or_default();
    let dir = cwd_of(t);
    let in_dir = |p: &Value| same_dir(p.s("cwd"), &dir) || same_dir(p.s("foreground_cwd"), &dir);
    let link = t.at("link");
    let by_pane = |pid: &str| panes.iter().find(|p| p.s("pane_id") == pid).cloned();
    let mut hit = panes.iter().find(|p| link.b("terminal") && p.at("terminal_id") == link.at("terminal") && in_dir(p)).cloned();
    if hit.is_none() && link.b("agent") { if let Some(a) = agents.iter().find(|a| a.at("name") == link.at("agent") && in_dir(a)) { hit = by_pane(a.s("pane_id")); } }
    if hit.is_none() && t.b("worktree") {
        let c: Vec<&Value> = agents.iter().filter(|a| in_dir(a) && a.at("pane_id") != link.at("reviewer").at("pane")).collect();
        if c.len() == 1 { hit = by_pane(c[0].s("pane_id")); }
    }
    let hit = hit?;
    let h = hit.clone();
    upd(t.s("id"), move |t| {
        let l = t.obj_mut("link");
        l.put("pane", h.at("pane_id").clone()); l.put("tab", h.at("tab_id").clone()); l.put("workspace", h.at("workspace_id").clone()); l.put("terminal", h.at("terminal_id").clone());
        t.obj_mut("last").put("gone", false);
        true
    });
    Some(hit)
}

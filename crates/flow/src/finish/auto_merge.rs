//! Auto-merge, the setting: a task that may merge does so by itself while `merge.by` is `itself`, and waits for you
//! while it is `you`. The pipeline asks the setting when a task turns green; this is the housekeeping pass's part, for
//! a setting changed while a task already waits.
use super::old_holds::lift_old_holds;
use super::shared::approval_message;
use crate::clock::now;
use crate::flow;
use crate::gate;
use crate::json::*;
use crate::limits::reached::over_now;
use crate::research::is_research;
use crate::runner::{runner_alive, spawn_runner, Guard};
use crate::store::repos::folder_of;
use crate::store::settings::lookup::setting;
use crate::store::tasks::{update, Change, Task};

/// Does this task merge by itself? Asked each time: the setting is read right before the merge. A task autopilot added
/// whose change touches the repo's own settings file never does while `autopilot.repoFileWaits` held when it was added.
pub fn by_itself(t: &Obj) -> bool { setting("merge.by", Some(t)) == json!("itself") && !crate::autopilot::care::held_for_you(t) }
/// Is this task's work reviewed at all?
pub fn reviewed(t: &Obj) -> bool { setting("review.when", Some(t)) != json!("never") }
/// Green to merge: checked, reviewed past the gate (or not to be reviewed at all), up to date with main, and nothing
/// that is yours to decide.
pub fn green(t: &Obj) -> bool {
    if t.s("stage") != "approval" || t.b("archivedAt") || !flow::task::is_agent_task(t) || !t.b("reviewedHead") || is_research(t) || crate::chat_only::is_chat(t) { return false; }
    if ["failure", "prFailure", "syncBlocked", "mergeFailed", "requireApproval", "mergeIntent", "autoMerge", "chat"].iter().any(|k| t.b(k)) { return false; }
    if over_now(t).is_some() { return false; } // at its token limit nothing merges by itself
    if t.at("review").is_object() { gate::passes(t.at("review")) } else { !reviewed(t) }
}
/// Will this task merge with nobody's yes?
pub fn merges_itself(t: &Obj) -> bool { if t.at("mergeIntent").is_object() { t.at("mergeIntent").b("auto") } else { by_itself(t) && !t.b("requireApproval") } }
/// A merge the board itself started and that still waits (for GitHub's checks or its merge queue).
fn pending(t: &Obj) -> bool { t.s("stage") == "merging" && t.b("autoMerge") && t.at("mergeIntent").b("auto") }
/// Turned on: a green task that waits for your yes merges now. Turned off: a task waiting to merge by itself waits.
pub fn auto_merge_pass(tasks: &[Obj], socket: &str) {
    lift_old_holds(tasks, socket);
    for seen in tasks {
        if seen.at("link").s("socket") != socket || (seen.s("stage") != "approval" && !pending(seen)) || runner_alive(seen) { continue; }
        if by_itself(seen) { if green(seen) { start(seen.s("id")); } } else if pending(seen) { hold(seen); }
    }
}
/// Asked by the land runner, when it starts and again right before it merges: a merge the board started by itself is
/// only carried out while the setting is still on. Turned off since: the task waits for your yes.
pub fn called_off(g: &mut Guard) -> bool {
    let Ok(cur) = g.check() else { return false };
    if !cur.at("mergeIntent").b("auto") || by_itself(&cur) { return false; }
    if let Ok(t) = g.set(|t| { t.put("stage", "approval"); t.put("autoMerge", false); t.put("autoMergeLocal", false); t.put("mergeIntent", Value::Null); t.put("requireApproval", false); t.put("since", now()); let m = approval_message(t); t.put("msg", m); }) { flow::sidebar::tell_sidebar(&t); }
    true
}
fn start(id: &str) {
    let mut go = false;
    let Ok(t) = update(id, |t| {
        if runner_alive(t) || !by_itself(t) || !green(t) { return Ok(Change::Keep); }
        t.put("stage", "merging"); t.put("msg", "Merging"); t.put("since", now());
        let mi = json!({ "approvedAt": now(), "repairs": 0, "verifiedHead": t.at("reviewedHead"), "auto": true });
        t.put("mergeIntent", mi);
        go = true;
        Ok(Change::Write)
    }) else { return };
    if !go { return; }
    flow::sidebar::tell_sidebar(&t);
    spawn_runner("land", id, &[]);
}
fn hold(seen: &Task) {
    // Where GitHub's own auto-merge is what waits (a merge queue), that is switched off first.
    if !seen.b("autoMergeLocal") && flow::git::gh(&["pr", "merge", &flow::git::pr_number(seen.s("pr")), "--disable-auto"], &folder_of(seen.s("id")), 60000).status != 0 { return; }
    let Ok(t) = update(seen.s("id"), |t| {
        if runner_alive(t) || t.s("stage") != "merging" || !t.at("mergeIntent").b("auto") || by_itself(t) { return Ok(Change::Keep); }
        t.put("stage", "approval"); t.put("autoMerge", false); t.put("autoMergeLocal", false); t.put("mergeIntent", Value::Null); t.put("requireApproval", false); t.put("since", now());
        let m = approval_message(t); t.put("msg", m);
        Ok(Change::Write)
    }) else { return };
    if t.s("stage") == "approval" { flow::sidebar::tell_sidebar(&t); }
}

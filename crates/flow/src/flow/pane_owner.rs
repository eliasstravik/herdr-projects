//! Whose agent runs in a herdr pane: herdr hands a closed pane's id to the next pane opened in its space, so a task's
//! saved pane id names its agent (or its reviewer) only while the terminal in that pane is one it was seen in. A done
//! task's old pane id given to a coordinator is the coordinator's, not the task's.
use super::live::{owns_pane, terminals};
use crate::herdr::Herdr;
use crate::json::*;
use crate::leftovers::sweep::ended;
use crate::store::tasks::Task;

/// The terminal in a pane now, "" when herdr cannot say.
pub fn terminal_of(socket: &str, pane: &str) -> String {
    if pane.is_empty() { return String::new(); }
    Herdr::new(Some(socket.into())).pane_get(pane).map(|p| p.t("terminal_id")).unwrap_or_default()
}
/// Is the pane with terminal `term` ("" when herdr cannot say) still the task's agent's? A terminal it was never seen
/// in is another's. Once its agent has ended (done, archived), only a terminal it was seen in counts: nothing looks
/// after an ended task's link, so its pane id has most likely gone to someone else.
pub fn agents_pane(t: &Task, term: &str) -> bool {
    if !ended(t) { return owns_pane(t, &json!({ "terminal_id": term })); }
    !term.is_empty() && terminals(t).iter().any(|k| k == term)
}
/// The same for its reviewer: the terminal saved with the reviewer's link. A link saved without one (an older version)
/// counts while the task has not ended.
pub fn reviewers_pane(t: &Task, term: &str) -> bool {
    let r = t.at("link").at("reviewer");
    if !term.is_empty() && r.b("terminal") { return r.s("terminal") == term; }
    !ended(t)
}
/// The task among `tasks` whose agent or reviewer runs in pane `pane` of herdr session `socket` now, and as which.
/// herdr is asked for the pane's terminal once, and only when a task names the pane.
pub fn whose(tasks: impl IntoIterator<Item = Task>, socket: &str, pane: &str) -> Option<(Task, &'static str)> {
    if pane.is_empty() { return None; }
    let mut term: Option<String> = None;
    let mut now = || term.get_or_insert_with(|| terminal_of(socket, pane)).clone();
    for t in tasks {
        let l = t.at("link");
        if l.s("socket") != socket { continue; }
        if l.s("pane") == pane && agents_pane(&t, &now()) { return Some((t, "agent")); }
        if l.at("reviewer").s("pane") == pane && reviewers_pane(&t, &now()) { return Some((t, "reviewer")); }
    }
    None
}

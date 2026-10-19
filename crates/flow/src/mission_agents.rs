//! Open Mission Control tasks in Herdr's native agent list, including tasks waiting at an idle shell.
use crate::herdr::{Herdr, api::TASK_AGENT, cli::args};
use crate::json::*;
const SOURCE: &str = "plugin:herdr-projects:task";

pub fn release(api: &Herdr, pane: &str) {
    let _ = api.call(args(&["pane", "release-agent", pane, "--source", SOURCE, "--agent", TASK_AGENT]));
}
pub fn sync(tasks: &[Obj], socket: &str) {
    let api = Herdr::new(Some(socket.into()));
    let Some(root) = crate::mission_layout::anchor(socket).and_then(|p| api.pane_get(&p)) else { return };
    let coordinator = crate::coordinator::state::get(socket).unwrap_or_default();
    if let Some(pane) = api.pane_get(coordinator.s("pane")) {
        title(&api, &pane, &format!("Orchestrator ({})", coordinator.at("run").s("kind")));
    }
    let mut kept = std::collections::HashSet::new();
    for t in tasks {
        if t.at("link").s("socket") != socket { continue; }
        let Some(pane) = api.pane_get(t.at("link").s("pane")).filter(|p| p.s("tab_id") == root.s("tab_id") && crate::flow::live::owns_pane(t, p) && p.s("pane_id") != coordinator.s("pane")) else { continue };
        let id = pane.s("pane_id");
        let open = !t.b("archivedAt") && t.s("stage") != "done";
        if !open { release(&api, id); api.metadata(id, &["--clear-token".into(), "task".into()]); continue; }
        kept.insert(id.to_string());
        title(&api, &pane, &crate::flow::sidebar::space_label(t));
        // The marker keeps a stopped task clickable without pretending that a coding CLI is running.
        let waiting = ["queued", "after", "failed", "asks", "approval", "reviewneeds", "nosummary", "plan", "blocked"].contains(&t.s("stage"));
        if (waiting || api.pane_idle(id)) && api.agent_get(id).ok().flatten().is_none() {
            let state = if ["queued", "after"].contains(&t.s("stage")) { "idle" } else { "blocked" };
            let raw = api.call(args(&["agent", "get", id])).unwrap_or(Value::Null);
            if raw.at("agent").s("agent") != TASK_AGENT || raw.at("agent").s("agent_status") != state {
                let _ = api.call(args(&["pane", "report-agent", id, "--source", SOURCE, "--agent", TASK_AGENT, "--state", state]));
            }
        } else if pane.s("agent") == TASK_AGENT { release(&api, id); }
    }
    for pane in api.panes(Some(root.s("workspace_id"))).unwrap_or_default() {
        if pane.s("tab_id") == root.s("tab_id") && pane.s("agent") == TASK_AGENT && !kept.contains(pane.s("pane_id")) {
            release(&api, pane.s("pane_id"));
            api.metadata(pane.s("pane_id"), &["--clear-token".into(), "task".into()]);
        }
    }
}
fn title(api: &Herdr, pane: &Value, text: &str) {
    if pane.at("tokens").s("task") != text { api.metadata(pane.s("pane_id"), &["--token".into(), format!("task={text}")]); }
}

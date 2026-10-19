//! Where the web UI's own chat of whoever the board's chat talks to is: the pane of the Coordinator (or of a machine's
//! or a repo's coordinator), or of the selected task's agent, on this machine or on another of the web UI's PCs. The
//! page docks that chat view under the board's list. It only looks: nothing is opened again or started for it (Open
//! agent does that), so a task whose agent is not in its pane now, another machine's Coordinator and a coordinator
//! not started yet have none, and the board's own chat stands in for them.
use super::web::Client;
use crate::board::update::coords::talking;
use crate::coordinator::co::Co;
use crate::coordinator::look::{look, Look};
use crate::json::*;

/// The chat view to dock, as the page goes to a pane: { pane, workspace, view, machine? }; null for none.
pub fn dock(c: &mut Client) -> Value {
    let s = &c.s;
    let socket = crate::herdr::cli::current_socket();
    // A machine's or a repo's coordinator of this machine (update/coords.rs).
    if let Some(t) = talking(s) { return if t.host.is_empty() { link(&look(Co::of(&socket, &t.to))) } else { Value::Null }; }
    if s.reply {
        let Some(r) = s.current().filter(|r| r.has("title") && !r.b("archivedAt")) else { return Value::Null };
        let pane = r.at("link").t("pane");
        if pane.is_empty() { return Value::Null; }
        let ws = { let w = r.at("link").t("workspace"); if w.is_empty() { pane.split(':').next().unwrap_or("").to_string() } else { w } };
        // On another machine: its pane on the web UI's PC for it, while that PC is connected there.
        if r.b("host") {
            let Some(m) = s.remotes.get(&r.t("host")) else { return Value::Null };
            let Some(pc) = s.web.pcs.as_ref().and_then(|l| crate::webui::pcs::pc_of(&m.machine, l)).filter(|p| p.connected) else { return Value::Null };
            return json!({ "machine": pc.id, "pane": pane, "workspace": ws, "view": "chat" });
        }
        // Here: only while an agent is in that pane now.
        let h = crate::flow::task::h(r);
        if h.pane_get(&pane).is_none() || !matches!(h.agent_get(&pane), Ok(Some(_))) { return Value::Null; }
        return json!({ "pane": pane, "workspace": ws, "view": "chat" });
    }
    // The Coordinator: this machine's, while this machine's board (or every machine's) shows.
    if !s.scope.is_empty() && s.scope != "*" && s.remotes.contains_key(&s.scope) { return Value::Null; }
    if !s.data.can_dispatch { return Value::Null; }
    link(&look(Co::top(&socket)))
}
/// A coordinator whose program runs in a pane: its chat view.
fn link(v: &Look) -> Value {
    if v.pane.is_empty() || v.status.is_none() { return Value::Null; }
    let ws = if v.workspace.is_empty() { v.pane.split(':').next().unwrap_or("").to_string() } else { v.workspace.clone() };
    json!({ "pane": v.pane, "workspace": ws, "view": "chat" })
}

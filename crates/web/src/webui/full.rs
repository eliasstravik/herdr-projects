//! The chat full screen: the web UI's own chat view of whoever the chat talks to (the Coordinator, a heading's
//! coordinator or the selected task's agent), which the page then goes to (`go`), as Open agent goes to a task's. Where
//! the web UI cannot show them (no pane of theirs, another machine's coordinator, a device that may only watch) the page
//! shows the board's own chat full screen instead.
use super::web::{effects, Client};
use crate::board::update::coords::talking;
use crate::board::update::web::{go, open_agent};
use crate::coordinator::co::Co;
use crate::coordinator::look::{look, Look};
use crate::json::*;

/// True when the page is to follow a link to the web UI's chat view (now, or when the reopened space answers).
pub fn full(c: &mut Client, drive: bool) -> bool {
    if !drive { return false; }
    let s = &mut c.s;
    let r = s.current().cloned();
    // Where a coordinator stands is asked now: the board's last look may be from before its first message started it.
    let socket = crate::herdr::cli::current_socket();
    // A machine's or a repo's coordinator of this machine (update/coords.rs).
    if let Some(t) = talking(s) { return t.host.is_empty() && link(s, &look(Co::of(&socket, &t.to))); }
    // The selected task's agent: Open agent, which reopens its space first (here or on its machine).
    if s.reply {
        let Some(r) = r.filter(|r| r.has("title")) else { return false };
        let can = !r.b("archivedAt") && if r.b("host") { r.at("link").b("pane") } else { crate::flow::task::is_agent_task(&r) };
        if !can { return false; }
        let mut fx = vec![];
        open_agent(s, &r, &mut fx);
        effects(c, fx, false);
        return true;
    }
    // The Coordinator: this machine's, while this machine's board (or every machine's) shows.
    if !s.scope.is_empty() && s.scope != "*" && s.remotes.contains_key(&s.scope) { return false; }
    if !s.data.can_dispatch { return false; }
    link(s, &look(Co::top(&socket)))
}
/// A coordinator whose program runs in a pane: its chat view.
fn link(s: &mut crate::board::state::State, v: &Look) -> bool {
    if v.pane.is_empty() || v.status.is_none() { return false; }
    let ws = if v.workspace.is_empty() { v.pane.split(':').next().unwrap_or("").to_string() } else { v.workspace.clone() };
    go(s, json!({ "pane": v.pane, "workspace": ws, "view": "chat" }));
    true
}

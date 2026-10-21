//! Whose turn a coordinator's command belongs to, found as a `Tab` turn is (coordinator/requests.rs: tab_turn): the
//! caller's pane is that coordinator's, it is still in the turn the last message started (the same run, at rest when
//! that message went in, `completion_seq` unchanged), and that message was an autopilot wake. A message of yours typed
//! in after it ends the autopilot turn: what follows is on your word. A turn a task's agent or reviewer started (its
//! `coordinator tell`) is found the same way.
use crate::coordinator::co::Co;
use crate::coordinator::state::{all, get};
use crate::herdr::Herdr;
use crate::json::*;

/// The coordinator whose pane this is, in this herdr session.
pub fn coordinator_of(pane: &str, socket: &str) -> Option<Co> {
    if pane.is_empty() { return None; }
    all(socket).into_iter().find(|co| get(co).map(|c| c.s("pane") == pane || c.s("paneWas") == pane).unwrap_or(false))
}
/// The caller's coordinator: the one whose pane runs this command.
pub fn caller() -> Option<Co> {
    if crate::util::env_set("HERDR_PLUGIN_ID") { return None; }
    let (pane, socket) = (crate::util::env_val("HERDR_PANE_ID")?, crate::util::env_val("HERDR_SOCKET_PATH")?);
    coordinator_of(&pane, &socket)
}
/// Is it still in the turn its last typed message started?
fn in_that_turn(c: &Obj, socket: &str) -> bool {
    if c.at("mark").i("run") != c.i("startedAt") || !c.b("pane") { return false; }
    Herdr::new(Some(socket.to_string())).agent_get(c.s("pane")).ok().flatten().map(|a| *a.at("completion_seq") == *c.at("mark").at("seq")).unwrap_or(false)
}
/// The last message typed in was its wake, it went in at rest, and the turn it started has not ended.
pub fn autopilot_turn(co: &Co) -> bool {
    let Some(c) = get(co) else { return false };
    let w = c.at("autopilot").at("turn");
    w.b("id") && w.s("id") == c.s("typedId") && c.at("mark").b("rest") && in_that_turn(&c, &co.socket)
}
/// The last message typed in came from a task's agent or reviewer, and its turn has not ended: who sent it.
pub fn agents_turn(co: &Co) -> Option<Value> {
    let c = get(co)?;
    if !c.at("typedFrom").is_object() || !in_that_turn(&c, &co.socket) { return None; }
    Some(c.at("typedFrom").clone())
}

//! A coordinator in herdr's sidebar: named as the board names it (`Coordinator`, `vm-str coordinator`), standing as the
//! line above the board's box says, with its last message under it. herdr names its pane after the agent's inner name
//! (`sw-coordinator`) and reads a finished turn as `done`: the board says its name and `idle`. Written by whoever
//! ticks, and only when a word changed.
use super::co::Co;
use super::look::look;
use super::state::get;
use crate::flow::sidebar::tell_pane;
use crate::flow::sidebar_entry::named_coordinator_entry;
use crate::herdr::cli::socket_exists;
use crate::status_words::coordinator_stands;
use crate::store::marks::due;
use crate::json::*;

pub fn tell_sidebar(co: impl Into<Co>) {
    let co = co.into();
    let Some(c) = get(&co) else { return };
    if !c.b("pane") || !socket_exists(&co.socket) || !due(&co.mark("coordinator-sidebar"), 2000) { return; }
    let v = look(&co);
    if !v.pane.is_empty() && v.status.is_some() {
        let said = if v.error.is_empty() { v.said.clone() } else { v.error.clone() };
        tell_pane(&co.socket, &v.pane, &named_coordinator_entry(&co.name(), &coordinator_stands(&v.state, &v.error), v.queue, &said), &v.agent);
    }
}

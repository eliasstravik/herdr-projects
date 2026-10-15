//! The Coordinator in herdr's sidebar: named `Coordinator`, standing as the line above the board's box says, with its
//! last message under it. herdr names its pane after the agent's inner name (`sw-coordinator`) and reads a finished turn
//! as `done`: the board says `Coordinator` and `idle`. Written by whoever ticks, and only when a word changed.
use super::look::look;
use super::state::get;
use crate::flow::sidebar::tell_pane;
use crate::flow::sidebar_entry::coordinator_entry;
use crate::herdr::cli::{session_name, socket_exists};
use crate::status_words::coordinator_stands;
use crate::store::marks::due;
use crate::json::*;

pub fn tell_sidebar(socket: &str) {
    let Some(c) = get(socket) else { return };
    if !c.b("pane") || !socket_exists(socket) || !due(&format!("coordinator-sidebar-{}", session_name(socket)), 2000) { return; }
    let v = look(socket);
    if !v.pane.is_empty() && v.status.is_some() {
        let said = if v.error.is_empty() { v.said.clone() } else { v.error.clone() };
        tell_pane(socket, &v.pane, &coordinator_entry(&coordinator_stands(&v.state, &v.error), v.queue, &said), &v.agent);
    }
}

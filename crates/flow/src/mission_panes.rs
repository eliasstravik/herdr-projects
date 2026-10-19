//! Retired Mission Control coordinators: save their output, then close only their own resting panes.
use crate::coordinator::{co::Co, state};
use crate::err::Res;
use crate::herdr::{Herdr, api::read_screen};
use crate::json::*;
use crate::store::files::{read_obj, write_atomic};

pub fn tidy(socket: &str) -> Res<()> {
    let api = Herdr::new(Some(socket.into()));
    let Some(root) = crate::mission_layout::anchor(socket).and_then(|p| api.pane_get(&p)) else { return Ok(()) };
    let co = Co::top(socket);
    let current = state::get(&co).unwrap_or_default();
    if api.agent_get(current.s("pane")).ok().flatten().is_none() { return Ok(()); }
    let dir = state::dir_of(&co);
    let records = std::fs::read_dir(&dir).into_iter().flatten().flatten();
    for entry in records {
        let path = entry.path();
        if !entry.file_name().to_string_lossy().starts_with("handover-") || path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
        let old = read_obj(&path).unwrap_or_default();
        if old.s("pane") == current.s("pane") || old.s("pane") == root.s("pane_id") || !old.b("terminal") { continue; }
        let Some(pane) = api.pane_get(old.s("pane")) else { continue };
        if pane.s("terminal_id") != old.s("terminal") || pane.s("tab_id") != root.s("tab_id") || std::path::Path::new(pane.s("cwd")) != state::home(&co) { continue; }
        let agent = api.agent_get(old.s("pane")).ok().flatten();
        let resting = agent.as_ref().map(|a| ["idle", "done"].contains(&a.s("agent_status"))).unwrap_or_else(|| api.pane_idle(old.s("pane")));
        if !resting { continue; }
        let log = path.with_extension("terminal.txt");
        let screen = read_screen(&api, old.s("pane"));
        write_atomic(&log, screen.as_bytes())?;
        api.pane_close(old.s("pane"));
    }
    Ok(())
}

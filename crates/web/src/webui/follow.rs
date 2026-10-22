//! The web board goes where the web UI goes: when the web UI's page is on another of its PCs, the board shows that
//! machine's board by itself, as the scope key shows it; back on this machine it shows what it showed before. A PC the
//! board has no machine for is said once; the board then stays on what it showed. Which PC is which machine: pcs.rs.
use super::pcs::{machine_of, Pc};
use super::web::Client;
use crate::shown::ink::Ink;

const RETRY: i64 = 10000;
fn back(c: &mut Client) {
    if let Some(was) = c.s.web.followed.take() { if c.s.scope != was { scope(c, &was); } }
}
fn scope(c: &mut Client, to: &str) {
    let mut fx = vec![];
    crate::board::update::remotes::set_scope(&mut c.s, to.to_string(), &mut fx);
    // It moved by itself, with the web UI: said, unlike a board you picked.
    let said = crate::board::update::remotes::scope_said(&c.s);
    c.s.flash(said, Ink::Muted);
    super::web::effects(c, fx, true);
}
/// What the front door passes with a call: the web UI's PCs as it last asked (None: it could not say), and the PC the
/// page is on ("" for this machine).
pub fn follow(c: &mut Client, pcs: Option<Vec<Pc>>, machine: Option<&str>) {
    if pcs.is_some() { c.s.web.pcs = pcs; }
    let Some(machine) = machine else { return };
    let at = if machine.is_empty() || machine == "local" { String::new() } else { machine.to_string() };
    let now = crate::clock::now();
    // On a PC the board knew no machine for: looked for again every few seconds (asking herdr starts a program), so the
    // board goes there by itself once the machine has been added.
    let again = !at.is_empty() && c.s.web.stranger == at && now - c.s.web.stranger_tried >= RETRY;
    if at == c.s.web.at.clone().unwrap_or_default() && !again { return; }
    // On another PC whose list has not come yet: asked again with the next call.
    let pc = if at.is_empty() { None } else { c.s.web.pcs.as_ref().and_then(|l| l.iter().find(|p| p.id == at).cloned()) };
    if !at.is_empty() && pc.is_none() { return; }
    c.s.web.at = Some(at.clone());
    if c.s.web.stranger != at { c.s.web.stranger.clear(); }
    let Some(pc) = pc else { return back(c) };
    if pc.local { return; }
    let Some(m) = machine_of(&pc, &crate::remote::machines()) else {
        c.s.web.stranger_tried = now;
        if c.s.web.stranger == at { return; } // said already
        c.s.web.stranger = at;
        // Not the board of the PC it was on before either: back to what it showed before it went anywhere.
        back(c);
        return c.s.flash(format!("The web UI is on {}, which the board knows no machine for: add it with herdr machine add (or name it in board.machines). The board shows what it showed before.", pc.name), Ink::Yellow);
    };
    c.s.web.stranger.clear();
    if c.s.web.followed.is_none() { c.s.web.followed = Some(c.s.scope.clone()); }
    if c.s.scope != m.label {
        // The machine may be new to this board: it is added before the board goes there.
        if !c.s.remotes.contains_key(&m.label) { let fx = crate::board::update::update(&mut c.s, crate::board::update::Event::Machines(crate::remote::machines())); super::web::effects(c, fx, false); }
        scope(c, &m.label);
    }
}

//! Open agent on the web board goes to the reopened pane only once the web UI (Ovis) shows it. A task's space that was
//! closed comes back as a new workspace and pane, and the web UI's page drops a link to a pane it does not know yet
//! (Ovis 0.4.13: its address goes back to the pane it was on, without a word). So the link is held
//! (board/update/web.rs `going`) while the web UI is asked for its panes (pcs.rs), for a few seconds at most; when it
//! still does not show it, the board says so and the page stays. A web UI that cannot say which panes it has gets the
//! link at once, as before. The page itself checks that the web UI went, and says so when it did not (talk.js).
use super::attach::far::WebUi;
use super::pcs::Pc;
use super::web::Client;
use crate::board::update::web::go;
use crate::json::*;
use crate::shown::ink::Ink;

/// How long a link waits for the web UI to show its pane, and how often it is asked meanwhile.
pub const BOUND_MS: i64 = 8000;
const EVERY_MS: u64 = 300;

/// Does the web UI show that pane, on its PC (the local one, or the one the link names)? None: it cannot say.
fn shown(to: &Value, pcs: Option<&[Pc]>) -> Option<bool> {
    let pcs = pcs?;
    let pc = if to.b("machine") { pcs.iter().find(|p| p.id == to.s("machine")) } else { pcs.iter().find(|p| p.local) }?;
    pc.panes.as_ref().map(|l| l.iter().any(|p| p == to.s("pane")))
}
/// The held link let go when the web UI shows its pane (or cannot say), with what this call already knows of its PCs
/// first. `long` (the press itself): the web UI is asked again until it shows it or the wait is over; else one look,
/// and a later call looks again.
pub fn settle(c: &mut Client, pcs: Option<Vec<Pc>>, ui: Option<&dyn WebUi>, long: bool) {
    let mut list = pcs;
    loop {
        let Some((to, since)) = c.s.web.going.clone() else { return };
        if list.is_none() { list = ui.and_then(|u| u.pcs_now()); }
        match shown(&to, list.as_deref()) {
            None | Some(true) => { c.s.web.going = None; return go(&mut c.s, to); }
            Some(false) if crate::clock::now() - since >= BOUND_MS => { c.s.web.going = None; return missed(c, &to); }
            Some(false) => {
                let Some(ui) = ui.filter(|_| long) else { return };
                crate::clock::pause(EVERY_MS);
                list = ui.pcs_now();
            }
        }
    }
}
/// The page stays: the board says which space is open, and where to find it.
fn missed(c: &mut Client, to: &Value) {
    let space = if to.b("space") { format!("“{}”", to.s("space")) } else { "The task's space".into() };
    c.s.flash(format!("{space} is open in herdr, but the web UI (Ovis) does not show it yet, so the page stayed here. Pick it in the web UI's sidebar in a moment, or open the agent again."), Ink::Yellow);
}

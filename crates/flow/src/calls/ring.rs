//! The terminal board's call key: a call rings for whoever the chat talks to, and a page on your computer picks it up
//! with its ticket. The page is the front door's, at the web UI's private link (`<link>/herdr-projects/call#<ticket>`):
//! the link is kept with the call for the helper that takes it, and the board shows it as a link while nobody has.
use crate::json::*;
use crate::voice::call::{self, Target};

/// The front door's private link, as the web UI's shared link points at it; None when there is none.
pub fn door_link() -> Option<String> {
    if let Ok(l) = std::env::var("HERDR_PROJECTS_CALL_LINK") { return Some(l).filter(|l| !l.is_empty()); } // a stand-in, for tests
    let d = crate::webui::keep::door_info()?;
    let port = d.i("port") as u16;
    crate::webui::link::look(d.at("to").i("port") as u16, Some(port)).ours.into_iter().find(|e| !e.funnel).map(|e| e.url)
}
/// Ring: a new call for `t`. Answers the call's id and the link that picks it up, or why it cannot ring.
/// `session`: the board's herdr session, whose herdr window on this computer (if in use) takes the call here.
pub fn ring(owner: &str, t: &Target, session: &str) -> Result<(String, String), String> {
    if crate::voice::key::find().is_none() { return Err(crate::voice::key::missing()); }
    let base = door_link().ok_or("A call from the terminal board opens its page from the board's front door, at the herdr web UI's private link, and there is none here: share the web UI and run herdr-projects web-ui link.")?;
    let (c, ticket, _) = call::start(owner, t);
    let link = format!("{base}/herdr-projects/call#{ticket}");
    call::change(|x| { x.put("link", link.clone()); x.put("session", if session.is_empty() { "default" } else { session }); });
    Ok((c.t("id"), link))
}
/// The link of the call ringing now, for a helper to open; None once a page took it.
pub fn ringing() -> Option<(String, String, bool, String)> { call::get().filter(|c| c.s("state") == "ringing" && c.b("link")).map(|c| (c.t("id"), c.t("link"), c.b("offered"), if c.b("session") { c.t("session") } else { "default".into() })) }

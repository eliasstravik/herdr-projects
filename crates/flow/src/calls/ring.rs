//! The terminal board's call key: a call rings for whoever the chat talks to, and the helper on the computer you sit
//! at picks it up. The call's ticket is kept with the call, on this machine only, for the listener that the winning
//! helper talks to (listen.rs): it starts the call with it, and the helper never sees the ticket or the call's key.
use crate::json::*;
use crate::voice::call::{self, Target};

/// Ring: a new call for `t`. `session`: the board's herdr session, whose herdr window on this computer (when in use)
/// takes the call here. Answers the call's id, or why it cannot ring.
pub fn ring(owner: &str, t: &Target, session: &str) -> Result<String, String> {
    if !crate::voice::calls_on() { return Err(crate::voice::OFF.into()); }
    if crate::voice::key::find().is_none() { return Err(crate::voice::key::missing()); }
    let (c, ticket, _) = call::start(owner, t);
    call::change(|x| { x.put("pickup", ticket.clone()); x.put("session", if session.is_empty() { "default" } else { session }); });
    Ok(c.t("id"))
}
/// The call ringing now, for a listener: its id, its ticket, whether a helper took it, the board's herdr session.
pub fn ringing() -> Option<(String, String, bool, String)> {
    call::get().filter(|c| c.s("state") == "ringing" && c.b("pickup")).map(|c| (c.t("id"), c.t("pickup"), c.b("offered"), if c.b("session") { c.t("session") } else { "default".into() }))
}

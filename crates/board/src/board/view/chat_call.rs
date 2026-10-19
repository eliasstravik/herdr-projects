//! A voice call on now, under the line that says who the chat talks to: with whom, the key that hangs up, and the last
//! thing said on it. While a call this board rang waits for the computer you sit at, and no call helper picked it up
//! after a few seconds, a line to Ctrl-click: it opens the call's page in this computer's browser (herdr opens a
//! clicked link on the computer its window runs on).
use crate::board::keys::key_name;
use crate::board::state::State;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::voice::call;

/// How long a helper has to pick a call up before the link shows.
pub const LINK_AFTER_MS: i64 = 4000;
const WHO: [(&str, &str); 4] = [("you", "You"), ("voice", "Voice"), ("sent", "Sent"), ("board", "Board")];
pub fn lines(s: &State) -> Vec<Inked> {
    let Some(c) = s.data.call.as_ref() else { return vec![] };
    let ringing = c.s("state") == "ringing";
    let target = call::target(c).name;
    let head = if ringing { format!("Calling with {target}…") } else { format!("On a call with {target}") };
    let from = if call::from_terminal(c) { "" } else { " (from the web board)" };
    let mut out = vec![Inked::of("☎ ", Ink::Green).add(Inked::of(format!("{head}{from}"), Ink::Plain).bold()).add(Inked::of(format!("  {} hangs up", key_name("call")), Ink::Muted))];
    // A helper that took it has a moment more to open it; one that could not leaves the link to you.
    let offered = c.b("offered");
    let link = s.call_link.as_ref().filter(|(id, _, at)| *id == c.s("id") && s.now - at >= if offered { 3 * LINK_AFTER_MS } else { LINK_AFTER_MS }).map(|(_, l, _)| l.clone());
    if ringing && offered { out.push(Inked::of(format!("   {} picks it up…", c.s("offered")), Ink::Muted)); }
    if let (true, Some(l)) = (ringing, link) { out.push(Inked::plain("   ").add(Inked::of(if offered { "Not picked up there? Ctrl-click here to take the call in this computer's browser" } else { "No call helper answered. Ctrl-click here to take the call in this computer's browser" }, Ink::Accent).link(&l))); }
    if let Some(l) = c.arr("lines").last() {
        let by = WHO.iter().find(|(k, _)| *k == l.s("by")).map(|(_, w)| *w).unwrap_or("");
        out.push(Inked::of(format!("   {by}: "), Ink::Muted).add(Inked::of(l.t("text"), Ink::Soft)));
    }
    out
}

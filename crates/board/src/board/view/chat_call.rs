//! A voice call on now, under the line that says who the chat talks to: with whom, the key that hangs up, where it is
//! held and whether headphones are needed there (no echo cancelling), and the last thing said on it. While a call this
//! board rang waits, which computer picks it up, or that no call helper did after a few seconds and what to do.
use crate::board::keys::key_name;
use crate::board::state::State;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::voice::call;

/// How long a helper has to pick a call up before the chat says none did.
pub const LINK_AFTER_MS: i64 = 4000;
const WHO: [(&str, &str); 5] = [("you", "You"), ("voice", "Voice"), ("sent", "Sent"), ("board", "Board"), ("switch", "Board")];
pub fn lines(s: &State) -> Vec<Inked> {
    let Some(c) = s.data.call.as_ref() else { return vec![] };
    let ringing = c.s("state") == "ringing";
    let target = call::target(c).name;
    let head = if ringing { format!("Calling with {target}…") } else { format!("On a call with {target}") };
    let from = if call::from_terminal(c) { "" } else { " (from the web board)" };
    let mut out = vec![Inked::of("☎ ", Ink::Green).add(Inked::of(format!("{head}{from}"), Ink::Plain).bold()).add(Inked::of(format!("  {} hangs up", key_name("call")), Ink::Muted))];
    let offered = c.b("offered");
    let late = s.call_link.as_ref().map(|(id, at)| *id == c.s("id") && s.now - at >= LINK_AFTER_MS).unwrap_or(false);
    if ringing && offered { out.push(Inked::of(format!("   {} picks it up…", c.s("offered")), Ink::Muted)); }
    else if ringing && late { out.push(Inked::of("   No call helper picked it up: the computer you sit at needs herdr-projects running (herdr-projects voice says how it stands there).", Ink::Yellow)); }
    // where it is held, and headphones where nothing cancels the echo
    let a = c.at("audio");
    if !ringing && a.s("via") == "native" { out.push(Inked::of(format!("   on {}{}", a.s("agent"), if a.b("aec") { "" } else { " · headphones needed: no echo cancelling there" }), if a.b("aec") { Ink::Muted } else { Ink::Yellow })); }
    if let Some(l) = c.arr("lines").last() {
        let by = WHO.iter().find(|(k, _)| *k == l.s("by")).map(|(_, w)| *w).unwrap_or("");
        out.push(Inked::of(format!("   {by}: "), Ink::Muted).add(Inked::of(l.t("text"), Ink::Soft)));
    }
    out
}

//! A voice call on now, under the line that says who the chat talks to and laid out as it is: its mark in the mark's
//! column (one cell wide, as every mark of the board), then with whom and the key that hangs up; under it, indented as
//! the chat's other lines, where it is held, whether headphones are needed there (no echo cancelling) or your
//! microphone is too quiet to be heard, and the last thing said on it. While a call this board rang waits: which
//! computer picks it up, or that no call helper did after a few seconds and what to do.
use crate::board::keys::key_name;
use crate::board::state::State;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::voice::call;

/// How long a helper has to pick a call up before the chat says none did. A microphone whose loudest sound in this
/// long stayed under this (0 to 1) is too quiet to be heard.
pub const QUIET_MIC: f64 = 0.01;
pub const QUIET_AFTER_S: i64 = 10;
pub const LINK_AFTER_MS: i64 = 4000;
const WHO: [(&str, &str); 5] = [("you", "You"), ("voice", "Voice"), ("sent", "Sent"), ("board", "Board"), ("switch", "Board")];
pub fn lines(s: &State) -> Vec<Inked> {
    let Some(c) = s.data.call.as_ref() else { return vec![] };
    let ringing = c.s("state") == "ringing";
    let target = call::target(c).name;
    let head = if ringing { format!("Calling with {target}…") } else { format!("On a call with {target}") };
    let from = if call::from_terminal(c) { "" } else { " (from the web board)" };
    // the mark's column and the name's, as the line that says who (view/chat.rs: chat_head)
    let mut out = vec![Inked::plain(" ").add(Inked::of("●", Ink::Green)).add(Inked::plain(" ")).add(Inked::of(format!("{head}{from}"), Ink::Plain).bold()).add(Inked::of(format!("  {} hangs up", key_name("call")), Ink::Muted))];
    let offered = c.b("offered");
    let late = s.call_link.as_ref().map(|(id, at)| *id == c.s("id") && s.now - at >= LINK_AFTER_MS).unwrap_or(false);
    if ringing && offered { out.push(Inked::of(format!("   {} picks it up…", c.s("offered")), Ink::Muted)); }
    else if ringing && late { out.push(Inked::of("   No call helper picked it up: the computer you sit at needs herdr-projects running (herdr-projects voice says how it stands there).", Ink::Yellow)); }
    // where it is held; headphones where nothing cancels the echo; a microphone too quiet to be heard
    let a = c.at("audio");
    if !ringing && a.s("via") == "native" {
        let quiet = a.i("secs") >= QUIET_AFTER_S && a.f("mic") < QUIET_MIC;
        let mut warn = vec![];
        if !a.b("aec") { warn.push("headphones needed: no echo cancelling there".to_string()); }
        if quiet { warn.push("your microphone is very quiet: the voice may not hear you (herdr-projects voice test there)".to_string()); }
        out.push(Inked::of(format!("   on {}", a.s("agent")), Ink::Muted).add(Inked::of(warn.iter().map(|w| format!(" · {w}")).collect::<String>(), Ink::Yellow)));
    }
    if let Some(l) = c.arr("lines").last() {
        let by = WHO.iter().find(|(k, _)| *k == l.s("by")).map(|(_, w)| *w).unwrap_or("");
        out.push(Inked::of(format!("   {by}: "), Ink::Muted).add(Inked::of(l.t("text"), Ink::Soft)));
    }
    out
}

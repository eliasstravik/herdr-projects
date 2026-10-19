//! The call key, from the board's side: it rings a voice call for whoever the chat talks to, or hangs up the call that
//! is on; and on every read, a call this board started follows its chat (the switch, moving between tasks), and makes
//! a switch the voice agent was asked for as the board's own switch. Who the call talks to is published in the call
//! record for the front door, which runs the call's tools (crates/web/src/webui/voice/).
use super::Effect;
use crate::board::job::Job;
use crate::board::state::State;
use crate::board::tags::number_tag;
use crate::json::*;
use crate::shown::ink::Ink;
use crate::voice::call::{self, Target};

pub fn toggle(s: &mut State, fx: &mut Vec<Effect>) {
    if s.data.call.is_some() { s.call_link = None; fx.push(Effect::Run(Job::CallEnd)); return; }
    if s.ctx.web { return; } // the web board has a call button of its own, on the page
    let target = target(s);
    s.flash(format!("Calling with {}…", target.name), Ink::Muted);
    fx.push(Effect::Run(Job::CallRing { target }));
}
/// Who the chat talks to now, as the call record keeps it.
pub fn target(s: &State) -> Target {
    let reply = s.reply && !s.sel.is_empty() && (s.current().is_some() || super::coords::talking(s).is_some());
    let name = if let Some(c) = super::coords::talking(s) { format!("the {}", c.name) }
        else if let (true, Some(r)) = (reply, s.current()) { format!("{}'s agent ({})", number_tag(r, &s.data.keys, &s.scope, true), crate::shown::words::clean(r.s("title"))) }
        else { match crate::board::view::chat::coord_name(s).as_str() { "Coordinator" => "the Coordinator".to_string(), "New task" => "the board (no Coordinator here: it adds tasks)".to_string(), other => format!("the {other}") } };
    Target { id: if reply { s.sel.clone() } else { String::new() }, reply, scope: s.scope.clone(), name }
}
/// On every read: a call of this board's follows its chat, and a switch asked for by voice is made here.
pub fn follow(s: &mut State, fx: &mut Vec<Effect>) {
    let Some(c) = s.data.call.clone() else { s.call_link = None; return };
    if c.s("owner") != "terminal" || s.ctx.web { return; }
    if c.s("state") != "ringing" { s.call_link = None; }
    let sw = c.at("switch");
    if sw.i("n") > s.call_switch {
        s.call_switch = sw.i("n");
        let to = sw.t("to");
        if to == "coordinator" { s.reply = false; s.reply_id.clear(); }
        else if super::coords::exists(s, &to) { s.sel = to.clone(); s.reply = true; s.reply_id = to; super::list::relist(s); }
    }
    let t = target(s);
    if call::target(&c) != t { fx.push(Effect::Run(Job::CallTarget { target: t })); }
}

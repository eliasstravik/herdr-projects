//! The chat at the bottom, from the board's side: showing and hiding the latest message, scrolling one longer than its
//! box, and what you just sent to a task's agent, which stands in the box until that agent says something new.
use super::Effect;
use crate::board::state::{Sent, State};
use crate::json::*;

/// The one key for the latest message, both ways. A start that failed says why even while the chat is hidden, so
/// hiding it also puts that one error away: the same error stays hidden, the next one shows again.
pub fn toggle(s: &mut State, fx: &mut Vec<Effect>) {
    let always = !s.reply && !s.data.coord.error.is_empty() && s.err_hid != s.data.coord.error;
    if always { s.err_hid = s.data.coord.error.clone(); s.chat_open = false; } else { s.chat_open = !s.chat_open; }
    s.chat_at = None; // an opened conversation closes with it
    fx.push(Effect::Save);
    sync_coord(s, fx);
}
/// Whether the box shows a message now (as the screen was last drawn): then esc hides it before it closes anything.
pub fn shown(s: &State) -> bool { s.chat_fit.is_some() }
/// A task's last words and the turn they were said in.
pub fn turn_mark(r: &Obj) -> (String, Value, Value) { let h = r.at("handled"); (r.t("said"), h.at("msg").clone(), h.at("comp").clone()) }
pub fn note_sent(s: &mut State, id: &str, text: &str) {
    let r = s.items.iter().find(|x| x.s("id") == id).cloned().unwrap_or_default();
    s.just_sent.insert(id.into(), Sent { text: text.into(), over: turn_mark(&r) });
}
/// What you just sent this task's agent, while it has said nothing new since.
pub fn sent_to(s: &mut State, r: &Obj) -> String {
    let Some(sent) = s.just_sent.get(r.s("id")).cloned() else { return String::new() };
    let now = turn_mark(r);
    let moved = |a: &Value, b: &Value| !a.is_null() && !b.is_null() && a != b;
    let kind = if r.b("agent") { r.t("agent") } else { r.t("app") };
    // A program whose conversation cannot be read never says something new here: its screen shows instead.
    if now.0 != sent.over.0 || moved(&now.1, &sent.over.1) || moved(&now.2, &sent.over.2) || !crate::agents::kinds::kind(&kind).map(|k| k.reads_turn).unwrap_or(false) { s.just_sent.remove(r.s("id")); return String::new(); }
    sent.text
}
/// Called on every beat and after every change: a Coordinator reply you can see counts as seen.
pub fn sync_coord(s: &mut State, fx: &mut Vec<Effect>) {
    // A machine's or a repo's coordinator of this machine you talk to: its reply you can see counts as seen too.
    let (here, open) = (super::coords::wanted(s), s.chat_open && s.setup.is_none());
    if let Some((to, v, _)) = s.data.sub.as_mut().filter(|x| !x.1.unseen.is_empty() && open && here == x.0) { fx.push(Effect::Run(crate::board::job::Job::CoordSeen { unseen: v.unseen.clone(), to: to.clone() })); v.unseen.clear(); }
    let unseen = s.data.coord.unseen.clone();
    if unseen.is_empty() { return; }
    if !s.reply && s.chat_open && s.setup.is_none() { fx.push(Effect::Run(crate::board::job::Job::CoordSeen { unseen, to: String::new() })); s.data.coord.unseen.clear(); }
    else if s.reply && s.told_reply != unseen && s.msg.is_empty() { s.told_reply = unseen; s.flash(format!("The Coordinator replied. {} goes back to it.", crate::board::keys::key_name("talk")), crate::shown::ink::Ink::Green); }
}
/// Scroll the message `by` lines (up when negative). Down again from the end of a conversation that was opened (or
/// of a message scrolled through) closes it to the newest message alone, from its start. Nothing but what shows
/// changes: the place is yours now (view/chat_keep.rs).
pub fn scroll(s: &mut State, by: i64) -> bool {
    let (Some(f), Some(at)) = (s.chat_fit.clone(), s.chat_at.as_mut()) else { return false };
    let last = f.total.saturating_sub(f.rows);
    if by > 0 && f.top >= last && at.top.is_some() { at.top = None; return true; }
    if f.total <= f.rows { return false; }
    let top = (f.top as i64 + by).clamp(0, last as i64) as usize;
    at.top = Some(top); at.end = top >= last;
    if let Some(f) = s.chat_fit.as_mut() { f.top = top; f.above = top; f.below = last - top; }
    true
}
/// How far a key scrolls: half of what shows, so the line you were reading stays in view.
pub fn page(s: &State) -> i64 { (s.chat_fit.as_ref().map(|f| f.rows).unwrap_or(2) / 2).max(1) as i64 }

//! The chat at the bottom, from the board's side: showing and hiding the latest message, scrolling one longer than its
//! box, and what you just sent to a task's agent or a heading's coordinator, which stands in the box as yours from the
//! moment you send it until their own conversation has it or they say something new.
use super::Effect;
use super::coords::Target;
use crate::board::state::{Sent, State};
use crate::coordinator::look::Look;
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
/// Your words to a task's agent, the moment you send them: they stand in its chat as yours at once.
pub fn note_sent(s: &mut State, id: &str, text: &str) {
    let r = s.items.iter().find(|x| x.s("id") == id).cloned().unwrap_or_default();
    s.just_sent.insert(id.into(), Sent { text: text.into(), over: turn_mark(&r), at: s.now, ..Default::default() });
}
/// Where a coordinator's last words stand: something new said, or an error, moves it.
fn talk_mark(v: &Look) -> String { format!("{:?}|{}|{}", v.talk.last().map(|m| (m.you, &m.text)), v.said, v.error) }
/// Your words to a coordinator the chat shows (a heading's, or another machine's Coordinator), the moment you send them.
pub fn note_coord(s: &mut State, t: &Target, text: &str) {
    let v = super::coords::look_of(s, t).unwrap_or_default();
    // Words sent it just before, which may still be on their way: they keep no row over the chat either.
    let before = s.just_sent.get(&t.key()).map(|x| x.before.iter().cloned().chain([x.text.clone()]).collect::<Vec<_>>()).unwrap_or_default();
    let before = before[before.len().saturating_sub(8)..].to_vec();
    s.just_sent.insert(t.key(), Sent { text: text.into(), over: (talk_mark(&v), Value::Null, Value::Null), at: s.now, before });
}
/// Another machine's dispatch that carries `text`, sent the coordinator kept under `key`.
fn carries(j: &Obj, key: &str, text: &str) -> bool {
    let host = key.strip_prefix("co:").and_then(|k| k.split('\t').next()).unwrap_or_default();
    !host.is_empty() && j.s("remote") == host && j.s("text") == format!("@{host} {text}")
}
/// A dispatch on its way that the chat shows as your words (or that went just before them): no row of its own over the
/// chat. One that failed has one.
pub fn in_chat(s: &State, j: &Obj) -> bool { j.s("state") != "error" && s.just_sent.iter().any(|(k, x)| x.before.iter().chain([&x.text]).any(|t| carries(j, k, t))) }
/// What you just sent this coordinator, while its own conversation does not show it yet (`v`: the coordinator as the
/// chat reads it now). It goes when that conversation has your words, says something new once they are there, or the
/// words did not get there.
pub fn coord_sent(s: &mut State, t: &Target, v: &Look) -> String {
    let key = t.key();
    let Some(sent) = s.just_sent.get(&key).cloned() else { return String::new() };
    let head: String = sent.text.chars().take(200).collect(); // as much as a queued message keeps of them
    let on_way = s.data.jobs.iter().any(|j| carries(j, &key, &sent.text) && j.s("state") != "error");
    let failed = s.data.jobs.iter().any(|j| carries(j, &key, &sent.text) && j.s("state") == "error" && j.f("created") as i64 >= sent.at);
    if failed || v.sent.contains(&head) || (!on_way && talk_mark(v) != sent.over.0) { s.just_sent.remove(&key); return String::new(); }
    sent.text
}
/// What you just sent this task's agent, while it has said nothing new since.
pub fn sent_to(s: &mut State, r: &Obj) -> String {
    let Some(sent) = s.just_sent.get(r.s("id")).cloned() else { return String::new() };
    let now = turn_mark(r);
    // Still on its way: what the agent finishes meanwhile is its earlier turn, so your words stay and wait from there.
    let reads = { let kind = if r.b("agent") { r.t("agent") } else { r.t("app") }; crate::agents::kinds::kind(&kind).map(|k| k.reads_turn).unwrap_or(false) };
    let going = reads && s.pending.get(r.s("id")).map(|p| ["Tell", "far:tell"].contains(&p.kind.as_str()) && p.answered.is_none() && p.failed.is_none()).unwrap_or(false);
    if going { if let Some(x) = s.just_sent.get_mut(r.s("id")) { x.over = now; } return sent.text; }
    let moved = |a: &Value, b: &Value| !a.is_null() && !b.is_null() && a != b;
    // A program whose conversation cannot be read never says something new here: its screen shows instead.
    if now.0 != sent.over.0 || moved(&now.1, &sent.over.1) || moved(&now.2, &sent.over.2) || !reads { s.just_sent.remove(r.s("id")); return String::new(); }
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
    else if s.reply && s.told_reply != unseen && s.msg.is_empty() { s.told_reply = unseen; let who = super::top::top(s).map(|t| crate::board::update::top::sentence(&t.name)).unwrap_or_else(|| "The coordinator".into()); s.flash(format!("{who} replied. {} goes back to it.", crate::board::keys::key_name("talk")), crate::shown::ink::Ink::Muted); }
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

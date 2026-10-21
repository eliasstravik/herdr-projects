//! The board's beat: once a second the time moves on (a hint that closes runs out, a popup that opens an agent goes),
//! and what the loader read comes in: the list is made again, a question herdr's approve action asked for is asked, a
//! line left for this board is said, a reply you can see counts as seen, and the bell rings for what newly needs you.
use super::list::relist;
use super::Effect;
use crate::board::data::Data;
use crate::board::state::{Held, State};
use crate::json::*;
use crate::shown::ink::Ink;

pub fn tick(s: &mut State, now: i64, fx: &mut Vec<Effect>) {
    s.now = now;
    s.ticks += 1;
    if let Some((until, _)) = s.closing { if now >= until { s.closing = None; } }
    if s.leaving && now >= s.leave_at { fx.push(Effect::Close); }
    if s.all() && s.ticks % 5 == 0 { fx.push(Effect::Poll(false)); }
    // herdr is asked what it shows of the board's pane every two seconds (size.rs).
    if s.ticks % 2 == 0 && !s.ctx.pane.is_empty() { s.size.ask = true; }
    if s.ticks == 3 { super::web_ui::look(s, fx); }
    if super::pending::tick(s) { relist(s); }
}
pub fn loaded(s: &mut State, d: Data, fx: &mut Vec<Effect>) {
    let first = !s.data.loaded;
    let approve = d.approve.clone();
    let heard = d.heard.clone();
    s.data = d;
    super::pending::read(s);
    relist(s);
    // The task left the list, so the selection went to its neighbour: the message says which.
    if !s.left.is_empty() && s.items.iter().any(|r| r.s("id") == s.left && r.b("archivedAt") != s.archive_view) {
        let left = std::mem::take(&mut s.left);
        if let Some(next) = s.current().filter(|n| n.s("id") != left).cloned() { if !s.msg.is_empty() { let n = super::actions::name(s, &next); s.msg = format!("{} Now on {n}.", s.msg); } }
    }
    // herdr's "approve the task in the focused pane" action lands here: the board opens on that task and asks first.
    if !approve.is_empty() { if let Some(r) = s.items.iter().find(|x| x.s("id") == approve && !x.b("archivedAt")).cloned() { s.menu = None; s.sel = r.t("id"); relist(s); super::actions::yes(s, &r, fx); } }
    if let Some((text, kind)) = heard { s.flash(text, ink_of(&kind)); }
    super::chat::sync_coord(s, fx);
    super::call::follow(s, fx);
    left_chats(s, fx);
    ring(s, first, fx);
    announce(s, fx);
}
/// A task whose chat changed its code waits for you to be done: on this board, being done is selecting another task.
fn left_chats(s: &State, fx: &mut Vec<Effect>) {
    for r in s.data.items.iter().filter(|r| !r.b("host") && r.s("id") != s.sel && crate::flow::chat::held(r) && r.at("chat").s("via") == "board" && !r.at("chat").b("left")) {
        fx.push(Effect::Run(crate::board::job::Job::ChatLeft { id: r.t("id") }));
    }
}
pub fn ink_of(kind: &str) -> Ink { match kind { "red" => Ink::Red, "green" => Ink::Green, "yellow" => Ink::Yellow, "peach" => Ink::Peach, "pilot" => Ink::Pilot, _ => Ink::Muted } }
/// `board.notify: bell`: the terminal bell, once for each task that newly needs you, is ready to approve or failed.
/// What was already so when the board opened rings nothing.
fn ring(s: &mut State, first: bool, fx: &mut Vec<Effect>) {
    let newest = s.data.items.iter().filter(|r| !r.b("host") && !r.b("archivedAt")).map(|r| r.at("notice").i("at")).max().unwrap_or(0);
    if first || s.rang == 0 { s.rang = newest.max(s.now); return; }
    if newest <= s.rang { return; }
    s.rang = newest;
    if s.data.notify == "bell" { fx.push(Effect::Bell); }
}
/// For a board run as a plain command: its tasks as OSC 7501 records on its own terminal. Output only.
fn announce(s: &mut State, fx: &mut Vec<Effect>) {
    if s.ctx.web || crate::util::env_set("HERDR_PLUGIN_ID") || std::env::var("HERDR_PROJECTS_PROGRAM_STATUS").as_deref() == Ok("0") { return; }
    let mut next = std::collections::HashMap::new();
    for r in s.data.items.iter().filter(|r| !r.b("host") && !r.b("archivedAt") && r.s("state") != "idle" && r.s("state") != "done") { next.insert(r.t("id"), osc(r)); }
    let mut out = String::new();
    for (id, seq) in &next { if s.osc_sent.get(id) != Some(seq) { out.push_str(seq); } }
    for id in s.osc_sent.keys() { if !next.contains_key(id) { out.push_str(&osc_clear(id)); } }
    s.osc_sent = next;
    if !out.is_empty() { fx.push(Effect::Osc(out)); }
}
pub fn osc(r: &Obj) -> String {
    let b64 = |x: &str| crate::util::base64(x.as_bytes());
    let mut pairs = vec![format!("state={}", r.t("state")), format!("id={}", r.t("id"))];
    if r.b("kind") { pairs.push(format!("kind={}", r.t("kind"))); }
    if r.has("progress") { pairs.push(format!("progress={}", r.t("progress"))); }
    if r.has("eta") { pairs.push(format!("eta={}", r.t("eta"))); }
    if crate::re!(r"^[A-Za-z0-9_.+-]{1,32}$").is_match(r.s("app")) { pairs.push(format!("app={}", r.s("app"))); }
    if r.b("title") { pairs.push(format!("title={}", b64(&crate::store::tasks::one_line(r.s("title"), 192)))); }
    if r.b("msg") { pairs.push(format!("msg={}", b64(&crate::store::tasks::one_line(r.s("msg"), 600)))); }
    format!("\x1b]7501;{}\x1b\\", pairs.join(":"))
}
pub fn osc_clear(id: &str) -> String { format!("\x1b]7501;state=clear:id={id}\x1b\\") }
/// A remote view holds the terminal at another size: said only after two looks in a row agree on that size, so a
/// window that is being resized is never taken for one.
pub fn held(s: &mut State, h: Option<Held>, viewed: Option<bool>) {
    if let Some(v) = viewed { s.viewed = v; }
    // The view the board was the guest of is gone: the window takes its terminal again.
    if s.guest && h.as_ref().map(|h| h.known && h.by.is_empty()).unwrap_or(true) { s.guest = false; s.size.hosts = None; }
    let seen = h.as_ref().map(|h| format!("{}x{}", h.columns, h.rows)).unwrap_or_default();
    s.held = if !seen.is_empty() && seen == s.size.held_seen { h } else { None };
    s.size.held_seen = seen;
}
/// The program changed under the open board: it loads the new version by itself, once the file has stopped changing,
/// and only while nothing of yours is half typed.
pub fn stamp(s: &mut State, st: String, fx: &mut Vec<Effect>) {
    if st.is_empty() { s.stale.clear(); return; }
    let settled = s.stale == st;
    s.stale = st;
    if settled && s.at_rest() && s.ctx.can_reload { fx.push(Effect::Save); fx.push(Effect::Reload); }
}

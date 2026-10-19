//! The board a call belongs to, as the front door holds it: the web board's own page state for a call started there (so
//! the call follows that page's switch), or one board of its own lined up with the terminal board's chat, which the
//! terminal board publishes in the call record (voice/call.rs).
use crate::board::view::chat::{chat_of, Who};
use crate::json::*;
use crate::voice::call::{self, Target};
use crate::webui::web::{self, Client};
use std::sync::{Arc, Mutex};

pub const TERMINAL: &str = "voice-terminal";
/// The page id of a call's web board, or None for the terminal board's.
pub fn page_of(c: &Obj) -> Option<String> { c.s("owner").strip_prefix("web:").map(String::from) }
/// The board the call's tools work on, lined up with who the call talks to.
pub fn of(c: &Obj) -> Arc<Mutex<Client>> {
    if let Some(p) = page_of(c) { let b = web::client(&p); { let mut g = b.lock().unwrap(); web::take_in(&mut g); web::reload(&mut g); } return b; }
    let b = web::client(TERMINAL);
    { let mut g = b.lock().unwrap(); web::take_in(&mut g); line_up(&mut g, &call::target(c)); }
    b
}
/// The board's chat pointed at `t`, as the terminal board has it.
pub fn line_up(g: &mut Client, t: &Target) {
    if g.s.scope != t.scope { g.s.scope = t.scope.clone(); crate::board::update::list::relist(&mut g.s); }
    g.s.reply = t.reply && !t.id.is_empty();
    g.s.reply_id = if g.s.reply { t.id.clone() } else { String::new() };
    if g.s.reply { g.s.sel = t.id.clone(); }
    web::reload(g);
}
/// Who the board's chat talks to now, as a target the call record keeps.
pub fn target_of(g: &mut Client) -> Target {
    let w = who(g);
    let reply = g.s.reply && !g.s.sel.is_empty();
    Target { id: if reply { g.s.sel.clone() } else { String::new() }, reply, scope: g.s.scope.clone(), name: w.map(|w| name_of(&w)).unwrap_or_else(|| "the Coordinator".into()) }
}
pub fn who(g: &mut Client) -> Option<Who> { let r = g.s.current().cloned(); chat_of(&mut g.s, web::theme(), r.as_ref()) }
/// Who it is, as said aloud: "HP-52's agent", "the Coordinator".
pub fn name_of(w: &Who) -> String {
    if !w.tag.is_empty() { return format!("{}'s agent ({})", w.tag, w.name); }
    match w.name.as_str() { "Coordinator" => "the Coordinator".into(), "New task" => "the board (no Coordinator here: it adds tasks)".into(), n if n.starts_with("Coordinator on ") => format!("the {n}"), n => n.to_string() }
}
/// The call of this machine as a web board's page shows it: none, or whether it is this page's own, where it was
/// started, who it talks to, the computer that holds it (`on`, a call helper's), how long it has run (`secs`, counted
/// here so that the page's clock does not matter) and the last things said.
pub fn shown(page: &str) -> Value {
    let Some(c) = call::get() else { return Value::Null };
    let lines: Vec<Value> = c.arr("lines").iter().rev().take(3).rev().map(|l| json!({ "by": l.s("by"), "text": crate::text::cut(l.s("text"), 300) })).collect();
    let live = c.s("state") == "live";
    let secs = if live { (crate::clock::now() - c.i("at").max(c.i("answered"))).max(0) / 1000 } else { 0 };
    let a = c.at("audio");
    let held = if live && a.s("via") == "native" { a.t("agent") } else { String::new() };
    json!({ "on": true, "mine": !page.is_empty() && page_of(&c).as_deref() == Some(page), "from": if call::from_terminal(&c) { "terminal" } else { "web" }, "state": c.s("state"), "target": call::target(&c).name, "held": held, "secs": secs, "lines": lines })
}

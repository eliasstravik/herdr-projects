//! What the board was like when it last closed: selection, filter, draft, side panel, chat, whose board, the words put
//! away, the folded headings, whose herdr sidebar it lists the machines in the order of. The popup has a file of its
//! own: a board may run in a tab at the same time, and each keeps its own. The first popup starts from the tab's view,
//! without its draft.
use crate::board::state::{Away, Mode, State};
use crate::json::*;
use crate::store::files::{data_dir, read_obj, write_json};
use std::path::PathBuf;

pub fn file(popup: bool) -> PathBuf { data_dir().join(if popup { "view-popup.json" } else { "view.json" }) }
pub fn view_file(popup: bool, name: &str) -> PathBuf {
    if name.is_empty() { return file(popup); }
    // "popup" is a valid view name, but view-popup.json belongs to the legacy popup.
    let dir = if popup { data_dir().join("view-popups") } else if name == "popup" { data_dir().join("view-named") } else { data_dir() };
    dir.join(format!("view-{name}.json"))
}
pub fn read_view(popup: bool, name: &str) -> Obj {
    if name.is_empty() { return read(popup); }
    if let Some(own) = read_obj(&view_file(popup, name)) { return own; }
    let mut tab = if popup { read_obj(&view_file(false, name)).unwrap_or_default() } else { Obj::new() };
    tab.del("draft"); tab.del("away"); tab
}
pub fn read(popup: bool) -> Obj {
    if !popup { return read_obj(&file(false)).unwrap_or_default(); }
    if let Some(own) = read_obj(&file(true)) { return own; }
    let mut tab = read_obj(&file(false)).unwrap_or_default();
    tab.del("draft"); tab.del("away");
    tab
}
pub fn apply(s: &mut State, p: &Obj) {
    s.by_repo = p.b("byRepo"); s.side = p.b("side"); s.scope = p.t("scope"); s.told = p.i("told");
    s.repos = p.arr("repos").iter().filter_map(|v| v.as_str()).map(String::from).collect();
    s.sel = p.t("sel"); s.filter = p.t("filter");
    s.folded = p.arr("folded").iter().filter_map(|x| x.as_str()).map(String::from).collect();
    if p.b("draft") { s.input.set_text(p.s("draft")); }
    // So do words that were put away: ctrl+z still brings them back after the board closed or reloaded itself.
    if p.at("away").b("text") { let a = p.at("away"); s.draft_away = Some(Away { text: a.t("text"), to: a.t("to"), cwd: if a.b("cwd") { a.t("cwd") } else { p.t("cwd") }, warned: false }); }
    s.chat_open = p.s("chat") != "closed";
    s.sidebar_from = p.at("sidebarFrom").as_str().map(String::from);
}
pub fn save(s: &State) {
    let draft = if s.mode == Mode::New && !s.reply { s.value() } else { String::new() };
    let away = if s.mode == Mode::New && s.reply && !s.value().trim().is_empty() { json!({ "text": s.value(), "to": s.reply_id }) }
        else { s.draft_away.as_ref().map(|d| json!({ "text": d.text, "to": d.to, "cwd": if d.cwd.is_empty() { s.ctx.cwd.clone() } else { d.cwd.clone() } })).unwrap_or(Value::Null) };
    let mut p = obj(json!({ "byRepo": s.by_repo, "side": s.side, "scope": s.scope, "sel": s.sel, "filter": s.filter, "told": s.told, "draft": draft, "chat": if s.chat_open { "open" } else { "closed" }, "cwd": s.ctx.cwd, "away": away, "folded": s.folded, "sidebarFrom": s.sidebar_from }));
    if !s.view_name.is_empty() { p.put("repos", json!(s.repos)); }
    let _ = write_json(&view_file(s.ctx.popup, &s.view_name), &Value::Object(p));
}

//! What the board was like when it last closed: selection, filter (and the filter row's), grouping, the tab shown, draft, side panel,
//! chat, whose board, the words put
//! away, the folded headings, whose herdr sidebar it lists the machines in the order of. The popup has a file of its
//! own: a board may run in a tab at the same time, and each keeps its own. The first popup starts from the tab's view,
//! without its draft.
use crate::board::state::{Away, Mode, State};
use crate::json::*;
use crate::store::files::{data_dir, read_obj, write_json};
use std::path::PathBuf;

pub fn file(popup: bool) -> PathBuf { data_dir().join(if popup { "view-popup.json" } else { "view.json" }) }
pub fn read(popup: bool) -> Obj {
    if !popup { return read_obj(&file(false)).unwrap_or_default(); }
    if let Some(own) = read_obj(&file(true)) { return own; }
    let mut tab = read_obj(&file(false)).unwrap_or_default();
    tab.del("draft"); tab.del("away");
    tab
}
pub fn apply(s: &mut State, p: &Obj) {
    s.by_repo = p.b("byRepo"); s.side = p.b("side"); s.scope = p.t("scope"); s.told = p.i("told");
    s.sel = p.t("sel"); s.filter = p.t("filter");
    s.folded = p.arr("folded").iter().filter_map(|x| x.as_str()).map(String::from).collect();
    if p.b("draft") { s.input.set_text(p.s("draft")); }
    // So do words that were put away: ctrl+z still brings them back after the board closed or reloaded itself.
    if p.at("away").b("text") { let a = p.at("away"); s.draft_away = Some(Away { text: a.t("text"), to: a.t("to"), cwd: if a.b("cwd") { a.t("cwd") } else { p.t("cwd") }, warned: false }); }
    s.chat_open = p.s("chat") != "closed";
    s.sidebar_from = p.at("sidebarFrom").as_str().map(String::from);
    // The filter shown and the third grouping (update/shows.rs, update/keys.rs).
    s.show = (p.i("show").max(0) as usize).min(crate::board::update::shows::SHOWS.len() - 1);
    s.by_machine = p.b("byMachine") && !s.by_repo;
    // Which tab shows, and the Pinned tab's own folds (update/pinned.rs).
    crate::board::update::pinned::load(s, p);
}
pub fn save(s: &State) {
    let draft = if s.mode == Mode::New && !s.reply { s.value() } else { String::new() };
    let away = if s.mode == Mode::New && s.reply && !s.value().trim().is_empty() { json!({ "text": s.value(), "to": s.reply_id }) }
        else { s.draft_away.as_ref().map(|d| json!({ "text": d.text, "to": d.to, "cwd": if d.cwd.is_empty() { s.ctx.cwd.clone() } else { d.cwd.clone() } })).unwrap_or(Value::Null) };
    let mut view = obj(json!({ "side": s.side, "scope": s.scope, "sel": s.sel, "filter": s.filter, "told": s.told, "draft": draft, "chat": if s.chat_open { "open" } else { "closed" }, "cwd": s.ctx.cwd, "away": away, "sidebarFrom": s.sidebar_from }));
    // The grouping, the filter row's and the folds are the board's own also while the Pinned tab shows (update/pinned.rs).
    view.extend(crate::board::update::pinned::saved(s));
    let _ = write_json(&file(s.ctx.popup), &Value::Object(view));
}

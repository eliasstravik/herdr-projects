//! What the keys do on an automation's row in the main list: enter opens it (and, on a firing in an open one, goes to
//! the task it made), space turns it on or off, tab fires it now, ctrl+x removes it (it asks first), ctrl+e opens its
//! settings. No task is selected on such a row, so what you type goes to the Coordinator. An automation of another
//! machine is changed on that machine, as its tasks are.
use super::ask::confirm;
use super::list::relist;
use super::menu::Item;
use super::Effect;
use crate::board::job::Job;
use crate::board::keys::{is, key_name, name, pair, typed, Press, KEYS, K};
use crate::board::state::{Editing, State, Valid, Whose};
use crate::json::*;
use crate::shown::ink::Ink;
use crate::shown::words::clean;

/// Keys that act on a task and on nothing else: on an automation's row they do nothing, and have no line in the menu.
const TASK_ONLY: [K; 4] = [K::Yes, K::SendBack, K::Archive, K::Pin];
/// The board's keys that still do what they always do while one automation is open, whole.
const OPEN_SAME: [K; 6] = [K::Menu, K::Close, K::Undo, K::ChatMessage, K::ChatScrollUp, K::ChatScrollDown];
/// A plain key's line in the ? menu, where it does something else on an automation's row than on a task's.
pub fn auto_help(k: K) -> Option<&'static str> {
    match k { K::Enter => Some("send what is typed; with nothing typed, open this automation (on a firing in it: go to the task it made)"), K::Talk => Some("turn this automation on or off"), K::Delegate => Some("fire this automation now"), _ => None }
}
pub fn firings(a: &Obj) -> Vec<Value> { a.at("shown").arr("firings").to_vec() }
pub fn can_set(a: &Obj) -> bool { !a.b("host") || !a.at("shown").arr("settings").is_empty() }
fn far(s: &mut State, a: &Obj, action: &str, extra: Value, fx: &mut Vec<Effect>) {
    let doing = match action { "auto-turn" => if extra.b("on") { "Turning on" } else { "Turning off" }, "auto-run" => "Firing", "auto-remove" => "Removing", _ => "Changing a setting of" };
    s.flash(format!("{doing} \"{}\" on {}…", a.s("name"), a.s("host")), Ink::Muted);
    let m = s.remotes.get(a.s("host")).map(|m| m.machine.clone()).unwrap_or_default();
    let mut req = obj(extra); req.put("id", a.t("id"));
    fx.push(Effect::Run(Job::Far { machine: m, host: a.t("host"), action: action.into(), req: Value::Object(req), quiet: false }));
}
pub fn toggle(s: &mut State, a: &Obj, fx: &mut Vec<Effect>) { if a.b("host") { return far(s, a, "auto-turn", json!({ "on": !a.b("on") }), fx); } fx.push(Effect::Run(Job::AutoTurn { id: a.t("id"), on: !a.b("on") })); }
pub fn fire(s: &mut State, a: &Obj, fx: &mut Vec<Effect>) {
    if !a.b("on") { return s.flash(format!("\"{}\" is off, and off it never fires. {} turns it on.", a.s("name"), key_name("talk")), Ink::Yellow); }
    if a.b("host") { return far(s, a, "auto-run", json!({}), fx); }
    fx.push(Effect::Run(Job::AutoFire { id: a.t("id") }));
}
pub fn remove(s: &mut State, a: &Obj) {
    let job = if a.b("host") { let m = s.remotes.get(a.s("host")).map(|m| m.machine.clone()).unwrap_or_default(); Job::Far { machine: m, host: a.t("host"), action: "auto-remove".into(), req: json!({ "id": a.t("id") }), quiet: false } } else { Job::AutoRemove { id: a.t("id"), name: a.t("name") } };
    let q = format!("Remove the automation \"{}\"{}? The tasks it made stay.", crate::text::cut(&clean(a.s("name")), 32), if a.b("host") { format!(" on {}", a.s("host")) } else { String::new() });
    confirm(s, q, job, Valid::Always, vec![], String::new(), false);
}
/// The whole automation, and back to the list.
pub fn fold(s: &mut State) { s.auto_open = !s.auto_open; s.full = false; s.editing = None; s.auto_at = -1; s.panel_top = 0; }
pub fn settings(s: &mut State, a: &Obj) {
    if s.ctx.web { return super::web::auto_settings(s); }
    if s.editing.as_ref().map(|e| e.whose == Whose::Auto).unwrap_or(false) { s.editing = None; s.panel_top = 0; return; }
    if !can_set(a) { return s.flash(format!("Its settings are changed on {}: herdr-projects set <name> <value> --automation \"{}\"", a.s("host"), a.s("name")), Ink::Yellow); }
    s.editing = Some(Editing { id: a.t("row"), at: 0, typing: String::new(), whose: Whose::Auto }); s.auto_open = true; s.full = false; s.panel_top = 0;
}
/// Enter on a firing: to the task it made, on the board.
pub fn go_to(s: &mut State, id: &str) {
    let Some(t) = s.items.iter().find(|r| r.s("id") == id).cloned() else { return s.flash(format!("{id} is gone: it was removed."), Ink::Yellow) };
    s.auto_open = false; s.auto_at = -1; s.editing = None; s.panel_top = 0;
    if t.b("archivedAt") != s.archive_view { super::actions::toggle_archive(s); }
    s.sel = id.into(); s.stick = id.into();
    relist(s);
}
/// A key while an automation's row is selected and nothing is typed. True when the row took it.
pub fn key(s: &mut State, p: &Press, fx: &mut Vec<Effect>) -> bool {
    if !s.value().is_empty() || s.mode != crate::board::state::Mode::New { return false; }
    let Some(a) = s.auto().cloned() else { return false };
    let open = s.auto_open;
    if is(p, K::Back) { if open { fold(s); } return open; }
    if open && (is(p, K::Up) || is(p, K::Down)) { s.auto_at = (s.auto_at + if is(p, K::Up) { -1 } else { 1 }).clamp(-1, firings(&a).len().min(20) as i64 - 1); return true; }
    if open && (is(p, K::PageUp) || is(p, K::PageDown)) { let by = s.panel_height.saturating_sub(1).max(1) as i64; super::actions::scroll_panel(s, if is(p, K::PageUp) { -by } else { by }); return true; }
    if is(p, K::Panel) { fold(s); return true; }
    if is(p, K::Enter) {
        let f = if open { usize::try_from(s.auto_at).ok().and_then(|n| firings(&a).get(n).cloned()) } else { None };
        if !open { fold(s); }
        else if let Some(t) = f.as_ref().filter(|f| f.b("task")) { let id = if a.b("host") { format!("{}:{}", a.s("host"), t.s("task")) } else { t.t("task") }; go_to(s, &id); }
        else { s.flash(if f.is_some() { "That firing was skipped: it made no task.".to_string() } else { format!("{} pick a firing; {} then goes to the task it made.", pair(K::Up), name(K::Enter)) }, Ink::Muted); }
        return true;
    }
    if is(p, K::Talk) { toggle(s, &a, fx); return true; }
    if is(p, K::Delegate) { fire(s, &a, fx); return true; }
    if is(p, K::Stop) { remove(s, &a); return true; }
    if is(p, K::Settings) { settings(s, &a); return true; }
    match typed(p, |k| super::keys::has_run(k.id)) { Some(k) => TASK_ONLY.contains(&k) || (open && !OPEN_SAME.contains(&k)), None => false }
}
/// The ? menu on an automation's row: the keys that do something there, in the words of what they do there.
pub fn menu_items(s: &State, a: &Obj) -> Vec<Item> {
    let words = |k: K| -> Option<String> { match k {
        K::Stop => Some("remove this automation (it asks first)".into()),
        K::Settings => Some(if can_set(a) { "this automation's settings: see and change each one".into() } else { String::new() }),
        K::Panel => Some(if s.auto_open { "back to the list".into() } else { "show the whole automation, full screen".into() }),
        K::Side => Some(format!("{} the side panel: this automation, beside the list", if s.side { "hide" } else { "show" })),
        _ => None } };
    KEYS.iter().filter(|k| (k.group == Some("ctrl") || k.group == Some("alt")) && !TASK_ONLY.contains(&k.id) && (!s.auto_open || words(k.id).is_some() || OPEN_SAME.contains(&k.id)))
        .map(|k| Item { id: k.id, group: k.group.unwrap(), key: name(k.id), mine: false, label: words(k.id).unwrap_or_else(|| super::menu::label(s, k.id, None)) }).filter(|i| !i.label.is_empty()).collect()
}
pub fn run_menu(s: &mut State, k: K, fx: &mut Vec<Effect>) {
    let Some(a) = s.auto().cloned() else { return };
    match k { K::Stop => remove(s, &a), K::Settings => settings(s, &a), K::Panel => fold(s), K::Side => super::actions::toggle_side(s, fx), _ => super::keys::run(s, k, None, fx) }
}

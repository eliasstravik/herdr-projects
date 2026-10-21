//! Where the board runs in herdr. The board key opens it in a tab of its own (an ordinary pane that stays when you go
//! into an agent) or, for whoever chose it (`board.open: popup`), in herdr's popup. Where it is, is written down while
//! it runs, so opening the board from anywhere goes back to that one instead of starting a second one. Pressed on the
//! board itself, the key goes back to where you came from. The board's own tab is named `herdr-projects` while it
//! runs, and gets its old name back when it closes.
use super::say::say;
use crate::herdr::cli::{current_socket, session_name};
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::files::{data_dir, pid_alive, read_json, write_json};
use crate::store::settings::lookup::setting;

fn note_file(socket: &str) -> std::path::PathBuf { data_dir().join(format!("board-pane-{}.json", session_name(socket))) }
fn popup_file(socket: &str) -> std::path::PathBuf { data_dir().join(format!("board-popup-{}.json", session_name(socket))) }
fn env(k: &str) -> String { std::env::var(k).unwrap_or_default() }
/// The popup is no pane and has no id: this stands for it where a board is named.
pub const POPUP: &str = "popup";
/// Is this board in herdr's popup? herdr gives a popup's program its plugin's id and no pane.
pub fn in_popup() -> bool { env("HERDR_ENV") == "1" && !env("HERDR_PLUGIN_ID").is_empty() && env("HERDR_PANE_ID").is_empty() }
/// The herdr pane this board runs in, to come back to: none outside herdr and none in the popup.
pub fn own_pane() -> String { if env("HERDR_ENV") == "1" && !in_popup() { env("HERDR_PANE_ID") } else { String::new() } }
pub fn own_name() -> String { let p = own_pane(); if !p.is_empty() { p } else if in_popup() { POPUP.into() } else { String::new() } }
/// Where you are, as herdr tells a plugin's command and pane: the pane that has the keyboard, and its tab.
pub fn context() -> (String, String) {
    let c: Value = serde_json::from_str(&env("HERDR_PLUGIN_CONTEXT_JSON")).unwrap_or(Value::Null);
    (c.t("focused_pane_id"), c.t("tab_id"))
}
pub fn arrive() {
    let socket = current_socket();
    if in_popup() { let _ = write_json(&popup_file(&socket), &json!({ "tab": context().1, "pid": std::process::id() })); return; }
    let pane = own_pane();
    if pane.is_empty() { return; }
    let old = read_json(&note_file(&socket));
    let came = if env("HERDR_PLUGIN_ID").is_empty() { String::new() } else { context().0 };
    let from = match &old { Some(o) if o.s("pane") == pane && o.b("from") => o.t("from"), _ => if came != pane { came } else { String::new() } };
    let mut n = obj(json!({ "pane": pane, "pid": std::process::id() }));
    if !from.is_empty() { n.put("from", from); }
    let _ = write_json(&note_file(&socket), &Value::Object(n));
}
/// Only the board that wrote the note takes it away: another one may have started meanwhile.
pub fn depart() {
    let socket = current_socket();
    for f in [note_file(&socket), popup_file(&socket)] { if read_json(&f).map(|n| n.i("pid") == std::process::id() as i64).unwrap_or(false) { let _ = std::fs::remove_file(f); } }
}
fn go_to(api: &Herdr, pane: &Value) { let _ = api.workspace_focus(pane.s("workspace_id")); let _ = api.tab_focus(pane.s("tab_id")); let _ = api.pane_focus(pane.s("pane_id")); }
/// The board that runs in a pane of this herdr: its note and its pane as herdr has it.
fn running(socket: &str) -> Option<(Herdr, Value, Value)> {
    let n = read_json(&note_file(socket))?;
    if !n.b("pane") || !pid_alive(n.i("pid")) { return None; }
    let api = Herdr::new(Some(socket.into()));
    let pane = api.pane_get(n.s("pane"))?;
    Some((api, n, pane))
}
/// Go to the running board. False when there is none.
pub fn go_back(socket: &str) -> bool { match running(socket) { Some((api, _, pane)) => { go_to(&api, &pane); true } None => false } }
fn why(e: &crate::herdr::HerdrError) -> String {
    if e.code == "ui_busy" { "herdr has another popup open, maybe hidden behind another tab, and shows one at a time. Close that one for the board's popup".into() }
    else { format!("herdr did not open the popup ({})", crate::text::cut(&crate::text::squash(if e.message.is_empty() { &e.code } else { &e.message }), 120)) }
}
fn popup_wanted() -> bool { setting("board.open", None) == json!("popup") }
/// How much of herdr's screen the popup takes: in a small window all of it.
pub fn size_for(area: Option<&Value>) -> (&'static str, &'static str) { match area { Some(a) if a.i("width") < 110 || a.i("height") < 30 => ("100%", "100%"), _ => ("90%", "90%") } }
/// The board key with the popup chosen: none open: opened; one shows: closed (the toggle; `keep`: left open).
fn toggle(socket: &str, here: &(String, String), keep: bool) -> Result<String, crate::herdr::HerdrError> {
    let f = popup_file(socket);
    let at = read_json(&f).filter(|n| (n.i("pid") > 0 && pid_alive(n.i("pid"))) || (n.i("pid") == 0 && crate::clock::now() - n.i("at") < 10000));
    if read_json(&f).is_some() && at.is_none() { let _ = std::fs::remove_file(&f); }
    if let Some(n) = &at {
        let shows = here.1.is_empty() || !n.b("tab") || n.s("tab") == here.1;
        if shows && keep { return Ok(POPUP.into()); }
        let _ = crate::herdr::socket::popup_close(Some(socket));
        if n.i("pid") == 0 { let _ = std::fs::remove_file(&f); }
        let until = crate::clock::now() + 2000;
        while crate::clock::now() < until && n.i("pid") > 0 && pid_alive(n.i("pid")) { std::thread::sleep(std::time::Duration::from_millis(50)); }
        if shows { return Ok(String::new()); }
    }
    let api = Herdr::new(Some(socket.into()));
    let area = if here.0.is_empty() { None } else { api.pane_layout(&here.0).map(|l| l.at("area").clone()).filter(|a| a.is_object()) };
    let (w, h) = size_for(area.as_ref());
    api.open_board_popup(w, h)?;
    let _ = write_json(&f, &json!({ "tab": here.1, "pid": 0, "at": crate::clock::now() }));
    Ok(POPUP.into())
}
/// Opening the board from anywhere in herdr (your key, an action). Answers the board that shows now, as it is named
/// (its pane, or the popup), "" when there is none. `keep`: the board is needed, so the key's way back is off.
pub fn show(on: Option<&str>, keep: bool) -> String {
    let socket = on.map(String::from).unwrap_or_else(current_socket);
    let here = context();
    let mut told = String::new();
    let mut keep = keep;
    if popup_wanted() { match toggle(&socket, &here, keep) { Ok(x) => return x, Err(e) => { told = format!("{}. Until then the board is here, in a tab.", why(&e)); keep = true; } } }
    if let Some((api, n, pane)) = running(&socket) {
        if !keep && here.0 == n.s("pane") {
            // Pressed on the board itself: back to the pane you came from; with none, the board says it is the board.
            match api.pane_get(n.s("from")).filter(|_| n.b("from")) { Some(back) => go_to(&api, &back), None => say(n.s("pane"), "This is the board already. There is no pane to go back to.", "overlay0", &socket) }
            return n.t("pane");
        }
        if !here.0.is_empty() && here.0 != n.s("pane") && here.0 != n.s("from") { let mut m = n.as_object().cloned().unwrap_or_default(); m.put("from", here.0.clone()); let _ = write_json(&note_file(&socket), &Value::Object(m)); }
        go_to(&api, &pane);
        say(n.s("pane"), &told, "yellow", &socket);
        return n.t("pane");
    }
    let api = Herdr::new(Some(socket.clone()));
    let Ok(Some(pane_id)) = api.open_board() else { return String::new() };
    let Some(pane) = api.pane_get(&pane_id) else { return String::new() };
    api.tab_rename(pane.s("tab_id"), "board");
    go_to(&api, &pane);
    say(&pane_id, &told, "yellow", &socket);
    pane_id
}
/// The task whose agent (or reviewer) runs in a pane: the popup opens on the task you were looking at.
pub fn task_at(pane: &str) -> Option<Obj> {
    if pane.is_empty() { return None; }
    let socket = current_socket();
    crate::flow::pane_owner::whose(crate::store::tasks::list(Default::default()), &socket, pane).map(|(t, _)| t)
}
// ---- the board's own tab -------------------------------------------------------------------------------------------
pub const BOARD_TAB: &str = "herdr-projects";
fn tabs_file() -> std::path::PathBuf { data_dir().join("board-tabs.json") }
/// The herdr pane of a board you started yourself in a terminal, "" when there is none to name.
pub fn board_pane() -> String { if env("HERDR_ENV") == "1" && env("HERDR_PLUGIN_ID").is_empty() && env("TMUX").is_empty() { env("HERDR_PANE_ID") } else { String::new() } }
pub fn name_board_tab(pane: &str) -> Option<(String, String)> {
    if pane.is_empty() { return None; }
    let socket = current_socket();
    let api = Herdr::new(Some(socket.clone()));
    let tab = api.pane_get(pane)?.t("tab_id");
    let now = api.tab_get(&tab)?;
    if now.s("label") != BOARD_TAB {
        let key = format!("{socket}|{tab}");
        let mut all: Vec<(String, Value)> = read_json(&tabs_file()).and_then(|v| v.as_object().cloned()).unwrap_or_default().into_iter().filter(|(k, _)| *k != key).collect();
        let n = all.len(); if n > 49 { all.drain(..n - 49); }
        all.push((key, json!(now.t("label"))));
        let _ = write_json(&tabs_file(), &Value::Object(all.into_iter().collect()));
        crate::flow::tabs::name_tab(&api, &tab, BOARD_TAB);
    }
    Some((socket, tab))
}
pub fn restore_board_tab(named: Option<(String, String)>) {
    let Some((socket, tab)) = named else { return };
    let key = format!("{socket}|{tab}");
    let mut all = read_json(&tabs_file()).and_then(|v| v.as_object().cloned()).unwrap_or_default();
    let Some(old) = all.get(&key).cloned() else { return };
    let api = Herdr::new(Some(socket));
    if api.tab_get(&tab).map(|t| t.s("label") == BOARD_TAB).unwrap_or(false) { crate::flow::tabs::name_tab(&api, &tab, &text(&old)); }
    all.shift_remove(&key);
    let _ = write_json(&tabs_file(), &Value::Object(all));
}

//! Telling herdr's own sidebar about a task: one entry per live agent, a small copy of the task's row on the board.
//! An entry has three rows: 1. herdr's dot, then the task's number and title (the name of the task's space); 2. who is
//! on it and where it stands, then the review round, the pull request and the tags (the agent's shown name); 3. the
//! percent and the time to go, bare (the pane's `progress` token; its `summary` is only the Coordinator's). This file is
//! the only writer of a task's panes and space: by whoever ticks, once a second (`sync_sidebar`), and at once where a
//! task comes to wait for you, goes wrong, is taken up again or ends (`tell_sidebar`). Written again only when a word
//! changed; taken off again when the task is archived or removed while its pane lives on. Never into a pane or space
//! that is no longer the task's (herdr hands their ids out again), nor ever into a coordinator's.
use super::live::{owns_pane, saved};
use super::sidebar_entry::{entry_of, reviewer_entry, task_name, Entry};
use super::task::is_agent_task;
use crate::herdr::cli::{args, current_socket, session_name};
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::files::{data_dir, read_obj, write_json};
use crate::store::marks::due;
use std::path::PathBuf;

fn sidebar_file() -> PathBuf { data_dir().join("sidebar.json") }
const SOURCE: &str = "plugin:herdr-projects";
fn token(name: &str, value: &str) -> Vec<String> { if value.is_empty() { args(&["--clear-token", name]) } else { vec!["--token".into(), format!("{name}={value}")] } }
/// Row 2 as herdr keeps it: where the task stands, then the row's time, last, so that it is the first thing cut.
pub fn shown_name(e: &Entry) -> String { [e.stands.clone(), e.age.clone()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ").chars().take(80).collect() }
fn pane_args(e: &Entry, title: &str) -> Vec<String> {
    let mut a = args(&["--clear-state-labels", "--display-agent"]);
    a.push(shown_name(e));
    if !title.is_empty() { a.extend(["--title".to_string(), title.to_string()]); }
    a.extend(token("summary", &e.line)); a.extend(token("progress", &e.progress)); a.extend(args(&["--clear-token", "eta"]));
    a
}
fn clear_args() -> Vec<String> { args(&["--clear-state-labels", "--clear-display-agent", "--clear-title", "--clear-token", "summary", "--clear-token", "progress", "--clear-token", "eta", "--clear-token", "task"]) }
type Write = (String, Value, Box<dyn Fn() -> bool>);
/// What a task's panes and space are to say: [key, what is written there, how].
fn wanted(t: &Obj, now: i64) -> Vec<Write> {
    let socket = t.at("link").t("socket");
    let api = Herdr::new(Some(socket.clone()));
    let e = entry_of(t, "", now);
    let mut out: Vec<Write> = vec![];
    let pane = t.at("link").t("pane");
    { let (api, e2, p) = (api.clone(), e.clone(), pane.clone()); out.push((format!("e|{socket}|{pane}"), json!(e), Box::new(move || api.metadata(&p, &pane_args(&e2, &e2.name)).is_some()))); }
    let rv = t.at("link").at("reviewer").t("pane");
    if !rv.is_empty() { let r = reviewer_entry(t, now); let (api, name) = (api.clone(), e.name.clone()); let rv2 = rv.clone(); let r2 = r.clone(); out.push((format!("e|{socket}|{rv}"), json!(r), Box::new(move || api.metadata(&rv2, &pane_args(&r2, &name)).is_some()))); }
    // The task's space: its name is the task's number and title, and under it the row's numbers. One writer. Only
    // while its pane is in it: a space removed by hand leaves its id to the next one herdr opens.
    let pane_ws = t.at("live").at("pane").at("workspace_id");
    let ws = if t.at("link").b("ownsWorkspace") && (pane_ws.is_null() || pane_ws == t.at("link").at("workspace")) { t.at("link").t("workspace") } else { String::new() };
    if !ws.is_empty() {
        { let (api, ws2, name) = (api.clone(), ws.clone(), e.name.clone()); out.push((format!("n|{socket}|{ws}"), json!(e.name), Box::new(move || api.call(args(&["workspace", "rename", &ws2, &name])).is_ok()))); }
        let (api, ws2, line, prog) = (api.clone(), ws.clone(), e.line.clone(), e.progress.clone());
        out.push((format!("w|{socket}|{ws}"), json!([e.line, e.progress]), Box::new(move || { let mut a = args(&["workspace", "report-metadata", &ws2, "--source", SOURCE]); a.extend(token("summary", &line)); a.extend(token("progress", &prog)); api.call(a).is_ok() })));
    }
    out
}
/// The panes and spaces of this session's coordinators: never a task's, whatever a task's old link says.
fn coordinators(socket: &str) -> std::collections::HashSet<String> {
    crate::coordinator::state::all(socket).into_iter().filter_map(crate::coordinator::state::get).flat_map(|c| ["pane", "space", "workspace"].map(|k| c.t(k))).filter(|s| !s.is_empty()).collect()
}
/// Where a write goes: the pane or space its key names.
fn target(key: &str) -> &str { key.splitn(3, '|').nth(2).unwrap_or("") }
/// What was written, by where: `e` a task's pane, `n` its space's name, `w` its space's tokens, `c` the Coordinator's pane.
fn read() -> Obj { read_obj(&sidebar_file()).unwrap_or_default().into_iter().filter(|(k, _)| ["e|", "n|", "w|", "c|"].iter().any(|p| k.starts_with(p))).collect() }
/// Writes what changed, and remembers what herdr took.
fn write(list: &[Write], seen: &mut Obj) -> bool {
    let mut changed = false;
    for (key, value, how) in list {
        let sig = value.to_string();
        if seen.get(key).and_then(|v| v.as_str()) == Some(sig.as_str()) { continue; }
        if !how() { continue; }
        seen.put(key, sig);
        changed = true;
    }
    changed
}
fn told(t: &Obj) -> bool { is_agent_task(t) && !t.b("archivedAt") }
/// Did herdr lose what was written to a pane (it restarted since, and a pane starts with nothing)?
fn forgotten(sig: Option<&Value>, have: &Value) -> bool {
    let Some(sig) = sig.and_then(|s| s.as_str()) else { return false };
    have.is_object() && !have.b("display_agent") && serde_json::from_str::<Value>(sig).map(|v| v.b("stands")).unwrap_or(false)
}
/// A task that just changed, said in herdr's sidebar at once, by what was saved. Never fails: a sidebar is a convenience.
pub fn tell_sidebar(t: &Obj) {
    if !told(t) { return; }
    // What was saved says nothing of herdr: its pane is looked at, and a pane that is no longer the task's is left alone.
    let socket = t.at("link").t("socket");
    let Some(pane) = Herdr::new(Some(socket.clone())).pane_get(t.at("link").s("pane")).filter(|p| owns_pane(t, p)) else { return };
    let mut now = saved(t);
    now.put("live", json!({ "pane": pane }));
    let theirs = coordinators(&socket);
    let mut list = wanted(&now, crate::clock::now());
    list.retain(|(k, _, _)| !theirs.contains(target(k)));
    let mut seen = read();
    if write(&list, &mut seen) { let _ = write_json(&sidebar_file(), &Value::Object(seen)); }
}
/// Keeps every live pane of one herdr session current. A pane that is no longer a live task's has what the board wrote
/// taken off, once.
pub fn sync_sidebar(tasks: &[Obj], socket: Option<&str>, now: i64) {
    let socket = socket.map(String::from).or_else(|| tasks.iter().find(|t| t.at("live").is_object() && !t.at("live").b("away")).map(|t| t.at("link").t("socket"))).filter(|s| !s.is_empty()).unwrap_or_else(current_socket);
    let mut seen = read();
    let mut list: Vec<Write> = vec![];
    let (mut down, mut lost) = (false, false);
    let mut gone: Vec<String> = vec![];
    // A task's attempts have panes of their own: each has an entry like a task's. A task that waits for your pick has
    // no pane yet, and no numbers: what its space had written is taken off.
    let mut all: Vec<Obj> = vec![];
    for t in tasks {
        all.push(t.clone());
        for c in t.arr("tries").iter().filter_map(|c| c.as_object()).filter(|c| !c.b("chosen")) { let mut c = c.clone(); c.put("parentStage", t.at("stage").clone()); all.push(c); }
        if t.s("stage") == "pick" && !is_agent_task(t) && t.at("link").b("workspace") && t.at("link").s("socket") == socket && !t.b("archivedAt") {
            let (api, ws) = (Herdr::new(Some(socket.clone())), t.at("link").t("workspace"));
            list.push((format!("w|{socket}|{ws}"), json!(["", ""]), Box::new(move || api.call(args(&["workspace", "report-metadata", &ws, "--source", SOURCE, "--clear-token", "summary", "--clear-token", "progress"])).is_ok())));
        }
    }
    for t in &all {
        if !is_agent_task(t) || t.at("link").s("socket") != socket { continue; }
        let live = t.at("live");
        if live.b("down") { down = true; }
        // Its pane is gone, or its id is another's now: what was written there is forgotten, not taken off.
        if live.b("gone") { gone.extend([t.at("link").t("pane"), t.at("link").t("workspace")]); }
        if !told(t) || !live.is_object() || live.b("gone") || live.b("down") || live.b("away") { continue; }
        if forgotten(seen.get(&format!("e|{socket}|{}", t.at("link").s("pane"))), live.at("pane")) { lost = true; }
        list.extend(wanted(t, now));
    }
    // herdr restarted: it kept none of what was written, so all of it is written again.
    if lost && due(&format!("sidebar-again-{}", session_name(&socket)), 10000) { seen.retain(|k, _| k.split('|').nth(1) != Some(socket.as_str())); }
    let mut theirs = coordinators(&socket);
    list.retain(|(k, _, _)| !theirs.contains(target(k)));
    let mut changed = write(&list, &mut seen) || lost;
    theirs.extend(gone);
    if !down {
        let keep: std::collections::HashSet<String> = list.iter().map(|(k, _, _)| k.clone()).collect();
        let api = Herdr::new(Some(socket.clone()));
        let keys: Vec<String> = seen.keys().cloned().collect();
        for key in keys {
            let parts: Vec<&str> = key.splitn(3, '|').collect();
            if parts.len() < 3 || parts[1] != socket || keep.contains(&key) || parts[0] == "c" { continue; }
            // A coordinator's now, or no longer the task's: what it says is not the board's to take off.
            if theirs.contains(parts[2]) { seen.shift_remove(&key); changed = true; continue; }
            if parts[0] == "e" { api.metadata(parts[2], &clear_args()); }
            if parts[0] == "w" { let _ = api.call(args(&["workspace", "report-metadata", parts[2], "--source", SOURCE, "--clear-token", "summary", "--clear-token", "progress"])); }
            seen.shift_remove(&key);
            changed = true;
        }
    }
    if changed { let _ = write_json(&sidebar_file(), &Value::Object(seen)); }
}
/// The name a task's space is opened under: what its row starts with.
pub fn space_label(t: &Obj) -> String { task_name(t) }
/// The same for a pane that is no task's: the Coordinator's. `have`: herdr's own reading of the pane.
pub fn tell_pane(socket: &str, pane: &str, entry: &Entry, have: &Value) {
    let mut seen = read();
    let key = format!("c|{socket}|{pane}");
    if forgotten(seen.get(&key), have) { seen.shift_remove(&key); }
    let (api, p, e) = (Herdr::new(Some(socket.to_string())), pane.to_string(), entry.clone());
    let w: Vec<Write> = vec![(key, json!(entry), Box::new(move || api.metadata(&p, &pane_args(&e, &e.name)).is_some()))];
    if write(&w, &mut seen) { let _ = write_json(&sidebar_file(), &Value::Object(seen)); }
}
/// What `tell_sidebar` says of a task: its entry by what was saved alone.
pub fn entry_now(t: &Obj, now: i64) -> Entry { entry_of(&saved(t), "", now) }

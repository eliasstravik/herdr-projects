//! What the pane of a task's agent shows (its reviewer's while the task is under review), read through herdr: only for
//! a program whose conversation cannot be read. Only the visible screen is read: no history collection or scrolling.
//! A finished turn is cached; while busy it is read at most every two seconds. Read by the loader, never on a key press.
use crate::herdr::{api::read_screen, Herdr};
use crate::json::*;
use crate::shown::states::reviewing;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

static SCREENS: LazyLock<Mutex<HashMap<String, (String, i64, String)>>> = LazyLock::new(Default::default);
pub fn screen_of(r: &Obj) -> String {
    let live = r.at("live");
    if !live.is_object() || live.b("away") || live.b("down") || live.b("gone") { return String::new(); }
    let reviewer = if reviewing(r) { r.at("link").at("reviewer").t("pane") } else { String::new() };
    let status = if !reviewer.is_empty() { "working".to_string() } else { live.t("status") };
    let settled = status == "idle" || status == "done";
    let agent = live.at("agent");
    let key = if settled { format!("settled:{}", if agent.has("completion_seq") { agent.t("completion_seq") } else { agent.t("state_change_seq") }) } else { format!("live:{reviewer}") };
    let now = crate::clock::now();
    if let Some((k, at, text)) = SCREENS.lock().unwrap().get(r.s("id")) { if *k == key && (settled || now - at < 2000) { return text.clone(); } }
    let pane = if reviewer.is_empty() { r.at("link").t("pane") } else { reviewer };
    let raw = read_screen(&Herdr::new(Some(r.at("link").t("socket"))), &pane);
    let mut text = raw.split('\n').map(|l| l.trim_end()).collect::<Vec<_>>().join("\n");
    while text.contains("\n\n\n") { text = text.replace("\n\n\n", "\n\n"); }
    let text = text.trim().to_string();
    SCREENS.lock().unwrap().insert(r.t("id"), (key, now, text.clone()));
    text
}

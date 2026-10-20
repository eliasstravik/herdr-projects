//! An automation as it is shown, in the same words on the board's list, in its panel and on the command line: on or off,
//! its trigger in plain words, its repo, the last firing (how long ago, and the task it made or "skipped"), the next
//! one for a schedule; one firing as a line; and what the board says when one was turned on or off, or fired by hand.
use super::triggers::say_trigger;
use crate::json::*;
use crate::research::is_research;
use crate::shown::ink::Ink;
use crate::shown::words::age;
use crate::store::automations::firings_of;
use crate::store::keys::{repo_keys, say_id, say_ids};

pub fn ago(at: i64, now: i64) -> String { format!("{} ago", age(now - at)) }
/// One firing: when, what fired it, the task it made or why it was skipped.
pub fn firing_words(f: &Value, now: i64) -> (String, String, String, String) {
    let result = if f.b("task") { format!("made {}{}", say_id(f.s("task"), &repo_keys()), if f.b("note") { format!(" ({})", f.s("note")) } else { String::new() }) }
        else { format!("skipped{}: {}", if f.i("n") > 1 { format!(" ×{}", f.i("n")) } else { String::new() }, say_ids(f.s("skipped"), &repo_keys())) };
    (ago(f.i("at"), now), if f.b("what") { f.t("what") } else { f.t("cause") }, f.t("task"), result)
}
pub fn firing_line(f: &Value, now: i64) -> String { let (w, what, _, r) = firing_words(f, now); format!("{w:<8} {what}  →  {r}") }
/// Its firings, newest first.
pub fn firings(a: &Obj) -> Vec<Value> { let mut f = firings_of(a.s("id")); f.reverse(); f }
/// What its row says: on or off as a word, the next firing while a schedule is on, else the last, else "never fired".
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct AutoRow { pub when: String, pub id: String, pub name: String, pub on: bool, pub state: String, pub trigger: String, pub repo: String, pub last: String,
    #[serde(rename = "lastTask")] pub last_task: String, pub next: String, pub lead: String, pub age: String, pub ago: bool, pub made: String, pub why: String }
pub fn row_of(a: &Obj, now: i64, all: Option<&[Value]>) -> AutoRow {
    let owned;
    let all = match all { Some(a) => a, None => { owned = firings_of(a.s("id")); &owned } };
    let last = all.last();
    let next = if a.b("on") && a.at("trigger").s("kind") == "schedule" && a.b("nextAt") { if a.i("nextAt") <= now { "now".to_string() } else { format!("in {}", age(a.i("nextAt") - now)) } } else { String::new() };
    AutoRow {
        when: if !next.is_empty() { format!("next {next}") } else if let Some(l) = last { format!("{} {}", if l.b("task") { "fired" } else { "skipped" }, ago(l.i("at"), now)) } else { "never fired".into() },
        id: a.t("id"), name: a.t("name"), on: a.b("on"), state: if a.b("on") { "on" } else { "off" }.into(), trigger: say_trigger(a.at("trigger")), repo: a.t("repo"),
        last: last.map(|l| format!("{} · {}", ago(l.i("at"), now), if l.b("task") { say_id(l.s("task"), &repo_keys()) } else { "skipped".into() })).unwrap_or_else(|| "never fired".into()),
        last_task: last.map(|l| l.t("task")).unwrap_or_default(),
        lead: if !next.is_empty() { if next == "now" { "next" } else { "next in" }.into() } else if let Some(l) = last { if l.b("task") { "fired" } else { "skipped" }.into() } else { "never".into() },
        age: if !next.is_empty() { next.trim_start_matches("in ").to_string() } else { last.map(|l| age(now - l.i("at"))).unwrap_or_default() },
        ago: next.is_empty() && last.is_some(), next,
        made: all.iter().rev().find(|f| f.b("task")).map(|f| f.t("task")).unwrap_or_default(),
        why: a.t("offWhy"),
    }
}
/// Everything about one, for `show` and --json.
pub fn describe(a: &Obj, now: i64) -> Value {
    let r = row_of(a, now, None);
    let mut out = obj(json!({ "id": a.at("id"), "name": a.at("name"), "on": r.on, "trigger": r.trigger, "repo": a.at("repo"), "folder": a.at("root"),
        "research": is_research(&obj(json!({ "id": format!("{}/0", a.s("repo")), "repo": a.at("repo"), "settings": a.o("settings").cloned().unwrap_or_default() }))),
        "chat": crate::chat_only::is_chat(&obj(json!({ "id": format!("{}/0", a.s("repo")), "repo": a.at("repo"), "settings": a.o("settings").cloned().unwrap_or_default() }))),
        "prompt": a.at("prompt"), "settings": a.o("settings").cloned().unwrap_or_default(), "last": firings_of(a.s("id")).last().cloned(), "next": if a.b("on") && a.b("nextAt") { a.at("nextAt").clone() } else { Value::Null } }));
    if a.b("offWhy") { out.put("offWhy", a.at("offWhy").clone()); }
    Value::Object(out)
}
pub fn list_line(a: &Obj, now: i64) -> String {
    let r = row_of(a, now, None);
    format!("{:<3}  {}  ·  {}  ·  {}  ·  {}{}{}", r.state, r.name, r.trigger, r.repo, r.last, if r.next.is_empty() { String::new() } else { format!("  ·  next {}", r.next) }, if r.why.is_empty() { String::new() } else { format!("  ·  {}", r.why) })
}
/// What the board says after one was turned on or off, and after one was fired by hand: words and their ink.
pub fn turned_words(a: &Obj) -> (String, Ink) { (format!("\"{}\" is {}.", a.s("name"), if a.b("on") { "on" } else { "off: it never fires, and keeps everything" }), if a.b("on") { Ink::Green } else { Ink::Yellow }) }
pub fn fired_words(a: &Obj, f: &Value) -> (String, Ink) {
    if f.b("task") { (format!("\"{}\" fired: {} was added{}.", a.s("name"), f.s("task"), if f.b("note") { format!(", {}", f.s("note")) } else { " and an agent is on it".into() }), if f.b("note") { Ink::Yellow } else { Ink::Green }) }
    else { (format!("\"{}\" did not fire: {}.", a.s("name"), f.s("skipped")), Ink::Yellow) }
}

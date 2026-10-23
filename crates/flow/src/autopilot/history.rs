//! What an autopilot turn reads of the past: the board is its only memory. `list` in an autopilot turn also gives its
//! area's finished, stopped and archived tasks of the last 30 days (at most 50, newest first), each with who added it
//! (you, an automation, which coordinator's autopilot, or which coordinator you asked) and the first 300 characters of
//! its closing note; `details <id>` the whole note.
use super::care::in_area;
use super::limits::board_tasks;
use crate::clock::now;
use crate::coordinator::co::{named, Co, Level};
use crate::json::*;
use crate::store::keys::{repo_keys, say_id};

const DAYS: i64 = 30;
const MOST: usize = 50;
const NOTE: usize = 300;
fn note_of(t: &Obj) -> String { std::fs::read_to_string(crate::flow::task::note_file(t)).unwrap_or_default().trim().to_string() }
/// Who added a task, in words (also in every `details`).
pub fn added_by(t: &Obj) -> String {
    if t.at("autopilot").is_object() { return format!("the {}'s autopilot{}", t.at("autopilot").s("name"), if t.at("autopilot").b("machine") { format!(" on {}", t.at("autopilot").s("machine")) } else { String::new() }); }
    if t.at("automation").is_object() { return format!("the automation \"{}\"", t.at("automation").s("name")); }
    if t.b("coordinator") { return format!("the {}, asked by the human", coordinator_name(t)); }
    "the human".into()
}
/// The coordinator whose pane added it, by its name: another machine's task names that machine.
fn coordinator_name(t: &Obj) -> String {
    let co = super::record::co_of("", t.s("coordinator"));
    match (&co.level, t.b("host")) { (Level::Machine, true) => named(t.s("host")), (_, true) => format!("{} on {}", co.name(), t.s("host")), _ => co.name() }
}
fn ended_at(t: &Obj) -> i64 { [t.i("archivedAt"), t.i("mergedAt"), t.i("at"), t.i("updated")].into_iter().max().unwrap_or(0) }
fn past(co: &Co) -> Vec<Obj> {
    let since = now() - DAYS * 86_400_000;
    let mut list: Vec<Obj> = board_tasks().into_iter().filter(|t| in_area(co, t) && (t.b("archivedAt") || ["done", "stopped"].contains(&t.s("stage")) || t.b("mergedAt")) && ended_at(t) >= since).collect();
    list.sort_by_key(|t| std::cmp::Reverse(ended_at(t)));
    list.truncate(MOST);
    list
}
/// `list`'s answer, with its area's history.
pub fn with_history(co: &Co, v: Value, text: String) -> (Value, String) {
    let keys = repo_keys();
    let list = past(co);
    let rows: Vec<Value> = list.iter().map(|t| json!({ "id": t.at("id"), "title": t.at("title"), "stage": t.at("stage"), "archived": t.b("archivedAt"), "addedBy": added_by(t), "note": note_of(t).chars().take(NOTE).collect::<String>() })).collect();
    let lines: Vec<String> = list.iter().map(|t| {
        let n: String = note_of(t).chars().take(NOTE).collect::<String>().replace('\n', " ");
        format!("{:<9} {:<8} {}  · added by {}{}", say_id(t.s("id"), &keys), if t.b("archivedAt") { "archived" } else { t.s("stage") }, t.s("title"), added_by(t), if n.is_empty() { String::new() } else { format!("\n          {n}") })
    }).collect();
    let said = format!("{text}\n\nFinished, stopped and archived in your area, the last {DAYS} days (newest first):\n{}", if lines.is_empty() { "none".into() } else { lines.join("\n") });
    (json!({ "tasks": v, "history": rows }), said)
}
/// `details`' answer with the whole closing note.
pub fn with_note(d: &mut Value) {
    let id = d.at("task").t("id");
    let Some(t) = crate::store::tasks::get(&id) else { return };
    if let Some(o) = d.as_object_mut() { o.put("closingNote", note_of(&t)); }
}

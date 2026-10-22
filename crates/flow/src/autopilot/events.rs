//! The board events that wake a coordinator on autopilot (`autopilot.when`), in its area: `done` (a task was merged or
//! done), `stuck` (a task it may look after starts to need looking after: once, not again while it stays so) and
//! `added` (a task was added by anyone but this coordinator). Each is read off the tasks as they stand and fired once:
//! what was seen is kept in its record (`seen`), and the first look after it was turned on only takes note. A task
//! that starts to need looking after once its retries are used is left to you instead, and wakes nothing.
use super::care::{added_by, in_area, may_look_after, need_of};
use super::record::{task_rec, with_task};
use crate::coordinator::co::Co;
use crate::json::*;

/// What is so of a task now, one mark per thing that wakes.
fn marks_of(ap: &Obj, t: &Obj, approve: bool) -> Vec<String> {
    let mut out = vec!["added".to_string()];
    if t.b("mergedAt") || t.s("stage") == "done" { out.push("done".into()); }
    if may_look_after(ap, t) { if let Some(w) = need_of(t, approve) { out.push(format!("stuck:{w}")); } }
    out
}
/// A reason to wake, as kept: why (`on`, `timer`, `done`, `stuck`, `added`), the task, and its words in plain words
/// (`AFS-4 finished`, `AFS-4 is stuck`, `AFS-4 has a question`, `AFS-4 is ready to approve`, `AFS-4 was added`).
pub fn reason(why: &str, task: &str, words: &str) -> Value { json!({ "why": why, "task": task, "words": words }) }
/// The task as its reason names it: `HSW-84`.
fn tag(id: &str) -> String { crate::store::keys::say_id(id, &crate::store::keys::repo_keys()) }
/// What happened since the last look: the reasons it wakes for (of those `when` names), the tasks now left to you,
/// and what is seen now. `retries`: autopilot.retries.
pub fn look(co: &Co, ap: &mut Obj, board: &[Obj], when: &str, approve: bool, retries: i64) -> Vec<Value> {
    let first = !ap.at("seen").is_object();
    let seen = ap.o("seen").cloned().unwrap_or_default();
    let mut next = Obj::new();
    let mut out = vec![];
    let wants = |w: &str| when.split_whitespace().any(|x| x == w);
    // The oldest first, then by number: the reasons of one wake are said in the order things happened.
    let mut area: Vec<&Obj> = board.iter().filter(|t| in_area(co, t)).collect();
    area.sort_by(|a, b| (a.i("created"), a.s("host"), a.i("n"), a.s("id")).cmp(&(b.i("created"), b.s("host"), b.i("n"), b.s("id"))));
    for t in area {
        let id = t.t("id");
        // Another machine's task of its own, seen over: its slot is free also once that machine is heard no more.
        if t.b("host") && super::care::over(t) && task_rec(ap, &id).map(|r| !r.b("over")).unwrap_or(false) { with_task(ap, &id, |r| { r.put("over", true); }); }
        let marks = marks_of(ap, t, approve);
        let had: Vec<String> = seen.get(&id).map(strings).unwrap_or_default();
        next.put(&id, marks.clone());
        // Never seen and from before it was turned on (another machine's, heard only now): nothing new happened.
        if first || t.b("archivedAt") || (had.is_empty() && t.i("created") < ap.i("since")) { continue; }
        for m in marks.iter().filter(|m| !had.contains(m)) {
            match m.split(':').next().unwrap_or("") {
                "added" if !added_by(co, t) && wants("added") => out.push(reason("added", &id, &format!("{} was added", tag(&id)))),
                "done" if wants("done") => out.push(reason("done", &id, &format!("{} finished", tag(&id)))),
                "stuck" => {
                    let w = m.split(':').nth(1).unwrap_or("");
                    // Failing or asking again once its retries are used: yours from now on.
                    let used = task_rec(ap, &id).map(|r| r.i("retries")).unwrap_or(0);
                    if w != "yes" && used >= retries { with_task(ap, &id, |r| { r.put("left", true); }); continue; }
                    if wants("stuck") { out.push(reason("stuck", &id, &format!("{} {}", tag(&id), match w { "asks" => "has a question", "yes" => "is ready to approve", _ => "is stuck" }))); }
                }
                _ => {}
            }
        }
    }
    ap.put("seen", next);
    out
}

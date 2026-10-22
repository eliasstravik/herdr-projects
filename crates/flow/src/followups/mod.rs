//! A finished task's open follow-ups (docs/follow-ups.md): what its closing note lists under "Still open" and "Also
//! noticed", kept on the task as a list (`followups`) when its turn ends, so they outlive its pane, its worktree and
//! its note. Each is numbered, and open until you add it as a task or dismiss it; the task's row, `details`, its panel
//! and `herdr-projects followups` show the open ones once the task is done, archived or not.
pub mod act;
pub mod command;
pub mod parse;

use crate::json::*;

/// What an agent is told to write of what its task leaves open, so the board can read it (parse.rs).
pub const STILL_OPEN_RULE: &str = "If part of what this task needs is left that you could not do here (a manual check on staging, a step on a platform such as a missing service account, a change another repo still needs), list it under a heading \"Still open\" in your closing note, one line each: start a line with [repo: <name>] when it belongs to another repo, and end it with (blocks) when the change is not complete or usable without it. Leave the heading out when nothing is left. The board keeps these lines, and your \"Also noticed\" list, as the task's follow-ups for the human.";
/// What a follow-up is: open, made a task (`added`, with that task's id) or dismissed.
pub const OPEN: &str = "open";
pub const ADDED: &str = "added";
pub const DISMISSED: &str = "dismissed";

/// The list as the note now has it. What you did to one already kept (added or dismissed) stays with it: an item is
/// the same one when its words are.
pub fn merged(old: &Value, note: &str) -> Value {
    let was: Vec<&Value> = old.as_array().map(|a| a.iter().collect()).unwrap_or_default();
    let list: Vec<Value> = parse::parse(note).into_iter().enumerate().map(|(i, mut f)| {
        if let Some(o) = was.iter().find(|o| o.s("text") == f.s("text") && o.s("state") != OPEN) { for k in ["state", "task", "at"] { if o.has(k) { f[k] = o.at(k).clone(); } } }
        if !f.has("state") { f["state"] = json!(OPEN); }
        f["n"] = json!(i + 1);
        f
    }).collect();
    Value::Array(list)
}
/// The task's follow-ups, read again from the note its agent ended with. A note that lists none (a later round's
/// "fixed the findings") leaves the list it had.
pub fn record(t: &mut Obj, note: &str) {
    let list = merged(t.at("followups"), note);
    if list.as_array().map(|a| a.is_empty()).unwrap_or(true) { return; }
    t.put("followups", list);
}
/// Finished: merged or done. Follow-ups are shown only then: before, the work is still going on.
pub fn finished(t: &Obj) -> bool { t.s("stage") == "done" || t.b("mergedAt") }
/// Its follow-ups not yet added as a task or dismissed.
pub fn open_of(t: &Obj) -> Vec<&Value> { t.arr("followups").iter().filter(|f| f.s("state") == OPEN).collect() }
/// How many open follow-ups a finished task's row says it has: 0 before it is finished.
pub fn shown_count(t: &Obj) -> usize { if finished(t) { open_of(t).len() } else { 0 } }
/// "1 follow-up", "2 follow-ups".
pub fn count_words(n: usize) -> String { format!("{n} follow-up{}", if n == 1 { "" } else { "s" }) }
/// What a coordinator's wake says of a finished task's follow-ups: `, with 2 follow-ups open (board followups HSW-8)`.
pub fn wake_words(t: &Obj) -> String {
    match shown_count(t) { 0 => String::new(), n => format!(", with {} open (board followups {})", count_words(n), crate::store::keys::say_id(t.s("id"), &crate::store::keys::repo_keys())) }
}
/// One follow-up in a line: `2. [web] Show the new field (blocks)`, and what became of it when it is not open.
pub fn line(f: &Value) -> String {
    let repo = if f.b("repo") { format!("[{}] ", f.s("repo")) } else { String::new() };
    let state = match f.s("state") { ADDED => format!("  · added as {}", crate::store::keys::say_id(f.s("task"), &crate::store::keys::repo_keys())), DISMISSED => "  · dismissed".into(), _ => String::new() };
    format!("{}. {repo}{}{}{}{state}", f.i("n"), f.s("text"), if f.b("blocks") { " (blocks)" } else { "" }, if f.s("kind") == "noticed" { " (noticed)" } else { "" })
}
/// What `details` shows: every follow-up, and how many are open.
pub fn detail(t: &Obj) -> Value {
    if !t.b("followups") { return Value::Null; }
    json!({ "open": open_of(t).len(), "items": t.at("followups"), "lines": t.arr("followups").iter().map(line).collect::<Vec<_>>() })
}

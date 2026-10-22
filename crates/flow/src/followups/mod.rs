//! A finished task's open follow-ups (docs/follow-ups.md): what its closing note lists under "Still open" and "Also
//! noticed", kept on the task as a list (`followups`) when its turn ends, so they outlive its pane, its worktree and
//! its note. Each is numbered, and open until you add it as a task or hide it; the task's row, `details`, its panel
//! and `herdr-projects followups` show the open ones once the task is done, archived or not.
pub mod act;
pub mod command;
pub mod parse;

use crate::json::*;

/// What an agent is told to write of what its task leaves open, so the board can read it (parse.rs).
pub const STILL_OPEN_RULE: &str = "If part of what this task needs is left that you could not do here (a manual check on staging, a step on a platform such as a missing service account, a change another repo still needs), list it under a heading \"Still open\" in your closing note, one line each: start a line with [repo: <name>] when it belongs to another repo, and end it with (blocks) when the change is not complete or usable without it. Leave the heading out when nothing is left. The board keeps these lines, and your \"Also noticed\" list, as the task's follow-ups for the human.";
/// What a follow-up is: open, made a task (`added`, with that task's id) or hidden.
pub const OPEN: &str = "open";
pub const ADDED: &str = "added";
pub const HIDDEN: &str = "hidden";

/// The list as the note now has it. An item is the same one when its words are: it keeps its number and what you
/// did to it. A new one gets the next number. One you added as a task or hid stays on the list (its link with it)
/// when the note no longer says it; an open one it no longer says is gone.
pub fn merged(old: &Value, note: &str) -> Value {
    let was: Vec<&Value> = old.as_array().map(|a| a.iter().collect()).unwrap_or_default();
    let mut next = was.iter().map(|o| o.i("n")).max().unwrap_or(0);
    let mut list: Vec<Value> = parse::parse(note).into_iter().map(|mut f| {
        match was.iter().find(|o| o.s("text") == f.s("text")) {
            Some(o) => { for k in ["state", "task", "at", "n"] { if o.has(k) { f[k] = o.at(k).clone(); } } }
            None => { next += 1; f["n"] = json!(next); }
        }
        if !f.has("state") { f["state"] = json!(OPEN); }
        f
    }).collect();
    let kept: Vec<Value> = was.iter().filter(|o| o.s("state") != OPEN && !list.iter().any(|f| f.s("text") == o.s("text"))).map(|o| (*o).clone()).collect();
    list.extend(kept);
    list.sort_by_key(|f| f.i("n"));
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
/// Its follow-ups not yet added as a task or hidden.
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
    let state = match f.s("state") { ADDED => format!("  · added as {}", crate::store::keys::say_id(f.s("task"), &crate::store::keys::repo_keys())), HIDDEN => "  · hidden".into(), _ => String::new() };
    format!("{}. {repo}{}{}{}{state}", f.i("n"), f.s("text"), if f.b("blocks") { " (blocks)" } else { "" }, if f.s("kind") == "noticed" { " (noticed)" } else { "" })
}
/// What `details` shows: every follow-up, and how many are open.
pub fn detail(t: &Obj) -> Value {
    if !t.b("followups") { return Value::Null; }
    json!({ "open": open_of(t).len(), "items": t.at("followups"), "lines": t.arr("followups").iter().map(line).collect::<Vec<_>>() })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_later_note_keeps_numbers_and_what_you_did_and_drops_only_open_ones_it_no_longer_says() {
        let first = merged(&Value::Null, "Still open:\n- A\n- B\n- C");
        let mut acted = first.as_array().unwrap().clone();
        acted[1]["state"] = json!(ADDED); acted[1]["task"] = json!("x/9");
        acted[2]["state"] = json!(HIDDEN);
        let again = merged(&Value::Array(acted), "Still open:\n- A\n- C\n- D\n- E");
        let got: Vec<(i64, String, String)> = again.as_array().unwrap().iter().map(|f| (f.i("n"), f.t("text"), f.t("state"))).collect();
        let want = [(1, "A", OPEN), (2, "B", ADDED), (3, "C", HIDDEN), (4, "D", OPEN), (5, "E", OPEN)].map(|(n, t, s)| (n, t.to_string(), s.to_string()));
        assert_eq!(got, want.to_vec());
        assert_eq!(again[1].s("task"), "x/9");
        // an open one the note no longer says is gone; the next new one is not given its number again
        let third = merged(&again, "Still open:\n- F");
        let got: Vec<(i64, String)> = third.as_array().unwrap().iter().map(|f| (f.i("n"), f.t("text"))).collect();
        assert_eq!(got, vec![(2, "B".into()), (3, "C".into()), (6, "F".into())]);
    }
}

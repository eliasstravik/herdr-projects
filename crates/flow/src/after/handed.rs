//! What a task that waited is told when it starts: for each task it waited for (on this machine or another), its id,
//! title and closing note, and
//! the pull request and merge commit where its work merged, or the report where it was research. Taken when the wait
//! ends and kept on the task (`afterDone`), so a clean-up of the task before it loses nothing of it.
use crate::flow::gitlab_words::pr_word;
use super::{after_of, say, stand_of, Stand};
use crate::clock::now;
use crate::flow::task::note_stat;
use crate::json::*;
use crate::research::is_research;
use crate::store::files::dir_of;
use crate::store::tasks::{get, Task};

/// The longest closing note handed over, in characters: a longer one is cut, and its task's `details` has the rest.
const NOTE_MAX: usize = 4000;
/// A task's closing note, wherever the board has it: the note itself, the copy the finish line kept, what it said last.
fn note_of(d: &Task) -> String {
    if let Some((_, n)) = note_stat(d) { if !n.is_empty() { return n; } }
    if let Ok(n) = std::fs::read_to_string(dir_of(d.s("id")).join("closing.md")) { if !n.trim().is_empty() { return n.trim().to_string(); } }
    [d.t("summary"), d.t("said")].into_iter().map(|s| s.trim().to_string()).find(|s| !s.is_empty()).unwrap_or_default()
}
/// One task waited for, as it is now.
pub fn of(id: &str) -> Value {
    if super::far::is_far(id) { return super::far::handed(id); } // a task of another machine: what it said last
    let Some(d) = get(id) else { return json!({ "id": id, "gone": true }) };
    let mut o = obj(json!({ "id": id, "title": d.at("title"), "repo": d.at("repo"), "done": stand_of(id) == Stand::Done, "stage": d.at("stage"), "research": is_research(&d) }));
    let note: String = note_of(&d).chars().take(NOTE_MAX).collect();
    if !note.is_empty() { o.put("note", note); }
    if d.b("pr") { o.put("pr", d.at("pr").clone()); }
    if d.b("prMerged") || d.s("prState") == "merged" || d.b("mergeCommit") { o.put("merged", true); }
    if d.b("mergeCommit") { o.put("commit", d.at("mergeCommit").clone()); }
    if d.at("report").b("link") { o.put("report", d.at("report").at("link").clone()); }
    Value::Object(o)
}
/// The wait is over (or cut short by you, `early`): the task goes on to a slot like any delegated one, with what it
/// waited for kept for its first prompt.
pub fn hand_over(t: &mut Task, early: bool) {
    let done: Vec<Value> = after_of(t).iter().map(|id| of(id)).collect();
    t.put("afterDone", done);
    if early { t.put("afterEarly", true); }
    for k in ["afterLeft", "afterBlocked"] { t.del(k); }
    t.put("stage", "queued"); t.put("queuedAt", now()); t.put("msg", "");
}
/// The part of the first prompt that hands over what came before: nothing for a task that waited for nothing.
pub fn words(t: &Obj) -> String {
    let list = t.arr("afterDone");
    if list.is_empty() { return String::new(); }
    let mut out = vec![if t.b("afterEarly") {
        "This task was to wait for the tasks below to be done, and the human started it before they all were. Take in what is there of their work, and leave alone what is not done yet.".to_string()
    } else { "This task waited for the tasks below to be done, and they are. It builds on their work: take it in (fetch it, read it, follow what it changed) before you go on.".to_string() }];
    for d in list {
        let id = d.t("id");
        if d.b("gone") { out.push(format!("- {}: removed from the board, nothing is known of it.", say(&id))); continue; }
        let on = if d.b("machine") { format!(" on {}", d.s("machine")) } else { String::new() };
        let mut line = format!("- {} \"{}\" (repo {}{on}): {}", say(&id), d.s("title"), d.s("repo"), if d.b("done") { "done" } else { "not done yet" });
        if d.b("merged") {
            line.push_str(", its work merged");
            if d.b("pr") { line.push_str(&format!(": {} {}", pr_word(d.s("pr")), d.s("pr"))); }
            if d.b("commit") { line.push_str(&format!(", merge commit {}", d.s("commit"))); }
        } else if d.b("pr") { line.push_str(&format!(", {} {} (not merged)", pr_word(d.s("pr")), d.s("pr"))); }
        if d.b("report") { line.push_str(&format!(", {}report: {}", if d.b("research") { "research only, its " } else { "its " }, d.s("report"))); }
        line.push('.');
        if d.b("note") { line.push_str(&format!(" Its closing note:\n{}", d.s("note").lines().map(|l| format!("    {l}")).collect::<Vec<_>>().join("\n"))); }
        out.push(line);
    }
    out.join("\n")
}

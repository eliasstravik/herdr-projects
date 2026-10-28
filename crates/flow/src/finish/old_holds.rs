//! Approval holds an older version left behind: tasks owing you a yes although you never stepped into them. Their
//! failure or their review was sent back to the agent as it stood, and that used to count as stepping in. It no longer
//! does. A hold from before is lifted only where the agent's own conversation proves that nothing of yours was ever
//! said in it; where it cannot, you are asked.
use crate::agents::prompts::prompts_of;
use crate::flow;
use crate::gate;
use crate::json::*;
use crate::store::marks::stamp_file;
use crate::store::reviews::review_history;
use crate::store::settings::lookup::setting;
use crate::store::tasks::{update_wait, Change, Task};
use std::collections::HashSet;
use std::path::Path;

/// The moment this rule first ran on this board, written down once.
fn rule_since() -> i64 {
    let file = stamp_file("send-back-is-no-hold");
    if let Ok(t) = std::fs::read_to_string(&file) { return t.trim().parse().unwrap_or(0); }
    let now = crate::clock::now();
    if let Some(d) = file.parent() { let _ = std::fs::create_dir_all(d); }
    match std::fs::OpenOptions::new().write(true).create_new(true).open(&file) { Ok(mut f) => { let _ = std::io::Write::write_all(&mut f, now.to_string().as_bytes()); now } Err(_) => std::fs::read_to_string(&file).ok().and_then(|t| t.trim().parse().ok()).unwrap_or(0) }
}
fn review_texts(t: &Task) -> HashSet<String> {
    review_history(t).iter().filter(|e| !e.b("error")).flat_map(|e| [gate::review_message(e, false), gate::review_message(e, true)]).collect()
}
/// Messages that waited behind a dialog go out together, one after the other, each whole.
fn boards_own(text: &str, reviews: &HashSet<String>) -> bool {
    if reviews.contains(text) { return true; }
    for rv in reviews { if let Some(rest) = text.strip_prefix(&format!("{rv}\n\n")) { return boards_own(rest, reviews); } }
    false
}
/// Was this task never stepped into? Its agent was told its task, once, before any review, and after that only reviews
/// of its own, as the board words them.
pub fn never_stepped_into(t: &Task) -> bool {
    let Some(told) = prompts_of(t.s("agent"), t.so("transcript").map(Path::new)) else { return false };
    let Some((first_at, first)) = told.first() else { return false };
    let reviewed = review_history(t).first().map(|e| e.i("at")).unwrap_or(0);
    let tail = flow::send::prompt_tail(t);
    let reviews = review_texts(t);
    if !first.starts_with(&format!("Work board task {}, role: agent.", t.s("id"))) || *first_at <= 0 || reviewed <= *first_at { return false; }
    told[1..].iter().all(|(_, p)| { let text = if !tail.is_empty() && p.ends_with(&format!("\n\n{tail}")) { &p[..p.len() - tail.len() - 2] } else { p.as_str() }; boards_own(text, &reviews) })
}
/// Looked at once for every turn of a task that holds.
pub fn lift_old_holds(tasks: &[Obj], socket: &str) {
    let mut since: Option<i64> = None;
    let held = |t: &Task| t.s("stage") == "approval" && t.b("requireApproval") && t.at("holdLooked") != &json!(t.i("armedAt")) && !t.b("archivedAt") && crate::herdr::cli::same_socket(t.at("link").s("socket"), socket) && flow::task::is_agent_task(t) && setting("merge.by", Some(t)) == json!("itself");
    for seen in tasks {
        if !held(seen) { continue; }
        let s = *since.get_or_insert_with(rule_since);
        let free = seen.i("armedAt") < s && never_stepped_into(seen);
        let tr = seen.at("transcript").clone();
        let _ = update_wait(seen.s("id"), 10000, |t| { if !held(t) || *t.at("transcript") != tr { return Ok(Change::Keep); } let a = t.i("armedAt"); t.put("holdLooked", a); if free { t.put("requireApproval", false); } Ok(Change::Write) });
    }
}

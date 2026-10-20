//! Chat-only tasks: a task that is a conversation (a consultation, a discussion, thinking out loud), not work. The
//! setting `chat.only`: its agent changes nothing in the repo, every stop of its agent waits for the human, and it ends
//! only when the human says so (docs/chat.md). With `research.only` on as well, chat wins: it is research that never
//! finishes by itself. Read when the agent starts (`chatRun`), as research only is (research.rs).
use crate::json::*;
use crate::research::report_folder;
use crate::store::settings::lookup::setting;
use crate::store::tasks::Task;
use std::collections::BTreeMap;

pub const CHAT: &str = "chat.only";

/// What this run was started as: `chatRun`, or not chat for a task started before there was chat only. None: the
/// next start decides (not started yet, or a start was asked for).
fn run_of(t: &Task) -> Option<bool> {
    if t.b("researchOpen") { return None; }
    t.at("chatRun").as_bool().or_else(|| t.at("researchRun").is_boolean().then_some(false))
}
/// Is this task chat only? A row of the board says it (as the machine that owns the task worked it out); a task whose
/// agent was started is what it was started as; one not started yet is what the setting says now.
pub fn is_chat(t: &Task) -> bool {
    if let Some(b) = t.at("chatOnly").as_bool() { return b; }
    if t.contains_key("state") || t.b("host") { return false; }
    run_of(t).unwrap_or_else(|| setting(CHAT, Some(t)) == json!(true))
}
/// The same answer for every task of one list: tasks of one repo that say nothing of their own share one lookup (kept
/// beside research only's in the same map, under a key of its own).
pub fn is_chat_among(t: &Task, seen: &mut BTreeMap<String, bool>) -> bool {
    let asks = !t.at("chatOnly").is_boolean() && run_of(t).is_none();
    if !asks || t.at("settings").as_object().map(|s| s.contains_key(CHAT)).unwrap_or(false) || t.b("carried") || t.b("repoFile") { return is_chat(t); }
    let repo = if t.s("repo").is_empty() { t.s("id").split('/').next().unwrap_or("").to_string() } else { t.s("repo").to_string() };
    *seen.entry(format!("{CHAT} {repo}")).or_insert_with(|| is_chat(t))
}
/// A started task whose setting now says something else than it runs as: said beside the setting, wherever it shows.
pub fn later_words(t: &Task, value: &Value) -> String {
    match run_of(t) {
        Some(run) if run != (*value == json!(true)) => format!("counts from its next start: this run is {}", if run { "chat only" } else { "not chat only" }),
        _ => String::new(),
    }
}
/// The moment an agent starts: what the setting says now is what this run is, kept on the task until the next start.
pub fn starts_as(t: &mut Task) {
    let only = setting(CHAT, Some(t)) == json!(true);
    if let Some(run) = t.at("chatRun").as_bool() { if run != only && t.b("startedAgentAt") { t.put("chatTurned", true); } }
    t.put("chatRun", only);
}

// ---- what agents are told ----------------------------------------------------------------------------------------------
pub fn chat_rule(t: &Task) -> String {
    format!("This task is chat only: it is a conversation with the human (a consultation, a discussion, thinking out loud), not work to deliver. Change nothing in the repo: add, edit and delete no file, and commit nothing. Talk, read, look things up and answer. You may write notes or a report into {}/ (this task's own folder among the board's reports, outside every repo) when it helps, but you never have to. Work that should follow from the conversation you suggest in words, in your answer: you add no task for it. Every time you stop, the board waits for the human's next message: nothing is checked, reviewed, pushed or merged, and you need no command to say that you wait. The conversation ends only when the human says it is done.", report_folder(t).display())
}
pub fn chat_tree(t: &Task) -> String {
    format!("You are in your own git worktree on branch {}; the human's checkout is elsewhere and untouched. Commit nothing on this branch, and do not push, merge or switch branches.", t.s("branch"))
}
pub const CHAT_STOP: &str = "Stop whenever it is the human's turn: when you have answered, or need their answer. End with what you have to say to them, not with a summary of work.";
pub const CHAT_WAIT: &str = "Every stop of yours waits for the human, so you need that command only for a permission or a login.";

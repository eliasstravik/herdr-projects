//! Helpers an agent runs inside its own work (Codex's sub-agents, Claude Code's background agents). A helper's turn
//! ending is not the agent's: Codex reports it like any turn end, with the helper's own thread, whose saved
//! conversation names the thread that started it. And while a helper the agent started is still at work, the agent's
//! own turn ending is not the end of its work: the helper's answer starts the agent again.
use super::adapter::transcript;
use super::read_turn::{head_of, read_turn, tail_lines, when, Turn, TAIL};
use crate::json::*;
use std::io::BufRead;
use std::path::Path;

/// Is this conversation a helper's? Codex: its saved conversation names the thread that started it.
pub fn is_helper(kind: &str, session: &str) -> bool {
    if kind != "codex" || session.is_empty() { return false; }
    transcript("codex", session, Path::new("")).map(|f| codex_helper_file(&f)).unwrap_or(false)
}
/// A Codex conversation started by another thread: the first line (its `session_meta`) names a parent.
pub fn codex_helper_file(f: &Path) -> bool {
    let head = head_of(f, 400000);
    let first = head.split('\n').next().unwrap_or("");
    match serde_json::from_str::<Value>(first) {
        Ok(m) => { let p = m.at("payload"); p.b("parent_thread_id") || p.at("source").at("subagent").is_object() }
        Err(_) => first.contains("\"parent_thread_id\":\"") || first.contains("\"subagent\":{"),
    }
}
/// The agent's turn ended while helpers it started are still at work: not the end of its work. `since`: when the agent's
/// program was started.
pub fn still_at_work(kind: &str, turn: Option<&Turn>, since: i64) -> bool { turn.and_then(|t| t.file.as_deref()).map(|f| running(kind, f, since) > 0).unwrap_or(false) }
/// How many helpers this conversation started are still at work at its end. A helper lives in its agent's program, so
/// one set going before the program now running was started (`since`: a conversation resumed by a new program) is
/// gone. Nothing else ends one: a helper may wait on a long command, without a word, for as long as it takes.
pub fn running(kind: &str, file: &Path, since: i64) -> usize {
    match kind { "codex" => codex_running(file, since), "claude" => claude_running(file, since), "commandcode" => commandcode_running(file, since), _ => 0 }
}
fn commandcode_running(file: &Path, since: i64) -> usize {
    let mut live = std::collections::HashSet::new();
    for (_, e) in lines_with(file, &["herdr-projects/helper"]) {
        if e.s("customType") != "herdr-projects/helper" || when(e.at("timestamp")) < since { continue; }
        let d = e.at("data");
        if !d.b("id") { continue; }
        if d.at("running") == &json!(true) { live.insert(d.t("id")); } else { live.remove(d.s("id")); }
    }
    live.len()
}
/// The whole file's lines that hold these words, as JSON: a helper may have been started long before the tail.
fn lines_with(file: &Path, words: &[&str]) -> Vec<(String, Value)> {
    let Ok(f) = std::fs::File::open(file) else { return vec![] };
    std::io::BufReader::new(f).lines().map_while(Result::ok).filter(|l| words.iter().any(|w| l.contains(w))).filter_map(|l| serde_json::from_str::<Value>(&l).ok().map(|v| (l, v))).collect()
}
/// Codex: each sub-agent this thread started has a conversation of its own, at work while its last turn is open.
fn codex_running(file: &Path, since: i64) -> usize {
    let mut ids: Vec<String> = vec![];
    for (_, e) in lines_with(file, &["SubAgentActivity"]) {
        let i = e.at("payload").at("item");
        if i.s("type") == "SubAgentActivity" && i.s("kind") == "started" && i.b("agent_thread_id") && when(e.at("timestamp")) >= since && !ids.contains(&i.t("agent_thread_id")) { ids.push(i.t("agent_thread_id")); }
    }
    ids.iter().filter(|id| transcript("codex", id, Path::new("")).and_then(|f| read_turn("codex", &f, TAIL)).map(|t| t.pending).unwrap_or(false)).count()
}
/// The word after `key` in a line, as far as it goes on with letters, digits, `-` and `_`.
fn word_after(line: &str, key: &str) -> Vec<String> {
    line.match_indices(key).map(|(i, _)| line[i + key.len()..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect::<String>()).filter(|w| !w.is_empty()).collect()
}
/// Claude Code: an agent sent to the background is at work from its launch (or a message that resumes it) until its
/// `<task-notification>` comes back. One that comes back while the agent's turn is still going is no message of its own:
/// it is written as a queued command (`queue-operation`, then an `attachment` taken into the running turn), as seen on
/// HSW-218 (Claude Code 2.1.300, 2026-10-15), where the board waited for a helper back for half an hour.
fn claude_running(file: &Path, since: i64) -> usize {
    let mut live: Vec<(String, i64)> = vec![];
    let mut known: Vec<String> = vec![];
    for (l, e) in lines_with(file, &["Async agent launched", "<task-notification>", "\"SendMessage\""]) {
        let at = when(e.at("timestamp"));
        if e.s("type") == "user" && l.contains("Async agent launched") {
            for id in word_after(&l, "agentId: ") { live.retain(|(x, _)| *x != id); live.push((id.clone(), at)); known.push(id); }
        } else if ["user", "attachment", "queue-operation"].contains(&e.s("type")) && l.contains("<task-notification>") {
            for id in word_after(&l, "<task-id>") { live.retain(|(x, _)| *x != id); }
        } else if e.s("type") == "assistant" {
            for c in e.at("message").arr("content") {
                let to = c.at("input").t("to");
                if c.s("type") == "tool_use" && c.s("name") == "SendMessage" && known.contains(&to) { live.retain(|(x, _)| *x != to); live.push((to, at)); }
            }
        }
    }
    live.iter().filter(|(id, at)| *at >= since && !claude_back(file, id, *at)).count()
}
/// A Claude Code helper's own conversation (`<conversation>/subagents/agent-<id>.jsonl`) ends in its finished reply,
/// written since it was last started: it is back, whatever the agent's own file shows of its notification.
fn claude_back(file: &Path, id: &str, at: i64) -> bool {
    let own = file.with_extension("").join("subagents").join(format!("agent-{id}.jsonl"));
    let Ok(lines) = tail_lines(&own, 256 * 1024) else { return false };
    let Some(e) = lines.iter().rev().find(|e| ["user", "assistant"].contains(&e.s("type"))) else { return false };
    e.s("type") == "assistant" && e.at("message").s("stop_reason") == "end_turn" && when(e.at("timestamp")) >= at
}

//! The text of a saved conversation, one reader per agent program: what was said to the program and what it said
//! back, oldest first. Only words: no tool call, no tool's output, no thinking, nothing the program wrote for itself.
//! It is what the board shows of a chat (the box, the task panel, the web board).
use super::read_turn::{tail_lines, when, TAIL};
use super::talk_commands::{self as cmd, claude_markup, is_stop, STOPPED};
use crate::json::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Debug, PartialEq)]
pub struct Said { pub you: bool, pub text: String, pub at: i64 }
fn text_of(c: &Value, types: &[&str]) -> String {
    match c { Value::String(s) => s.trim().to_string(), Value::Array(a) => a.iter().filter(|x| types.contains(&x.s("type"))).map(|x| x.t("text")).collect::<String>().trim().to_string(), _ => String::new() }
}
/// Two entries in a row from the same side with the same words are one.
fn add(out: &mut Vec<Said>, you: bool, text: &str, at: i64) {
    let t = text.trim();
    if t.is_empty() || out.last().map(|l| l.you == you && l.text == t).unwrap_or(false) { return; }
    out.push(Said { you, text: t.to_string(), at });
}
/// Text pasted in arrives wrapped in a tag: taken off where the prompt is that one paste and nothing else.
pub fn unwrap_paste(s: &str) -> String {
    let t = s.trim();
    if let (Some(open_end), true) = (t.strip_prefix("<pasted_content").and_then(|r| r.find('>')), t.ends_with('>')) {
        let inner_start = "<pasted_content".len() + open_end + 1;
        if let Some(close) = t.rfind("</pasted_content") { if close >= inner_start && t[close..].find('>') == Some(t.len() - close - 1) { return t[inner_start..close].trim().to_string(); } }
    }
    t.to_string()
}
/// Claude Code. An entry is the program's own only on evidence: it names another origin than the human, or it lacks
/// the mark the program puts on every submitted prompt (`promptSource`) while entries of that same version carry it.
/// A command typed to it, a stop and a compaction are read in plain words (talk_commands.rs), never as its markup.
fn claude_talk(rows: &[Value]) -> Vec<Said> {
    let marks: std::collections::HashSet<String> = rows.iter().filter(|e| e.s("type") == "user" && e.b("promptSource")).map(|e| e.t("version")).collect();
    let own = |e: &Value| if e.at("origin").b("kind") { e.at("origin").s("kind") != "human" } else { !e.b("promptSource") && marks.contains(&e.t("version")) };
    let mut out = vec![];
    let mut id: Option<String> = None;
    let mut waits = None;
    for e in rows {
        if e.b("isSidechain") || e.b("isMeta") || e.b("isCompactSummary") { continue; }
        let at = when(e.at("timestamp"));
        // A newer version files a command's outcome as a note of its own; a compaction it did by itself is one too.
        if e.s("type") == "system" {
            let said = match e.s("subtype") {
                "local_command" => claude_markup(e.s("content")).map(|m| cmd::read(m, &mut waits)).unwrap_or_default(),
                "compact_boundary" if e.at("compactMetadata").s("trigger") != "manual" => vec![(false, cmd::COMPACTED_ITSELF.to_string())],
                _ => vec![],
            };
            for (you, text) in &said { add(&mut out, *you, text, at); id = None; }
            continue;
        }
        // Words typed while it works wait in its queue and go in during its turn: written down as that, never as a
        // prompt of their own. Only a person's words (a helper that finished is queued the same way).
        if let Some(text) = queued(e) { add(&mut out, true, &text, when(e.at("timestamp"))); id = None; continue; }
        if !e.at("message").is_object() { continue; }
        if e.s("type") == "user" {
            let text = unwrap_paste(&text_of(e.at("message").at("content"), &["text"]));
            if is_stop(&text) { add(&mut out, false, STOPPED, at); id = None; continue; }
            if let Some(m) = claude_markup(&text) {
                if e.at("origin").b("kind") && e.at("origin").s("kind") != "human" { continue; }
                for (you, text) in cmd::read(m, &mut waits) { add(&mut out, you, &text, at); id = None; }
                continue;
            }
            if text.is_empty() || own(e) { continue; }
            add(&mut out, true, &text, when(e.at("timestamp"))); id = None;
        } else if e.s("type") == "assistant" && !e.b("isApiErrorMessage") {
            let text = text_of(e.at("message").at("content"), &["text"]);
            if text.is_empty() { continue; }
            let mid = e.at("message").t("id");
            match out.last_mut() { Some(l) if !l.you && id.as_deref() == Some(mid.as_str()) && !mid.is_empty() => { l.text.push_str(&format!("\n\n{text}")); } _ => add(&mut out, false, &text, when(e.at("timestamp"))) }
            id = Some(mid).filter(|m| !m.is_empty());
        }
    }
    out
}
/// A message to Claude Code typed while it worked, as it went in.
fn queued(e: &Value) -> Option<String> {
    let a = e.at("attachment");
    if e.s("type") != "attachment" || a.s("type") != "queued_command" || a.s("commandMode") != "prompt" || (a.at("origin").b("kind") && a.at("origin").s("kind") != "human") { return None; }
    Some(unwrap_paste(&text_of(a.at("prompt"), &["text"]))).filter(|t| !t.is_empty())
}
/// Codex. What was typed to it is written down as an event of its own; the same words stand among the model's input
/// too, beside what Codex itself put there, so that input is read only in a file that has no such event.
fn codex_talk(rows: &[Value]) -> Vec<Said> {
    let typed = |e: &Value| -> Option<String> {
        let p = e.at("payload");
        if e.s("type") != "event_msg" { return None; }
        if p.s("type") == "user_message" { return Some(p.t("message")); }
        if p.s("type") == "item_completed" && p.at("item").s("type") == "UserMessage" { return Some(text_of(p.at("item").at("content"), &["text"])); }
        None
    };
    let events = rows.iter().any(|e| typed(e).is_some());
    let mut out = vec![];
    for e in rows {
        let (p, at) = (e.at("payload"), when(e.at("timestamp")));
        if let Some(t) = typed(e) { add(&mut out, true, &t, at); }
        else if e.s("type") == "event_msg" && p.s("type") == "turn_aborted" && ["", "interrupted"].contains(&p.s("reason")) { add(&mut out, false, STOPPED, at); }
        else if e.s("type") == "compacted" { add(&mut out, false, cmd::COMPACTED, at); }
        else if e.s("type") == "response_item" && p.s("type") == "message" && p.s("role") == "assistant" { add(&mut out, false, &text_of(p.at("content"), &["output_text", "text"]), at); }
        else if !events && e.s("type") == "response_item" && p.s("type") == "message" && p.s("role") == "user" { add(&mut out, true, &text_of(p.at("content"), &["input_text", "text"]), at); }
    }
    out
}
/// pi. Its model and thinking level are set at the start of a session; a change of one after that was asked for.
fn pi_talk(rows: &[Value]) -> Vec<Said> {
    let mut out = vec![];
    let (mut model, mut level) = (String::new(), String::new());
    for e in rows {
        let (m, at) = (e.at("message"), when(e.at("timestamp")));
        if e.s("type") == "message" && (m.s("role") == "user" || m.s("role") == "assistant") {
            add(&mut out, m.s("role") == "user", &text_of(m.at("content"), &["text"]), at);
            if m.s("role") == "assistant" && m.s("stopReason") == "aborted" { add(&mut out, false, STOPPED, at); }
        } else if e.s("type") == "compaction" { add(&mut out, false, cmd::COMPACTED, at); }
        else if e.s("type") == "model_change" && e.t("modelId") != model {
            if !out.is_empty() && !model.is_empty() { add(&mut out, false, &format!("Model switched to {}.", e.t("modelId")), at); }
            model = e.t("modelId");
        } else if e.s("type") == "thinking_level_change" && e.t("thinkingLevel") != level {
            if !out.is_empty() && !level.is_empty() { add(&mut out, false, &format!("Thinking level set to {}.", e.t("thinkingLevel")), at); }
            level = e.t("thinkingLevel");
        }
    }
    out
}
fn reader(kind: &str) -> Option<fn(&[Value]) -> Vec<Said>> { match kind { "claude" => Some(claude_talk), "codex" => Some(codex_talk), "pi" => Some(pi_talk), _ => None } }
pub fn can_read_talk(kind: &str) -> bool { reader(kind).is_some() }

// Read again only when the file changed, and a conversation's file is looked for once: the board asks on every beat.
static FILES: LazyLock<Mutex<HashMap<String, PathBuf>>> = LazyLock::new(Default::default);
static READ: LazyLock<Mutex<HashMap<PathBuf, (std::time::SystemTime, u64, Vec<Said>)>>> = LazyLock::new(Default::default);
const FAR: u64 = 6 * 1024 * 1024;
/// The end of a conversation's text: the last `keep` things said, or None where nothing can be read. Never fails.
pub fn talk_of(kind: &str, file: Option<&Path>, session: &str, cwd: &Path, keep: usize) -> Option<Vec<Said>> {
    let read = reader(kind)?;
    let mut f = file.filter(|f| f.exists()).map(|f| f.to_path_buf());
    let key = format!("{kind}|{session}|{}", cwd.display());
    if f.is_none() && !session.is_empty() {
        let mut files = FILES.lock().ok()?;
        f = files.get(&key).filter(|p| p.exists()).cloned().or_else(|| { let found = super::adapter::transcript(kind, session, cwd); if let Some(p) = &found { files.insert(key.clone(), p.clone()); } found });
    }
    let f = f?;
    let st = std::fs::metadata(&f).ok()?;
    let (mtime, size) = (st.modified().ok()?, st.len());
    if let Some((m, s, list)) = READ.lock().ok()?.get(&f) { if *m == mtime && *s == size { return Some(tail_of(list, keep)); } }
    // The end of the file is read; where that holds little of what was said (a pasted picture is one very long line),
    // a longer end is.
    let mut list = read(&tail_lines(&f, TAIL).ok()?);
    if list.len() < keep && size > TAIL { list = read(&tail_lines(&f, FAR).ok()?); }
    let mut cache = READ.lock().ok()?;
    if cache.len() > 200 { cache.clear(); }
    cache.insert(f, (mtime, size, list.clone()));
    Some(tail_of(&list, keep))
}
fn tail_of(list: &[Said], keep: usize) -> Vec<Said> { list[list.len().saturating_sub(keep)..].to_vec() }

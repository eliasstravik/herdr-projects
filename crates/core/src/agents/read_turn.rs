//! Reading the end of a saved conversation, one reader per agent program: last message, error, tokens. Only the tail
//! is read: these files grow to many megabytes. Every reader fails soft and returns None.
use super::provider_error::provider_error;
use crate::json::*;
use crate::store::failures::is_auth_failure;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub const TAIL: u64 = 768 * 1024;
/// The last `bytes` of a file as JSON lines (a cut first line is left out).
pub fn tail_lines(file: &Path, bytes: u64) -> std::io::Result<Vec<Value>> {
    let size = std::fs::metadata(file)?.len();
    let len = size.min(bytes);
    let mut f = std::fs::File::open(file)?;
    f.seek(SeekFrom::Start(size - len))?;
    let mut buf = vec![0u8; len as usize];
    f.read_exact(&mut buf)?;
    let text = String::from_utf8_lossy(&buf);
    let mut lines: Vec<&str> = text.split('\n').collect();
    if len < size && !lines.is_empty() { lines.remove(0); }
    Ok(lines.into_iter().filter(|l| !l.is_empty()).filter_map(|l| serde_json::from_str(l).ok()).collect())
}
pub fn head_of(file: &Path, bytes: usize) -> String {
    let Ok(mut f) = std::fs::File::open(file) else { return String::new() };
    let mut buf = vec![0u8; bytes];
    let n = f.read(&mut buf).unwrap_or(0);
    String::from_utf8_lossy(&buf[..n]).into_owned()
}
/// A timestamp as milliseconds; 0 when it is not one.
pub fn when(ts: &Value) -> i64 {
    match ts {
        Value::String(s) => chrono::DateTime::parse_from_rfc3339(s).map(|d| d.timestamp_millis()).unwrap_or(0),
        Value::Number(n) => n.as_f64().unwrap_or(0.0) as i64,
        _ => 0,
    }
}
/// Does a saved conversation hold these words, written since a moment? The longest line of the text stands for it.
/// Read back as far as that moment, however much was written since. A program's note that the words wait in its
/// queue (Claude Code: `queue-operation`) is not the program taking them.
pub fn holds(file: &Path, text: &str, since: i64) -> std::io::Result<bool> {
    let mut lines: Vec<&str> = text.split('\n').map(|l| l.trim()).collect();
    lines.sort_by_key(|l| std::cmp::Reverse(l.chars().count()));
    let line: String = lines.first().map(|l| l.chars().take(80).collect()).unwrap_or_default();
    if line.is_empty() { return Ok(false); }
    let kept = { let s = serde_json::to_string(&line).unwrap_or_default(); s[1..s.len() - 1].to_string() };
    // Letters every way of writing the line down keeps as they are: only the lines that have them are read as JSON.
    let mut plain = String::new();
    let mut cur = String::new();
    for c in line.chars().chain(std::iter::once('\0')) {
        if c.is_ascii_alphanumeric() || c == ' ' { cur.push(c); } else { if cur.len() >= 8 && cur.len() > plain.len() { plain = cur.clone(); } cur.clear(); }
    }
    let size = std::fs::metadata(file)?.len();
    let mut f = std::fs::File::open(file)?;
    let mut bytes = TAIL;
    loop {
        let len = size.min(bytes);
        f.seek(SeekFrom::Start(size - len))?;
        let mut buf = vec![0u8; len as usize];
        f.read_exact(&mut buf)?;
        let cut = if len < size { buf.iter().position(|b| *b == b'\n') } else { None };
        if len < size && cut.is_none() { bytes *= 4; continue; }
        let start = cut.map(|c| c + 1).unwrap_or(0);
        let text = String::from_utf8_lossy(&buf[start..]);
        let mut oldest: Option<i64> = None;
        for l in text.split('\n') {
            if l.is_empty() { continue; }
            if oldest.is_none() { oldest = serde_json::from_str::<Value>(l).ok().map(|e| when(e.at("timestamp"))).filter(|t| *t != 0); }
            if !plain.is_empty() && !l.contains(&plain) { continue; }
            let Ok(e) = serde_json::from_str::<Value>(l) else { continue };
            if e.s("type") != "queue-operation" && when(e.at("timestamp")) >= since - 2000 && e.to_string().contains(&kept) { return Ok(true); }
        }
        if len >= size || oldest.map(|o| o < since - 2000).unwrap_or(false) { return Ok(false); }
        bytes *= 4;
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Usage { #[serde(rename = "in")] pub inp: f64, pub out: f64, pub context: f64, pub window: Option<f64> }
/// What every reader returns. `finished`: a complete assistant message ends the file. `pending`: a tool call or a
/// partial message does. `interrupted`: the human stopped it. `error`: the program recorded an API, login or limit
/// error; `passing`: the error is the provider's and waiting may fix it, `reason` says which. `began`: when the turn this
/// ends was started (the time of the last prompt in the part read; 0: none there).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Turn {
    pub id: String, pub pos: i64, pub began: i64, pub said: String, pub stop: String, pub error: String,
    pub finished: bool, pub pending: bool, pub interrupted: bool, pub usage: Option<Usage>, pub model: String,
    pub auth: bool, pub passing: bool, pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub file: Option<PathBuf>,
}
/// The provider's side of an error, decided once the turn is read.
fn judge(mut t: Turn, code: &str, status: i64, auth_code: bool) -> Turn {
    if t.error.is_empty() { return t; }
    let pe = provider_error(&t.error, code, status);
    t.auth = auth_code || pe.auth || is_auth_failure(&t.error);
    t.passing = pe.passing && !t.auth;
    t.reason = if t.passing { pe.reason } else { String::new() };
    t
}
fn window_of(model: &str) -> Option<f64> {
    let known = ["claude-opus-4", "claude-sonnet-4", "claude-haiku-4", "claude-3"];
    known.iter().any(|p| model.starts_with(p)).then_some(200000.0)
}
fn usage_of(i: f64, o: f64, model: &str, window: Option<f64>) -> Option<Usage> { (i > 0.0 || o > 0.0).then(|| Usage { inp: i, out: o, context: i + o, window: window.or_else(|| window_of(model)) }) }
fn texts(content: &Value, types: &[&str]) -> String {
    match content { Value::String(s) => s.clone(), Value::Array(a) => a.iter().filter(|c| types.contains(&c.s("type"))).map(|c| c.t("text")).collect::<Vec<_>>().join(""), _ => String::new() }
}
/// Codex writes its code as a word, or as a table with the word as its one key ({"http_connection_failed": {…}}).
fn codex_code(c: &Value) -> String {
    let w = match c { Value::Object(o) => o.keys().next().cloned().unwrap_or_default(), _ => text(c) };
    let mut out = String::new();
    for (i, ch) in w.chars().enumerate() { if ch.is_ascii_uppercase() { if i > 0 { out.push('_'); } out.push(ch.to_ascii_lowercase()); } else { out.push(ch); } }
    out
}

pub fn read_claude(file: &Path, bytes: u64) -> Option<Turn> {
    let rows = tail_lines(file, bytes).ok()?;
    enum Last<'a> { Say { id: String, text: String, tool: bool, e: &'a Value }, Error(&'a Value), Interrupt(&'a Value, String), User(&'a Value) }
    let (mut last, mut usage, mut model, mut latest, mut began): (Option<Last>, Option<(f64, f64)>, String, String, i64) = (None, None, String::new(), String::new(), 0);
    for e in &rows {
        let m = e.at("message");
        if e.s("type") == "assistant" && m.is_object() {
            if e.b("isApiErrorMessage") { last = Some(Last::Error(e)); continue; }
            let text = texts(m.at("content"), &["text"]);
            let tool = m.arr("content").iter().any(|c| c.s("type") == "tool_use");
            let mid = m.t("id");
            if !text.trim().is_empty() {
                latest = match &last { Some(Last::Say { id, text: t, .. }) if *id == mid && !t.is_empty() => format!("{t}{text}"), _ => text.clone() };
            }
            match &mut last {
                Some(Last::Say { id, text: t, tool: tl, e: le }) if *id == mid => { t.push_str(&text); *tl = *tl || tool; *le = e; }
                _ => last = Some(Last::Say { id: mid, text, tool, e }),
            }
            if m.at("usage").is_object() { let u = m.at("usage"); usage = Some((u.f("input_tokens") + u.f("cache_creation_input_tokens") + u.f("cache_read_input_tokens"), u.f("output_tokens"))); if m.b("model") { model = m.t("model"); } }
        } else if e.s("type") == "user" && m.is_object() && !e.b("isCompactSummary") {
            // (A compaction's summary, written also when Claude Code shrinks a session left idle, is the program's own
            // note, not a prompt: the turn before it is still the last one.)
            let text = texts(m.at("content"), &["text"]);
            if text.starts_with("[Request interrupted by user") {
                let said = match &last { Some(Last::Say { text, .. }) => text.clone(), _ => String::new() };
                last = Some(Last::Interrupt(e, said));
            } else {
                last = Some(Last::User(e)); // a real prompt or a tool result: either way the agent has not answered it yet
                if !text.trim().is_empty() && !e.b("isMeta") && !e.b("isSidechain") { began = when(e.at("timestamp")); }
            }
        }
    }
    let last = last?;
    let e = match &last { Last::Say { e, .. } | Last::Error(e) | Last::Interrupt(e, _) | Last::User(e) => *e };
    let base = Turn { pos: when(e.at("timestamp")), began, usage: usage.and_then(|(i, o)| usage_of(i, o, &model, None)), model: model.clone(), ..Default::default() };
    Some(match last {
        Last::Error(e) => {
            let text = e.at("message").arr("content").iter().map(|c| c.t("text")).collect::<String>().trim().to_string();
            let error = if text.is_empty() { if e.b("error") { e.t("error") } else { "error".into() } } else { text.clone() };
            judge(Turn { id: e.t("uuid"), said: text, stop: "error".into(), error, ..base }, e.s("error"), e.i("apiErrorStatus"), e.s("error") == "authentication_failed")
        }
        Last::Interrupt(e, said) => Turn { id: e.t("uuid"), said, stop: "interrupted".into(), interrupted: true, ..base },
        Last::User(e) => Turn { id: e.t("uuid"), pending: true, said: latest.trim().to_string(), ..base },
        Last::Say { id, text, tool, e } => {
            let stop = e.at("message").t("stop_reason");
            let done = !tool && ["end_turn", "stop_sequence", "max_tokens", "refusal"].contains(&stop.as_str());
            let said = if !text.trim().is_empty() { text.trim().to_string() } else if done { String::new() } else { latest.trim().to_string() };
            Turn { id, said, stop: match stop.as_str() { "max_tokens" => "length".into(), "refusal" => "refusal".into(), _ if done => "stop".into(), _ => String::new() }, finished: done, pending: !done, ..base }
        }
    })
}
pub fn read_codex(file: &Path, bytes: u64) -> Option<Turn> {
    let rows = tail_lines(file, bytes).ok()?;
    #[derive(PartialEq)] enum K { Final, Partial, User, Tool, Error, Done, Interrupt }
    let mut last: Option<(K, &Value, String, String, String)> = None; // kind, entry, id, error text, code
    let (mut said, mut usage, mut model, mut window, mut began) = (String::new(), None::<(f64, f64)>, String::new(), None::<f64>, 0);
    for e in &rows {
        let p = e.at("payload");
        let (ty, pty) = (e.s("type"), p.s("type"));
        if ty == "turn_context" { if p.b("model") { model = p.t("model"); } }
        else if ty == "response_item" && pty == "message" && p.s("role") == "assistant" { said = p.arr("content").iter().map(|c| c.t("text")).collect(); last = Some((if p.s("phase") == "final_answer" { K::Final } else { K::Partial }, e, String::new(), String::new(), String::new())); }
        else if ty == "response_item" && pty == "message" && p.s("role") == "user" { last = Some((K::User, e, String::new(), String::new(), String::new())); began = when(e.at("timestamp")); }
        else if ty == "response_item" && pty.contains("call") { last = Some((K::Tool, e, String::new(), String::new(), String::new())); }
        else if ty == "event_msg" && pty == "token_count" && p.at("info").is_object() {
            let i = p.at("info");
            let u = if i.at("last_token_usage").is_object() { i.at("last_token_usage") } else { i.at("total_token_usage") };
            usage = Some((u.f("input_tokens"), u.f("output_tokens")));
            if i.b("model_context_window") { window = Some(i.f("model_context_window")); }
        }
        // A turn the provider cut off also ends in `task_complete`: with no last message, and the error it ended on.
        else if ty == "event_msg" && pty == "task_complete" && p.b("error") {
            let err = p.at("error");
            let msg = if err.is_object() { err.t("message") } else { text(err) };
            last = Some((K::Error, e, p.t("turn_id"), if msg.is_empty() { "error".into() } else { msg }, codex_code(err.at("codex_error_info"))));
        }
        else if ty == "event_msg" && pty == "task_complete" { last = Some((K::Done, e, p.t("turn_id"), String::new(), String::new())); if p.b("last_agent_message") { said = p.t("last_agent_message"); } }
        else if ty == "event_msg" && pty == "turn_aborted" { last = Some((K::Interrupt, e, p.t("turn_id"), String::new(), String::new())); }
        else if ty == "event_msg" && (pty == "error" || pty == "stream_error") {
            let msg = if p.b("message") { p.t("message") } else if p.b("error") { p.t("error") } else { "error".into() };
            last = Some((K::Error, e, String::new(), msg, codex_code(p.at("codex_error_info"))));
        }
        else if ty == "event_msg" && pty == "task_started" { last = Some((K::User, e, String::new(), String::new(), String::new())); began = when(e.at("timestamp")); }
    }
    let (kind, e, id, err, code) = last?;
    let pos = when(e.at("timestamp"));
    let base = Turn { pos, began, usage: usage.and_then(|(i, o)| usage_of(i, o, &model, window)), model: model.clone(), id: if id.is_empty() { pos.to_string() } else { id }, ..Default::default() };
    Some(match kind {
        K::Error => judge(Turn { said: err.clone(), stop: "error".into(), error: err, ..base }, &code, 0, false),
        K::Interrupt => Turn { said, stop: "interrupted".into(), interrupted: true, ..base },
        K::Done | K::Final => Turn { said: said.trim().to_string(), stop: "stop".into(), finished: true, ..base },
        _ => Turn { pending: true, said: said.trim().to_string(), ..base },
    })
}
pub fn read_pi(file: &Path, bytes: u64) -> Option<Turn> {
    let rows = tail_lines(file, bytes).ok()?;
    let mut last: Option<(bool, &Value)> = None; // (said by the agent, entry)
    let (mut usage, mut model, mut latest, mut began) = (None::<(f64, f64)>, String::new(), String::new(), 0);
    for e in &rows {
        let m = e.at("message");
        if e.s("type") != "message" || !m.is_object() { continue; }
        if m.s("role") == "assistant" {
            last = Some((true, e));
            let tx = texts(m.at("content"), &["text"]).trim().to_string();
            if !tx.is_empty() { latest = tx; }
            if m.at("usage").is_object() { let u = m.at("usage"); usage = Some((u.f("input") + u.f("cacheRead") + u.f("cacheWrite"), u.f("output"))); }
            if m.b("model") { model = m.t("model"); }
        } else if m.s("role") == "user" || m.s("role") == "toolResult" { last = Some((false, e)); if m.s("role") == "user" { began = when(e.at("timestamp")); } }
    }
    let (said_by_agent, e) = last?;
    let base = Turn { pos: when(e.at("timestamp")), began, usage: usage.and_then(|(i, o)| usage_of(i, o, &model, None)), model: model.clone(), id: e.t("id"), ..Default::default() };
    if !said_by_agent { return Some(Turn { pending: true, said: latest, ..base }); }
    let m = e.at("message");
    let text = texts(m.at("content"), &["text"]).trim().to_string();
    let stop = m.t("stopReason");
    Some(match stop.as_str() {
        "error" => { let err = if m.b("errorMessage") { m.t("errorMessage") } else if !text.is_empty() { text.clone() } else { "error".into() }; judge(Turn { said: text, stop: "error".into(), error: err, ..base }, "", 0, false) }
        "aborted" => Turn { said: text, stop: "interrupted".into(), interrupted: true, ..base },
        _ => { let done = stop == "stop" || stop == "length"; Turn { said: text, stop: if stop == "length" { "length".into() } else if done { "stop".into() } else { String::new() }, finished: done, pending: !done, ..base } }
    })
}
/// The end of a conversation, by this program's reader.
pub fn read_turn(kind: &str, file: &Path, bytes: u64) -> Option<Turn> {
    match kind { "claude" => read_claude(file, bytes), "codex" => read_codex(file, bytes), "pi" => read_pi(file, bytes), _ => None }
}
/// A file whose name passes `test`, anywhere under a folder (a few levels deep), newest first.
pub fn find_file(root: &Path, test: &dyn Fn(&str) -> bool, depth: usize) -> Vec<PathBuf> {
    fn walk(dir: &Path, d: usize, test: &dyn Fn(&str) -> bool, hits: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) { if d > 0 { walk(&p, d - 1, test, hits); } }
            else if test(&e.file_name().to_string_lossy()) { hits.push(p); }
        }
    }
    let mut hits = vec![];
    walk(root, depth, test, &mut hits);
    let mtime = |p: &PathBuf| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    hits.sort_by(|a, b| mtime(b).cmp(&mtime(a)));
    hits
}

//! The tokens a saved conversation has used so far, one counter per agent program: everything the model read and
//! everything it wrote, every request of the conversation added up, as the provider counts it (what a request read
//! again from the cache too). A conversation file only grows, so it is read on from where the last count stopped:
//! `state` is what the counter gave back the time before ({ at: the byte it read to, n: the tokens so far, … }).
use crate::json::*;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// The whole lines written since byte `at`, and where they end. A last line still being written waits.
fn grown(file: &Path, at: u64) -> std::io::Result<(Vec<Value>, u64)> {
    let size = std::fs::metadata(file)?.len();
    let at = if size < at { 0 } else { at };
    if size == at { return Ok((vec![], at)); }
    let mut f = std::fs::File::open(file)?;
    f.seek(SeekFrom::Start(at))?;
    let mut buf = vec![0u8; (size - at) as usize];
    f.read_exact(&mut buf)?;
    let end = buf.iter().rposition(|b| *b == b'\n').map(|i| i + 1).unwrap_or(0);
    let rows = String::from_utf8_lossy(&buf[..end]).split('\n').filter(|l| !l.is_empty()).filter_map(|l| serde_json::from_str(l).ok()).collect();
    Ok((rows, at + end as u64))
}
fn from(file: &Path, state: &Value) -> Obj {
    let size = std::fs::metadata(file).map(|m| m.len()).unwrap_or(0) as f64;
    if state.is_object() && size >= state.f("at") { state.as_object().cloned().unwrap_or_default() } else { Obj::new() }
}
/// Claude Code writes one line per part of a reply, each with the whole reply's use: a reply counts once, by its id.
pub fn claude_tokens(file: &Path, state: &Value) -> std::io::Result<Value> {
    let s = from(file, state);
    let (mut n, mut id, mut id_n) = (s.f("n"), s.t("id"), s.f("idN"));
    let (rows, at) = grown(file, s.f("at") as u64)?;
    for e in rows {
        let u = e.at("message").at("usage");
        if e.s("type") != "assistant" || e.b("isApiErrorMessage") || !u.is_object() { continue; }
        let v = u.f("input_tokens") + u.f("cache_creation_input_tokens") + u.f("cache_read_input_tokens") + u.f("output_tokens");
        let mid = e.at("message").t("id");
        if !mid.is_empty() && mid == id { n += (v - id_n).max(0.0); id_n = id_n.max(v); } else { n += v; id = mid; id_n = v; }
    }
    Ok(json!({ "at": at, "n": n, "id": id, "idN": id_n }))
}
/// Codex writes the conversation's running total after every request. A total that went down began again.
pub fn codex_tokens(file: &Path, state: &Value) -> std::io::Result<Value> {
    let s = from(file, state);
    let (mut n, mut last) = (s.f("n"), s.f("last"));
    let (rows, at) = grown(file, s.f("at") as u64)?;
    for e in rows {
        let p = e.at("payload");
        let u = p.at("info").at("total_token_usage");
        if e.s("type") != "event_msg" || p.s("type") != "token_count" || !u.is_object() { continue; }
        let v = u.f("input_tokens") + u.f("output_tokens");
        n += if v >= last { v - last } else { v };
        last = v;
    }
    Ok(json!({ "at": at, "n": n, "last": last }))
}
/// pi writes each reply once, with its own use.
pub fn pi_tokens(file: &Path, state: &Value) -> std::io::Result<Value> {
    let s = from(file, state);
    let mut n = s.f("n");
    let (rows, at) = grown(file, s.f("at") as u64)?;
    for e in rows {
        let m = e.at("message");
        if e.s("type") == "message" && m.s("role") == "assistant" && m.at("usage").is_object() {
            let u = m.at("usage");
            n += u.f("input") + u.f("cacheRead") + u.f("cacheWrite") + u.f("output");
        }
    }
    Ok(json!({ "at": at, "n": n }))
}
/// Command Code writes usage on each assistant entry, outside the message; cache writes include the 1h subset.
pub fn commandcode_tokens(file: &Path, state: &Value) -> std::io::Result<Value> {
    let s = from(file, state);
    let mut n = s.f("n");
    let (rows, at) = grown(file, s.f("at") as u64)?;
    for e in rows {
        if e.s("type") != "message" || e.at("message").s("role") != "assistant" { continue; }
        let u = e.at("usage");
        n += u.f("inputTokens") + u.f("outputTokens") + u.f("cacheReadTokens") + u.f("cacheWriteTokens");
    }
    Ok(json!({ "at": at, "n": n }))
}
/// This program's counter, where it has one.
pub fn tokens(kind: &str, file: &Path, state: &Value) -> Option<Value> {
    match kind { "claude" => claude_tokens(file, state).ok(), "codex" => codex_tokens(file, state).ok(), "pi" => pi_tokens(file, state).ok(), "commandcode" => commandcode_tokens(file, state).ok(), _ => None }
}

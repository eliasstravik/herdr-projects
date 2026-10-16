//! Command Code's active conversation branch, its last turn and the human-readable chat.
use crate::agents::read_turn::{judge, tail_lines, when, Turn, Usage};
use crate::agents::talk::{add, text_of, Said};
use crate::json::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// The file is an append-only tree. Rewinds leave the abandoned messages on disk, outside the active ancestry.
fn branch(rows: &[Value]) -> Vec<&Value> {
    let by_id: HashMap<&str, &Value> = rows.iter().filter(|e| e.b("id")).map(|e| (e.s("id"), e)).collect();
    let mut out = vec![];
    let mut seen = HashSet::new();
    let mut last = rows.iter().rev().find(|e| e.b("id") && e.s("type") != "session");
    while let Some(e) = last {
        if !seen.insert(e.s("id")) { break; }
        out.push(e);
        last = by_id.get(e.s("parentId")).copied();
    }
    out.reverse();
    out
}
fn human(m: &Value) -> bool {
    let meta = m.at("meta");
    m.s("role") == "user" && !meta.b("isMeta") && ["", "user", "steering"].contains(&meta.s("source")) && !m.arr("content").iter().any(|c| c.s("type") == "tool_result")
}
fn usage(e: &Value) -> Option<Usage> {
    let u = e.at("usage");
    if !u.is_object() { return None; }
    let inp = u.f("inputTokens") + u.f("cacheReadTokens") + u.f("cacheWriteTokens");
    Some(Usage { inp, out: u.f("outputTokens"), context: inp + u.f("outputTokens"), window: None })
}
pub fn read(file: &Path, bytes: u64) -> Option<Turn> {
    let rows = tail_lines(file, bytes).ok()?;
    let mut t = Turn::default();
    for e in branch(&rows) {
        let m = e.at("message");
        if e.s("type") == "message" {
            // A failed run flushes its original prompt and synthetic error feedback after onRunEnd.
            if t.stop == "run_error" && m.s("role") == "user" && when(m.at("meta").at("createdAt")) <= t.pos { continue; }
            t.id = e.t("id"); t.pos = when(e.at("timestamp"));
            t.finished = false; t.pending = true; t.interrupted = false; t.error.clear(); t.stop.clear();
            t.auth = false; t.passing = false; t.reason.clear();
            if human(m) { t.began = t.pos; t.said.clear(); }
            if m.s("role") == "assistant" {
                let text = text_of(m.at("content"), &["text"]);
                if !text.is_empty() { t.said = text.clone(); }
                t.pending = m.arr("content").iter().any(|c| c.s("type") == "tool_use");
                t.finished = !t.pending && !text.is_empty();
                t.pending = !t.finished;
                if let Some(u) = usage(e) { t.usage = Some(u); }
                if e.b("model") { t.model = e.t("model"); }
            }
        } else if e.s("type") == "custom" && e.s("customType") == "herdr-projects/run-start" {
            t.id = e.t("id"); t.pos = when(e.at("timestamp")); t.began = t.pos;
            t.finished = false; t.pending = true; t.interrupted = false; t.error.clear(); t.said.clear(); t.stop.clear();
            t.auth = false; t.passing = false; t.reason.clear();
        } else if e.s("type") == "custom" && e.s("customType") == "herdr-projects/run-end" {
            let d = e.at("data");
            t.id = e.t("id"); t.pos = when(e.at("timestamp")); t.stop = d.t("stop");
            t.said = d.t("finalText"); t.error = d.t("error"); t.pending = false;
            t.interrupted = ["interrupted", "permission_denied", "terminate"].contains(&t.stop.as_str());
            if t.stop == "max_turns" && t.error.is_empty() { t.error = "Command Code reached its maximum turn limit before finishing.".into(); }
            t.finished = t.error.is_empty() && !t.interrupted;
            t = judge(t, d.s("code"), d.i("status"), false);
        }
    }
    (!t.id.is_empty()).then_some(t)
}
/// A provider can fail before the store ever writes an assistant message. The authenticated callback still carries it.
pub fn reported(turn: Option<Turn>, event: &Value) -> Option<Turn> {
    let interrupted = ["interrupted", "permission_denied", "terminate"].contains(&event.s("stop"));
    if !event.b("error") && !interrupted && event.s("stop") != "max_turns" { return turn; }
    let at = if event.i("finished_at") > 0 { event.i("finished_at") } else { event.i("at") };
    if turn.as_ref().is_some_and(|t| t.pos > at) { return turn; }
    Some(judge(Turn { id: if event.i("finished_at") > 0 { format!("commandcode-{}-{at}", event.s("session")) } else { format!("report-{}", event.i("n")) }, pos: at, began: at,
        error: if event.s("stop") == "max_turns" && !event.b("error") { "Command Code reached its maximum turn limit before finishing.".into() } else { event.t("error") },
        stop: if interrupted { "interrupted".into() } else { "error".into() }, finished: false, pending: false, interrupted,
        ..turn.unwrap_or_default() }, event.s("code"), event.i("status"), false))
}
pub fn talk(rows: &[Value]) -> Vec<Said> {
    let mut out = vec![];
    for e in branch(rows) {
        let m = e.at("message");
        let at = when(e.at("timestamp"));
        if e.s("type") == "message" && (human(m) || m.s("role") == "assistant") {
            add(&mut out, m.s("role") == "user", &text_of(m.at("content"), &["text"]), at);
        } else if e.s("type") == "compaction" { add(&mut out, false, crate::agents::talk_commands::COMPACTED, at); }
        else if e.s("type") == "model_change" { add(&mut out, false, &format!("Model switched to {}.", e.s("model")), at); }
        else if e.s("type") == "effort_change" { add(&mut out, false, &format!("Thinking level set to {}.", e.s("effort")), at); }
        else if e.s("customType") == "herdr-projects/run-end" && e.at("data").s("stop") == "interrupted" { add(&mut out, false, crate::agents::talk_commands::STOPPED, at); }
    }
    out
}

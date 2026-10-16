//! Command Code's native session schema: active branches, private chat, provider errors and incremental tokens.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::agents::{adapter, commandcode, read_tokens::tokens, read_turn::{read_turn, TAIL}, talk::talk_of};
use sw::json::*;
use std::path::{Path, PathBuf};

fn write_rows(file: &Path, rows: &[Value]) { std::fs::create_dir_all(file.parent().unwrap()).unwrap(); std::fs::write(file, rows.iter().map(|e| format!("{e}\n")).collect::<String>()).unwrap(); }
fn message(id: &str, parent: Option<&str>, role: &str, content: Value) -> Value { json!({ "type": "message", "id": id, "parentId": parent, "timestamp": "2026-10-16T12:00:01Z", "message": { "role": role, "content": content } }) }
fn session(w: &World, id: &str, cwd: &Path) -> PathBuf {
    let f = sw::store::files::user_home().join(".commandcode/projects/test").join(format!("{id}.jsonl"));
    write_rows(&f, &[json!({ "type": "session", "id": id, "cwd": cwd, "timestamp": sw::clock::iso(sw::clock::now()) })]);
    assert!(f.starts_with(&w.base)); f
}

#[test]
fn commandcode_reads_the_active_branch_without_tools_thinking_or_injected_prompts_in_chat() {
    let w = world();
    let f = w.base.join("conversation.jsonl");
    let rows = vec![
        message("u", None, "user", json!([{ "type": "text", "text": "Fix 日本語" }])),
        message("discarded", Some("u"), "assistant", json!([{ "type": "text", "text": "Abandoned answer" }])),
        message("a", Some("u"), "assistant", json!([{ "type": "thinking", "thinking": "private" }, { "type": "text", "text": "Checking." }, { "type": "tool_use", "id": "t", "name": "read" }])),
        message("r", Some("a"), "user", json!([{ "type": "tool_result", "tool_use_id": "t", "content": "private output" }])),
    ];
    write_rows(&f, &rows);
    let t = read_turn("commandcode", &f, TAIL).unwrap(); assert!(t.pending && !t.finished);
    let mut rows = rows;
    rows.push(message("last", Some("r"), "assistant", json!([{ "type": "text", "text": "Fixed." }])));
    rows.push(json!({ "type": "custom", "id": "end", "parentId": "last", "timestamp": "2026-10-16T12:00:02Z", "customType": "herdr-projects/run-end", "data": { "stop": "end_turn", "finalText": "Fixed." } }));
    write_rows(&f, &rows);
    let t = read_turn("commandcode", &f, TAIL).unwrap(); assert!(t.finished && !t.pending); assert_eq!(t.said, "Fixed.");
    let chat = talk_of("commandcode", Some(&f), "", &w.root, 20).unwrap();
    assert_eq!(chat.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), ["Fix 日本語", "Checking.", "Fixed."]);
    let mut injected = message("injected", Some("end"), "user", json!([{ "type": "text", "text": "Private injected instruction" }]));
    injected["message"]["meta"] = json!({ "source": "stop_hook" });
    let mut compacted = rows.clone(); compacted.push(injected); compacted.push(json!({ "type": "compaction", "id": "compact", "parentId": "injected", "summary": "Private summary" }));
    let chat = commandcode::turn::talk(&compacted);
    assert_eq!(chat.last().unwrap().text, sw::agents::talk_commands::COMPACTED);
    assert!(!chat.iter().any(|s| s.text.contains("Private")));
    rows.push(json!({ "type": "custom", "id": "stop", "parentId": "end", "timestamp": "2026-10-16T12:00:03Z", "customType": "herdr-projects/run-end", "data": { "stop": "interrupted", "finalText": "" } }));
    write_rows(&f, &rows); assert!(read_turn("commandcode", &f, TAIL).unwrap().interrupted);
}

#[test]
fn commandcode_errors_before_any_transcript_keep_the_provider_classification() {
    let _w = world();
    for (error, code, status, auth, passing) in [("Provider overloaded", "overloaded_error", 529, false, true), ("Login required", "unauthorized", 401, true, false), ("Quota exhausted", "billing_error", 429, false, false)] {
        let t = commandcode::turn::reported(None, &json!({ "n": 1, "at": 99, "error": error, "code": code, "status": status })).unwrap();
        assert_eq!((t.auth, t.passing), (auth, passing), "{t:?}"); assert!(!t.finished && !t.pending);
    }
    let interrupted = commandcode::turn::reported(None, &json!({ "n": 2, "at": 100, "stop": "interrupted" })).unwrap();
    assert!(interrupted.interrupted && !interrupted.finished);
    let limit = commandcode::turn::reported(None, &json!({ "n": 3, "at": 101, "stop": "max_turns" })).unwrap();
    assert!(!limit.error.is_empty() && !limit.finished);
    let report = |at| commandcode::turn::reported(None, &json!({ "n": at, "at": at, "finished_at": 95, "session": "s1", "error": "Unauthorized", "status": 401 })).unwrap();
    assert_eq!(report(100), report(110), "duplicate callbacks keep the same id and end time");
}

#[test]
fn commandcode_tokens_count_each_request_and_cache_once_across_incremental_reads() {
    let w = world(); let f = w.base.join("tokens.jsonl");
    let mut a = message("a", None, "assistant", json!([]));
    a["usage"] = json!({ "inputTokens": 10, "outputTokens": 2, "cacheReadTokens": 20, "cacheWriteTokens": 3, "cacheWriteTokens1h": 3 });
    write_rows(&f, &[a.clone()]);
    let first = tokens("commandcode", &f, &Value::Null).unwrap(); assert_eq!(first.f("n"), 35.0);
    assert_eq!(tokens("commandcode", &f, &first).unwrap(), first);
    use std::io::Write;
    let body = a.to_string(); let cut = body.len() / 2;
    let mut out = std::fs::OpenOptions::new().append(true).open(&f).unwrap(); out.write_all(body[..cut].as_bytes()).unwrap();
    assert_eq!(tokens("commandcode", &f, &first).unwrap(), first, "partial JSONL is not consumed");
    writeln!(out, "{}", &body[cut..]).unwrap();
    let second = tokens("commandcode", &f, &first).unwrap(); assert_eq!(second.f("n"), 70.0);
    write_rows(&f, &[a]); assert_eq!(tokens("commandcode", &f, &second).unwrap().f("n"), 35.0);
}

#[test]
fn commandcode_discovers_only_the_matching_project_and_refuses_an_ambiguous_task_session() {
    let w = world(); let since = sw::clock::now() - 1000;
    let f = session(&w, "own", &w.root);
    session(&w, "other", &w.base);
    assert_eq!(adapter::transcript("commandcode", "own", &w.root), Some(f.clone()));
    assert!(adapter::transcript("commandcode", "other", &w.root).is_none());
    assert!(adapter::transcript("commandcode", "../own", &w.root).is_none());
    use std::io::Write;
    writeln!(std::fs::OpenOptions::new().append(true).open(&f).unwrap(), "{}", message("u", None, "user", json!([{ "type": "text", "text": "Work board task proj/1" }]))).unwrap();
    assert_eq!(adapter::find_session("commandcode", &w.root, since, "Work board task proj/1"), Some("own".into()));
    let other = session(&w, "copy", &w.root); std::fs::copy(&f, &other).unwrap();
    let mut body = read(&other); body = body.replacen("\"id\":\"own\"", "\"id\":\"copy\"", 1); std::fs::write(&other, body).unwrap();
    assert!(adapter::find_session("commandcode", &w.root, since, "Work board task proj/1").is_none());
    assert_eq!(adapter::find_session_but("commandcode", &w.root, since, "Work board task proj/1", "copy"), Some("own".into()));
    std::fs::write(other.with_extension("meta.json"), r#"{"entrypoint":"print"}"#).unwrap();
    assert_eq!(adapter::find_session("commandcode", &w.root, since, "Work board task proj/1"), Some("own".into()), "headless helpers are not interactive task sessions");
}

#[test]
fn commandcode_background_helpers_keep_the_parent_open_until_they_return() {
    let w = world(); let f = w.base.join("helpers.jsonl");
    let helper = |running: bool| json!({ "type": "custom", "timestamp": "2026-10-16T12:00:01Z", "customType": "herdr-projects/helper", "data": { "id": "helper-1", "running": running } });
    write_rows(&f, &[helper(true)]); assert_eq!(sw::agents::helpers::running("commandcode", &f, 0), 1);
    write_rows(&f, &[helper(true), helper(false)]); assert_eq!(sw::agents::helpers::running("commandcode", &f, 0), 0);
    write_rows(&f, &[helper(true)]); assert_eq!(sw::agents::helpers::running("commandcode", &f, i64::MAX), 0, "helpers from before the new process started are gone");
}

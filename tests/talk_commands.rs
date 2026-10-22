//! A command typed to a program, a stop and a compaction, as a chat shows them: the command as typed, then in plain
//! words what it did, for every program; never the program's markup (`<command-name>`, `<local-command-stdout>` …).
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use sw::agents::talk::talk_of;
use sw::json::*;

fn jsonl(file: &Path, rows: &[Value]) -> PathBuf { std::fs::create_dir_all(file.parent().unwrap()).unwrap(); std::fs::write(file, rows.iter().map(|r| r.to_string()).collect::<Vec<_>>().join("\n") + "\n").unwrap(); file.to_path_buf() }
fn ts(n: u32) -> String { format!("2026-10-13T12:00:{n:02}.000Z") }
fn words(kind: &str, file: &Path) -> Vec<String> { talk_of(kind, Some(file), "", Path::new("/"), 50).unwrap_or_default().iter().map(|m| format!("{}: {}", if m.you { "you" } else { "them" }, m.text)).collect() }
fn user(n: u32, content: &str) -> Value { json!({ "type": "user", "timestamp": ts(n), "version": "2.1.296", "message": { "role": "user", "content": content } }) }
fn typed(n: u32, content: &str) -> Value { json!({ "type": "user", "timestamp": ts(n), "version": "2.1.296", "promptSource": "typed", "origin": { "kind": "human" }, "message": { "role": "user", "content": content } }) }
fn said(n: u32, text: &str) -> Value { json!({ "type": "assistant", "timestamp": ts(n), "message": { "id": format!("m{n}"), "role": "assistant", "content": [{ "type": "text", "text": text }] } }) }
fn command(n: u32, name: &str, args: &str) -> Value { user(n, &format!("<command-name>/{name}</command-name>\n            <command-message>{name}</command-message>\n            <command-args>{args}</command-args>")) }
fn printed(n: u32, out: &str) -> Value { json!({ "type": "system", "subtype": "local_command", "timestamp": ts(n), "content": format!("<local-command-stdout>{out}</local-command-stdout>"), "level": "info", "isMeta": false }) }
const CAVEAT: &str = "<local-command-caveat>The command below was run directly in Claude Code, not sent to you as a request.</local-command-caveat>";
fn caveat(n: u32) -> Value { json!({ "type": "user", "isMeta": true, "timestamp": ts(n), "message": { "role": "user", "content": CAVEAT } }) }

#[test]
fn a_command_reads_as_the_command_and_what_it_did_never_as_markup() {
    let dir = std::env::temp_dir().join(format!("sw-talk-commands-{}", std::process::id()));
    // /clear: the fresh session's file holds only the command, as Claude Code writes it
    let clear = jsonl(&dir.join("clear.jsonl"), &[caveat(1), command(1, "clear", ""), printed(1, "")]);
    assert_eq!(words("claude", &clear), vec!["you: /clear", "them: Chat reset: a fresh session started."]);
    // in the middle of a conversation, in a file whose prompts carry their mark; an older version wrote what it printed as a prompt
    let rows = [typed(2, "Fix the greeting"), said(3, "Fixed."),
        caveat(4), command(4, "model", "opus"), printed(4, "Set model to \u{1b}[1mOpus 4.1\u{1b}[22m"),
        command(5, "effort", "high"), user(5, "<local-command-stdout>Set effort level to high</local-command-stdout>"),
        command(6, "usage", ""), printed(6, "(no content)"),
        command(7, "compact", ""), json!({ "type": "system", "subtype": "compact_boundary", "timestamp": ts(7), "content": "Conversation compacted", "compactMetadata": { "trigger": "manual" } }), printed(7, "Compacted"),
        typed(8, "and now?"), said(9, "Working on it"), user(10, "[Request interrupted by user for tool use]"),
        json!({ "type": "system", "subtype": "compact_boundary", "timestamp": ts(11), "content": "Conversation compacted", "compactMetadata": { "trigger": "auto" } }),
        user(12, "<bash-input>ls</bash-input>"), user(12, "<bash-stdout>f.txt</bash-stdout><bash-stderr></bash-stderr>"),
        typed(13, "what does <command-name>/clear</command-name> mean?"), said(14, "It resets the chat.")];
    let all = words("claude", &jsonl(&dir.join("mid.jsonl"), &rows));
    assert_eq!(all, vec!["you: Fix the greeting", "them: Fixed.",
        "you: /model opus", "them: Model switched to Opus 4.1.",
        "you: /effort high", "them: Set effort level to high",
        "you: /usage", "them: /usage: done.",
        "you: /compact", "them: Conversation compacted: it goes on from a summary of what was said.",
        "you: and now?", "them: Working on it", "them: Stopped: interrupted before it finished.",
        "them: Conversation compacted to make room: it goes on from a summary of what was said.",
        "you: ! ls",
        "you: what does <command-name>/clear</command-name> mean?", "them: It resets the chat."]);
    // what the program printed for none of yours is its own, and a command of another origin is not yours
    let own = [typed(1, "hi"), said(2, "Hello."), user(3, "<local-command-stdout>ok</local-command-stdout>"),
        json!({ "type": "user", "timestamp": ts(4), "origin": { "kind": "task-notification" }, "message": { "role": "user", "content": "<command-name>/x</command-name>" } })];
    assert_eq!(words("claude", &jsonl(&dir.join("own.jsonl"), &own)), vec!["you: hi", "them: Hello."]);
    // Codex: a stop and a compaction
    let codex = jsonl(&dir.join("codex.jsonl"), &[
        json!({ "timestamp": ts(20), "type": "event_msg", "payload": { "type": "user_message", "message": "Review it" } }),
        json!({ "timestamp": ts(21), "type": "event_msg", "payload": { "type": "turn_aborted", "reason": "interrupted" } }),
        json!({ "timestamp": ts(22), "type": "compacted", "payload": { "message": "summary" } }),
        json!({ "timestamp": ts(23), "type": "event_msg", "payload": { "type": "turn_aborted", "reason": "replaced" } })]);
    assert_eq!(words("codex", &codex), vec!["you: Review it", "them: Stopped: interrupted before it finished.", "them: Conversation compacted: it goes on from a summary of what was said."]);
    // pi: the model and thinking level it starts with are no change; a later one is, as are a stop and a compaction
    let pi = jsonl(&dir.join("pi.jsonl"), &[json!({ "type": "session", "id": "s" }),
        json!({ "type": "model_change", "timestamp": ts(30), "modelId": "gpt-a" }), json!({ "type": "thinking_level_change", "timestamp": ts(30), "thinkingLevel": "high" }),
        json!({ "type": "message", "timestamp": ts(31), "message": { "role": "user", "content": [{ "type": "text", "text": "Say hello" }] } }),
        json!({ "type": "message", "timestamp": ts(32), "message": { "role": "assistant", "content": [], "stopReason": "aborted" } }),
        json!({ "type": "model_change", "timestamp": ts(33), "modelId": "gpt-b" }), json!({ "type": "thinking_level_change", "timestamp": ts(33), "thinkingLevel": "low" }),
        json!({ "type": "compaction", "timestamp": ts(34), "summary": "…" })]);
    assert_eq!(words("pi", &pi), vec!["you: Say hello", "them: Stopped: interrupted before it finished.", "them: Model switched to gpt-b.", "them: Thinking level set to low.", "them: Conversation compacted: it goes on from a summary of what was said."]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn after_clear_a_chat_says_the_chat_was_reset_and_shows_no_markup() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let t = task(&id);
    let file = PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap()).join("projects").join("x").join(format!("{}.jsonl", t.s("session")));
    jsonl(&file, &[caveat(1), command(1, "clear", ""), printed(1, "")]);
    let mut s = open(&w.root, 34); s.s.side = false;
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    let boxed = crate::board_chat::chat(&mut s, 120).boxed;
    assert_eq!(boxed[..1], ["Chat reset: a fresh session started.".to_string()], "{boxed:?}");
    let mut all = boxed.clone();
    for _ in 0..3 { s.press(&["alt+k"]); all.extend(crate::board_chat::chat(&mut s, 120).boxed); }
    assert!(all.iter().any(|l| l == "/clear"), "{all:?}");
    assert!(!all.iter().chain(s.screen(120).iter()).any(|l| l.contains("<command-") || l.contains("local-command")), "{all:?}");
}

#[test]
fn the_coordinators_box_after_clear_says_the_chat_was_reset_also_while_your_command_is_its_newest_words() {
    let dir = std::env::temp_dir().join(format!("sw-talk-commands-coord-{}", std::process::id()));
    let talk = talk_of("claude", Some(&jsonl(&dir.join("clear.jsonl"), &[caveat(1), command(1, "clear", ""), printed(1, "")])), "", Path::new("/"), 12).unwrap();
    for sent in ["", "/clear"] {
        let v = sw::coordinator::look::Look { state: "idle".into(), status: Some("idle".into()), readable: true, talk: talk.clone(), sent: sent.into(), ..Default::default() };
        let parts = sw::board::view::talk::coord_talk(&v, "").0;
        assert_eq!(parts.iter().map(|p| format!("{}: {}", p.by, p.text)).collect::<Vec<_>>(), vec!["You: /clear", "Coordinator: Chat reset: a fresh session started."], "sent {sent:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

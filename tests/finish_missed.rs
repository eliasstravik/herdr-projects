//! An agent that ended with its closing summary while the board believed a helper of its was still at work: seen on
//! HSW-218 (2026-10-15), where the helper's answer came back during the turn, written only as a queued command, and the
//! board held the task at "Finishing" through every turn after, a reply's too. The helper is seen back that way, and by
//! its own saved conversation; and `finish` takes a task stuck like that to the check and the review.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use sw::json::*;

fn work(ends: &str) -> Value { json!({ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1.", "ends": ends, "hook": true }) }
fn reviewer_starts(w: &World) -> usize { calls(w).iter().filter(|c| regex::Regex::new(r"^agent start \S+-r ").unwrap().is_match(c)).count() }
fn write(file: &Path, lines: &[Value]) { std::fs::create_dir_all(file.parent().unwrap()).unwrap(); std::fs::write(file, lines.iter().map(|l| format!("{l}\n")).collect::<String>()).unwrap(); }
fn launched(at: &str, id: &str) -> Value { json!({ "timestamp": at, "type": "user", "message": { "role": "user", "content": [{ "type": "tool_result", "content": format!("Async agent launched successfully.\nagentId: {id} (internal ID)") }] } }) }

#[test]
fn a_helper_back_during_the_agents_turn_does_not_hold_its_finish() {
    let w = cut_world("30", json!({ "worker.agent": "claude" }));
    w.script(json!({ "worker": [work("claude-helper-back-mid-turn-made")] }));
    let id = w.add("Old installs");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(reviewer_starts(&w), 1);
    assert!(t.s("said").starts_with("Done. Old installs"), "{}", t.s("said"));
}

#[test]
fn a_helper_whose_own_conversation_ends_in_its_reply_is_back() {
    let _w = world();
    let started = sw::clock::now();
    let (launch, end) = (sw::clock::iso(started + 1000), sw::clock::iso(started + 5000));
    let file = PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap()).join("projects").join("x").join("s1.jsonl");
    let own = file.with_extension("").join("subagents").join("agent-aback01.jsonl");
    write(&file, &[launched(&launch, "aback01")]);
    assert_eq!(sw::agents::helpers::running("claude", &file, started), 1);
    // its own conversation, still at work: a tool call and no reply yet
    write(&own, &[json!({ "timestamp": launch, "type": "user", "message": { "role": "user", "content": "Map the old install." } }),
        json!({ "timestamp": launch, "type": "assistant", "message": { "role": "assistant", "content": [{ "type": "tool_use", "name": "Bash" }], "stop_reason": "tool_use" } })]);
    assert_eq!(sw::agents::helpers::running("claude", &file, started), 1);
    let reply = json!({ "timestamp": end, "type": "assistant", "message": { "role": "assistant", "content": [{ "type": "text", "text": "Mapped." }], "stop_reason": "end_turn" } });
    let mut lines = vec![json!({ "timestamp": launch, "type": "user", "message": { "role": "user", "content": "Map the old install." } }), reply.clone(), json!({ "type": "api-request", "timestamp": end })];
    write(&own, &lines);
    assert_eq!(sw::agents::helpers::running("claude", &file, started), 0);
    // sent on again after that reply (SendMessage): at work until a newer reply
    let later = sw::clock::iso(started + 9000);
    write(&file, &[launched(&launch, "aback01"), json!({ "timestamp": later, "type": "assistant", "message": { "role": "assistant", "content": [{ "type": "tool_use", "name": "SendMessage", "input": { "to": "aback01", "message": "Also the config." } }] } })]);
    assert_eq!(sw::agents::helpers::running("claude", &file, started), 1);
    lines.push(json!({ "timestamp": sw::clock::iso(started + 12000), "type": "assistant", "message": { "role": "assistant", "content": [{ "type": "text", "text": "And the config." }], "stop_reason": "end_turn" } }));
    write(&own, &lines);
    assert_eq!(sw::agents::helpers::running("claude", &file, started), 0);
}

/// A helper the board never sees come back: the turn's end, and the reply's after a tell, are waited past. Its pane.
fn stuck(w: &World) -> (String, String) {
    w.script(json!({ "worker": [work("claude-helper-running-made"), { "ends": "claude-finished-made", "hook": true }] }));
    let id = w.add("Stuck at finishing");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working");
    sw::actions::talk::reply(&id, "Reply with your closing summary.", "you").unwrap();
    let pane = task(&id).at("link").t("pane");
    set_status(w, &pane, "working"); set_status(w, &pane, "idle"); // as herdr saw it: at work, then at rest
    (id, pane)
}

#[test]
fn finish_takes_a_task_stuck_at_finishing_to_the_check_and_the_review() {
    let w = cut_world("30", json!({ "worker.agent": "claude" }));
    let (id, pane) = stuck(&w);
    let t = settle(&id);
    // HSW-218's task as it was found: at work for the board, the reply's turn open, herdr's count handled, the agent idle
    assert_eq!((t.s("stage"), t.b("turnOpen"), t.s("turnBy"), t.at("last").s("status")), ("working", true, "reply", "idle"), "{}", t.s("msg"));
    assert_eq!(t.at("last").at("comp"), t.at("handled").at("comp"));
    assert_eq!(row(&id).s("note"), "Finishing");
    assert_eq!(reviewer_starts(&w), 0);
    // While herdr sees its agent at work, finish is refused; at rest, it runs the finish line on what the agent said last.
    set_status(&w, &pane, "working");
    assert!(sw::actions::finish_line::finish_anyway(&id).unwrap_err().msg.contains("still at work"));
    set_status(&w, &pane, "idle");
    sw::actions::finish_line::finish_anyway(&id).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(reviewer_starts(&w), 1);
    assert!(!t.b("turnOpen") && !t.b("turnBy"));
    assert!(sw::actions::finish_line::finish_anyway(&id).unwrap_err().msg.contains("did not stop without a summary"));
}

#[test]
fn the_finish_key_is_offered_on_a_task_stuck_at_finishing_and_finishes_it() {
    let w = cut_world("30", json!({ "worker.agent": "claude" }));
    let (id, _) = stuck(&w);
    settle(&id);
    let mut s = common::board::open(&w.root, 34);
    s.s.sel = id.clone(); s.load();
    let text = s.screen(140).join("\n");
    assert!(text.contains("Finishing") && text.contains(&format!("{} finish with what its agent said last", sw::board::keys::key_name("yes"))), "{text}");
    s.press(&["ctrl+l"]);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(reviewer_starts(&w), 1);
}

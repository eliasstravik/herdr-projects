//! A task on one of herdr's other programs, whose conversation the board does not read (issue #138: agy as the
//! worker and the reviewer): herdr's state ends the turn and the closing note says what was done; the reviewer's verdict
//! file ends the review; a reviewer that stops without one ends its round as herdr sees it stop, not after the
//! review's whole time; a program started again reopens the conversation herdr's integration named.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::json::*;

fn work() -> Value { json!({ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }) }
fn pass() -> Value { json!({ "verdict": { "meets": true, "findings": [], "text": "Looks right." } }) }
fn starts_of(w: &World, kind: &str) -> Vec<String> { calls(w).into_iter().filter(|c| c.starts_with("agent start") && c.contains(&format!("--kind {kind} "))).collect() }

#[test]
fn agy_works_and_reviews_a_task_from_start_to_approval() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "agy/gemini-3-pro", "worker.effort": "high", "reviewer.agent": "agy", "review.when": "always" }), ..Default::default() });
    w.script(json!({ "worker": [work()], "reviewer": [pass()] }));
    let id = w.add("On agy");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(t.s("said"), "Did v1.");
    let starts = starts_of(&w, "agy");
    assert_eq!(starts.len(), 2, "{starts:?}"); // the worker and its reviewer
    assert!(starts[0].ends_with("-- --dangerously-skip-permissions --model gemini-3-pro --effort high"), "{}", starts[0]);
    assert!(starts[1].ends_with("-- --dangerously-skip-permissions"), "{}", starts[1]);
    // It cannot be told its rules at start: they ride in its first prompt, with the closing note.
    let first = w.state().arr("panes").iter().filter_map(|p| p.at("agent").arr("prompts").first().map(text)).next().unwrap_or_default();
    assert!(first.contains("closing-note.md"), "{first}");
}

#[test]
fn a_reviewer_the_board_cannot_read_that_stops_without_a_verdict_ends_its_round_when_herdr_sees_it_stop() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "claude", "reviewer.agent": "qwen", "review.when": "always" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_ENDED_MS", "0"); std::env::set_var("HERDR_PROJECTS_QUIET_MS", "0"); }
    w.script(json!({ "worker": [work()], "reviewer": [{}, {}] }));
    let id = w.add("No verdict");
    delegate(&id);
    let t = settle(&id);
    let reviews = t.arr("reviews").to_vec();
    let said = format!("{} {} {:?}", t.s("msg"), t.at("failure"), reviews);
    assert!(said.contains("the reviewer finished without a verdict"), "{said}");
    assert!(!said.contains("timed out"), "{said}");
    unsafe { std::env::remove_var("HERDR_PROJECTS_ENDED_MS"); std::env::remove_var("HERDR_PROJECTS_QUIET_MS"); }
}

#[test]
fn a_program_started_again_reopens_the_conversation_herdrs_integration_named() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "kimi", "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1.", "session": "kimi-s-1" }, { "note": "Again." }] }));
    let id = w.add("On kimi");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("session"), "kimi-s-1", "{} {}", t.s("msg"), t.at("failure"));
    // Its program ends, and the task is told something: it starts again in the same conversation.
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.at("agent").s("kind") == "kimi" { p["agent"] = Value::Null; } } });
    sw::actions::talk::tell(&id, "One more thing.", false).unwrap_or_else(|e| panic!("{}", e.msg));
    settle(&id);
    let starts = starts_of(&w, "kimi");
    assert_eq!(starts.len(), 2, "{starts:?}");
    assert!(starts[1].contains("-- --session kimi-s-1 --auto"), "{}", starts[1]);
}

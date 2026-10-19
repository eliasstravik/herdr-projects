//! The working agent's turn cut off by the model's provider: told to carry on by itself after a wait (said on the row,
//! the task panel and herdr's sidebar), started again when its program exited, given up on after the last try with
//! the real error, and an error that does not pass held for you at once.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::housekeeping::{housekeeping, Force};
use sw::json::*;
use sw::store::retries::waiting_of;
use sw::store::tasks::patch;

fn work() -> Value { json!({ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }) }
fn pass() -> Value { json!({ "verdict": { "meets": true, "findings": [], "text": "Looks right." } }) }
/// The wait of a working agent is over: time is moved on, not slept through.
fn elapse(id: &str) { patch(id, |t| { let at = sw::clock::now() - 1; t.obj_mut("waiting").put("until", at); }).unwrap(); }
fn sent(w: &World, id: &str) -> Vec<String> { prompts(w, task(id).at("link").s("pane")) }
fn tries(t: &Obj) -> i64 { waiting_of(t).map(|w| w.i("tries")).unwrap_or(-1) }

#[test]
fn an_interactive_cli_quota_error_read_from_its_idle_pane_hands_over_to_the_worker_fallback() {
    let w = cut_world("60000", json!({ "worker.agent": "gemini", "worker.fallback": "codex" }));
    w.script(json!({ "worker": [
        { "screen": "API Error: usage limit reached. Your quota resets at 9:00 PM.", "status": "done" },
        work()
    ], "reviewer": [pass()] }));
    let id = w.add("Quota on an interactive CLI");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(t.s("on"), "codex");
    assert_eq!(t.arr("handovers").last().unwrap().s("why"), "out of allowance until 21:00");
    assert_eq!(starts(&w), ["gemini", "codex"]);
}

/// The working agent's own turn-end report, as the hook the board installed runs it (pi: the conversation file on stdin).
fn report_end(id: &str) {
    let t = task(id);
    let mut dir = sw::store::files::data_dir().join("tasks"); for part in id.split('/') { dir.push(part); }
    let tokens: Value = serde_json::from_str(&read(&dir.join("tokens.json"))).unwrap_or(Value::Null);
    sw::turn::turn_ended(id, "agent", &tokens.t("agent"), 0, &[json!({ "transcript_path": t.s("transcript") }).to_string()]);
}
fn keep(w: &World) { housekeeping(&w.socket, Force::Yes); }

#[test]
fn a_turn_cut_off_by_a_passing_error_is_told_to_carry_on_by_itself_not_held_as_failed() {
    let w = cut_world("120000,240000", json!({ "worker.agent": "pi" }));
    w.script(json!({ "worker": [{ "ends": "pi-socket-closed" }, work()], "reviewer": [pass()] }));
    let id = w.add("Dropped");
    delegate(&id);
    let t = settle(&id);
    // under the wait the failure is saved, so a reply or the retry key works; the row shows work going on, and why
    assert_eq!((t.s("stage"), t.at("failure").s("title")), ("failed", "Agent cut off"));
    assert!(t.at("failure").s("detail").contains("socket connection was closed"));
    assert_eq!(tries(&t), 1);
    let r = row(&id);
    assert_eq!((r.s("state"), r.s("note")), ("working", "Agent's connection dropped, trying again in 2m"));
    let e = sw::flow::sidebar::entry_now(&t, sw::clock::now());
    assert_eq!((e.place.as_str(), e.line.as_str()), ("agent · working", "")); // herdr's sidebar says where the row stands, not its sentence
    // on the board: under "working", the row and the task panel say why and when, and nothing asks for you
    let mut b = common::board::open(&w.root, 40);
    let screen = b.screen(130).join("\n");
    assert!(regex::Regex::new(r"\n ▾ working\s[^\n]*\n[ ▌]. PRJ-1 Dropped [^\n]*\n[ ▌] {8}Agent's connection dropped, trying again in 2m").unwrap().is_match(&screen), "{screen}");
    assert!(!screen.contains("needs you") && !screen.contains("Agent cut off"), "{screen}");
    assert!(regex::Regex::new(r"Cut off ─+[\s\S]*Trying again by itself[\s\S]*Cut off by[\s\S]*The socket connection was closed").unwrap().is_match(&screen), "{screen}");
    keep(&w); // too early: nothing is sent
    assert_eq!(sent(&w, &id).len(), 1);
    elapse(&id);
    // herdr does not answer the lookup just then: the wait stays, and the next pass sends
    w.patch(|s| s["agentGetFails"] = json!(true));
    assert!(!sw::flow::send::resume_cut(&id));
    assert_eq!((tries(&task(&id)), row(&id).t("state")), (1, "working".into()));
    w.patch(|s| s["agentGetFails"] = json!(false));
    keep(&w);
    let p = sent(&w, &id);
    assert_eq!(p.len(), 2);
    assert!(regex::Regex::new(r"(?s)cut off by an error at the model's provider.*socket connection was closed.*Continue the task where you left off\.").unwrap().is_match(&p[1]), "{}", p[1]);
    assert_eq!(task(&id).at("expectedWake").s("reason"), "retry");
    keep(&w); // and only once
    assert_eq!(sent(&w, &id).len(), 2);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert!(!t.b("failure") && !t.b("waiting"));
}

#[test]
fn after_the_last_try_the_board_gives_up_with_the_real_error_and_other_errors_are_held_at_once() {
    let w = cut_world("60000", json!({ "worker.agent": "pi" }));
    w.script(json!({ "worker": [{ "ends": "pi-overloaded" }, { "ends": "pi-overloaded" }] }));
    let id = w.add("Overloaded");
    delegate(&id);
    settle(&id);
    assert_eq!(row(&id).s("state"), "working");
    elapse(&id);
    keep(&w);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed"); assert!(waiting_of(&t).is_none()); assert_eq!(row(&id).s("state"), "error");
    assert_eq!(t.s("msg"), "Agent stopped: the agent's provider is overloaded, still after 2 tries over 1m. Its error: Our servers are currently overloaded. Please try again later.");
    assert_eq!(sent(&w, &id).len(), 2);
    // a reply is a new start: the count begins again
    w.script(json!({ "worker": [{ "ends": "pi-overloaded" }] }));
    sw::actions::talk::tell(&id, "try once more", false).unwrap();
    let t = settle(&id);
    assert_eq!(tries(&t), 1);
    // you reply while it waits: the wait is over, and the board sends nothing of its own on top
    w.script(json!({ "worker": [work()] }));
    let until = task(&id).at("waiting").at("until").clone();
    sw::actions::talk::tell(&id, "go on", false).unwrap();
    assert!(!task(&id).b("waiting"));
    // even a wait left behind sends nothing: the task has moved on
    patch(&id, |t| { t.put("waiting", json!({ "who": "agent", "reason": "provider is overloaded", "error": "x", "until": sw::clock::now() - 1, "tries": 1, "for": until })); }).unwrap();
    keep(&w);
    assert_eq!(sent(&w, &id).len(), 4);
    // an error that does not pass: held for you at once
    w.script(json!({ "worker": [{ "ends": "pi-unknown-provider" }] }));
    let id = w.add("No such provider");
    delegate(&id);
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.at("failure").s("title")), ("failed", "Agent stopped"));
    assert!(t.s("msg").starts_with("Agent stopped: Unknown provider"), "{}", t.s("msg"));
    assert!(waiting_of(&t).is_none()); assert_eq!(row(&id).s("state"), "error");
}

#[test]
fn a_program_that_exited_with_the_error_is_started_again_and_told_to_carry_on_and_the_tries_still_count() {
    let w = cut_world("60000", json!({ "worker.agent": "pi" }));
    w.script(json!({ "worker": [{ "ends": "pi-socket-closed", "hook": true, "exit": true }, { "ends": "pi-socket-closed", "hook": true, "exit": true }] }));
    let id = w.add("Crashed");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(tries(&t), 1); assert!(agent_of(&w, t.at("link").s("pane")).is_null());
    elapse(&id);
    assert!(sw::flow::send::resume_cut(&id));
    settle(&id);
    // The shared fake finds an agent's task by the first prompt it was given, and a program started again was given
    // only the words to carry on: its own turn-end report (which knows its task from the board's token) is run here.
    report_end(&id);
    let t = settle(&id);
    // started again in the same conversation, with the words; it is cut off again, and that was the last try
    let s: Vec<String> = calls(&w).into_iter().filter(|c| c.starts_with("agent start") && !c.contains("-r ")).collect();
    assert_eq!(s.len(), 2); assert!(s[1].contains(&format!("--session-id {}", t.s("session"))), "{}", s[1]);
    assert_eq!(t.s("stage"), "failed"); assert!(waiting_of(&t).is_none());
    assert!(t.s("msg").starts_with("Agent stopped: the agent's connection dropped, still after 2 tries over 1m."), "{}", t.s("msg"));
    // once more, and this time it finishes: the count starts again
    w.script(json!({ "worker": [work()], "reviewer": [pass()] }));
    // (the same: the started-again program's note is put where the board reads it, stamped after the retry arms the turn)
    let note = sw::flow::task::note_file(&t);
    std::fs::write(&note, "Did v1.").unwrap();
    std::fs::File::options().write(true).open(&note).unwrap().set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5)).unwrap();
    sw::actions::finish_line::retry(&id).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(!t.b("cutRetry"));
}

#[test]
fn no_report_no_note_and_herdr_cannot_tell_if_it_stopped_the_error_at_the_end_of_its_file_is_enough() {
    let w = cut_world("60000", json!({ "worker.agent": "codex" }));
    let mut finished = work(); finished["status"] = json!("unknown");
    w.script(json!({ "worker": [{ "ends": "codex-at-capacity", "status": "unknown" }, finished], "reviewer": [pass()] }));
    let id = w.add("Nobody says it stopped");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed"); assert_eq!(waiting_of(&t).unwrap().s("reason"), "model is at capacity");
    assert!(regex::Regex::new(r"^Agent's model is at capacity, trying again in (1m|\d+s)$").unwrap().is_match(row(&id).s("note")), "{}", row(&id).s("note"));
    elapse(&id);
    keep(&w);
    assert_eq!(sent(&w, &id).len(), 2);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
}

#[test]
fn with_its_own_turn_end_report_cut_off_told_to_carry_on_and_the_finished_turn_goes_on_to_review() {
    let w = cut_world("60000", json!({}));
    let mut finished = work(); finished["note"] = Value::Null; finished["ends"] = json!("claude-finished-made"); finished["hook"] = json!(true);
    w.script(json!({ "worker": [{ "ends": "claude-overloaded-made", "hook": true }, finished], "reviewer": [pass()] }));
    let id = w.add("Reported cut");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed"); assert_eq!(waiting_of(&t).unwrap().s("reason"), "provider is overloaded");
    elapse(&id);
    keep(&w);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert_eq!(t.s("summary"), "Carried on and finished it.");
}

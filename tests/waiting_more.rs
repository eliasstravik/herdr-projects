//! More of an agent that waits for the human: a research task's rules, interim report and note that end nothing, a
//! review stopped by the agent saying it waits, and when a saved conversation's last turn began, for every program.
//! Its own agent or reviewer cannot mark its task done, failed or idle (HP-133): its turn's end is the finish.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use sw::json::*;

fn line(file: &Path, o: Value) { use std::io::Write; let mut f = std::fs::OpenOptions::new().create(true).append(true).open(file).unwrap(); writeln!(f, "{o}").unwrap(); }
fn iso(at: i64) -> String { sw::clock::iso(at) }
fn prompt(file: &Path, at: i64, text: &str) { line(file, json!({ "type": "user", "uuid": format!("u{at}"), "timestamp": iso(at), "message": { "role": "user", "content": text } })); }
fn says(file: &Path, at: i64, text: &str) { line(file, json!({ "type": "assistant", "uuid": format!("a{at}"), "timestamp": iso(at), "message": { "id": format!("msg_{at}"), "role": "assistant", "stop_reason": "end_turn", "content": [{ "type": "text", "text": text }] } })); }
fn pane(w: &World, id: &str, status: &str, complete: bool) {
    let p = task(id).at("link").t("pane");
    w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); for x in s["panes"].as_array_mut().unwrap() { if x.s("pane_id") == p { x["agent"]["status"] = json!(status); x["agent"]["seq"] = json!(seq); if complete { x["agent"]["completion"] = json!(seq); } } } });
    on_pane(w, &p);
}
fn reported(id: &str, file: &Path) { sw::turn::turn_ended(id, "agent", "tok", 0, &[json!({ "session_id": "s1", "transcript_path": file.display().to_string() }).to_string()]); }
fn ask(id: &str, msg: &str) { sw::actions::filing::report(id, sw::actions::filing::Report { state: "blocked".into(), kind: "question".into(), msg: msg.into(), own: "agent", ..Default::default() }).unwrap(); }
fn say(id: &str, state: &str, own: &'static str) -> Result<sw::store::tasks::Task, String> { sw::actions::filing::report(id, sw::actions::filing::Report { state: state.into(), msg: "all done".into(), own, ..Default::default() }).map_err(|e| e.msg) }
fn reviewers(w: &World) -> usize { let r = regex::Regex::new(r"-r(-\d+)?$").unwrap(); w.state().arr("panes").iter().filter(|p| r.is_match(p.at("agent").s("name"))).count() }
fn at_work(w: &World, research: bool) -> (String, PathBuf) {
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = sw::actions::filing::add(&w.root, "Interview the human", sw::actions::filing::AddOpts { research: Some(research), ..Default::default() }).unwrap().t("id");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "working");
    sw::store::marks::set_token(&id, "agent", "tok", 0).unwrap();
    let file = w.base.join("conversation.jsonl");
    prompt(&file, sw::clock::now() - 60000, "Go on.");
    (id, file)
}

#[test]
fn research_only_the_rules_say_how_to_wait_and_how_to_finish_an_interim_report_and_a_note_then_a_question_end_nothing() {
    let w = world();
    let (id, file) = at_work(&w, true);
    let said = sw::delegate::rules::rules(&task(&id));
    for want in ["report proj/1 --state blocked", "only way to say that you are waiting, and it holds for one stop: run it every time you stop for the human's answer",
        "A stop without it means you are finished, whatever your last message, your report or your closing note says", "Never run it to put off the review of finished work",
        "Saving the report finishes nothing: you may save it early"] { assert!(said.contains(want), "{want}: {said}"); }
    let dir = sw::research::report_folder(&task(&id));
    std::fs::create_dir_all(&dir).unwrap(); std::fs::write(dir.join("report.md"), "# Interim\n\nAnswered 1 to 12.\n").unwrap();
    std::fs::write(sw::store::files::dir_of(&id).join("closing-note.md"), "NOT FINISHED: waiting for the human.\n").unwrap();
    ask(&id, "Question 13 of 15");
    says(&file, task(&id).at("blocked").i("at") + 1, "Question 13?");
    pane(&w, &id, "done", true); reported(&id, &file);
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("kind"), t.s("msg"), t.arr("reviews").len()), ("asks", "question", "Question 13 of 15", 0), "{}", t.at("failure"));
    assert_eq!(row(&id).s("state"), "blocked");
    assert_eq!(t.at("report").s("file"), "report.md");
    assert_eq!(reviewers(&w), 0);
    // Finished, with the same report: now it is reviewed and done.
    w.script(json!({ "worker": [{ "status": "working" }] }));
    sw::actions::talk::tell(&id, "1", false).unwrap();
    let at = sw::clock::now();
    prompt(&file, at, "1"); says(&file, at + 1, "The plan is in the report.");
    reported(&id, &file); pane(&w, &id, "done", true);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(t.arr("reviews").len(), 1);
}

/// The review runner waits for its verdict in the test's own thread: the agent's "I wait" comes from another thread,
/// the moment the review has its reviewer, as the agent's own command would.
#[test]
fn a_review_that_is_running_when_the_agent_says_it_waits_is_stopped_and_sends_nothing_back() {
    let w = world();
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }], "reviewer": [{ "status": "working" }] }));
    let id = w.add("Work");
    delegate(&id);
    let asker = { let id = id.clone(); std::thread::spawn(move || {
        let end = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while std::time::Instant::now() < end { if let Some(t) = sw::store::tasks::get(&id) { if t.s("stage") == "reviewing" && t.at("link").at("reviewer").b("pane") { ask(&id, "Should the old file go?"); return true; } } std::thread::sleep(std::time::Duration::from_millis(5)); }
        false
    }) };
    settle(&id);
    assert!(asker.join().unwrap(), "the review never started");
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("kind"), t.s("msg")), ("asks", "question", "Should the old file go?"), "{}", t.at("failure"));
    assert!(t.at("link").at("reviewer").is_null());
    assert_eq!((t.arr("reviews").len(), t.i("reviewRounds")), (0, 0));
    assert_eq!(prompts(&w, t.at("link").s("pane")).len(), 1); // no failed round went to the agent
}

#[test]
fn a_saved_conversation_says_when_its_last_turn_began_for_every_agent_program() {
    let dir = std::env::temp_dir().join(format!("sw-began-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ts = |n: i64| iso(1000000 + n);
    let write = |name: &str, rows: Vec<Value>| -> PathBuf { let f = dir.join(name); std::fs::write(&f, rows.iter().map(|r| r.to_string()).collect::<Vec<_>>().join("\n") + "\n").unwrap(); f };
    use sw::agents::read_turn::{read_claude, read_codex, read_pi, TAIL};
    let claude = read_claude(&write("c.jsonl", vec![
        json!({ "type": "user", "timestamp": ts(1), "message": { "role": "user", "content": "first" } }),
        json!({ "type": "user", "timestamp": ts(2), "message": { "role": "user", "content": [{ "type": "text", "text": "second" }] } }),
        json!({ "type": "assistant", "timestamp": ts(3), "message": { "id": "m1", "role": "assistant", "stop_reason": "tool_use", "content": [{ "type": "tool_use", "id": "t1", "name": "Bash", "input": {} }] } }),
        json!({ "type": "user", "timestamp": ts(4), "message": { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": "t1", "content": "ok" }] } }), // a tool's result is no prompt
        json!({ "type": "user", "timestamp": ts(5), "isMeta": true, "message": { "role": "user", "content": "a note of the program's own" } }),
        json!({ "type": "assistant", "timestamp": ts(6), "message": { "id": "m2", "role": "assistant", "stop_reason": "end_turn", "content": [{ "type": "text", "text": "Done." }] } }),
    ]), TAIL).unwrap();
    assert_eq!((claude.finished, claude.began, claude.pos), (true, 1000002, 1000006));
    let codex = read_codex(&write("x.jsonl", vec![
        json!({ "timestamp": ts(1), "type": "event_msg", "payload": { "type": "task_started" } }),
        json!({ "timestamp": ts(2), "type": "response_item", "payload": { "type": "message", "role": "user", "content": [{ "type": "input_text", "text": "go" }] } }),
        json!({ "timestamp": ts(3), "type": "response_item", "payload": { "type": "function_call", "name": "shell" } }),
        json!({ "timestamp": ts(4), "type": "event_msg", "payload": { "type": "task_complete", "turn_id": "t1", "last_agent_message": "Done." } }),
    ]), TAIL).unwrap();
    assert_eq!((codex.finished, codex.began), (true, 1000002));
    let pi = read_pi(&write("p.jsonl", vec![
        json!({ "type": "message", "id": "u1", "timestamp": ts(1), "message": { "role": "user", "content": [{ "type": "text", "text": "go" }] } }),
        json!({ "type": "message", "id": "r1", "timestamp": ts(2), "message": { "role": "toolResult", "content": [] } }),
        json!({ "type": "message", "id": "a1", "timestamp": ts(3), "message": { "role": "assistant", "stopReason": "stop", "content": [{ "type": "text", "text": "Done." }] } }),
    ]), TAIL).unwrap();
    assert_eq!((pi.finished, pi.began), (true, 1000001));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn its_own_agent_cannot_mark_its_task_done_failed_or_idle_and_nothing_changes() {
    let w = world();
    let (id, _) = at_work(&w, false);
    for state in ["done", "error", "idle"] {
        let e = say(&id, state, "agent").unwrap_err();
        assert!(e.contains(&format!("{id} is your own task, so you cannot mark it {state}")), "{e}");
        assert!(e.contains("Your task finishes when you end your turn; the board then checks, reviews and merges it"), "{e}");
        let t = task(&id);
        assert_eq!((t.s("stage"), t.s("msg")), ("working", ""), "{state}");
        assert!(t.at("tidy").is_null());
    }
}

#[test]
fn its_reviewer_cannot_mark_it_done_either() {
    let w = world();
    let (id, _) = at_work(&w, false);
    for state in ["done", "error", "idle"] {
        let e = say(&id, state, "reviewer").unwrap_err();
        assert!(e.contains(&format!("{id} is the task you review, so you cannot mark it {state}")), "{e}");
        assert!(e.contains("Your review finishes when you end your turn"), "{e}");
        assert_eq!(task(&id).s("stage"), "working", "{state}");
    }
}

#[test]
fn its_own_agent_still_says_it_waits_and_the_human_still_marks_it_done() {
    let w = world();
    let (id, _) = at_work(&w, false);
    let t = sw::actions::filing::report(&id, sw::actions::filing::Report { state: "blocked".into(), kind: "auth".into(), msg: "Need a GitHub login".into(), own: "agent", ..Default::default() }).unwrap();
    assert_eq!((t.s("stage"), t.s("kind"), t.s("msg")), ("asks", "auth", "Need a GitHub login"));
    assert!(t.b("selfBlocked"));
    // The human (or the Coordinator on their word): done, with nothing merged.
    let t = say(&id, "done", "").unwrap();
    assert_eq!(t.s("stage"), "done");
}

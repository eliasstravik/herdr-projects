//! An agent that stopped to wait for the human is waiting, not finished: no check, no review, no merge follow, in
//! whichever order the two signals of its turn's end arrive. And when a saved conversation's last turn began.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use sw::json::*;
use sw::store::tasks::{update, Change};

fn line(file: &Path, o: Value) { use std::io::Write; let mut f = std::fs::OpenOptions::new().create(true).append(true).open(file).unwrap(); writeln!(f, "{o}").unwrap(); }
fn iso(at: i64) -> String { sw::clock::iso(at) }
fn prompt(file: &Path, at: i64, text: &str) { line(file, json!({ "type": "user", "uuid": format!("u{at}"), "timestamp": iso(at), "message": { "role": "user", "content": text } })); }
fn says(file: &Path, at: i64, text: &str) { line(file, json!({ "type": "assistant", "uuid": format!("a{at}"), "timestamp": iso(at), "message": { "id": format!("msg_{at}"), "role": "assistant", "stop_reason": "end_turn", "content": [{ "type": "text", "text": text }] } })); }
/// herdr's reading of the pane, one of the two signals of a turn's end.
fn pane(w: &World, id: &str, status: &str, complete: bool) {
    let p = task(id).at("link").t("pane");
    w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); for x in s["panes"].as_array_mut().unwrap() { if x.s("pane_id") == p { x["agent"]["status"] = json!(status); x["agent"]["seq"] = json!(seq); if complete { x["agent"]["completion"] = json!(seq); } } } });
    on_pane(w, &p);
}
/// The agent's own turn-end report, the other.
fn reported(id: &str, file: &Path) { sw::turn::turn_ended(id, "agent", "tok", 0, &[json!({ "session_id": "s1", "transcript_path": file.display().to_string() }).to_string()]); }
fn ask(id: &str, msg: &str) { sw::actions::filing::report(id, sw::actions::filing::Report { state: "blocked".into(), kind: "question".into(), msg: msg.into(), own: "agent", ..Default::default() }).unwrap(); }
fn reviewers(w: &World) -> usize { let r = regex::Regex::new(r"-r(-\d+)?$").unwrap(); w.state().arr("panes").iter().filter(|p| r.is_match(p.at("agent").s("name"))).count() }
fn reviewed(id: &str) -> i64 { let t = task(id); t.arr("reviews").len() as i64 + t.i("reviewRound") }
fn blocked_at(id: &str) -> i64 { task(id).at("blocked").i("at") }
const Q2: &str = "Question 2 of 13: who is it for?";

/// A task whose agent is at work on its first prompt, which its conversation holds from a minute ago.
fn at_work(w: &World, research: bool, steps: usize) -> (String, PathBuf) {
    w.script(json!({ "worker": vec![json!({ "status": "working" }); steps] }));
    let id = sw::actions::filing::add(&w.root, "Interview the human", sw::actions::filing::AddOpts { research: Some(research), ..Default::default() }).unwrap().t("id");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    sw::store::marks::set_token(&id, "agent", "tok", 0).unwrap();
    let file = w.base.join("conversation.jsonl");
    prompt(&file, sw::clock::now() - 60000, "Go on.");
    (id, file)
}
fn waiting(id: &str, msg: &str) {
    let t = task(id);
    assert_eq!((t.s("stage"), t.s("kind"), t.s("msg"), reviewed(id)), ("asks", "question", msg, 0), "{}", t.at("failure"));
    assert_eq!(row(id).s("state"), "blocked");
}

#[test]
fn it_said_it_waits_then_stopped_and_herdr_saw_the_stop_first_no_review_nor_when_its_conversation_cannot_be_read() {
    let w = world();
    let (id, file) = at_work(&w, false, 2);
    ask(&id, Q2);
    says(&file, blocked_at(&id) + 1, "Question 2: who is it for?");
    // What happened on open-crm/1: the pane was seen at rest a moment before the agent's own report was read.
    pane(&w, &id, "done", true); reported(&id, &file);
    settle(&id);
    waiting(&id, Q2);
    assert_eq!(reviewers(&w), 0); // no reviewer was started
    assert_eq!(prompts(&w, task(&id).at("link").s("pane")).len(), 1); // and nothing went back to the agent
    // The next question, and this time nothing can be read of the turn: the task goes on waiting.
    sw::actions::talk::tell(&id, "1", false).unwrap();
    ask(&id, "Question 3 of 13");
    pane(&w, &id, "done", true); reported(&id, &w.base.join("no-such-file.jsonl"));
    settle(&id);
    waiting(&id, "Question 3 of 13");
    assert_eq!(reviewers(&w), 0);
}

#[test]
fn a_long_turn_its_prompt_far_back_in_the_saved_conversation_its_end_is_still_known_for_the_turn_it_waits_in() {
    let w = world();
    let (id, file) = at_work(&w, false, 1);
    let big = sw::clock::now() - 30000;
    line(&file, json!({ "type": "assistant", "uuid": "a-tool", "timestamp": iso(big), "message": { "id": "msg_tool", "role": "assistant", "stop_reason": "tool_use", "content": [{ "type": "tool_use", "id": "t1", "name": "Bash", "input": {} }] } }));
    line(&file, json!({ "type": "user", "uuid": "u-tool", "timestamp": iso(big + 1), "message": { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": "t1", "content": "x".repeat(900000) }] } }));
    std::fs::write(Path::new(task(&id).s("worktree")).join("draft.txt"), "half done\n").unwrap(); // unfinished work that a finish would commit and review
    sw::actions::filing::report(&id, sw::actions::filing::Report { state: "blocked".into(), kind: "permission".into(), msg: "Need your OK to delete the test data".into(), own: "agent", ..Default::default() }).unwrap();
    says(&file, blocked_at(&id) + 1, "Waiting for your OK.");
    assert_eq!(sw::agents::read_turn::read_claude(&file, sw::agents::read_turn::TAIL).unwrap().began, 0); // the end of the file holds no prompt
    pane(&w, &id, "done", true); reported(&id, &file);
    settle(&id);
    let t = task(&id);
    assert_eq!((t.s("stage"), t.s("kind"), t.s("msg"), reviewed(&id), reviewers(&w)), ("asks", "permission", "Need your OK to delete the test data", 0, 0));
}

#[test]
fn herdr_takes_the_waiting_agent_for_a_new_turn_a_flap_of_its_status_its_end_is_still_not_a_finish() {
    let w = world();
    let (id, file) = at_work(&w, false, 1);
    let ok = "Need your OK to delete the test data";
    ask(&id, ok);
    pane(&w, &id, "idle", false); // at rest for a moment, in the middle of the turn
    pane(&w, &id, "working", false);
    assert_eq!(task(&id).s("stage"), "working"); // herdr's guess: somebody typed
    says(&file, blocked_at(&id) + 1, "Waiting for your OK.");
    reported(&id, &file);
    pane(&w, &id, "done", true);
    settle(&id);
    waiting(&id, ok);
    // The same seen through herdr alone, with no turn-end report of the agent's own.
    sw::actions::talk::tell(&id, "Yes, delete it.", false).unwrap();
    prompt(&file, sw::clock::now() - 1000, "Go on.");
    ask(&id, "And the backups?");
    pane(&w, &id, "idle", false); pane(&w, &id, "working", false);
    says(&file, sw::clock::now() + 5, "And the backups?"); // its last words come after the flap
    let f = file.display().to_string();
    update(&id, |t| { t.put("transcript", f); Ok(Change::Write) }).unwrap();
    pane(&w, &id, "done", true);
    settle(&id);
    waiting(&id, "And the backups?");
}

#[test]
fn it_asks_again_and_again_each_answer_a_new_turn_never_reviewed_while_it_waits_reviewed_when_it_finishes() {
    let w = world();
    let (id, file) = at_work(&w, false, 3);
    for n in 2..=4 {
        let q = format!("Question {n} of 13");
        ask(&id, &q);
        says(&file, blocked_at(&id) + 1, &format!("Question {n}?"));
        if n % 2 == 1 { pane(&w, &id, "done", true); reported(&id, &file); } else { reported(&id, &file); settle(&id); pane(&w, &id, "done", true); } // either signal first
        settle(&id);
        waiting(&id, &q);
        if n == 4 { break; }
        sw::actions::talk::tell(&id, "1", false).unwrap();
        assert!(task(&id).at("blocked").is_null()); // the answer ends the wait
        prompt(&file, sw::clock::now() - 1000, "1");
    }
    // The last answer, and this time it does the work and stops without saying that it waits: that is a finish.
    w.script(json!({ "worker": [{ "files": { "plan.txt": "the plan\n" }, "commit": "plan", "status": "working" }] }));
    sw::actions::talk::tell(&id, "2", false).unwrap();
    let at = sw::clock::now();
    prompt(&file, at, "2"); says(&file, at + 1, "Wrote the plan.");
    reported(&id, &file); pane(&w, &id, "done", true);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(t.arr("reviews").len(), 1);
    assert_eq!(t.at("review").at("pass"), &json!(true));
}

#[test]
fn an_answer_typed_in_the_agent_s_own_terminal_starts_a_new_turn_its_end_is_a_finish_the_old_mark_does_not_hide_it() {
    let w = world();
    let (id, file) = at_work(&w, false, 1);
    ask(&id, Q2);
    let asked = blocked_at(&id);
    says(&file, asked + 1, "Question 2?");
    reported(&id, &file); pane(&w, &id, "done", true);
    settle(&id);
    waiting(&id, Q2);
    // Nobody saw this turn start: only its end arrives, in a task that still says "asks".
    std::fs::write(Path::new(task(&id).s("worktree")).join("plan.txt"), "the plan\n").unwrap();
    prompt(&file, asked + 10, "1"); says(&file, asked + 20, "Wrote the plan.");
    reported(&id, &file); pane(&w, &id, "done", true);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(t.arr("reviews").len(), 1); // it cannot dodge its review by having asked once
}

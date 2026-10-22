//! An agent's turns: approve refusing work that changed after its review, an agent herdr cannot classify, an agent
//! that says it is blocked, a verdict for another commit, a reviewer that touches the tree, and the words herdr's
//! sidebar says of where a task stands.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::json::*;
use sw::store::tasks::{update, Change};

fn put(id: &str, f: impl FnOnce(&mut Obj)) { update(id, |t| { f(t); Ok(Change::Write) }).unwrap(); }
fn with(t: &Obj, extra: Value) -> Obj { let mut o = t.clone(); for (k, v) in extra.as_object().cloned().unwrap_or_default() { o.insert(k, v); } o }
fn place(t: &Obj) -> String { sw::flow::sidebar::entry_now(t, sw::clock::now()).place }

#[test]
fn approve_refuses_to_merge_work_that_changed_after_its_review_the_finish_line_runs_first() {
    let w = world();
    let id = to_approval(&w, "Work");
    let t = task(&id);
    std::fs::write(std::path::Path::new(t.s("worktree")).join("extra.txt"), "late\n").unwrap(); // dirty tree behind the approval, no herdr event
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(read(&w.root.join("extra.txt")), "late\n"); // committed, re-reviewed, then merged
}

#[test]
fn an_agent_herdr_cannot_classify_unknown_finishes_when_its_closing_note_appears() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "codex", "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "u.txt": "u\n" }, "commit": "u", "note": "Codex is done.", "status": "unknown" }] }));
    let id = w.add("Unknown status");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    let args: Vec<String> = w.pane_of(&t).at("agent").arr("args").iter().map(text).take(2).collect();
    assert_eq!(args, vec!["--dangerously-bypass-approvals-and-sandbox", "--no-daemon"]);
    assert!(read(&sw::agents::homes::codex_file()).contains("trust_level = \"trusted\""));
}

#[test]
fn an_agent_that_reports_itself_blocked_gets_the_right_kind_and_stopping_after_that_is_not_a_finish() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Needs a token");
    delegate(&id);
    let t = settle(&id);
    sw::actions::filing::report(&id, sw::actions::filing::Report { state: "blocked".into(), kind: "auth".into(), msg: "NPM_TOKEN is missing".into(), own: "agent", ..Default::default() }).unwrap();
    let pane = t.at("link").t("pane");
    w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["status"] = json!("done"); p["agent"]["seq"] = json!(seq); p["agent"]["completion"] = json!(seq); } } });
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("kind"), t.s("msg")), ("asks", "auth", "NPM_TOKEN is missing"));
    let r = row(&id);
    assert_eq!((r.s("state"), r.s("kind")), ("blocked", "auth"));
}

#[test]
fn review_a_verdict_for_another_commit_is_ignored_a_missing_verdict_is_a_saved_failure_reviewer_edits_are_put_back() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.rounds": 0, "review.timeout": "1s" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }], "reviewer": [{ "verdict": { "meets": true, "findings": [], "text": "Fine.", "commit": "deadbeefdeadbeef" } }] }));
    let id = w.add("Wrong commit");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed");
    assert!(t.at("failure").s("detail").contains("review timed out"), "{}", t.at("failure"));
    assert!(t.at("review").is_null());
    assert_eq!(row(&id).s("state"), "error");
    // ctrl+r sends the error back; this time the reviewer answers properly but also touches the tree (the JS fake's
    // `dirtyTree`: a file of its own in the folder it reviews)
    w.script(json!({ "worker": [{ "note": "Nothing to change." }], "reviewer": [{ "files": { "reviewer-scratch.txt": "the reviewer changed something\n" },
        "verdict": { "meets": true, "text": "Has a real problem.", "findings": [{ "severity": "major", "subject": "f.txt is wrong", "where": "f.txt", "proof": "cat f.txt" }] }, "evidence": { "out.txt": "test output\n" } }] }));
    sw::actions::finish_line::send_error(&id).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(t.at("review").at("pass"), &json!(false));
    assert!(t.s("msg").starts_with("Approve to merge? Review: Fail · 1 major."), "{}", t.s("msg")); // gate "off": a failed review waits for you
    assert_eq!(git(std::path::Path::new(t.s("worktree")), &["status", "--porcelain"]), ""); // what you approve is exactly what the agent committed
    assert!(!sw::flow::task::review_dir(&t).join("verdict.json").exists());
    assert!(sw::store::files::artifacts_dir(&id).join("verdict.json").exists());
    // ctrl+r now hands the findings to the agent
    w.script(json!({ "worker": [{ "files": { "f.txt": "v2\n" }, "commit": "v2", "note": "Fixed it." }] }));
    let said = sw::actions::finish_line::send_review(&id).unwrap().msg;
    assert!(said.contains("review sent"), "{said}");
    let last = prompts(&w, task(&id).at("link").s("pane")).last().cloned().unwrap_or_default();
    assert!(regex::Regex::new(r"(?s)Result: Fail · 1 major\..*Required: fix each of these\.\n- \[major\] f\.txt is wrong \(f\.txt\)\n  Proof: cat f\.txt").unwrap().is_match(&last), "{last}");
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval");
    assert_eq!(t.at("review").at("pass"), &json!(true));
}

/// The rest of the JS test "a review that fails the gate goes straight back" (the loop itself is in tests/gate.rs):
/// what herdr's sidebar says of a task, in the board's own status words.
#[test]
fn herdr_s_sidebar_says_where_a_task_stands_in_the_board_s_status_words() {
    let w = world();
    let id = to_approval(&w, "Loop");
    put(&id, |t| { t.put("reviewRound", 3); t.put("review", json!({ "pass": false, "meets": true, "gate": "major", "text": "Still not it.", "findings": [{ "id": "F1", "severity": "major", "subject": "f.txt still holds the wrong number", "where": "f.txt", "proof": "cat f.txt" }] })); });
    let t = task(&id);
    assert_eq!(place(&t), "review failed");
    let old = with(&t, json!({ "review": { "score": 4, "recommend": true, "text": "Old." } }));
    let e = sw::flow::sidebar::entry_now(&old, sw::clock::now());
    assert_eq!((e.place.as_str(), e.line.as_str()), ("ready to approve", ""));
    assert_eq!(place(&with(&t, json!({ "stage": "reviewing" }))), "agent · under review");
    assert_eq!(place(&with(&t, json!({ "stage": "reviewing", "reviewRound": 0 }))), "agent · under review"); // the agent's own entry: the reviewer's says `reviewer`
    assert_eq!(place(&with(&t, json!({ "stage": "reviewneeds" }))), "needs you");
    assert_eq!(place(&with(&t, json!({ "stage": "reviewneeds", "reviewRound": 0 }))), "needs you");
    let cases = [json!({ "stage": "approval" }), json!({ "stage": "approval", "mergeFailed": true }), json!({ "stage": "approval", "prFailure": {} }), json!({ "stage": "approval", "syncBlocked": {} }), json!({ "stage": "asks" }), json!({ "stage": "asks", "kind": "auth" }), json!({ "stage": "nosummary" }), json!({ "stage": "failed" }), json!({ "stage": "failed", "failure": { "auth": true } }), json!({ "stage": "failed", "failure": { "phase": "sync" } }), json!({ "stage": "merging" }), json!({ "stage": "merging", "autoMerge": true }), json!({ "stage": "checking" }), json!({ "stage": "pushing" }), json!({ "stage": "working", "expectedWake": { "reason": "conflict" } }), json!({ "stage": "reviewing" }), json!({ "stage": "reviewneeds" })];
    let tail = regex::Regex::new(r" (?:pass|fail)$").unwrap();
    let mut labels: Vec<String> = cases.iter().map(|x| tail.replace(&place(&with(&t, x.clone())), "").into_owned()).collect();
    labels.sort(); labels.dedup();
    assert_eq!(labels, vec!["agent · under review", "agent · working", "asks you", "behind main", "board · checking", "board · merging", "board · pushing", "needs a login", "needs you", "review failed", "went wrong"]);
    assert_eq!(place(&obj(json!({ "id": "proj/9", "stage": "approval" }))), "ready to approve");
}

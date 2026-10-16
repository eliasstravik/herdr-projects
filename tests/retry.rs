//! A review cut off by the model's provider: the wait (growing, a few times, said on the row), the review started
//! again by itself and not counted as a round, the board giving up with the real error, an error waiting cannot fix,
//! and a reviewer that really finished without a verdict. The endings are recorded ones (tests/fixtures/endings).
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::json::*;
use sw::store::retries::{gave_up, retry_waits, wait_line, waiting_of};
use sw::store::reviews::{review_history, review_number, review_round};

fn work() -> Value { json!({ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }) }
fn pass() -> Value { json!({ "verdict": { "meets": true, "findings": [], "text": "Looks right." } }) }
fn reviewer_starts(w: &World) -> usize { calls(w).iter().filter(|c| regex::Regex::new(r"^agent start \S+-r ").unwrap().is_match(c)).count() }
fn history(t: &Obj) -> Vec<(i64, String, Value)> { review_history(t).iter().map(|e| (e.i("round"), e.t("error"), e.at("pass").clone())).collect() }
/// What the row said while the jobs ran: a look from another thread at the saved task, as often as it changes (the
/// waits between tries are real sleeps, short ones in these worlds).
fn watching(id: &str, f: impl FnOnce() -> Obj) -> (Obj, Vec<String>) {
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (s2, id2) = (stop.clone(), id.to_string());
    let look = std::thread::spawn(move || {
        let mut said: Vec<String> = vec![];
        while !s2.load(std::sync::atomic::Ordering::SeqCst) {
            if let Some(t) = sw::store::tasks::get(&id2) { let r = sw::flow::live::saved(&t); let l = format!("{}: {}", r.s("state"), r.s("note")); if !said.contains(&l) { said.push(l); } }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        said
    });
    let t = f();
    stop.store(true, std::sync::atomic::Ordering::SeqCst);
    (t, look.join().unwrap())
}

#[test]
fn the_wait_grows_a_few_times_says_why_and_when_and_holds_only_where_it_was_set() {
    let _w = world();
    assert_eq!(retry_waits(), vec![60000, 120000, 240000, 480000]);
    let now = 1000000;
    assert_eq!(wait_line(&json!({ "who": "reviewer", "reason": "model is at capacity", "until": now + 120000 }), now), "Reviewer's model is at capacity, trying again in 2m");
    assert_eq!(wait_line(&json!({ "who": "agent", "reason": "connection dropped", "until": now + 20000 }), now), "Agent's connection dropped, trying again in 20s");
    assert_eq!(wait_line(&json!({ "who": "agent", "reason": "connection dropped", "until": now - 5 }), now), "Agent's connection dropped, trying again now");
    assert_eq!(gave_up("reviewer", "model is at capacity", 5), "the reviewer's model is at capacity, still after 5 tries over 15m");
    let rv = json!({ "who": "reviewer", "reason": "model is at capacity", "until": sw::clock::now() + 120000 });
    let t = |v: Value| obj(v);
    // a review waiting to start again is still the review the task is on: its row keeps the R# tag
    assert_eq!(review_number(&t(json!({ "stage": "reviewing", "reviewRound": 0, "waiting": rv }))), 1);
    assert_eq!(review_number(&t(json!({ "stage": "reviewing", "reviewRound": 2, "waiting": rv }))), 3);
    let mut ag_rv = rv.clone(); ag_rv["who"] = json!("agent");
    assert_eq!(review_number(&t(json!({ "stage": "failed", "reviewRound": 0, "waiting": ag_rv }))), 0);
    assert_eq!(waiting_of(&t(json!({ "stage": "reviewing", "waiting": rv }))), Some(&rv));
    assert_eq!(waiting_of(&t(json!({ "stage": "failed", "waiting": rv }))), None);
    let ag = json!({ "who": "agent", "reason": "connection dropped", "until": sw::clock::now() + 60000, "for": 7 });
    assert_eq!(waiting_of(&t(json!({ "stage": "failed", "failure": { "at": 7 }, "waiting": ag }))), Some(&ag));
    assert_eq!(waiting_of(&t(json!({ "stage": "failed", "failure": { "at": 8 }, "waiting": ag }))), None); // another failure since
    assert_eq!(waiting_of(&t(json!({ "stage": "working", "failure": { "at": 7 }, "waiting": ag }))), None); // a reply moved it on
}

#[test]
fn a_review_cut_off_by_a_passing_error_is_started_again_by_itself_and_is_not_a_round() {
    let w = cut_world("300,300", json!({}));
    // as it happened: Codex reports its turn ended, and its file ends in the error
    w.script(json!({ "worker": [work()], "reviewer": [{ "ends": "codex-at-capacity", "hook": true }, { "ends": "codex-at-capacity", "hook": true, "status": "working" }, pass()] }));
    let id = w.add("Cut off twice");
    delegate(&id);
    let (t, said) = watching(&id, || settle(&id));
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(reviewer_starts(&w), 3);
    let re = regex::Regex::new(r"^working: Reviewer's model is at capacity, trying again (in \d+[sm]|now)$").unwrap();
    assert!(said.iter().any(|s| re.is_match(s)), "{}", said.join(" | "));
    assert!(!said.iter().any(|s| s.contains("without a verdict") || s.starts_with("error")), "{}", said.join(" | "));
    // one review happened: the two that were cut off used up no number and are not on the list
    assert_eq!(t.at("review").at("pass"), &json!(true)); assert_eq!(review_round(&t), 1); assert_eq!(t.i("reviewRounds"), 0);
    assert_eq!(history(&t), vec![(1, String::new(), json!(true))]);
    assert!(waiting_of(&t).is_none()); assert!(!t.b("failure"));
}

#[test]
fn a_review_cut_off_again_and_again_gives_up_with_the_real_error_and_retry_reviews_as_round_one() {
    let w = cut_world("30,30", json!({}));
    let cut = json!({ "ends": "codex-at-capacity", "hook": true });
    w.script(json!({ "worker": [work()], "reviewer": [cut, cut, cut] }));
    let id = w.add("Never gets through");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed"); assert_eq!(reviewer_starts(&w), 3);
    assert_eq!(t.s("msg"), "Review failed: the reviewer's model is at capacity, still after 3 tries over 1s. Its error: Selected model is at capacity. Please try a different model. Retry the task to review again");
    let both = format!("{}\n{}", t.s("msg"), t.at("failure").s("detail"));
    assert!(!both.contains("without a verdict") && !both.contains("last message"), "{both}");
    assert_eq!(row(&id).s("state"), "error"); assert!(waiting_of(&t).is_none());
    assert!(review_history(&t).is_empty()); assert_eq!(review_round(&t), 0);
    w.script(json!({ "reviewer": [pass()] }));
    sw::actions::finish_line::retry(&id).unwrap();
    let t = settle(&id);
    assert_eq!((t.s("stage"), review_round(&t), review_history(&t).len()), ("approval", 1, 1));
}

#[test]
fn the_report_of_a_try_that_was_cut_off_does_not_end_the_next_try() {
    let w = cut_world("30", json!({}));
    unsafe { std::env::set_var("HERDR_PROJECTS_REVIEW_MS", "1500") }; // the second reviewer works on and runs into the time limit
    unsafe { std::env::set_var("HERDR_PROJECTS_ENDED_MS", "60000") }; // only reports count here: the first try's file is seconds old
    w.script(json!({ "worker": [work()], "reviewer": [{ "ends": "codex-at-capacity", "hook": true }, { "status": "working" }] }));
    let id = w.add("Second try is slow");
    delegate(&id);
    let t = settle(&id);
    unsafe { std::env::set_var("HERDR_PROJECTS_REVIEW_MS", "120000") };
    assert_eq!(t.s("stage"), "failed"); assert_eq!(reviewer_starts(&w), 2);
    assert!(t.at("failure").s("detail").contains("review timed out"), "{}", t.at("failure"));
    assert_eq!(review_history(&t).iter().map(|e| (e.i("round"), e.t("error"))).collect::<Vec<_>>(), vec![(1, "review timed out".to_string())]);
}

#[test]
fn an_error_waiting_cannot_fix_is_not_retried_a_missing_model_fails_at_once_a_missing_login_asks_for_one() {
    let w = cut_world("30,30", json!({}));
    w.script(json!({ "worker": [work()], "reviewer": [{ "ends": "codex-model-not-supported" }] }));
    let id = w.add("Wrong model");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed"); assert_eq!(reviewer_starts(&w), 1);
    assert!(regex::Regex::new(r"^Review failed: gpt-6\.1 can't be used with your ChatGPT sign-in\. Choose a model it can use in reviewer\.agent ").unwrap().is_match(t.s("msg")), "{}", t.s("msg"));
    assert_eq!(t.at("failure").at("auth"), &json!(false));
    assert!(review_history(&t).is_empty()); assert_eq!(review_round(&t), 0);
    w.script(json!({ "worker": [work()], "reviewer": [{ "ends": "codex-unauthorized" }] }));
    let id = w.add("Logged out");
    delegate(&id);
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.at("failure").s("title"), row(&id).t("kind")), ("failed", "Needs a login", "auth".into()));
    assert_eq!(reviewer_starts(&w), 2);
}

#[test]
fn a_reviewer_that_really_finished_without_a_verdict_twice_is_still_a_failed_review_with_its_last_message() {
    let w = cut_world("30,30", json!({}));
    w.script(json!({ "worker": [work()], "reviewer": [{ "ends": "codex-finished-made" }, { "ends": "codex-finished-made" }] }));
    let id = w.add("No verdict");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed"); assert_eq!(reviewer_starts(&w), 2); // once more by itself (tests/helpers.rs)
    assert!(t.at("failure").s("detail").contains("the reviewer finished without a verdict. Its last message: I looked at it and it seems fine."), "{}", t.at("failure"));
    assert_eq!(review_round(&t), 1); assert!(review_history(&t)[0].s("error").contains("finished without a verdict"));
}

#[test]
fn without_the_reviewers_own_turn_end_report_its_file_ending_in_the_error_is_enough() {
    let w = cut_world("300", json!({ "reviewer.agent": "claude" }));
    w.script(json!({ "worker": [work()], "reviewer": [{ "ends": "claude-overloaded-made" }, pass()] }));
    let id = w.add("Not reported");
    delegate(&id);
    let (t, said) = watching(&id, || settle(&id));
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(reviewer_starts(&w), 2);
    assert!(said.iter().any(|s| s.contains("Reviewer's provider is overloaded, trying again")), "{}", said.join(" | "));
    assert_eq!(review_history(&t).iter().map(|e| e.i("round")).collect::<Vec<_>>(), vec![1]);
}

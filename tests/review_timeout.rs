//! `review.timeout`: a review that runs past it fails and says the limit, `none` waits on, a task's own value wins over
//! yours, and the repo's own file cannot set it. A review that ends without a verdict quotes the reviewer's last screen.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::store::settings::lookup::setting_text;

#[test]
fn a_review_past_its_limit_fails_and_says_the_limit_the_tasks_own_value_wins_over_yours() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.rounds": 0, "review.timeout": "20m" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }], "reviewer": [{ "status": "working" }, { "status": "working" }] }));
    let id = w.add("Slow review");
    delegate_with(&id, json!({ "review.timeout": "1s" }));
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed", "{}", t.at("failure"));
    assert!(t.at("failure").s("detail").contains("review timed out after 1s (review.timeout)"), "{}", t.at("failure"));
}

#[test]
fn none_waits_on_for_a_slow_reviewer() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.rounds": 0, "review.timeout": "1s" }), ..Default::default() });
    let pass = json!({ "verdict": { "meets": true, "findings": [], "text": "Fine." } });
    // The reviewer works on for many looks of the board (one every 20ms), well past your 1s, then answers.
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }], "reviewer": [{ "status": "working", "then": pass, "after": 120 }] }));
    let id = w.add("Slow but fine");
    delegate_with(&id, json!({ "review.timeout": "none" }));
    let began = std::time::Instant::now();
    let t = settle(&id);
    assert!(began.elapsed().as_millis() > 1000, "the reviewer answered within the 1s it would have had");
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
}

#[test]
fn the_repos_own_file_cannot_set_it() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "review.timeout": "1s" })), ..Default::default() });
    let id = w.add("Any");
    assert_eq!(setting_text("review.timeout", Some(&task(&id))), "20m");
}

#[test]
fn a_review_that_ends_without_a_verdict_quotes_the_reviewers_last_screen() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.rounds": 0, "review.timeout": "1s" }), ..Default::default() });
    // A dialog herdr does not report as one: the reviewer stays "working" on a screen that does not move.
    let earlier: String = (1..=9).map(|n| format!("earlier line {n}\n")).collect();
    let dialog = format!("{earlier}\n╭──────────────────────╮\n│ Allow this command?  │\n│ ❯ 1. Yes             │\n│   2. No              │\n╰──────────────────────╯\n");
    w.script(json!({ "worker": [step(1)], "reviewer": [{ "status": "working", "screen": dialog }] }));
    let id = w.add("Stuck review");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed", "{}", t.at("failure"));
    let detail = t.at("failure").s("detail");
    assert!(detail.starts_with("Review failed: review timed out after 1s (review.timeout)\n\nThe reviewer's screen when the review ended:\n"), "{detail}");
    // Its last 8 lines, its words without the dialog's frame.
    assert!(detail.ends_with("earlier line 5\nearlier line 6\nearlier line 7\nearlier line 8\nearlier line 9\nAllow this command?\n❯ 1. Yes\n  2. No"), "{detail}");
    assert!(!detail.contains("earlier line 4"), "{detail}");
    assert!(!t.s("msg").contains("1. Yes"), "the row's line stays one short line: {}", t.s("msg"));
}

#[test]
fn a_reviewer_whose_terminal_closes_says_so() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.rounds": 0 }), ..Default::default() });
    // Its terminal closes while it reviews, in the first try and in the one the board starts once more by itself.
    let closes = json!({ "status": "working", "then": { "closed": true }, "after": 2 });
    w.script(json!({ "worker": [step(1)], "reviewer": [closes.clone(), closes] }));
    let id = w.add("Closed review");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed", "{}", t.at("failure"));
    assert_eq!(t.at("failure").s("detail"), "Review failed: the reviewer's terminal was closed", "{}", t.at("failure"));
}

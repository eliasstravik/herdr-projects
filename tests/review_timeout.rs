//! `review.timeout`: a review that runs past it fails and says the limit, `none` waits on, a task's own value wins over
//! yours, and the repo's own file cannot set it.
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

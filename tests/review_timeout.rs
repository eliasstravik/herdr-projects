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

#[test]
fn a_reviewer_whose_pane_shows_nothing_new_ends_the_round_and_quotes_its_screen() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.rounds": 0, "review.timeout": "none" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_REVIEW_QUIET_MS", "200") };
    // The reviewer stops at a prompt of its own: its pane shows the dialog and never draws again, and it never
    // answers. `review.timeout` is none, so only the watchdog can end this round.
    let screen = "guardrails: allow this command?\n❯ 1. Yes\n  2. No, and tell the model why";
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }], "reviewer": [{ "status": "working", "screen": screen }] }));
    let id = w.add("Reviewer at a prompt");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed", "{}", t.at("failure"));
    let detail = t.at("failure").s("detail");
    assert!(detail.contains("the reviewer produced no output"), "{detail}");
    assert!(detail.contains("❯ 1. Yes"), "it quotes the pane's last lines: {detail}");
    assert!(!detail.contains("timed out"), "the round did not wait for review.timeout: {detail}");
    // The decision on its own, with no pane: one screen for the whole window is no output; a new one is output.
    let (mut seen, quiet) = (sw::finish::review_watch::Screen::default(), 200);
    assert!(seen.look(screen, 1000, quiet).is_none(), "the first look only starts the clock");
    assert!(seen.look("a new line", 1200, quiet).is_none(), "new output is not no output");
    let e = seen.look("a new line", 1500, quiet).expect("two looks of one screen past the window");
    assert!(e.contains("produced no output") && e.contains("a new line"), "{e}");
}

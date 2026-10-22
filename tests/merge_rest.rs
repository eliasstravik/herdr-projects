//! Merging, the rest of the old suite's checks: a merge conflict with your checkout waits for you as a question of
//! permission, not as a review (the rest of that test is lifecycle.rs). And the GitHub issues a merge with no pull
//! request closes (`merge.closes`): closed with gh once it has landed, never on a merge that failed, and a close that
//! fails leaves the merge done with the reason on its line.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;

#[test]
fn a_merge_conflict_with_your_checkout_waits_for_you_and_is_not_shown_as_a_review() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "a.txt": "from the agent\n" }, "commit": "agent", "note": "Changed a.txt." }] }));
    let id = w.add("Conflict");
    sw::store::settings::change::set_value("review.rounds", &json!(0), &sw::store::settings::lookup::Scope::task(&task(&id))).unwrap();
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "approval");
    std::fs::write(w.root.join("a.txt"), "from the human\n").unwrap(); git(&w.root, &["commit", "-qam", "human"]);
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.at("mergeFailed")), ("approval", &json!(true)));
    let r = row(&id);
    assert_eq!((r.s("state"), r.s("kind"), r.at("asReview")), ("blocked", "permission", &json!(false)));
}

/// A task whose work closes `issues`, through its one commit to waiting for approval, in a repo with no remote.
fn closing(w: &World, issues: &str) -> String {
    w.script(json!({ "worker": [step(1)] }));
    let id = sw::actions::filing::add(&w.root, "Fix it", sw::actions::filing::AddOpts { settings: vec![("merge.closes".into(), json!(issues))], ..Default::default() }).unwrap().t("id");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "approval");
    id
}
fn plain_world() -> World { world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() }) }

#[test]
fn a_merge_with_no_pull_request_closes_its_issues_with_gh_once_it_has_landed() {
    let w = plain_world();
    let id = closing(&w, "5, 6");
    assert!(!gh_log(&w).contains("issue close"), "nothing closed before the merge: {}", gh_log(&w));
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    let made = git(&w.root, &["rev-parse", "HEAD"]);
    assert_eq!(t.s("mergeCommit"), made);
    let comment = format!("--comment Closed by task {} \"Fix it\": its work landed in {}.", sw::store::keys::task_ref(&id, &sw::store::keys::repo_keys()), &made[..12]);
    for n in ["5", "6"] { assert!(gh_log(&w).contains(&format!("issue close {n} {comment}")), "{}", gh_log(&w)); }
    assert!(t.s("msg").starts_with("Merged into main. Closed #5, #6."), "{}", t.s("msg"));
}

#[test]
fn a_close_that_fails_leaves_the_merge_done_and_says_why_on_its_line() {
    let w = plain_world();
    let id = closing(&w, "5");
    w.gh_says(json!({ "issueFail": "GraphQL: Could not resolve to an issue or pull request with the number of 5." }));
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(read(&w.root.join("f.txt")), "v1\n");
    assert!(t.s("msg").contains("Could not close #5: GraphQL: Could not resolve to an issue or pull request with the number of 5."), "{}", t.s("msg"));
}

#[test]
fn nothing_is_closed_on_a_merge_that_failed() {
    let w = plain_world();
    let id = closing(&w, "5");
    std::fs::write(w.root.join("mine.txt"), "unsaved\n").unwrap();
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.at("failure").s("title"), "Merge needs a clean checkout");
    assert!(!gh_log(&w).contains("issue close"), "{}", gh_log(&w));
}

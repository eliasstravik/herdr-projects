//! The shorter check of a fix round (`check.again`, finish/check_again.rs): the first finish runs the full check, a
//! round the review sent back runs the shorter one from the commit the full one passed on; without GitHub's checks
//! guarding the merge, or on a branch that no longer holds that commit, every round runs the full check.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::finish::check_again::choose;
use sw::finish::review_brief::{review_brief, Brief};
use sw::json::*;
use sw::store::settings::lookup::work;

const CI: &str = ".github/workflows/ci.yml";
const FLOW: &str = "name: CI\non:\n  pull_request:\njobs:\n  build:\n    runs-on: x\n";
fn finding() -> Value { json!({ "severity": "critical", "subject": "It does not do it", "where": "f.txt", "proof": "ran it" }) }
fn verdict(findings: Value) -> Value { json!({ "verdict": { "meets": true, "text": "Looked at it.", "findings": findings } }) }
fn work_step(n: i64, ci: bool) -> Value {
    let mut files = json!({ "f.txt": format!("{n}\n") });
    if ci { files[CI] = json!(FLOW); }
    json!({ "files": files, "commit": format!("v{n}"), "note": format!("Did v{n}.") })
}
/// A repo with a pull request and these settings, whose checks write down what ran (`full`, or `again <commit>`).
fn checked(w_settings: Value) -> World {
    world_with(Opts { repo: true, origin: true, settings: w_settings, repo_settings: Some(json!({ "check": {
        "command": "echo full >> \"$HOME/checks.log\"",
        "again": "echo \"again $HERDR_PROJECTS_CHECKED\" >> \"$HOME/checks.log\"" } })), ..Default::default() })
}
fn ran(w: &World) -> Vec<String> { read(&w.base.join("home/checks.log")).lines().map(String::from).collect() }
/// Done once, sent back by the review once, done again: the task as it waits for your yes.
fn two_rounds(w: &World, ci: bool) -> (String, Obj) {
    w.script(json!({ "worker": [work_step(1, ci), work_step(2, false)], "reviewer": [verdict(json!([finding()])), verdict(json!([]))] }));
    let id = w.add("Fix it");
    delegate(&id);
    let mut t = settle(&id);
    for _ in 0..3 { if t.s("stage") != "approval" { t = settle(&id); } }
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(t.i("reviewRounds"), 1);
    (id, t)
}

#[test]
fn a_round_the_review_sent_back_runs_the_shorter_check_from_the_commit_the_full_one_passed_on() {
    let w = checked(json!({}));
    let (_, t) = two_rounds(&w, true);
    let first = t.at("checkPassed").t("head");
    assert!(!first.is_empty(), "the full check's commit is kept: {}", t.at("checkPassed"));
    assert_eq!(ran(&w), vec!["full".to_string(), format!("again {first}")]);
    assert_eq!(t.at("checkPassed").s("with"), "echo full >> \"$HOME/checks.log\"", "the shorter one passing moves nothing");
    // the reviewer hears which check passed on its commit, with the commit written in, and that GitHub runs the rest
    assert_eq!(t.s("checkShort"), format!("echo \"again {first}\" >> \"$HOME/checks.log\""));
    let brief = review_brief(&t, &Brief { text: "", change: None, to_land: true, ch: &work(Some(&t)), earlier: None, check: "npm test" });
    assert!(brief.contains(&format!("its shorter form, `echo \"again {first}\" >> \"$HOME/checks.log\"`")), "{brief}");
    assert!(brief.contains("the full check runs on GitHub before the merge"), "{brief}");
}

#[test]
fn without_github_guarding_the_merge_or_with_the_commit_gone_from_the_branch_every_round_runs_the_full_check() {
    // no workflow runs for the pull request: nothing on GitHub runs the full suite
    let w = checked(json!({}));
    let (_, t) = two_rounds(&w, false);
    assert_eq!(ran(&w), vec!["full", "full"]);
    assert!(!t.b("checkShort"));
    drop(w);
    // GitHub's checks are not waited for before the merge
    let w = checked(json!({ "merge.checks": "none" }));
    two_rounds(&w, true);
    assert_eq!(ran(&w), vec!["full", "full"]);
    drop(w);
    // all is in place, but the branch no longer holds the commit the full check passed on (rebuilt): the full one
    let w = checked(json!({}));
    let (_, t) = two_rounds(&w, true);
    let tree = std::path::PathBuf::from(t.s("worktree"));
    let ch = work(Some(&t));
    let full = "echo full >> \"$HOME/checks.log\"";
    assert!(choose(&t, &ch, full, t.s("worktree"), &w.root).short(), "as it stands, the shorter one");
    // GitHub Actions turned off for the repo: its workflow never runs there (#416)
    w.gh_says(json!({ "actionsOff": true }));
    assert!(!choose(&t, &ch, full, t.s("worktree"), &w.root).short());
    w.gh_says(json!({}));
    git(&tree, &["commit", "-q", "--allow-empty", "-m", "elsewhere"]);
    let gone = git(&tree, &["rev-parse", "HEAD"]);
    git(&tree, &["reset", "-q", "--hard", "HEAD~1"]);
    let mut moved = t.clone();
    moved.put("checkPassed", json!({ "head": gone.trim(), "with": full }));
    assert!(!choose(&moved, &ch, full, t.s("worktree"), &w.root).short());
    // and the check's command changed since: the full one
    assert!(!choose(&t, &ch, "npm test", t.s("worktree"), &w.root).short());
}

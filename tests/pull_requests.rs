//! Pull requests through a fake gh: pushed as a draft, merged through gh on approval, remembered while GitHub's checks
//! run, watched; failed checks wake the agent; merged elsewhere without a newer local commit; comments of the human
//! only wake the agent; a `Closes #N` line in its body for each issue of `merge.closes`.
use crate::common;
use common::*;
use crate::pull_requests_checks::waits;
use serde_json::json;
use std::path::Path;
use sw::actions::{archive, finish_line, talk};
use sw::housekeeping::poll_prs;
use sw::json::*;
use sw::store::tasks::{update, Change};

fn pr_world(more: serde_json::Value) -> World {
    let mut s = json!({ "review.when": "never" });
    for (k, v) in more.as_object().cloned().unwrap_or_default() { s[k] = v; }
    world_with(Opts { repo: true, origin: true, settings: s, ..Default::default() })
}
fn pane_prompts(w: &World, id: &str) -> Vec<String> { prompts(w, task(id).at("link").s("pane")) }

#[test]
fn pull_requests_pushed_as_a_draft_merged_through_gh_on_approval_remembered_when_checks_are_pending_watched() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "With a PR");
    let t = task(&id);
    assert_eq!(t.s("pr"), "https://github.com/o/r/pull/7");
    assert!(gh_log(&w).contains("pr create --draft --head work/1-with-a-pr --title With a PR"), "{}", gh_log(&w));
    assert_eq!(git(&w.root, &["ls-remote", "--heads", "origin", t.s("branch")]).split('\t').next().unwrap(), git(Path::new(t.s("worktree")), &["rev-parse", "HEAD"]));
    assert!(t.s("msg").contains("Pull request: https://github.com/o/r/pull/7"), "{}", t.s("msg"));
    // checks still running: the approval is remembered
    let pending = json!({ "state": "OPEN", "headRefOid": t.at("prHead"), "statusCheckRollup": [{ "name": "build", "state": "PENDING" }], "comments": [], "reviews": [], "author": { "login": "me" } });
    w.gh_says(json!({ "mergeFail": "checks pending", "view": pending }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging"); assert_eq!(t.at("autoMerge"), &json!(true));
    waits(&t, "Waiting for GitHub checks · 0 of 1 passed", "build");
    assert!(!regex::Regex::new("--auto|--delete-branch").unwrap().is_match(&gh_log(&w)), "remembered here, not handed to GitHub; gh never deletes local work");
    assert!(archive::archive(&id).unwrap_err().msg.contains("still active"));
    poll_prs();
    assert_eq!(task(&id).at("autoMerge"), &json!(true), "still running");
    // a task of an older version has GitHub's own auto-merge on: replying switches that off first, or sends nothing
    update(&id, |c| { c.put("autoMergeLocal", false); Ok(Change::Write) }).unwrap();
    w.gh_says(json!({ "disableFail": true }));
    let e = talk::tell(&id, "wait", false).unwrap_err().msg;
    assert!(e.contains("Could not cancel the pending auto-merge; nothing was sent"), "{e}");
    assert_eq!(pane_prompts(&w, &id).len(), 1);
    update(&id, |c| { c.put("autoMergeLocal", true); Ok(Change::Write) }).unwrap();
    // merged elsewhere: the poll marks it done and cleans up
    w.gh_says(json!({ "view": { "state": "MERGED" } }));
    poll_prs();
    let t = task(&id);
    assert_eq!(t.s("stage"), "done"); assert_eq!(t.at("prMerged"), &json!(true)); assert_eq!(t.s("msg"), "Merged: https://github.com/o/r/pull/7");
    assert!(!w.base.join("fake/worktrees/proj/work-1-with-a-pr").exists());
}

#[test]
fn pull_requests_failed_checks_wake_the_agent_capped_and_the_blocker_survives_a_passing_local_check() {
    let w = pr_world(json!({ "review.rounds": 1 }));
    let id = to_approval(&w, "CI fails");
    let t = task(&id);
    // the agent finds nothing to change: the commit that failed on GitHub is still the one that would merge
    w.script(json!({ "worker": [{ "note": "Nothing to fix here." }] }));
    w.gh_says(json!({ "view": { "state": "OPEN", "headRefOid": t.at("prHead"), "statusCheckRollup": [{ "name": "build", "conclusion": "FAILURE" }], "comments": [], "reviews": [], "author": { "login": "me" } } }));
    poll_prs();
    let last = pane_prompts(&w, &id).last().cloned().unwrap_or_default();
    assert!(regex::Regex::new(r"Checks failed on the pull request[\s\S]*build").unwrap().is_match(&last), "{last}");
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval"); assert_eq!(t.at("prFailure").s("title"), "Pull request checks failed");
    assert!(t.s("msg").contains("pull request still blocked"), "local success cannot clear failed CI: {}", t.s("msg"));
    assert_eq!(t.at("prFailure").s("head"), t.s("prHead"));
    // CI on that head passes: normal approval wording returns
    w.gh_says(json!({ "view": { "state": "OPEN", "headRefOid": task(&id).at("prHead"), "statusCheckRollup": [{ "name": "build", "conclusion": "SUCCESS" }], "comments": [], "reviews": [], "author": { "login": "me" } } }));
    poll_prs();
    let t = task(&id);
    assert!(t.at("prFailure").is_null()); assert!(t.s("msg").starts_with("Approve to merge?"), "{}", t.s("msg")); assert_eq!(t.s("prState"), "passing");
}

#[test]
fn a_pull_request_merged_elsewhere_does_not_take_a_newer_local_commit_with_it() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "With a PR");
    let t = task(&id);
    let tree = Path::new(t.s("worktree"));
    let pushed = t.t("prHead");
    assert_eq!(pushed, git(tree, &["rev-parse", "HEAD"]));
    // one more commit here that was never pushed; meanwhile the pull request is merged on GitHub at the pushed head
    std::fs::write(tree.join("late.txt"), "late\n").unwrap(); git(tree, &["add", "-A"]); git(tree, &["commit", "-qm", "late"]);
    w.gh_says(json!({ "view": { "state": "MERGED", "headRefOid": pushed } }));
    poll_prs();
    let t = task(&id);
    assert_eq!(t.s("stage"), "done"); assert_eq!(t.at("prMerged"), &json!(true));
    assert!(t.arr("kept").first().map(text).unwrap_or_default().contains("unmerged work on work/"), "{}", t.at("kept"));
    assert!(t.s("msg").contains("Kept: ")); assert!(!t.b("tidy") && !t.b("cleaned"));
    assert!(tree.join("late.txt").exists()); assert_eq!(git(&w.root, &["log", "-1", "--format=%s", t.s("branch")]), "late");
    assert!(!w.state().arr("workspaces").iter().any(|x| x.s("workspace_id") == t.at("link").s("workspace")), "it still leaves the sidebar");
}

#[test]
fn pull_request_comments_only_the_humans_own_wake_the_agent_not_others_nor_the_boards_own_review_comment() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Commented on");
    let t = task(&id);
    let before = pane_prompts(&w, &id).len();
    let gh = |comments: serde_json::Value| w.gh_says(json!({ "user": "elias", "view": { "state": "OPEN", "headRefOid": t.at("prHead"), "statusCheckRollup": [], "reviews": [], "author": { "login": "elias" }, "comments": comments } }));
    let say = |login: &str, body: &str, n: i64| json!({ "id": format!("c{n}"), "author": { "login": login }, "body": body, "updatedAt": format!("2026-01-0{n}") });
    // someone else and a review bot: read by nobody here
    gh(json!([say("a-colleague", "Could this be faster?", 1), say("review-bot[bot]", "3 issues found.", 2)]));
    poll_prs();
    assert_eq!(pane_prompts(&w, &id).len(), before); assert_eq!(task(&id).s("stage"), "approval"); assert!(!task(&id).b("failure"));
    // the board's own comment about a review is posted as the same user: it is the board's, not the human's
    gh(json!([say("elias", &format!("**Review 1 · Fail · 1 major**\n\nFix it.{}", sw::flow::git::BOARD_COMMENT), 3), say("elias", "**Review 1 · Pass**\n\nNo findings.", 4)]));
    poll_prs();
    assert_eq!(pane_prompts(&w, &id).len(), before); assert!(!sw::flow::git::board_comment("Looks **good** to me"));
    // the human's own words do
    w.script(json!({ "worker": [{ "status": "working" }] }));
    gh(json!([say("a-colleague", "Could this be faster?", 1), say("elias", "Please rename the flag.", 5)]));
    poll_prs();
    let p = pane_prompts(&w, &id);
    assert_eq!(p.len(), before + 1); assert!(p.last().unwrap().contains("New feedback on the pull request"), "{}", p.last().unwrap());
    let log = gh_log(&w);
    assert!(log.lines().any(|l| l == "api user --jq .login"), "{log}");
    assert_eq!(log.lines().filter(|l| l.starts_with("api user")).count(), 1, "asked once, then remembered");
}

#[test]
fn a_pull_request_the_board_opens_closes_the_issues_of_merge_closes_by_its_body() {
    let w = pr_world(json!({}));
    w.script(json!({ "worker": [step(1)] }));
    let id = sw::actions::filing::add(&w.root, "Fix it", sw::actions::filing::AddOpts { settings: vec![("merge.closes".into(), json!("12, 13"))], ..Default::default() }).unwrap().t("id");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "approval");
    assert!(gh_log(&w).contains("Task: PRJ-1 (proj/1)\n\nCloses #12\nCloses #13 --base main"), "{}", gh_log(&w));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.s("msg").starts_with("Merged: https://github.com/o/r/pull/7. GitHub closes #12, #13."), "{}", t.s("msg"));
    assert!(!regex::Regex::new("pr edit|issue close").unwrap().is_match(&gh_log(&w)), "GitHub closes them itself: {}", gh_log(&w));
}

fn body_now(w: &World) -> String { serde_json::from_str::<serde_json::Value>(&sw::flow::git::gh(&["pr", "view", "7", "--json", "body"], &w.root, 1000).stdout).unwrap().t("body") }
fn closing(w: &World, issues: &str) -> String {
    w.script(json!({ "worker": [step(1)] }));
    let id = sw::actions::filing::add(&w.root, "Fix it", sw::actions::filing::AddOpts { settings: vec![("merge.closes".into(), json!(issues))], ..Default::default() }).unwrap().t("id");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "approval");
    id
}
fn closes(id: &str, issues: &str) { sw::store::settings::change::set_value("merge.closes", &json!(issues), &sw::store::settings::lookup::Scope::task(&task(id))).unwrap(); }

#[test]
fn an_issue_taken_off_merge_closes_is_taken_off_its_pull_request_before_it_merges() {
    let w = pr_world(json!({}));
    let id = closing(&w, "12, 13");
    closes(&id, "13, 14");
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    let body = body_now(&w);
    assert!(!body.contains("#12") && body.contains("Closes #13") && body.contains("Closes #14"), "{body}");
    assert!(t.s("msg").contains("GitHub closes #13, #14."), "{}", t.s("msg"));
    assert!(!gh_log(&w).contains("issue close"), "{}", gh_log(&w));
}

#[test]
fn a_cleared_merge_closes_leaves_no_closes_line_and_nothing_merges_while_it_cannot_be_taken_off() {
    let w = pr_world(json!({}));
    let id = closing(&w, "12");
    closes(&id, "none");
    w.gh_says(json!({ "editFail": "HTTP 502" }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.at("failure").s("title")), ("approval", "Could not take an issue off the pull request"));
    assert!(t.at("failure").s("detail").contains("HTTP 502") && !gh_log(&w).contains("pr merge"), "{}", gh_log(&w));
    w.gh_says(json!({}));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(!body_now(&w).contains("Closes"), "{}", body_now(&w));
    assert!(!t.s("msg").contains("closes") && !gh_log(&w).contains("issue close"), "{}", t.s("msg"));
}

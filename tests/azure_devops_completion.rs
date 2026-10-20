//! Completing a pull request on Azure DevOps when it does not go through at once: refused while a reviewer is awaited
//! (it waits, and completes once the vote is there), refused with nothing to wait for (it goes back to you), and
//! accepted but not carried out (its merge conflicts, or it stays active past `check.timeout`: it goes back to you).
use crate::azure_devops_flow::az_world;
use crate::common;
use common::az::policy;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::actions::finish_line;
use sw::housekeeping::poll_prs;
use sw::json::*;

fn approved(w: &World, title: &str, says: serde_json::Value) -> (String, String) {
    let id = to_approval(w, title);
    let reviewed = git(Path::new(task(&id).s("worktree")), &["rev-parse", "HEAD"]);
    let mut s = says; s["head"] = json!(reviewed);
    w.az_says(s);
    finish_line::approve(&id, None, false).unwrap();
    settle(&id);
    (id, reviewed)
}
fn completions(w: &World) -> usize { w.az.sent.borrow().iter().filter(|s| s.s("resource") == "pullRequests").count() }

#[test]
fn a_completion_refused_while_a_reviewer_is_awaited_waits_and_completes_once_the_vote_is_there() {
    let w = az_world(json!({}));
    let (id, reviewed) = approved(&w, "Awaits a vote", json!({ "completeFail": "The pull request needs a reviewer's approval.", "policies": [policy("Minimum number of reviewers", "", "queued", None)] }));
    let t = task(&id);
    assert_eq!(t.s("stage"), "merging", "{}", t.at("failure"));
    assert!(t.b("autoMerge") && t.b("autoMergeLocal") && !t.b("failure"), "{}", t.s("msg"));
    assert!(t.s("msg").starts_with("Approved: merges when Azure DevOps' policies pass"), "{}", t.s("msg"));
    assert_eq!(completions(&w), 1);
    // the vote is there: completed at the reviewed commit
    w.az_says(json!({ "head": reviewed, "policies": [policy("Minimum number of reviewers", "", "approved", None)] }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(completions(&w), 2);
    assert!(t.b("prMerged") && t.b("pipelineWait"), "{} {}", t.s("msg"), t.at("failure"));
}

#[test]
fn a_completion_refused_with_nothing_to_wait_for_goes_back_to_you_with_azure_devops_answer() {
    let w = az_world(json!({}));
    let (id, _) = approved(&w, "Refused", json!({ "completeFail": "TF401192: the target branch has moved.", "policies": [policy("Build", "CI build", "approved", Some(7))] }));
    let t = task(&id);
    assert_eq!(t.at("failure").s("title"), "Pull request could not complete", "{}", t.at("failure"));
    assert!(t.at("failure").s("detail").contains("TF401192") && !t.b("autoMerge") && !t.b("prMerged"), "{}", t.at("failure"));
}

#[test]
fn a_completion_azure_devops_accepted_whose_merge_conflicts_goes_back_with_the_conflict() {
    let w = az_world(json!({}));
    let (id, reviewed) = approved(&w, "Conflicts", json!({ "completeQueued": true, "mergeStatus": "queued" }));
    let t = task(&id);
    assert!(t.b("autoMerge") && !t.b("autoMergeLocal") && t.b("completeQueued"), "{} {}", t.s("msg"), t.at("failure"));
    assert!(t.s("msg").starts_with("Approved: Azure DevOps completes it"), "{}", t.s("msg"));
    // still queued: it waits
    poll_prs();
    assert!(task(&id).b("autoMerge") && !task(&id).b("failure"));
    // its merge conflicts: the pull request stays active, and the task says why
    w.az_says(json!({ "head": reviewed, "completeQueued": true, "mergeStatus": "conflicts" }));
    poll_prs();
    let t = task(&id);
    assert_eq!((t.s("stage"), t.s("msg")), ("approval", "Pull request could not complete: its branch conflicts"), "{}", t.at("failure"));
    assert!(!t.b("autoMerge") && !t.b("completeQueued") && t.b("requireApproval") && t.at("failure").b("agentCanFix"), "{}", t.at("failure"));
    assert!(t.at("failure").s("detail").contains("Merge status: conflicts"), "{}", t.at("failure"));
}

#[test]
fn a_completion_azure_devops_accepted_but_never_carried_out_stops_waiting_after_the_check_timeout() {
    let w = az_world(json!({ "check.timeout": "15m" }));
    let (id, _) = approved(&w, "Never completes", json!({ "completeQueued": true, "mergeStatus": "queued" }));
    sw::clock::advance(14 * 60000);
    poll_prs();
    assert!(task(&id).b("autoMerge") && !task(&id).b("failure"), "not yet: {}", task(&id).s("msg"));
    sw::clock::advance(2 * 60000);
    poll_prs();
    let t = task(&id);
    assert_eq!((t.s("stage"), t.at("failure").s("title")), ("approval", "Pull request was not completed"), "{}", t.at("failure"));
    assert!(t.at("failure").s("detail").contains("still active after 15m") && !t.b("autoMerge"), "{}", t.at("failure"));
}

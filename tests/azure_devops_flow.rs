//! A repo at Azure DevOps, from the finish line to done: its draft pull request is opened with az, it merges only once
//! its policies pass and is completed by `merge.method`, then the task waits for the base branch's pipeline for the
//! merge commit and is done once that passed. A pull request the human completed in Azure DevOps is taken for merged,
//! and work the agent merged itself is `Already merged in !N`. No login to Azure DevOps is said as one.
use crate::common;
use common::az::{at_azure, policy, run, WEB};
use common::*;
use serde_json::{json, Value};
use std::path::Path;
use sw::actions::finish_line;
use sw::housekeeping::poll_prs;
use sw::json::*;

pub fn az_world(settings: Value) -> World {
    let mut s = json!({ "review.when": "never" });
    for (k, v) in settings.as_object().cloned().unwrap_or_default() { s[k] = v; }
    let w = world_with(Opts { repo: true, origin: true, settings: s, ..Default::default() });
    at_azure(&w);
    w
}
pub fn merge_commit() -> String { "m".repeat(40) }
/// The base branch's stages, as Azure Pipelines' timeline of run `id` gives them.
pub fn stages(list: &[(&str, &str)]) -> Value { json!({ "records": list.iter().enumerate().map(|(i, (n, s))| json!({ "id": format!("s{i}"), "type": "Stage", "name": n, "state": s, "order": i })).collect::<Vec<_>>() }) }

#[test]
fn a_task_at_azure_devops_opens_its_draft_pull_request_merges_once_its_policies_pass_and_is_done_once_the_main_pipeline_passed() {
    let w = az_world(json!({}));
    let id = to_approval(&w, "Shop change");
    let t = task(&id);
    assert_eq!(t.s("pr"), format!("{WEB}/pullrequest/42"));
    assert!(t.s("msg").contains(&format!("{WEB}/pullrequest/42")), "{}", t.s("msg"));
    let log = az_log(&w);
    assert!(log.contains(&format!("repos pr create --project Big Project --repository shop --source-branch {} --title Shop change --description", t.s("branch"))), "{log}");
    assert!(log.contains("--draft true") && gh_log(&w).is_empty(), "never gh: {log}");
    assert_eq!(git(&w.root, &["ls-remote", "origin", &format!("refs/heads/{}", t.s("branch"))]).split('\t').next().unwrap_or(""), git(Path::new(t.s("worktree")), &["rev-parse", "HEAD"]), "pushed");
    assert_eq!(sw::shown::words::pr_number(&t), "PR#42");
    // approved while its build validation runs: it waits for it, said as Azure DevOps' checks
    let reviewed = git(Path::new(t.s("worktree")), &["rev-parse", "HEAD"]);
    w.az_says(json!({ "head": reviewed, "policies": [policy("Build", "CI build", "running", Some(7)), policy("Minimum number of reviewers", "", "approved", None)] }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging", "{}", t.at("failure")); assert!(t.b("autoMerge"));
    assert_eq!(t.s("msg"), "Waiting for Azure DevOps checks · 0 of 1 passed");
    assert!(az_log(&w).contains("repos pr update --id 42 --draft false"), "marked ready: {}", az_log(&w));
    assert!(w.az.sent.borrow().iter().all(|s| s.s("resource") != "pullRequests"), "not completed while it runs");
    // watched meanwhile: the panel says its votes and policies, `details` the pull request as Azure DevOps has it now
    w.az_says(json!({ "head": reviewed, "reviewers": [{ "displayName": "Ann", "vote": 10 }], "policies": [policy("Build", "CI build", "running", Some(7)), policy("Minimum number of reviewers", "", "approved", None)] }));
    poll_prs();
    assert_eq!(task(&id).s("prRemote"), "Reviewers: Ann approved · Policies: CI build running");
    let mut b = common::board::open(&w.root, 44);
    b.s.sel = id.clone(); b.load();
    let screen = b.screen(160).join("\n");
    assert!(screen.contains("Azure        Reviewers: Ann approved · Policies: CI build") && screen.contains("waits for Azure DevOps' policies") && !screen.contains("GitHub"), "{screen}");
    let d = finish_line::details(&id).unwrap();
    assert_eq!((d.at("azure").s("status"), d.at("azure").at("policies")[0].s("result")), ("OPEN", "running"), "{}", d.at("azure"));
    assert_eq!(d.at("azure").at("reviewers")[0].s("name"), "Ann");
    // it passed: completed at the reviewed commit, squashed, its source branch deleted; then the main pipeline
    w.az_says(json!({ "head": reviewed, "policies": [policy("Build", "CI build", "approved", Some(7))] }));
    poll_prs();
    let t = settle(&id);
    let sent = w.az.sent.borrow().iter().find(|s| s.s("resource") == "pullRequests").cloned().unwrap_or(Value::Null);
    assert!(sent.s("body").starts_with("--id 42 --status completed --squash true --delete-source-branch true --merge-commit-message"), "{sent}");
    assert_eq!(t.s("stage"), "merging", "not done until its pipeline passed: {}", t.s("msg"));
    assert!(t.b("prMerged") && t.at("pipelineWait").s("commit") == merge_commit() && t.at("pipelineWait").s("base") == "main", "{}", t.at("pipelineWait"));
    assert!(Path::new(t.s("worktree")).exists() && !t.b("tidy"), "nothing cleaned up yet");
    assert_eq!(sw::flow::live::board(true).iter().find(|r| r.s("id") == id).map(|r| r.t("state")).unwrap_or_default(), "working");
    // its run of the merge commit goes through its stages: the row says how far
    w.az_says(json!({ "head": reviewed, "runs": [run(5, "CI", &merge_commit(), "inProgress", ""), run(4, "CI", &"0".repeat(40), "completed", "failed")], "timeline": { "5": stages(&[("Build", "completed"), ("Test", "inProgress"), ("Deploy staging", "pending")]) } }));
    poll_prs();
    assert_eq!(task(&id).s("msg"), "Main pipeline running · 1 of 3 stages");
    w.az_says(json!({ "head": reviewed, "runs": [run(5, "CI", &merge_commit(), "inProgress", "")], "timeline": { "5": stages(&[("Build", "completed"), ("Test", "completed"), ("Deploy staging", "inProgress")]) } }));
    poll_prs();
    assert_eq!(task(&id).s("msg"), "Deploying staging");
    let d = finish_line::details(&id).unwrap();
    assert_eq!((d.at("azure").at("pipeline").at("runs")[0].s("pipeline"), d.at("azure").at("pipeline").at("runs")[0].s("status")), ("CI", "inprogress"), "{}", d.at("azure"));
    assert!(az_log(&w).contains("pipelines runs list --project Big Project --branch refs/heads/main"), "{}", az_log(&w));
    w.az_says(json!({ "head": reviewed, "runs": [run(5, "CI", &merge_commit(), "completed", "succeeded")] }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.s("msg").starts_with(&format!("Merged: {WEB}/pullrequest/42. Main pipeline passed: CI passed")), "{}", t.s("msg"));
    assert!(t.b("cleaned") && t.at("pipelineWait").is_null());
}

#[test]
fn merge_method_and_merge_pipeline_none_complete_it_as_asked_and_done_at_once() {
    let w = az_world(json!({ "merge.method": "merge", "merge.pipeline": "none" }));
    let id = to_approval(&w, "No gate");
    let reviewed = git(Path::new(task(&id).s("worktree")), &["rev-parse", "HEAD"]);
    w.az_says(json!({ "head": reviewed }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(t.s("mergeCommit"), merge_commit());
    let sent = w.az.sent.borrow().iter().find(|s| s.s("resource") == "pullRequests").cloned().unwrap_or(Value::Null);
    assert!(sent.s("body").starts_with("--id 42 --status completed --squash false --delete-source-branch true"), "{sent}");
    assert!(!az_log(&w).contains("pipelines runs"), "{}", az_log(&w));
}

#[test]
fn merge_method_rebase_completes_it_through_the_repositorys_own_route_at_the_reviewed_commit() {
    let w = az_world(json!({ "merge.method": "rebase", "merge.pipeline": "none" }));
    let id = to_approval(&w, "Rebased");
    let reviewed = git(Path::new(task(&id).s("worktree")), &["rev-parse", "HEAD"]);
    w.az_says(json!({ "head": reviewed }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
    let sent = w.az.sent.borrow().iter().find(|s| s.s("resource") == "pullRequests").cloned().unwrap_or(Value::Null);
    let body = sent.at("body");
    assert_eq!((body.s("status"), body.at("lastMergeSourceCommit").s("commitId")), ("completed", reviewed.as_str()), "{sent}");
    assert_eq!((body.at("completionOptions").s("mergeStrategy"), body.at("completionOptions").at("deleteSourceBranch")), ("rebase", &json!(true)));
    assert!(az_log(&w).contains("rest --method patch --url https://dev.azure.com/acme/Big%20Project/_apis/git/repositories/shop/pullRequests/42?api-version=7.1"), "{}", az_log(&w));
}

#[test]
fn a_pull_request_the_human_completed_in_azure_devops_is_merged_without_the_board_and_waits_for_its_pipeline() {
    let w = az_world(json!({}));
    let id = to_approval(&w, "Completed by hand");
    // your own checkout has unsaved files: nothing here needs it clean
    std::fs::write(w.root.join("mine.txt"), "unsaved\n").unwrap();
    let head = git(Path::new(task(&id).s("worktree")), &["rev-parse", "HEAD"]);
    w.az_says(json!({ "head": head, "status": "completed" }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging", "{}", t.s("msg"));
    assert!(t.b("prMerged") && t.b("pipelineWait"));
    assert!(w.az.sent.borrow().iter().all(|s| s.s("resource") != "pullRequests"), "the board completed nothing");
    w.az_says(json!({ "head": head, "status": "completed", "runs": [run(9, "CI", &merge_commit(), "completed", "succeeded")] }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.b("cleaned"), "{}", t.s("msg"));
}

#[test]
fn an_abandoned_pull_request_whose_merge_was_awaited_waits_for_you() {
    let w = az_world(json!({}));
    let id = to_approval(&w, "Abandoned");
    let head = git(Path::new(task(&id).s("worktree")), &["rev-parse", "HEAD"]);
    w.az_says(json!({ "head": head, "policies": [policy("Build", "CI build", "queued", Some(7))] }));
    finish_line::approve(&id, None, false).unwrap();
    assert!(settle(&id).b("autoMerge"));
    w.az_says(json!({ "head": head, "status": "abandoned" }));
    poll_prs();
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("msg")), ("approval", "Pull request was closed without merging"));
    assert!(t.at("failure").s("next").contains("Reactivate the pull request in Azure DevOps"), "{}", t.at("failure"));
}

#[test]
fn a_failed_build_validation_goes_back_to_the_agent_with_its_failing_job_and_the_end_of_its_log() {
    let w = az_world(json!({ "review.rounds": 0 }));
    let id = to_approval(&w, "Red build");
    let head = git(Path::new(task(&id).s("worktree")), &["rev-parse", "HEAD"]);
    let tl = json!({ "records": [{ "id": "j", "type": "Job", "name": "Unit tests", "state": "completed", "result": "failed", "order": 1 },
        { "id": "t", "parentId": "j", "type": "Task", "name": "npm test", "state": "completed", "result": "failed", "log": { "id": 4 }, "order": 2 }] });
    w.az_says(json!({ "head": head, "policies": [policy("Build", "CI build", "rejected", Some(7))], "timeline": { "7": tl }, "log": ["2026-10-20T10:00:00.0000000Z > npm test", "Error: expected 2 to be 3"] }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.s("msg"));
    assert_eq!(t.s("msg"), "Azure DevOps' checks failed: CI build. Send the details to the agent, or merge it anyway");
    let d = t.at("prFailure").s("detail");
    assert!(d.contains("Failed in job Unit tests, step npm test") && d.contains("Error: expected 2 to be 3") && !d.contains("2026-10-20T"), "{d}");
    assert!(d.contains("az repos pr policy list --id 42"), "{d}");
}

#[test]
fn work_the_agent_merged_in_its_own_pull_request_on_azure_devops_is_already_merged_naming_it_the_azure_way() {
    let w = az_world(json!({}));
    let squash = json!([["push", "-q", "origin", "HEAD"], ["fetch", "-q", "origin", "main"], ["checkout", "-q", "--detach", "FETCH_HEAD"], ["merge", "-q", "--squash", "@{-1}"],
        ["-c", "user.email=agent@test", "-c", "user.name=agent", "commit", "-qm", "Merged PR 1234: release"], ["push", "-q", "origin", "HEAD:main"], ["checkout", "-q", "-"]]);
    w.az_says(json!({ "prs": [{ "pullRequestId": 1234, "status": "completed", "targetRefName": "refs/heads/main", "lastMergeSourceCommit": { "commitId": "x" } }] }));
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1.", "git": squash }] }));
    let id = w.add("Release");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(t.s("doneWhy"), "Already merged in !1234");
    assert!(!az_log(&w).contains("repos pr create"), "{}", az_log(&w));
}

#[test]
fn no_login_to_azure_devops_is_said_on_the_task_as_a_login_the_human_must_give() {
    let w = az_world(json!({}));
    w.az_says(json!({ "noLogin": true }));
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = w.add("No login");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.at("failure").s("title"), "Could not open the pull request", "{}", t.at("failure"));
    assert!(t.at("failure").s("detail").contains("az repos pr create") && t.at("failure").s("detail").contains("az login"), "{}", t.at("failure"));
    assert_eq!(t.at("failure").at("auth"), &json!(true), "{}", t.at("failure"));
}

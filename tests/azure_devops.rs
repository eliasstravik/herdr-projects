//! Azure DevOps read the board's way (azure/): which host an origin URL is at, a pull request's status and votes as
//! gh words them, its policies as checks, a build's failing job, and the main pipeline's runs and how far they are.
use crate::common;
use common::az::{policy, run};
use serde_json::json;
use sw::azure::{logs, parse, policies, pr, runs, Azure};
use sw::json::*;

fn az(org: &str, project: &str, repo: &str) -> Option<Azure> { Some(Azure { org: org.into(), project: project.into(), repo: repo.into() }) }

#[test]
fn an_origin_at_azure_devops_is_found_in_every_form_it_is_cloned_by_and_github_is_not_taken_for_it() {
    assert_eq!(parse("git@ssh.dev.azure.com:v3/acme/Shop/web"), az("acme", "Shop", "web"));
    assert_eq!(parse("ssh://git@ssh.dev.azure.com/v3/acme/Shop/web"), az("acme", "Shop", "web"));
    assert_eq!(parse("https://acme@dev.azure.com/acme/Big%20Project/_git/web"), az("acme", "Big Project", "web"));
    assert_eq!(parse("https://dev.azure.com/acme/Shop/_git/web/"), az("acme", "Shop", "web"));
    assert_eq!(parse("https://user:token@dev.azure.com/acme/Shop/_git/web"), az("acme", "Shop", "web"));
    assert_eq!(parse("https://dev.azure.com/acme/_git/Shop"), az("acme", "Shop", "Shop"), "a repo named as its project");
    assert_eq!(parse("https://acme.visualstudio.com/Shop/_git/web"), az("acme", "Shop", "web"));
    assert_eq!(parse("https://acme.visualstudio.com/DefaultCollection/Shop/_git/web"), az("acme", "Shop", "web"));
    assert_eq!(parse("acme@vs-ssh.visualstudio.com:v3/acme/Shop/web"), az("acme", "Shop", "web"));
    for other in ["git@github.com:o/r.git", "https://github.com/o/r", "/tmp/origin.git", "https://dev.azure.com/acme", "git@ssh.dev.azure.com:v3/acme/Shop", ""] { assert_eq!(parse(other), None, "{other}"); }
    let a = parse("https://acme@dev.azure.com/acme/Big%20Project/_git/web").unwrap();
    assert_eq!(a.org_url(), "https://dev.azure.com/acme");
    assert_eq!(a.pr_url("42"), "https://dev.azure.com/acme/Big%20Project/_git/web/pullrequest/42");
    assert_eq!(a.build_url("7"), "https://dev.azure.com/acme/Big%20Project/_build/results?buildId=7");
    assert_eq!(pr::id_of(&a.pr_url("42")), "42");
    assert!(pr::is_azure(&a.pr_url("42")) && !pr::is_azure("https://github.com/o/r/pull/7"));
    assert_eq!((sw::azure::numbered(&a.pr_url("42"), "42"), sw::azure::numbered("https://github.com/o/r/pull/7", "7")), ("!42".to_string(), "#7".to_string()));
}

#[test]
fn a_pull_request_reads_as_gh_says_one_its_status_its_votes_and_its_merge_commit_only_once_completed() {
    assert_eq!([pr::state_of("active"), pr::state_of("completed"), pr::state_of("abandoned")], ["OPEN", "MERGED", "CLOSED"]);
    let vote = |v: i64, req: bool| json!({ "vote": v, "isRequired": req });
    assert_eq!(pr::decision(&[vote(10, false)], false), "APPROVED");
    assert_eq!(pr::decision(&[vote(5, false), vote(-10, false)], false), "CHANGES_REQUESTED", "rejected");
    assert_eq!(pr::decision(&[vote(-5, false)], false), "CHANGES_REQUESTED", "waiting for the author");
    assert_eq!(pr::decision(&[vote(0, true), vote(10, false)], false), "REVIEW_REQUIRED", "a required reviewer has not voted");
    assert_eq!(pr::decision(&[vote(10, false)], true), "REVIEW_REQUIRED", "a policy of reviewers not met");
    assert_eq!(pr::decision(&[], false), "");
    let a = az("acme", "Shop", "web").unwrap();
    let raw = |status: &str| json!({ "pullRequestId": 42, "status": status, "sourceRefName": "refs/heads/work/1", "targetRefName": "refs/heads/develop", "lastMergeSourceCommit": { "commitId": "h" }, "lastMergeCommit": { "commitId": "m" }, "createdBy": { "uniqueName": "me@x" } });
    let open = pr::shaped(&a, &raw("active"));
    assert_eq!((open.s("state"), open.s("baseRefName"), open.s("headRefName"), open.s("headRefOid"), open.s("url")), ("OPEN", "develop", "work/1", "h", "https://dev.azure.com/acme/Shop/_git/web/pullrequest/42"));
    assert!(open.at("mergeCommit").is_null(), "an open one's merge commit is only Azure DevOps' trial merge");
    assert_eq!(pr::shaped(&a, &raw("completed")).at("mergeCommit").s("oid"), "m");
    assert_eq!([pr::strategy("squash"), pr::strategy("merge"), pr::strategy("rebase")], ["squash", "noFastForward", "rebase"]);
}

#[test]
fn its_policies_are_its_checks_builds_and_statuses_while_a_policy_of_people_is_a_review_still_required() {
    let a = az("acme", "Shop", "web").unwrap();
    let mut optional = policy("Build", "Lint", "rejected", Some(8)); optional["configuration"]["isBlocking"] = json!(false);
    let mut off = policy("Build", "Old", "rejected", None); off["configuration"]["isEnabled"] = json!(false);
    let mut status = policy("Status", "", "running", None); status["configuration"]["settings"] = json!({ "statusGenre": "sonar", "statusName": "quality" });
    let (checks, people) = policies::rollup(&a, &[policy("Build", "CI build", "rejected", Some(7)), optional, off, status, policy("Minimum number of reviewers", "", "approved", None)]);
    let seen: Vec<(String, String, String)> = checks.iter().map(|c| (c.t("name"), c.t("status"), c.t("conclusion"))).collect();
    assert_eq!(seen, vec![("CI build".into(), "COMPLETED".into(), "FAILURE".into()), ("Lint".into(), "COMPLETED".into(), "NEUTRAL".into()), ("sonar/quality".into(), "IN_PROGRESS".into(), "".into())]);
    assert_eq!(checks[0].s("buildId"), "7"); assert_eq!(checks[0].s("detailsUrl"), "https://dev.azure.com/acme/Shop/_build/results?buildId=7");
    assert!(!people);
    let (checks, people) = policies::rollup(&a, &[policy("Required reviewers", "", "queued", None), policy("Build", "CI build", "approved", Some(7))]);
    assert!(people, "a reviewer still to vote is a review required, not a check the agent can fix");
    assert_eq!(checks.len(), 1);
    assert!(matches!(sw::land::checks::sort(&checks), sw::land::checks::Look::Passed));
    let (queued, _) = policies::rollup(&a, &[policy("Build", "CI build", "queued", None)]);
    assert!(matches!(sw::land::checks::sort(&queued), sw::land::checks::Look::Running(_)));
    assert_eq!(sw::land::checks::waiting_words_on("Azure DevOps", &queued, &[], &[]), "Waiting for Azure DevOps checks · 0 of 1 passed");
}

#[test]
fn a_failed_build_names_its_failing_job_and_step_and_its_log_is_read_without_time_stamps() {
    let tl = json!({ "records": [
        { "id": "s", "type": "Stage", "name": "Build", "state": "completed", "result": "failed", "order": 1 },
        { "id": "j", "parentId": "s", "type": "Job", "name": "Unit tests", "state": "completed", "result": "failed", "order": 1 },
        { "id": "t1", "parentId": "j", "type": "Task", "name": "Checkout", "state": "completed", "result": "succeeded", "log": { "id": 2 }, "order": 1 },
        { "id": "t2", "parentId": "j", "type": "Task", "name": "npm test", "state": "completed", "result": "failed", "log": { "id": 5 }, "order": 2 } ] });
    let recs = logs::records_of(&tl);
    let failed = logs::failed(&recs);
    assert_eq!(failed.len(), 1);
    assert_eq!((failed[0].0.as_str(), failed[0].1.name.as_str(), failed[0].1.log.as_str()), ("Unit tests", "npm test", "5"));
    assert_eq!(logs::clean(&["2026-10-20T10:00:00.1234567Z \x1b[31merror\x1b[0m: boom".into(), "".into(), "done".into()]), vec!["error: boom".to_string(), "done".into()]);
}

#[test]
fn the_main_pipeline_s_runs_are_those_merge_pipeline_names_and_say_how_far_they_are() {
    let all: Vec<runs::Run> = [run(3, "CI", "c", "inProgress", ""), run(2, "Deploy", "c", "completed", "succeeded"), run(1, "CI", "c", "completed", "failed")].iter().map(runs::run_of).collect();
    let mut again = all.clone(); again[2].pipeline_id = again[0].pipeline_id.clone();
    assert_eq!(runs::wanted(&again, "all").iter().map(|r| r.id.clone()).collect::<Vec<_>>(), vec!["3", "2"], "a pipeline run again counts by its newest run");
    assert_eq!(runs::wanted(&all, "deploy").iter().map(|r| r.id.clone()).collect::<Vec<_>>(), vec!["2"], "by name, in any case");
    assert_eq!(runs::wanted(&all, "30").iter().map(|r| r.id.clone()).collect::<Vec<_>>(), vec!["3"], "by id");
    assert_eq!(runs::not_started(&all, "CI, Release"), vec!["Release".to_string()]);
    assert!(runs::not_started(&all, "all").is_empty());
    assert!(runs::passed(&all[1]) && runs::failed(&all[2]) && !runs::passed(&all[0]) && !runs::failed(&all[0]));
    assert!(runs::passed(&runs::run_of(&run(4, "CI", "c", "completed", "partiallySucceeded"))));
    assert!(runs::failed(&runs::run_of(&run(4, "CI", "c", "completed", "canceled"))));
    let st = |n: &str, state: &str| runs::Stage { name: n.into(), done: state == "completed", running: state == "inProgress" };
    assert_eq!(runs::progress(&[st("Build", "completed"), st("Test", "completed"), st("Package", "inProgress")], 1), "Main pipeline running · 2 of 3 stages");
    assert_eq!(runs::progress(&[st("Build", "completed"), st("Deploy_Staging", "inProgress"), st("Deploy_Prod", "pending")], 1), "Deploying Staging");
    assert_eq!(runs::progress(&[st("Deployment", "inProgress")], 1), "Deploying");
    assert_eq!(runs::progress(&[], 2), "Main pipelines running");
}

//! The main pipeline's gate on Azure DevOps (land/main_pipeline.rs): a failed or cancelled run of the merge commit
//! waits for you with the failing job and the end of its log, `fix` sends it to the agent and its next finish opens a
//! new pull request, `retry` looks at the runs again; a named pipeline is waited for until it starts, and past
//! `merge.pipelineTimeout` the task waits for you; with no pipeline at all it is done.
use crate::common;
use common::az::{policy, run};
use common::*;
use serde_json::{json, Value};
use std::path::Path;
use sw::actions::finish_line;
use sw::housekeeping::poll_prs;
use sw::json::*;
use super::azure_devops_flow::{az_world, merge_commit};

/// To approval, approved, its pull request completed: the task waits for its main pipeline.
fn merged(w: &World, title: &str) -> (String, String) {
    let id = to_approval(w, title);
    let head = git(Path::new(task(&id).s("worktree")), &["rev-parse", "HEAD"]);
    w.az_says(json!({ "head": head, "policies": [policy("Build", "CI build", "approved", Some(7))] }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert!(t.b("pipelineWait") && t.s("stage") == "merging", "{} {}", t.s("msg"), t.at("failure"));
    (id, head)
}
fn says(w: &World, head: &str, more: Value) { let mut c = json!({ "head": head, "status": "completed" }); for (k, v) in more.as_object().cloned().unwrap_or_default() { c[k] = v; } w.az_says(c); }

#[test]
fn a_failed_main_pipeline_waits_for_you_with_its_failing_job_fix_sends_it_to_the_agent_whose_next_finish_opens_a_new_pull_request() {
    let w = az_world(json!({}));
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }, { "files": { "f.txt": "v2\n" }, "commit": "v2", "note": "Fixed the deploy." }] }));
    let (id, head) = merged(&w, "Deploys");
    let tl = json!({ "records": [{ "id": "s", "type": "Stage", "name": "Deploy prod", "state": "completed", "result": "failed", "order": 1 },
        { "id": "j", "parentId": "s", "type": "Job", "name": "Deploy web", "state": "completed", "result": "failed", "order": 1 },
        { "id": "t", "parentId": "j", "type": "Task", "name": "kubectl apply", "state": "completed", "result": "failed", "log": { "id": 3 }, "order": 1 }] });
    says(&w, &head, json!({ "runs": [run(5, "CI", &merge_commit(), "completed", "failed")], "timeline": { "5": tl }, "log": ["2026-10-20T10:00:00.0000000Z applying", "error: image not found"] }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed", "{}", t.s("msg"));
    assert_eq!(t.at("failure").s("stage"), "pipeline");
    assert_eq!(t.at("failure").s("title"), "Main pipeline failed: The pipeline CI failed (run 5)");
    let d = t.at("failure").s("detail");
    assert!(d.contains("Failed in job Deploy web, step kubectl apply") && d.contains("error: image not found") && !d.contains("2026-10-20T10"), "{d}");
    assert_eq!(t.at("failure").at("agentCanFix"), &json!(true));
    assert!(t.b("pipelineFailed") && Path::new(t.s("worktree")).exists(), "nothing cleaned up: the agent fixes it there");
    assert_ne!(sw::flow::live::board(true).iter().find(|r| r.s("id") == id).map(|r| r.t("state")).unwrap_or_default(), "done");
    // `fix`: the agent hears it, fixes it, and its finish opens a new pull request for the fix
    w.az_says(json!({ "head": head }));
    finish_line::send_failure(&id).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    assert!(!t.b("prMerged") && !t.b("pipelineFailed"), "{}", t.at("pr"));
    assert_eq!(az_log(&w).matches("repos pr create").count(), 2, "{}", az_log(&w));
    assert!(prompts(&w, t.at("link").s("pane")).iter().any(|p| p.contains("error: image not found")), "the agent was told");
}

#[test]
fn retry_looks_at_the_main_pipeline_again_after_it_was_run_again_in_azure_devops() {
    let w = az_world(json!({}));
    let (id, head) = merged(&w, "Flaky");
    says(&w, &head, json!({ "runs": [run(5, "CI", &merge_commit(), "completed", "canceled")] }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.at("failure").s("title"), "Main pipeline failed: The pipeline CI was cancelled (run 5)");
    // run again in Azure DevOps: the newest run of the pipeline counts
    let mut again = run(6, "CI", &merge_commit(), "completed", "succeeded"); again["definition"]["id"] = json!(50);
    says(&w, &head, json!({ "runs": [again, run(5, "CI", &merge_commit(), "completed", "canceled")] }));
    finish_line::retry(&id).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
    assert!(t.b("cleaned"));
}

#[test]
fn a_named_pipeline_is_waited_for_until_it_starts_and_past_its_timeout_the_task_waits_for_you() {
    let w = az_world(json!({ "merge.pipeline": "Release", "merge.pipelineTimeout": "30m" }));
    let (id, head) = merged(&w, "Named");
    says(&w, &head, json!({ "runs": [run(5, "CI", &merge_commit(), "completed", "succeeded")] }));
    poll_prs();
    assert_eq!(task(&id).s("msg"), "Merged. Waiting for the pipeline Release to start");
    sw::clock::advance(31 * 60000);
    poll_prs();
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.at("failure").s("title")), ("failed", "The main pipeline did not finish"));
    assert!(t.at("failure").s("detail").contains("Release not started"), "{}", t.at("failure"));
    assert_eq!(t.at("failure").at("agentCanFix"), &json!(false));
}

#[test]
fn with_no_pipeline_in_the_repo_or_none_run_for_the_commit_the_task_is_done() {
    let w = az_world(json!({}));
    let (id, head) = merged(&w, "No pipeline");
    says(&w, &head, json!({ "pipelines": [] }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.s("msg"));
    assert!(t.s("msg").contains("No pipeline in this repo to wait for"), "{}", t.s("msg"));
    w.az.completed.set(false);
    w.script(json!({ "worker": [{ "files": { "g.txt": "v1\n" }, "commit": "g", "note": "Did g." }] }));
    let (id, head) = merged(&w, "Not triggered");
    says(&w, &head, json!({}));
    poll_prs();
    assert_eq!(task(&id).s("stage"), "merging");
    sw::clock::advance(11 * 60000);
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.s("msg"));
    assert!(t.s("msg").contains("No pipeline ran for the merge commit on main"), "{}", t.s("msg"));
}

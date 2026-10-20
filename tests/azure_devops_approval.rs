//! A run on Azure Pipelines held at an approval (its environment's check, `Checkpoint.Approval` under way) waits for
//! a person: the task waits for you, saying which build or pipeline and at which stage, and no timeout runs out on it
//! meanwhile. Once it is approved the wait goes on as before. Before the merge (a build validation of the pull
//! request) and after it (the main pipeline).
use crate::common;
use common::az::{policy, run};
use common::*;
use serde_json::{json, Value};
use std::path::Path;
use sw::actions::finish_line;
use sw::azure::{logs, runs};
use sw::housekeeping::poll_prs;
use sw::json::*;
use super::azure_devops_flow::{az_world, merge_commit};

/// A run's timeline as Azure Pipelines gives it: its stage `Staging` held at an approval before its deployment job.
fn held_at_staging() -> Value {
    let rec = |id: &str, parent: &str, kind: &str, name: &str, state: &str| json!({ "id": id, "parentId": parent, "type": kind, "name": name, "state": state });
    json!({ "records": [rec("s1", "", "Stage", "Build & Push", "completed"), rec("s2", "", "Stage", "Staging", "pending"),
        rec("c2", "s2", "Checkpoint", "Checkpoint", "inProgress"), rec("a2", "c2", "Checkpoint.Approval", "Checkpoint.Approval", "inProgress"),
        rec("e2", "c2", "Checkpoint.ExtendsCheck", "Checkpoint.ExtendsCheck", "pending"), rec("p2", "s2", "Phase", "Deploy", "pending")] })
}
fn state(id: &str) -> (String, String) { let r = row(id); (r.t("state"), r.t("kind")) }

#[test]
fn a_stage_held_at_an_approval_is_found_under_its_checkpoint() {
    assert_eq!(runs::approvals_of(&logs::records_of(&held_at_staging())), vec!["Staging".to_string()]);
    let mut through = held_at_staging();
    through["records"][3]["state"] = json!("completed");
    assert!(runs::approvals_of(&logs::records_of(&through)).is_empty(), "approved: nothing waits for you");
}

#[test]
fn a_build_validation_held_at_an_approval_makes_the_merge_wait_for_you_with_no_timeout_then_it_merges_once_approved() {
    let w = az_world(json!({ "merge.pipeline": "none" }));
    let id = to_approval(&w, "Needs a yes in Azure");
    let head = git(Path::new(task(&id).s("worktree")), &["rev-parse", "HEAD"]);
    w.az_says(json!({ "head": head, "policies": [policy("Build", "reports", "running", Some(7))], "timeline": { "7": held_at_staging() } }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("msg")), ("merging", "Waiting for your approval in Azure DevOps: the build reports, at Staging"), "{}", t.at("failure"));
    assert_eq!(state(&id), ("blocked".to_string(), "permission".to_string()), "it waits for you, not for the machine");
    // the approval is Azure DevOps' to give: the board offers no yes of its own, and its yes says where to give it
    let answer = format!("{} waits for your approval in Azure DevOps (the build reports, at Staging), not for a yes here: approve it there and the board goes on by itself.", task(&id).s("id"));
    assert_eq!(finish_line::approve(&id, None, false).err().map(|e| e.msg), Some(answer.clone()));
    let mut b = common::board::open(&w.root, 44);
    b.s.sel = id.clone(); b.load();
    let text = b.text();
    assert!(!text.contains("say yes to this task's agent") && !text.contains(&format!("{} ", sw::board::keys::key_name("yes"))), "{text}");
    b.press(&["ctrl+l"]);
    assert!(b.text().replace('\n', " ").contains("not for a yes here"), "{}", b.text());
    assert_eq!(task(&id).s("stage"), "merging", "nothing merged, nothing changed");
    // far past check.timeout: still yours to approve, nothing gave up
    sw::clock::advance(40 * 60000);
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging", "{} {}", t.s("msg"), t.at("failure"));
    assert!(t.at("failure").is_null() && t.at("checksWait").b("approval"));
    // approved in Azure DevOps: the build runs on, and the row says so again
    let mut going = held_at_staging(); going["records"][3]["state"] = json!("completed");
    w.az_says(json!({ "head": head, "policies": [policy("Build", "reports", "running", Some(7))], "timeline": { "7": going } }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("msg"), "Waiting for Azure DevOps checks · 0 of 1 passed");
    assert_eq!(state(&id).0, "working");
    assert!(!sw::land::checks::overdue(&t), "check.timeout counts from your yes, not from when the wait began");
    // it passed: completed
    w.az_says(json!({ "head": head, "policies": [policy("Build", "reports", "approved", Some(7))] }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
}

#[test]
fn a_main_pipeline_held_at_an_approval_waits_for_you_past_its_timeout() {
    let w = az_world(json!({ "merge.pipelineTimeout": "30m" }));
    let id = to_approval(&w, "Deploy needs a yes");
    let head = git(Path::new(task(&id).s("worktree")), &["rev-parse", "HEAD"]);
    w.az_says(json!({ "head": head }));
    finish_line::approve(&id, None, false).unwrap();
    assert!(settle(&id).b("pipelineWait"));
    w.az_says(json!({ "head": head, "status": "completed", "runs": [run(5, "reports", &merge_commit(), "inProgress", "")], "timeline": { "5": held_at_staging() } }));
    poll_prs();
    let t = task(&id);
    assert_eq!(t.s("msg"), "Waiting for your approval in Azure DevOps: the pipeline reports, at Staging");
    assert_eq!(state(&id), ("blocked".to_string(), "permission".to_string()));
    sw::clock::advance(45 * 60000);
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging", "no timeout while it waits for a person: {}", t.at("failure"));
    w.az_says(json!({ "head": head, "status": "completed", "runs": [run(5, "reports", &merge_commit(), "completed", "succeeded")] }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
}

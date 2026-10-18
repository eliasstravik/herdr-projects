//! GitLab as GitHub, all the way: what the agent and you read says merge request, pipeline and glab, never gh; a
//! merge with `merge.method rebase` rebases first; and GitLab's pipeline guards the merge for the shorter check.
use crate::common;
use common::glab::*;
use common::*;
use serde_json::{json, Value};
use sw::actions::finish_line;
use sw::json::*;

#[test]
fn on_gitlab_what_the_agent_and_you_read_says_merge_request_pipeline_and_glab_never_gh_or_github() {
    let (w, lab) = lab_world(json!({ "review.rounds": 0 }), "git@gitlab.com:o/r.git");
    let id = to_approval(&w, "Words on GitLab");
    assert_eq!(sw::shown::words::pr_number(&task(&id)), "MR!3", "the row names it as GitLab does");
    lab.borrow_mut().jobs = vec![job("test", "running")];
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert!(t.s("msg").starts_with("Waiting for GitLab pipeline checks"), "{}", t.s("msg"));
    { let mut l = lab.borrow_mut(); l.jobs = vec![job("test", "failed")]; l.trace = TRACE.into(); }
    sw::housekeeping::poll_prs();
    let t = settle(&id);
    let f = t.at("prFailure");
    let said = format!("{} {} {} {}", t.s("msg"), f.s("title"), f.s("detail"), f.s("next"));
    assert!(said.contains("merge request") && said.contains("glab ci status"), "{said}");
    assert!(!regex::Regex::new(r"\bgh\b").unwrap().is_match(&said) && !said.contains("GitHub") && !said.to_lowercase().contains("pull request"), "{said}");
    assert!(f.s("detail").contains("pipeline (failure):\nThe pipeline as a whole: its jobs below say what failed."), "{}", f.s("detail"));
    let e = finish_line::approve(&id, None, false).unwrap_err().msg;
    assert!(e.contains("GitLab's pipeline checks failed on its merge request: pipeline, test"), "{e}");
}

#[test]
fn merge_method_rebase_on_gitlab_rebases_the_merge_request_then_merges_it_without_squashing() {
    let (w, lab) = lab_world(json!({ "merge.method": "rebase" }), "git@gitlab.com:o/r.git");
    let id = to_approval(&w, "Rebased on GitLab");
    lab.borrow_mut().rebase_to = "d".repeat(40);
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}\n{}", t.at("failure"), lab_calls(&lab));
    let c = lab_calls(&lab);
    assert!(c.contains("-X PUT projects/:id/merge_requests/3/rebase"), "{c}");
    assert!(c.contains(&format!("-X PUT projects/:id/merge_requests/3/merge -f sha={} -F squash=false -F should_remove_source_branch=true", "d".repeat(40))), "merged at the rebased commit: {c}");
}

fn finding() -> Value { json!({ "severity": "critical", "subject": "It does not do it", "where": "f.txt", "proof": "ran it" }) }
fn verdict(findings: Value) -> Value { json!({ "verdict": { "meets": true, "text": "Looked at it.", "findings": findings } }) }
fn step_with(n: i64, ci: bool) -> Value {
    let mut files = json!({ "f.txt": format!("{n}\n") });
    if ci { files[".gitlab-ci.yml"] = json!("test:\n  script: [\"true\"]\n"); }
    json!({ "files": files, "commit": format!("v{n}"), "note": format!("Did v{n}.") })
}
/// Done once, sent back by the review once, done again, on GitLab: which checks ran (`full`, or `again <commit>`).
fn rounds(ci: bool) -> (Vec<String>, String) {
    let (w, _lab) = lab_world_with(Opts { repo: true, origin: true, settings: json!({}), repo_settings: Some(json!({ "check": {
        "command": "echo full >> \"$HOME/checks.log\"", "again": "echo \"again $HERDR_PROJECTS_CHECKED\" >> \"$HOME/checks.log\"" } })) }, "git@gitlab.com:o/r.git");
    w.script(json!({ "worker": [step_with(1, ci), step_with(2, false)], "reviewer": [verdict(json!([finding()])), verdict(json!([]))] }));
    let id = w.add("Fix it");
    delegate(&id);
    let mut t = settle(&id);
    for _ in 0..3 { if t.s("stage") != "approval" { t = settle(&id); } }
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    (read(&w.base.join("home/checks.log")).lines().map(String::from).collect(), t.at("checkPassed").t("head"))
}

#[test]
fn a_gitlab_pipeline_guards_the_merge_so_a_fix_round_runs_the_shorter_check_and_without_one_the_full_check() {
    let (ran, first) = rounds(true);
    assert_eq!(ran, vec!["full".to_string(), format!("again {first}")]);
    assert_eq!(rounds(false).0, vec!["full", "full"], "no .gitlab-ci.yml: nothing on GitLab runs the full suite");
}

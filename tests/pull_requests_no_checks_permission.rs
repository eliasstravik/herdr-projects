//! GitHub's checks read with a fine-grained token, which cannot have the Checks permission (land/readable.rs): GitHub
//! refuses it `statusCheckRollup` and the check-runs API, so the checks are read from GitHub Actions (its runs' jobs,
//! with their logs) and commit statuses. Passed, failed, still running, no CI at all; what cannot be read is said on the
//! task, and nothing asks you for a login.
use crate::common;
use crate::pull_requests_checks::{merged, none, pane_prompts, pr_world, LOG, waits};
use common::*;
use serde_json::{json, Value};
use sw::actions::finish_line;
use sw::housekeeping::poll_prs;
use sw::json::*;

const RUN: &str = "https://github.com/o/r/actions/runs/5";
fn job(name: &str, status: &str, conclusion: &str, n: i64) -> Value { json!([name, status, conclusion, format!("{RUN}/job/{n}")]) }
/// What a fine-grained token reads of the commit: one Actions run with its jobs, and the commit's statuses.
fn says(head: &str, run: (&str, &str), jobs: Value, statuses: Value) -> Value {
    json!({ "noChecks": true, "view": none(head), "runs": [[".github/workflows/ci.yml", run.0, run.1, RUN]], "jobs": { "5": jobs }, "statuses": statuses, "log": LOG })
}
fn no_login_asked(t: &Obj, id: &str) {
    assert!(!t.at("failure").b("auth"), "{}", t.at("failure"));
    assert_ne!(row(id).s("kind"), "auth");
    assert!(!t.t("msg").contains("Checks permission") && !t.at("failure").t("next").contains("Checks permission"), "{}", t.at("failure"));
}

#[test]
fn fallback_status_history_merges_only_when_newest_of_same_context_passed() {
    // Real fine-grained-token route: the statuses API returns both records for one context on this head.
    let w = pr_world(json!({ "review.rounds": 0 }));
    let id = to_approval(&w, "Status rerun");
    let head = task(&id).t("prHead");
    let mut api = says(&head, ("completed", "success"), json!([]), json!([
        ["deploy", "failure", "", "41", "2026-10-24T01:00:00Z"],
        ["deploy", "success", "", "42", "2026-10-24T02:00:00Z"]
    ]));
    api["runs"] = json!([]);
    w.gh_says(api);
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.at("failure"), gh_log(&w));
    assert!(merged(&w));
    assert!(gh_log(&w).contains("commits/"), "statuses must come through the fallback API");
    drop(w);

    let w = pr_world(json!({ "review.rounds": 0 }));
    let id = to_approval(&w, "New status fails");
    let head = task(&id).t("prHead");
    let mut api = says(&head, ("completed", "success"), json!([]), json!([
        ["deploy", "failure", "", "44", "2026-10-24T03:00:00Z"],
        ["deploy", "success", "", "43", "2026-10-24T02:00:00Z"]
    ]));
    api["runs"] = json!([]);
    w.gh_says(api);
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(strings(t.at("prFailure").at("checks")), vec!["deploy"]);
    assert!(!merged(&w));
}

#[test]
fn fallback_job_history_keeps_latest_pending_and_old_failure_does_not_override_new_success() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Rerun jobs");
    let head = task(&id).t("prHead");
    let make = |new_status: &str, new_conclusion: &str| json!({
        "noChecks": true, "view": none(&head), "statuses": [],
        "runs": [
            [".github/workflows/ci.yml", "completed", "failure", "https://github.com/o/r/actions/runs/5", "CI", "pull_request", "2026-10-24T01:00:00Z"],
            [".github/workflows/ci.yml", new_status, new_conclusion, "https://github.com/o/r/actions/runs/6", "CI", "pull_request", "2026-10-24T02:00:00Z"]
        ],
        "jobs": {
            "5": [["build", "completed", "failure", "https://github.com/o/r/actions/runs/5/job/50", "50", "2026-10-24T01:00:00Z"]],
            "6": [["build", new_status, new_conclusion, "https://github.com/o/r/actions/runs/6/job/60", "60", "2026-10-24T02:00:00Z"]]
        }
    });
    w.gh_says(make("in_progress", ""));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging", "{}", t.at("failure"));
    assert!(!merged(&w), "latest pending still waits");
    w.gh_says(make("completed", "success"));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.at("failure"), gh_log(&w));
    assert!(merged(&w), "newest run and job passed on same head");
}

#[test]
fn passed_jobs_and_statuses_merge_and_the_task_says_what_it_could_not_read() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Fine-grained, green");
    let head = task(&id).t("prHead");
    w.gh_says(says(&head, ("completed", "success"), json!([job("build", "completed", "success", 9), job("test (ubuntu)", "completed", "success", 10)]), json!([["Vercel", "success", "https://vercel.com/x"]])));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
    no_login_asked(&t, &id);
    assert!(t.s("msg").starts_with("Merged: ") && t.s("msg").contains("Not read: check runs of apps other than GitHub Actions"), "{}", t.s("msg"));
    // every page of runs, jobs and statuses, not only the first hundred
    let log = gh_log(&w);
    for asked in ["actions/runs?head_sha=", "actions/runs/5/jobs", &format!("commits/{head}/status")] {
        assert!(log.lines().any(|l| l.starts_with("api --paginate ") && l.contains(asked)), "{asked}: {log}");
    }
}

#[test]
fn a_run_still_going_or_failed_is_judged_as_itself_whatever_its_jobs_listed_so_far_say() {
    // a run no workflow of the branch predicts (dispatched by hand on the commit): one job passed, the run goes on
    let w = pr_world(json!({ "review.rounds": 0 }));
    let id = to_approval(&w, "Fine-grained, run goes on");
    let head = task(&id).t("prHead");
    w.gh_says(says(&head, ("in_progress", ""), json!([job("build", "completed", "success", 9)]), json!([])));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging", "{}", t.at("failure")); waits(&t, "Waiting for GitHub checks · 1 of 2 passed", "CI"); assert!(!merged(&w));
    poll_prs(); settle(&id);
    assert!(!merged(&w), "the job that passed is not the run: {}", gh_log(&w));
    // the run failed though every job listed passed (one GitHub did not list): nothing merges
    w.gh_says(says(&head, ("completed", "failure"), json!([job("build", "completed", "success", 9)]), json!([])));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval"); assert!(!merged(&w));
    assert_eq!(strings(t.at("prFailure").at("checks")), vec!["CI".to_string()]);
}

#[test]
fn a_failed_job_goes_back_to_the_agent_with_its_log_and_a_failed_status_fails_too() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Fine-grained, red");
    let head = task(&id).t("prHead");
    w.script(json!({ "worker": [{ "files": { "f.txt": "fixed\n" }, "commit": "fix", "note": "Fixed CI." }] }));
    w.gh_says(says(&head, ("completed", "failure"), json!([job("build", "completed", "success", 9), job("test (macos)", "completed", "failure", 10)]), json!([])));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    let sent = pane_prompts(&w, &id).last().cloned().unwrap_or_default();
    assert!(sent.contains("GitHub's checks failed on the pull request") && sent.contains("test (macos) (failure)") && sent.contains("test parses_flags ... FAILED"), "{sent}");
    assert!(gh_log(&w).contains("run view --job 10 --log-failed"), "{}", gh_log(&w));
    assert!(!merged(&w)); no_login_asked(&t, &id);
    drop(w); // one world at a time: each holds the tests' one lock
    // an outside CI's status failed: nothing merges, and with no rounds it waits for you
    let w = pr_world(json!({ "review.rounds": 0 }));
    let id = to_approval(&w, "Fine-grained, preview fails");
    let head = task(&id).t("prHead");
    w.gh_says(says(&head, ("completed", "success"), json!([job("build", "completed", "success", 9)]), json!([["Vercel", "failure", "https://vercel.com/x"]])));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval"); assert!(!merged(&w));
    assert_eq!(strings(t.at("prFailure").at("checks")), vec!["Vercel".to_string()]);
    no_login_asked(&t, &id);
}

#[test]
fn running_jobs_and_pending_statuses_are_waited_for_then_it_merges_and_github_is_not_asked_for_its_list_again() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Fine-grained, pending");
    let head = task(&id).t("prHead");
    // the run is queued with no job yet: waited for as itself, beside the pending status
    w.gh_says(says(&head, ("queued", ""), json!([]), json!([["Vercel", "pending", ""]])));
    let from = gh_log(&w).lines().count();
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging", "{}", t.at("failure")); waits(&t, "Waiting for GitHub checks · 0 of 2 passed", "CI, Vercel"); assert!(!merged(&w));
    no_login_asked(&t, &id);
    w.gh_says(says(&head, ("in_progress", ""), json!([job("build", "in_progress", "", 9)]), json!([["Vercel", "pending", ""]])));
    poll_prs(); let t = settle(&id);
    assert_eq!(t.s("stage"), "merging", "still running: still waiting"); waits(&t, "Waiting for GitHub checks · 0 of 2 passed", "CI, build, Vercel"); assert!(!merged(&w));
    w.gh_says(says(&head, ("completed", "success"), json!([job("build", "completed", "success", 9)]), json!([["Vercel", "success", ""]])));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
    assert_eq!(gh_log(&w).lines().skip(from).filter(|l| l.starts_with("pr view") && l.contains("statusCheckRollup")).count(), 1, "refused once, not asked again: {}", gh_log(&w));
}

#[test]
fn no_ci_at_all_merges_at_once_and_a_required_check_no_token_can_read_is_named_not_waited_for() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Fine-grained, no CI");
    let head = task(&id).t("prHead");
    // nothing runs, no status, and main has none: the check runs of main cannot be asked either, and that is no "yes"
    w.gh_says(json!({ "noChecks": true, "view": none(&head) }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.at("failure"), gh_log(&w)); assert!(merged(&w)); no_login_asked(&t, &id);
    drop(w);
    // the branch requires a check of another app: waited for while the job runs, then named on the task as not read
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Fine-grained, required app");
    let head = task(&id).t("prHead");
    let mut s = says(&head, ("in_progress", ""), json!([job("build", "in_progress", "", 9)]), json!([])); s["required"] = json!(["build", "app/scan"]);
    w.gh_says(s);
    finish_line::approve(&id, None, false).unwrap();
    waits(&settle(&id), "Waiting for GitHub checks · 0 of 2 passed", "CI, build, app/scan");
    let mut s = says(&head, ("completed", "success"), json!([job("build", "completed", "success", 9)]), json!([])); s["required"] = json!(["build", "app/scan"]);
    w.gh_says(s);
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
    assert!(t.s("msg").contains("the required check app/scan (listed by no GitHub Actions job and no commit status)"), "{}", t.s("msg"));
}

#[test]
fn a_token_that_may_not_read_actions_either_is_told_which_permissions_it_needs_none_of_them_checks() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Fine-grained, no Actions");
    let head = task(&id).t("prHead");
    w.gh_says(json!({ "noChecks": true, "actionsFail": true, "view": none(&head) }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert!(!merged(&w));
    let f = t.at("failure");
    assert_eq!(f.s("title"), "Could not read GitHub's checks");
    assert!(f.s("next").contains("read access to Actions and to Commit statuses") && f.s("next").contains("cannot have the Checks permission"), "{f}");
    assert!(f.s("detail").contains("GitHub Actions runs"), "{f}");
}

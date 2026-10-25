//! A merge waiting for GitHub's checks read from Actions only (a fine-grained token), as on this repo: the workflow
//! "Check" with its job "check", run for the push and for the pull request. All passed, it merges; after a keeper
//! restarted without gh's login it says it cannot read them, and goes on once gh reads again; names remembered for an
//! earlier commit, or read another way, never hold it.
use crate::common;
use crate::pull_requests_checks::{merged, none, pr_world, waits};
use common::*;
use serde_json::{json, Value};
use sw::actions::finish_line;
use sw::housekeeping::poll_prs;
use sw::json::*;
use sw::login_env::{gh_login, path_of};

const CI: &str = ".github/workflows/ci.yml";
const FLOW: &str = "name: Check\non:\n  push:\n    branches: [\"**\"]\n  pull_request:\njobs:\n  check:\n    runs-on: ubuntu-24.04\n";
fn url(run: i64) -> String { format!("https://github.com/o/r/actions/runs/{run}") }
/// What a fine-grained token reads of the commit: the workflow's run for the push (5) and for the pull request (6),
/// each with its one job, as `(status, conclusion)`.
fn actions(head: &str, push: (&str, &str), pull: (&str, &str)) -> Value {
    let job = |run: i64, (s, c): (&str, &str)| json!([["check", s, c, format!("{}/job/{}", url(run), run * 10), format!("{}", run * 10), if run == 5 { "2026-10-24T01:00:00Z" } else { "2026-10-24T02:00:00Z" }]]);
    json!({ "noChecks": true, "view": none(head), "statuses": [],
        "runs": [[CI, push.0, push.1, url(5), "Check", "push", "2026-10-24T01:00:00Z"], [CI, pull.0, pull.1, url(6), "Check", "pull_request", "2026-10-24T02:00:00Z"]],
        "jobs": { "5": job(5, push), "6": job(6, pull) } })
}
const GOING: (&str, &str) = ("in_progress", "");
const PASSED: (&str, &str) = ("completed", "success");
/// A task approved while both runs are going: it waits, and says for what, each name once.
fn waiting(w: &World, title: &str) -> (String, String) {
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n", CI: FLOW }, "commit": "v1", "note": "Did v1." }] }));
    let id = to_approval(w, title);
    let head = task(&id).t("prHead");
    w.gh_says(actions(&head, GOING, GOING));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging", "{}", t.at("failure"));
    waits(&t, "Waiting for GitHub checks · 0 of 1 passed", "Check");
    assert_eq!(t.at("checksWait").s("via"), "actions");
    assert!(!merged(w));
    (id, head)
}

#[test]
fn actions_only_a_workflow_check_with_job_check_for_push_and_pull_request_merges_once_all_passed() {
    let w = pr_world(json!({}));
    let (id, head) = waiting(&w, "Check and check");
    poll_prs(); settle(&id);
    assert!(!merged(&w), "still going: {}", gh_log(&w));
    // the push's run passed, the pull request's still goes: still waiting
    w.gh_says(actions(&head, PASSED, GOING));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging"); waits(&t, "Waiting for GitHub checks · 0 of 1 passed", "Check"); assert!(!merged(&w));
    w.gh_says(actions(&head, PASSED, PASSED));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w)); assert!(!t.b("checksWait"));
    drop(w); // one world at a time: each holds the tests' one lock
    // all passed already when it is approved: it merges at once
    let w = pr_world(json!({}));
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n", CI: FLOW }, "commit": "v1", "note": "Did v1." }] }));
    let id = to_approval(&w, "Check passed before");
    let head = task(&id).t("prHead");
    w.gh_says(actions(&head, PASSED, PASSED));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
}

#[test]
fn after_a_restart_without_gh_login_a_waiting_merge_says_so_and_goes_on_once_gh_reads_again() {
    let w = pr_world(json!({}));
    let (id, head) = waiting(&w, "Keeper restarted");
    // the keeper came back without the login shell's GH_TOKEN: gh cannot read anything, and the task says so
    w.gh_says(json!({ "noLogin": true }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging"); assert!(!merged(&w));
    assert_eq!(t.s("msg"), "Waiting for GitHub checks · could not read them (To get started with GitHub CLI, please run:  gh auth login)");
    // gh reads again (its login taken from the login shell, login_env.rs): the merge is looked at again and goes on
    w.gh_says(actions(&head, PASSED, PASSED));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
    drop(w);
    // still unreadable past `check.timeout`: it does not wait forever, the land step says why nothing merged
    let w = pr_world(json!({}));
    let (id, _) = waiting(&w, "Never logged in again");
    w.gh_says(json!({ "noLogin": true }));
    poll_prs(); settle(&id);
    let mut cw = task(&id).at("checksWait").clone(); cw["since"] = json!(sw::clock::now() - 16 * 60000); // past `check.timeout`
    sw::store::tasks::set(&id, json!({ "checksWait": cw })).unwrap();
    poll_prs();
    let t = settle(&id);
    assert!(!merged(&w)); assert!(!t.b("autoMerge"));
    assert_eq!(t.at("failure").s("title"), "Could not read GitHub's checks", "{}", t.at("failure"));
    assert!(t.at("failure").s("detail").contains("gh auth login"), "{}", t.at("failure"));
}

#[test]
fn names_remembered_for_an_earlier_commit_or_read_another_way_never_hold_a_merge() {
    // remembered for an earlier commit: a check of it that is not listed now is not waited for
    let w = pr_world(json!({}));
    let (id, head) = waiting(&w, "Earlier head");
    let old = json!({ "since": sw::clock::now(), "head": "0123456789abcdef0123456789abcdef01234567", "via": "actions", "for": task(&id).at("mergeIntent").at("approvedAt"),
        "names": ["Check", "check", "lint"], "flows": ["Check"], "seen": ["Check", "check", "lint"] });
    sw::store::tasks::set(&id, json!({ "checksWait": old })).unwrap();
    w.gh_says(actions(&head, PASSED, PASSED));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
    drop(w);
    // read another way since: GitHub's own list names the job "check" only, never the run "Check" read from Actions
    let w = pr_world(json!({}));
    let (id, head) = waiting(&w, "Read another way");
    poll_prs(); settle(&id);
    let job = |n: i64| json!({ "__typename": "CheckRun", "name": "check", "status": "COMPLETED", "conclusion": "SUCCESS", "workflowName": "Check", "detailsUrl": format!("{}/job/{}", url(n), n * 10) });
    let mut v = none(&head); v["statusCheckRollup"] = json!([job(5), job(6)]);
    w.gh_says(json!({ "view": v, "runs": [[CI, "completed", "success", url(5), "Check"], [CI, "completed", "success", url(6), "Check"]] }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
}

#[test]
fn a_keeper_started_without_gh_login_takes_it_from_the_login_shell_and_a_process_with_its_own_keeps_it() {
    let out = "Welcome to the machine\nPATH=/a/bin:/b/bin\nGH_HOST=github.localhost\nHOME=/home/me\nGH_ENTERPRISE_TOKEN=t0k\nGH_TOKEN=\n";
    let pair = |k: &str, v: &str| (k.to_string(), v.to_string());
    assert_eq!(gh_login(out, |_| false), vec![pair("GH_HOST", "github.localhost"), pair("GH_ENTERPRISE_TOKEN", "t0k")]);
    assert!(gh_login(out, |k| k == "GH_TOKEN").is_empty(), "a login of its own is kept");
    assert!(gh_login("PATH=/x\n", |_| false).is_empty());
    assert_eq!(path_of(out), "/a/bin:/b/bin");
}

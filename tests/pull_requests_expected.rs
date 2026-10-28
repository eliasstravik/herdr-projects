//! GitHub's checks the pull request is to get, waited for before GitHub lists them (land/expected.rs): each workflow
//! that runs for the pull request (its triggers as GitHub reads them: events, branches, paths) until its run on the
//! commit is through, every job and matrix member of it; a check the branch requires; a check gone from the list; with
//! nothing listed, an outside CI that reports in the repo; and a repo with no checks at all, which merges at once. A
//! workflow whose trigger types may leave this commit out is waited for only a short while to start (#420); none is
//! where the repo has GitHub Actions turned off (#416).
use crate::common;
use crate::pull_requests_checks::{merged, none, passed, pr_world, running, view, waits};
use common::*;
use serde_json::{json, Value};
use sw::actions::finish_line;
use sw::housekeeping::poll_prs;
use sw::json::*;
use sw::land::expected::MAYBE_MS;

const CI: &str = ".github/workflows/ci.yml";
fn run(status: &str, conclusion: &str) -> Value { json!([CI, status, conclusion, "https://github.com/o/r/actions/runs/5"]) }
fn job(name: &str, conclusion: &str) -> Value { json!({ "__typename": "CheckRun", "name": name, "status": if conclusion.is_empty() { "IN_PROGRESS" } else { "COMPLETED" }, "conclusion": conclusion, "workflowName": "CI" }) }
fn listed(head: &str, checks: Value) -> Value { json!({ "state": "OPEN", "headRefOid": head, "statusCheckRollup": checks, "comments": [], "reviews": [], "author": { "login": "me" } }) }
const PREVIEW: &str = ".github/workflows/preview.yml";
fn with_flow(w: &World, path: &str, workflow: &str) {
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n", path: workflow }, "commit": "v1", "note": "Did v1." }] }));
}
fn with_ci(w: &World, workflow: &str, files: Value) {
    let mut f = files; f[CI] = json!(workflow);
    w.script(json!({ "worker": [{ "files": f, "commit": "v1", "note": "Did v1." }] }));
}

#[test]
fn a_workflow_that_runs_for_the_pull_request_is_waited_for_until_its_run_is_through_whatever_is_listed_so_far() {
    // it starts only once the pull request is ready: right after `gh pr ready` GitHub lists nothing, and has no run
    let w = pr_world(json!({}));
    with_ci(&w, "name: CI\non:\n  pull_request:\n    types: [ready_for_review, synchronize]\njobs:\n  build:\n    runs-on: x\n", json!({ "f.txt": "v1\n" }));
    let id = to_approval(&w, "CI starts on ready");
    let head = task(&id).t("prHead");
    w.gh_says(json!({ "view": none(&head) }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging", "{}", t.at("failure")); waits(&t, "Waiting for GitHub checks · 0 of 1 passed", "CI"); assert!(!merged(&w));
    poll_prs(); settle(&id);
    assert_eq!(task(&id).s("stage"), "merging", "still nothing: still waiting"); assert!(!merged(&w));
    w.gh_says(json!({ "view": running(&head), "runs": [run("in_progress", "")] }));
    poll_prs(); let t = settle(&id);
    waits(&t, "Waiting for GitHub checks · 0 of 2 passed", "build, CI"); assert!(!merged(&w));
    // the check that was running is gone from the list again: no pass either
    w.gh_says(json!({ "view": none(&head), "runs": [run("in_progress", "")] }));
    poll_prs(); let t = settle(&id);
    assert_eq!(t.s("stage"), "merging"); assert!(t.b("autoMerge")); assert!(!merged(&w), "{}", gh_log(&w));
    // its job passed and the run is through: merged
    w.gh_says(json!({ "view": passed(&head), "runs": [run("completed", "success")] }));
    poll_prs(); let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
}

#[test]
fn no_check_listed_waits_where_the_branch_it_merges_into_has_checks_or_github_cannot_say_and_merges_where_it_has_none() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Outside CI, late");
    let head = task(&id).t("prHead");
    // no workflow here, none seen on the pull request, none listed yet: main's own commit has a status of an outside CI
    w.gh_says(json!({ "view": none(&head), "base": { "status": 1 } }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!((t.t("stage"), t.t("msg")), ("merging".to_string(), "Waiting for GitHub checks to start".to_string()), "{}", t.at("failure")); assert!(!merged(&w));
    assert!(gh_log(&w).contains("api repos/{owner}/{repo}/commits/main/status --jq .total_count"), "{}", gh_log(&w));
    // GitHub cannot say: waited for too
    w.gh_says(json!({ "view": none(&head), "baseFail": true }));
    poll_prs(); settle(&id);
    assert_eq!(task(&id).s("stage"), "merging"); assert!(!merged(&w));
    // the outside CI reports, and passes: merged
    w.gh_says(json!({ "view": view(&head, json!({ "__typename": "StatusContext", "context": "ci/outside", "state": "SUCCESS" })) }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
}

#[test]
fn a_check_seen_on_the_commit_and_gone_from_the_list_is_waited_for_even_when_the_others_passed() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Build goes missing");
    let head = task(&id).t("prHead");
    let lint = |state: &str| json!({ "__typename": "CheckRun", "name": "lint", "status": "COMPLETED", "conclusion": state });
    let build = json!({ "__typename": "CheckRun", "name": "build", "status": "IN_PROGRESS", "conclusion": "" });
    let listed = |checks: Value| json!({ "state": "OPEN", "headRefOid": head, "statusCheckRollup": checks, "comments": [], "reviews": [], "author": { "login": "me" } });
    w.gh_says(json!({ "view": listed(json!([build, lint("")])) }));
    finish_line::approve(&id, None, false).unwrap();
    waits(&settle(&id), "Waiting for GitHub checks · 0 of 2 passed", "build, lint");
    // only lint is listed now, and it passed: build never did
    w.gh_says(json!({ "view": listed(json!([lint("SUCCESS")])) }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging", "{}", t.at("failure")); waits(&t, "Waiting for GitHub checks · 1 of 2 passed", "build"); assert!(!merged(&w), "{}", gh_log(&w));
    // asked by the land step itself, the same
    sw::store::tasks::update(&id, |t| { t.put("autoMerge", false); t.put("autoMergeLocal", false); Ok(sw::store::tasks::Change::Write) }).unwrap();
    sw::jobs::run_job("land", &id, &[]);
    let t = settle(&id);
    waits(&t, "Waiting for GitHub checks · 1 of 2 passed", "build"); assert!(!merged(&w));
    let build_ok = json!({ "__typename": "CheckRun", "name": "build", "status": "COMPLETED", "conclusion": "SUCCESS" });
    w.gh_says(json!({ "view": listed(json!([build_ok, lint("SUCCESS")])) }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
}

#[test]
fn checks_not_registered_yet_are_waited_for_an_outside_ci_seen_on_other_pull_requests_and_a_workflow_job_not_listed() {
    // an outside CI that reports on pull requests only: none here yet, none on main, but the repo's other pull requests had some
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Outside CI on pull requests");
    let head = task(&id).t("prHead");
    w.gh_says(json!({ "view": none(&head), "others": 2 }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("msg"), "Waiting for GitHub checks to start", "{}", t.at("failure")); assert!(!merged(&w), "{}", gh_log(&w));
    drop(w); // one world at a time: each holds the tests' one lock
    // a workflow with two jobs, of which only lint has registered (and passed): its run is not through
    let w = pr_world(json!({ "review.rounds": 0 }));
    with_ci(&w, "name: CI\non: [pull_request]\njobs:\n  lint:\n    runs-on: x\n  build:\n    runs-on: x\n    steps:\n      - run: exit 1\n", json!({ "f.txt": "v1\n" }));
    let id = to_approval(&w, "Build registers late");
    let head = task(&id).t("prHead");
    w.gh_says(json!({ "view": listed(&head, json!([job("lint", "SUCCESS")])), "runs": [run("in_progress", "")] }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    waits(&t, "Waiting for GitHub checks · 1 of 2 passed", "CI"); assert!(!merged(&w), "{}", gh_log(&w));
    poll_prs(); settle(&id);
    assert_eq!(task(&id).s("stage"), "merging", "lint alone is listed, and passed: still waiting"); assert!(!merged(&w));
    // build registers and fails: nothing merges, and it waits for you (no rounds)
    w.gh_says(json!({ "view": listed(&head, json!([job("lint", "SUCCESS"), job("build", "FAILURE")])), "runs": [run("completed", "failure")] }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval"); assert_eq!(t.at("prFailure").s("remoteKind"), "checks"); assert!(!merged(&w));
}

#[test]
fn a_check_the_branch_requires_is_waited_for_until_it_is_listed_and_passes() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Required check");
    let head = task(&id).t("prHead");
    w.gh_says(json!({ "view": passed(&head), "required": ["ci/outside"] }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    waits(&t, "Waiting for GitHub checks · 1 of 2 passed", "ci/outside"); assert!(!merged(&w));
    let both = json!({ "state": "OPEN", "headRefOid": head, "statusCheckRollup": [passed(&head)["statusCheckRollup"][0], { "__typename": "StatusContext", "context": "ci/outside", "state": "SUCCESS" }], "comments": [], "reviews": [], "author": { "login": "me" } });
    w.gh_says(json!({ "view": both, "required": ["ci/outside"] }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
}

#[test]
fn a_push_workflow_whose_checks_appear_late_is_waited_for_and_a_repo_with_no_workflows_and_no_required_checks_merges() {
    // a workflow on every push, with no name: GitHub calls it by its file; nothing listed yet, then its job runs, then passes
    let w = pr_world(json!({}));
    with_ci(&w, "on: push\njobs:\n  test:\n    runs-on: ubuntu-latest\n", json!({ "f.txt": "v1\n" }));
    let id = to_approval(&w, "Push CI late");
    let head = task(&id).t("prHead");
    w.gh_says(json!({ "view": none(&head) }));
    finish_line::approve(&id, None, false).unwrap();
    waits(&settle(&id), "Waiting for GitHub checks · 0 of 1 passed", "ci.yml"); assert!(!merged(&w));
    w.gh_says(json!({ "view": view(&head, job("test", "")), "runs": [run("in_progress", "")] }));
    poll_prs(); settle(&id);
    assert_eq!(task(&id).s("stage"), "merging"); assert!(!merged(&w));
    w.gh_says(json!({ "view": view(&head, job("test", "SUCCESS")), "runs": [run("completed", "success")] }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
    drop(w); // one world at a time: each holds the tests' one lock
    // no workflows, no required checks, none anywhere: "no checks", merged at once as before
    let w = pr_world(json!({}));
    let id = to_approval(&w, "No CI at all");
    let head = task(&id).t("prHead");
    w.gh_says(json!({ "view": none(&head) }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
}

#[test]
fn a_matrix_member_not_reported_yet_is_waited_for_and_a_workflow_that_cannot_run_for_the_pull_request_is_not() {
    // a matrix of two: one member listed and passed, the run not through
    let w = pr_world(json!({}));
    with_ci(&w, "name: CI\non: [pull_request]\njobs:\n  build:\n    strategy:\n      matrix:\n        os: [ubuntu-latest, windows-latest]\n    runs-on: ${{ matrix.os }}\n", json!({ "f.txt": "v1\n" }));
    let id = to_approval(&w, "Matrix");
    let head = task(&id).t("prHead");
    w.gh_says(json!({ "view": listed(&head, json!([job("build (ubuntu-latest)", "SUCCESS")])), "runs": [run("in_progress", "")] }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    waits(&t, "Waiting for GitHub checks · 1 of 2 passed", "CI"); assert!(!merged(&w));
    w.gh_says(json!({ "view": listed(&head, json!([job("build (ubuntu-latest)", "SUCCESS"), job("build (windows-latest)", "SUCCESS")])), "runs": [run("completed", "success")] }));
    poll_prs();
    assert_eq!(settle(&id).s("stage"), "done"); assert!(merged(&w));
    drop(w);
    // only for changes under src/ (said inline), and this one changes src/: its late job is waited for although lint passed
    let w = pr_world(json!({}));
    with_ci(&w, "name: CI\non: {pull_request: {paths: ['src/**']}}\njobs:\n  build:\n    runs-on: x\n", json!({ "src/a.rs": "fn a() {}\n" }));
    let id = to_approval(&w, "Paths that match");
    let head = task(&id).t("prHead");
    w.gh_says(json!({ "view": listed(&head, json!([job("lint", "SUCCESS")])) }));
    finish_line::approve(&id, None, false).unwrap();
    waits(&settle(&id), "Waiting for GitHub checks · 1 of 2 passed", "CI"); assert!(!merged(&w));
    drop(w);
    // the same workflow for a change outside src/, and one only for pull requests into release: neither runs here
    let w = pr_world(json!({}));
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n", CI: "name: CI\non:\n  pull_request:\n    paths: ['src/**']\njobs:\n  build:\n    runs-on: x\n", ".github/workflows/rel.yml": "name: Release test\non:\n  pull_request: {branches: [release]}\njobs:\n  release-test:\n    runs-on: x\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = to_approval(&w, "Nothing runs here");
    let head = task(&id).t("prHead");
    w.gh_says(json!({ "view": listed(&head, json!([job("lint", "SUCCESS")])) }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
}

#[test]
fn a_workflow_whose_types_leave_out_how_the_commit_came_is_waited_for_a_short_while_unless_its_run_is_there() {
    // `labeled, synchronize`: a pull request opened on its head and never pushed to again gets no run of it, and it is
    // the repo's only check: nothing is listed
    let flow = "name: Preview\non:\n  pull_request:\n    types: [labeled, synchronize]\njobs:\n  deploy:\n    runs-on: x\n";
    let w = pr_world(json!({}));
    with_flow(&w, PREVIEW, flow);
    let id = to_approval(&w, "Preview never runs");
    let head = task(&id).t("prHead");
    w.gh_says(json!({ "view": none(&head) }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    waits(&t, "Waiting for GitHub checks · 0 of 1 passed", "Preview"); assert!(!merged(&w));
    sw::clock::set(Some(t.at("checksWait").i("since")));
    sw::clock::advance(MAYBE_MS - 1);
    poll_prs(); settle(&id);
    assert_eq!(task(&id).s("stage"), "merging", "not yet: it may still start"); assert!(!merged(&w));
    sw::clock::advance(1);
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w), "{}", gh_log(&w));
    drop(w); // one world at a time: each holds the tests' one lock
    // the same workflow with its run on the commit: waited for until it is through, however long that takes
    let w = pr_world(json!({}));
    with_flow(&w, PREVIEW, flow);
    let id = to_approval(&w, "Preview runs");
    let head = task(&id).t("prHead");
    let run = |status: &str, conclusion: &str| json!([[PREVIEW, status, conclusion, "https://github.com/o/r/actions/runs/7", "Preview"]]);
    w.gh_says(json!({ "view": passed(&head), "runs": run("in_progress", "") }));
    finish_line::approve(&id, None, false).unwrap();
    settle(&id);
    sw::clock::advance(MAYBE_MS * 2);
    poll_prs(); settle(&id);
    assert_eq!(task(&id).s("stage"), "merging"); assert!(!merged(&w));
    // GitHub does not give the commit's runs: not known to be missing, so still waited for
    w.gh_says(json!({ "view": passed(&head), "actionsFail": true }));
    sw::clock::advance(MAYBE_MS * 2);
    poll_prs(); settle(&id);
    assert_eq!(task(&id).s("stage"), "merging"); assert!(!merged(&w));
    w.gh_says(json!({ "view": passed(&head), "runs": run("completed", "success") }));
    poll_prs();
    assert_eq!(settle(&id).s("stage"), "done"); assert!(merged(&w));
}

#[test]
fn with_github_actions_turned_off_the_workflows_kept_in_the_repo_are_not_waited_for() {
    let w = pr_world(json!({}));
    with_flow(&w, ".github/workflows/ci.yml", "name: Build\non: [pull_request]\njobs:\n  build:\n    runs-on: x\n");
    let id = to_approval(&w, "CI moved elsewhere");
    let head = task(&id).t("prHead");
    let woodpecker = view(&head, json!({ "__typename": "StatusContext", "context": "ci/woodpecker/pr/linux", "state": "SUCCESS" }));
    w.gh_says(json!({ "view": woodpecker, "actionsOff": true }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.t("msg"), t.at("failure")); assert!(merged(&w), "{}", gh_log(&w));
    assert!(gh_log(&w).contains("actions/permissions"), "{}", gh_log(&w));
    drop(w);
    // still running there: still waited for
    let w = pr_world(json!({}));
    with_flow(&w, ".github/workflows/ci.yml", "name: Build\non: [pull_request]\njobs:\n  build:\n    runs-on: x\n");
    let id = to_approval(&w, "Outside CI running");
    let head = task(&id).t("prHead");
    w.gh_says(json!({ "view": view(&head, json!({ "__typename": "StatusContext", "context": "ci/woodpecker/pr/linux", "state": "PENDING" })), "actionsOff": true }));
    finish_line::approve(&id, None, false).unwrap();
    waits(&settle(&id), "Waiting for GitHub checks · 0 of 1 passed", "ci/woodpecker/pr/linux"); assert!(!merged(&w));
}

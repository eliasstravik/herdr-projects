//! GitHub's checks on the pull request, looked at before it merges (`merge.checks`): still running, the merge waits
//! and says so; failed, nothing merges and the failure goes back to the agent with the check's name and its log,
//! within `review.rounds`, then waits for you (`approve` refuses, `--anyway` merges); stuck past `check.timeout`, it
//! waits for you; `none` merges as before.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::actions::finish_line;
use sw::housekeeping::poll_prs;
use sw::json::*;
use sw::store::settings::{change::set_value, lookup::Scope};
use sw::store::tasks::{update, Change};

pub fn pr_world(more: Value) -> World {
    let mut s = json!({ "review.when": "never" });
    for (k, v) in more.as_object().cloned().unwrap_or_default() { s[k] = v; }
    world_with(Opts { repo: true, origin: true, settings: s, ..Default::default() })
}
pub fn view(head: &str, check: Value) -> Value { json!({ "state": "OPEN", "headRefOid": head, "statusCheckRollup": [check], "comments": [], "reviews": [], "author": { "login": "me" } }) }
pub fn none(head: &str) -> Value { json!({ "state": "OPEN", "headRefOid": head, "statusCheckRollup": [], "comments": [], "reviews": [], "author": { "login": "me" } }) }
pub fn running(head: &str) -> Value { view(head, json!({ "__typename": "CheckRun", "name": "build", "status": "IN_PROGRESS", "conclusion": "" })) }
pub fn passed(head: &str) -> Value { view(head, json!({ "__typename": "CheckRun", "name": "build", "status": "COMPLETED", "conclusion": "SUCCESS" })) }
pub fn failed(head: &str) -> Value { view(head, json!({ "__typename": "CheckRun", "name": "build", "status": "COMPLETED", "conclusion": "FAILURE", "detailsUrl": "https://github.com/o/r/actions/runs/11/job/22" })) }
pub const LOG: &str = "build\tRun tests\t2026-10-14T10:00:00.0000000Z running 3 tests\nbuild\tRun tests\t2026-10-14T10:00:01.0000000Z test parses_flags ... FAILED\nbuild\tRun tests\t2026-10-14T10:00:02.0000000Z \u{1b}[31merror: 1 test failed\u{1b}[0m";
/// The bare repo the board pushes to: GitHub names the commit the branch holds there.
pub fn origin(w: &World) -> String { w.base.join("origin.git").display().to_string() }
pub fn rollup(view: Value) -> Value { view["statusCheckRollup"].clone() }
pub fn merged(w: &World) -> bool { gh_log(w).lines().any(|l| l.starts_with("pr merge")) }
pub fn pane_prompts(w: &World, id: &str) -> Vec<String> { prompts(w, task(id).at("link").s("pane")) }
/// What a merge that waits for GitHub's checks says (how far they are) and which it waits for, each once.
#[track_caller]
pub fn waits(t: &Obj, says: &str, names: &str) {
    let w = t.at("checksWait");
    let waited = sw::land::checks::unique(strings(w.at("names")).into_iter().chain(strings(w.at("flows"))));
    assert_eq!((t.t("msg"), waited.join(", ")), (says.to_string(), names.to_string()));
}

#[test]
fn checks_still_running_the_merge_waits_and_says_so_then_merges_once_they_pass() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Waits for CI");
    let head = task(&id).t("prHead");
    w.gh_says(json!({ "view": running(&head) }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging"); assert!(t.b("autoMerge") && t.b("checksWait"));
    waits(&t, "Waiting for GitHub checks · 0 of 1 passed", "build");
    assert_eq!(row(&id).s("state"), "working", "waiting on GitHub is not waiting on you");
    assert!(!merged(&w), "nothing merges while they run: {}", gh_log(&w));
    poll_prs(); settle(&id);
    assert_eq!(task(&id).s("stage"), "merging", "still running: still waiting"); assert!(!merged(&w));
    w.gh_says(json!({ "view": passed(&head) }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w)); assert!(!t.b("checksWait"));
}

#[test]
fn failed_checks_go_back_to_the_agent_with_the_name_and_the_log_and_the_approval_carries_the_repair_through() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "CI fails at the merge");
    let head = task(&id).t("prHead");
    w.script(json!({ "worker": [{ "files": { "f.txt": "fixed\n" }, "commit": "fix", "note": "Fixed CI." }] }));
    w.gh_says(json!({ "origin": origin(&w), "view": failed(&head), "newer": rollup(running(&head)), "log": LOG }));
    finish_line::approve(&id, None, false).unwrap();
    // straight back to the agent: the name of the check and the end of its log, the time and colours taken off; the
    // repair is checked and pushed, and merges once GitHub's checks pass on it
    let t = settle(&id);
    let sent = pane_prompts(&w, &id).last().cloned().unwrap_or_default();
    assert!(sent.contains("GitHub's checks failed on the pull request"), "{sent}");
    assert!(sent.contains("build (failure)") && sent.contains("test parses_flags ... FAILED\nerror: 1 test failed"), "{sent}");
    assert!(gh_log(&w).contains("run view --job 22 --log-failed"));
    assert!(!merged(&w));
    let fixed = t.t("prHead");
    assert_ne!(fixed, head);
    // the old commit's failure says nothing of the repair (#309): gone, and the approval you gave waits for its checks
    assert_eq!(t.s("stage"), "merging", "{}", t.s("msg")); assert_eq!(t.i("prRounds"), 1); assert!(t.b("mergeIntent"), "the approval you gave carries the repair");
    assert!(t.at("prFailure").is_null(), "{}", t.at("prFailure")); assert_ne!(t.s("prState"), "failing");
    waits(&t, "Waiting for GitHub checks · 0 of 1 passed", "build");
    assert!(sw::land::checks::failed_on(&t).is_none(), "a new commit is not refused for the old one's checks");
    w.gh_says(json!({ "origin": origin(&w), "view": failed(&head), "newer": rollup(passed(&head)) }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
}

#[test]
fn failed_checks_of_an_older_commit_do_not_block_its_repair_that_passed_when_main_moved_before_it_merged() {
    // #309: checks fail on H1; the repair H2 waits for its own; main moves; H2 passes; its merge turn merges main in
    // and checks again. A record of H1's failure still on the task (as the board kept it before) blocks none of it.
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Old head failed");
    let head = task(&id).t("prHead");
    w.script(json!({ "worker": [{ "files": { "f.txt": "fixed\n" }, "commit": "fix", "note": "Fixed CI." }] }));
    w.gh_says(json!({ "origin": origin(&w), "view": failed(&head), "newer": rollup(running(&head)), "log": LOG }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging", "{}", t.s("msg")); assert!(t.b("checksWait"));
    let old = sw::land::checks::blocker(t.s("pr"), &head, &["build".into()], "build failed");
    update(&id, |t| { t.put("prFailure", old.clone()); Ok(Change::Write) }).unwrap();
    std::fs::write(w.root.join("other.txt"), "main\n").unwrap(); git(&w.root, &["add", "-A"]); git(&w.root, &["commit", "-qm", "main moves"]); git(&w.root, &["push", "-q", "origin", "main"]);
    let main = git(&w.root, &["rev-parse", "HEAD"]);
    w.gh_says(json!({ "origin": origin(&w), "view": failed(&head), "newer": rollup(passed(&head)) }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure")); assert!(merged(&w)); assert!(t.at("prFailure").is_null());
    let at = gh_log(&w).lines().find(|l| l.starts_with("pr merge")).and_then(|l| l.rsplit(' ').next()).unwrap_or_default().to_string();
    assert!(common::worktrees::git(&w.root, &["merge-base", "--is-ancestor", &main, &at]).is_ok(), "merged with main in: {at}");
}

#[test]
fn failed_checks_with_no_rounds_left_wait_for_you_approve_refuses_and_anyway_merges() {
    let w = pr_world(json!({ "review.rounds": 0 }));
    let id = to_approval(&w, "CI fails for good");
    let head = task(&id).t("prHead");
    let before = pane_prompts(&w, &id).len();
    w.gh_says(json!({ "view": failed(&head), "log": LOG }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(pane_prompts(&w, &id).len(), before, "no rounds: nothing goes back by itself");
    assert_eq!(t.s("stage"), "approval"); assert!(t.b("mergeFailed") && !t.b("mergeIntent")); assert!(!merged(&w));
    assert_eq!(t.at("prFailure").s("remoteKind"), "checks"); assert!(t.at("prFailure").s("detail").contains("test parses_flags ... FAILED"));
    assert_eq!(row(&id).s("state"), "blocked");
    let e = finish_line::approve(&id, None, false).unwrap_err().msg;
    assert!(e.contains("was not merged: GitHub's checks failed on its pull request: build") && e.contains("--anyway"), "{e}");
    let e = finish_line::retry(&id).unwrap_err().msg;
    assert!(e.contains("--anyway"), "retry is no way around it: {e}");
    assert!(!merged(&w));
    finish_line::approve(&id, None, true).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
}

#[test]
fn checks_that_never_finish_wait_for_you_after_the_check_timeout() {
    let w = pr_world(json!({ "check.timeout": "10m" }));
    let id = to_approval(&w, "CI never reports");
    let head = task(&id).t("prHead");
    // An hour back: the stamps that say when a pass is due are files, timed by the real clock, which stays ahead.
    let t0 = sw::clock::now() - 3600000; sw::clock::set(Some(t0));
    w.gh_says(json!({ "view": running(&head) }));
    finish_line::approve(&id, None, false).unwrap();
    waits(&settle(&id), "Waiting for GitHub checks · 0 of 1 passed", "build");
    sw::clock::advance(9 * 60000);
    poll_prs(); settle(&id);
    assert_eq!(task(&id).s("stage"), "merging", "not yet");
    sw::clock::advance(2 * 60000);
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval"); assert!(!t.b("autoMerge") && !t.b("checksWait")); assert!(!merged(&w));
    assert_eq!(t.at("failure").s("title"), "GitHub's checks did not finish");
    assert!(t.at("failure").s("detail").contains("after 10m (check.timeout): build"), "{}", t.at("failure"));
    assert_eq!(row(&id).s("state"), "blocked");
    // approved again it waits again, from the start
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging"); assert_eq!(t.at("checksWait").i("since"), sw::clock::now());
}

#[test]
fn merging_by_itself_never_goes_past_a_failed_check_and_none_ignores_them() {
    let w = pr_world(json!({ "merge.by": "itself", "review.rounds": 0 }));
    w.gh_says(json!({ "view": { "state": "OPEN", "statusCheckRollup": [{ "name": "build", "conclusion": "FAILURE" }], "comments": [], "reviews": [], "author": { "login": "me" } } }));
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = w.add("Merges by itself");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(!merged(&w));
    assert!(t.at("prFailure").s("detail").contains("GitHub gave no link to its log"), "{}", t.at("prFailure"));
    // `none`: GitHub's checks are not looked at, and the yes merges as before
    set_value("merge.checks", &json!("none"), &Scope::task(&task(&id))).unwrap();
    let looks = || gh_log(&w).matches("headRefOid,statusCheckRollup").count();
    let looked = looks();
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(merged(&w));
    assert_eq!(looks(), looked, "{}", gh_log(&w));
}

#[test]
fn every_failing_check_has_the_end_of_its_log_sent_not_only_the_first_three() {
    let w = pr_world(json!({ "review.rounds": 0 }));
    let id = to_approval(&w, "Four fail");
    let head = task(&id).t("prHead");
    let check = |n: i64| json!({ "__typename": "CheckRun", "name": format!("job{n}"), "status": "COMPLETED", "conclusion": "FAILURE", "detailsUrl": format!("https://github.com/o/r/actions/runs/11/job/{n}") });
    w.gh_says(json!({ "view": { "state": "OPEN", "headRefOid": head, "statusCheckRollup": [check(1), check(2), check(3), check(4)], "comments": [], "reviews": [], "author": { "login": "me" } }, "log": LOG }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    for n in 1..=4 { assert!(gh_log(&w).contains(&format!("run view --job {n} --log-failed")), "job {n}: {}", gh_log(&w)); }
    assert_eq!(t.at("prFailure").s("detail").matches("test parses_flags ... FAILED").count(), 4, "{}", t.at("prFailure"));
}

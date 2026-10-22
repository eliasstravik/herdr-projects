//! A pull request merges only at the commit that was reviewed (a commit pushed from elsewhere stops it, also the
//! remembered merge), and one that GitHub only queued is not taken for merged until GitHub says so, or closed. Main
//! merged in after the review is pushed to the pull request before it merges (#307).
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::Path;
use sw::actions::finish_line;
use sw::finish::line::in_line;
use sw::housekeeping::poll_prs;
use sw::json::*;
use sw::store::settings::change::set_value;
use sw::store::settings::lookup::Scope;
use sw::store::tasks::{update, Change};

fn pr_world() -> World { world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() }) }
fn view(head: &str, state: &str) -> Value {
    let check = if state == "PENDING" { json!({ "name": "build", "state": state }) } else { json!({ "name": "build", "conclusion": state }) };
    json!({ "state": "OPEN", "headRefOid": head, "statusCheckRollup": [check], "comments": [], "reviews": [], "author": { "login": "me" } })
}
fn plain(head: &str, state: &str) -> Value { json!({ "state": state, "headRefOid": head, "statusCheckRollup": [], "comments": [], "reviews": [], "author": { "login": "me" } }) }
fn in_sidebar(w: &World, t: &sw::json::Obj) -> bool { w.state().arr("workspaces").iter().any(|x| x.s("workspace_id") == t.at("link").s("workspace")) }

#[test]
fn a_pull_request_only_merges_at_the_commit_that_was_reviewed_a_commit_pushed_from_elsewhere_stops_it_also_the_auto_merge() {
    let w = pr_world();
    let id = to_approval(&w, "With a PR");
    let t = task(&id);
    let reviewed = git(Path::new(t.s("worktree")), &["rev-parse", "HEAD"]);
    let other = "1".repeat(40);
    let branch = t.t("branch");
    // GitHub says the pull request's head is another commit, and so does the branch at the remote: said plainly, nothing merges
    let pushed = |sha: &str| { git(&w.root, &["push", "-q", "--force", "origin", &format!("{sha}:refs/heads/{branch}")]); };
    w.gh_says(json!({ "head": other, "view": { "state": "OPEN", "headRefOid": other } }));
    pushed("main");
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    pushed(&reviewed);
    assert_eq!(t.s("stage"), "approval"); assert_eq!(t.at("mergeFailed"), &json!(true)); assert!(!t.b("autoMerge"));
    assert_eq!(t.at("failure").s("title"), "Pull request has commits that were not reviewed"); assert!(t.at("failure").s("detail").contains(&other));
    assert!(!gh_log(&w).contains("pr merge"));
    // it arrives between that look and the merge: GitHub itself refuses (the look itself is first answered late, with a
    // commit the branch at the remote does not stand at: asked again, not taken for a commit nobody reviewed)
    w.gh_says(json!({ "head": other, "late": { "head": other, "times": 1 } }));
    let looks = || gh_log(&w).lines().filter(|l| *l == "pr view 7 --json headRefOid").count();
    let looked = looks();
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval"); assert!(!t.b("autoMerge")); assert_eq!(t.at("failure").s("title"), "Pull request could not merge");
    assert!(t.at("failure").s("detail").contains("Head branch was modified"), "{}", t.at("failure"));
    assert_eq!(looks(), looked + 2);
    assert!(gh_log(&w).contains(&format!("pr merge 7 --squash --match-head-commit {reviewed}\n")), "{}", gh_log(&w));
    assert!(!regex::Regex::new("--auto|--delete-branch").unwrap().is_match(&gh_log(&w)));
    assert!(Path::new(t.s("worktree")).exists()); assert!(!t.b("tidy"));
    // checks still running: remembered. Someone pushes to the pull request meanwhile: the approval is withdrawn
    w.gh_says(json!({ "head": reviewed, "mergeFail": "checks pending", "view": view(&reviewed, "PENDING") }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging"); assert_eq!(t.at("autoMerge"), &json!(true));
    // GitHub naming another commit while the branch at the remote is still the reviewed one is only a late answer (it
    // was asked right after the board's own push): nothing is withdrawn, the next poll asks again
    w.gh_says(json!({ "head": other, "view": view(&other, "SUCCESS") }));
    poll_prs(); settle(&id);
    let t = task(&id);
    assert_eq!((t.t("stage"), t.at("autoMerge").clone(), t.b("failure")), ("merging".to_string(), json!(true), false));
    pushed("main"); // someone really pushed to it
    poll_prs(); settle(&id);
    let t = task(&id);
    assert_eq!(t.s("stage"), "approval"); assert!(!t.b("autoMerge") && !t.b("mergeIntent")); assert_eq!(t.at("failure").s("title"), "Pull request has commits that were not reviewed");
    pushed(&reviewed);
    // remembered again, the checks pass on the reviewed commit: the board merges it itself, the remote branch goes, clean-up follows
    w.gh_says(json!({ "head": reviewed, "mergeFail": "checks pending", "view": view(&reviewed, "PENDING") }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging"); assert_eq!(t.at("autoMerge"), &json!(true));
    w.gh_says(json!({ "head": reviewed, "view": view(&reviewed, "SUCCESS") }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(t.b("cleaned") && !t.b("autoMerge"));
    assert_eq!(git(&w.root, &["ls-remote", "--heads", "origin", &branch]), "", "deleted at the remote, by the lease on the merged commit");
}

#[test]
fn a_pull_request_that_github_only_queued_is_not_taken_for_merged_nothing_is_cleaned_up_until_github_says_so() {
    let w = pr_world();
    let id = to_approval(&w, "With a PR");
    let head = task(&id).t("prHead");
    // a repo with a merge queue: gh pr merge answers fine, the pull request stays open
    w.gh_says(json!({ "queue": true, "view": plain(&head, "OPEN") }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging"); assert_eq!(t.at("autoMerge"), &json!(true)); assert!(!t.b("prMerged") && !t.b("cleaned") && !t.b("tidy"));
    assert!(t.s("msg").contains("GitHub merges it when"), "{}", t.s("msg"));
    assert!(Path::new(t.s("worktree")).exists()); assert!(in_sidebar(&w, &t));
    assert_eq!(git(&w.root, &["ls-remote", "--heads", "origin", t.s("branch")]).split('\t').next().unwrap(), head, "the remote branch is still there");
    poll_prs();
    assert_eq!(task(&id).s("stage"), "merging", "still watched, still waiting");
    // the queue merges it: now it is done and cleaned up
    w.gh_says(json!({ "view": plain(&head, "MERGED") }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done"); assert!(t.b("prMerged") && t.b("cleaned") && !t.b("autoMerge"));
    assert_eq!(w.state().arr("workspaces").len(), 0); assert_eq!(git(&w.root, &["ls-remote", "--heads", "origin", t.s("branch")]), "");
}

#[test]
fn a_queued_pull_request_that_is_closed_without_merging_stops_waiting_and_says_so() {
    let w = pr_world();
    let id = to_approval(&w, "With a PR");
    let head = task(&id).t("prHead");
    w.gh_says(json!({ "queue": true, "view": plain(&head, "OPEN") }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging"); assert_eq!(t.at("autoMerge"), &json!(true));
    w.gh_says(json!({ "view": plain(&head, "CLOSED") }));
    poll_prs();
    let t = task(&id);
    assert_eq!(t.s("stage"), "approval"); assert!(!t.b("autoMerge") && !t.b("mergeIntent") && !t.b("prMerged")); assert_eq!(t.at("mergeFailed"), &json!(true));
    assert_eq!(t.at("failure").s("title"), "Pull request was closed without merging"); assert_eq!(t.s("msg"), t.at("failure").s("title"));
    assert_eq!(row(&id).s("state"), "blocked");
    assert!(Path::new(t.s("worktree")).exists()); assert!(in_sidebar(&w, &t), "nothing was cleaned up");
    poll_prs();
    assert_eq!(task(&id).s("stage"), "approval");
}

/// Pull requests stand where the board pushed them: GitHub names the commit their branch holds at the remote.
fn pushed_world(settings: Value) -> World {
    let w = world_with(Opts { repo: true, origin: true, settings, repo_settings: Some(json!({ "check.command": "true" })), ..Default::default() });
    w.gh_says(json!({ "origin": w.base.join("origin.git").display().to_string() }));
    w
}
fn at_origin(w: &World, branch: &str) -> String { git(&w.base.join("origin.git"), &["rev-parse", &format!("refs/heads/{branch}")]) }
fn main_moves(w: &World, file: &str, text: &str) -> String {
    std::fs::write(w.root.join(file), text).unwrap(); git(&w.root, &["add", "-A"]); git(&w.root, &["commit", "-qm", "main moves"]); git(&w.root, &["push", "-q", "origin", "main"]);
    git(&w.root, &["rev-parse", "HEAD"])
}
fn holds(w: &World, head: &str, sha: &str) -> bool { common::worktrees::git(&w.root, &["merge-base", "--is-ancestor", sha, head]).is_ok() }
/// The commit GitHub was told to merge, and that it merged at it.
fn merged_at(w: &World, id: &str) -> String {
    let t = task(id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure")); assert!(t.b("prMerged"));
    let log = gh_log(w);
    let line = log.lines().find(|l| l.starts_with("pr merge ")).unwrap_or_else(|| panic!("never merged: {log}"));
    line.rsplit(' ').next().unwrap().to_string()
}
/// Approved after main moved: what was merged in is pushed, then merges.
fn approved_after_main_moved(w: &World, id: &str, reviewed: &str) {
    let main = main_moves(w, "other.txt", "main\n");
    sw::actions::finish_line::approve(id, None, false).unwrap();
    settle(id);
    let head = merged_at(w, id);
    assert_ne!(head, reviewed); assert!(holds(w, &head, &main) && holds(w, &head, reviewed), "the reviewed work with main merged in");
    assert!(!gh_log(w).contains("Head branch was modified"));
}

#[test]
fn merging_by_itself_main_merged_in_after_the_review_passed_is_pushed_to_the_pull_request_and_merges_there() {
    // The reporter's sequence: merge.by itself, the review passes on A, and while it waits its turn main moves.
    let w = pushed_world(json!({ "merge.by": "itself" }));
    let ahead = w.add("Ahead");
    update(&ahead, |t| { let mut m = sw::store::files::marker(); m.put("job", "land"); t.put("lineTurn", m.at("started").clone()); t.put("runner", Value::Object(m)); t.put("stage", "merging"); Ok(Change::Write) }).unwrap();
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = w.add("Mine");
    delegate(&id);
    let t = settle(&id);
    assert!(in_line(&t), "{} {}", t.s("stage"), t.at("failure"));
    let reviewed = t.t("reviewedHead");
    assert_eq!(at_origin(&w, t.s("branch")), reviewed); assert_eq!(t.at("review").at("pass"), &json!(true));
    let main = main_moves(&w, "other.txt", "main\n");
    // Its turn: main is merged in (B), the check passes, B is pushed and merges, the review standing for it.
    update(&ahead, |t| { t.put("stage", "done"); t.put("runner", Value::Null); Ok(Change::Write) }).unwrap();
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    settle(&id);
    let head = merged_at(&w, &id);
    assert_ne!(head, reviewed); assert!(holds(&w, &head, &main) && holds(&w, &head, &reviewed));
    assert_eq!(sw::store::reviews::review_history(&task(&id)).len(), 1, "a clean merge of main keeps the review");
}

#[test]
fn main_merged_in_while_it_waits_for_your_yes_is_pushed_and_the_approval_merges_it() {
    let w = pushed_world(json!({}));
    let id = to_approval(&w, "Waits");
    let reviewed = task(&id).t("reviewedHead");
    let main = main_moves(&w, "other.txt", "main\n");
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(holds(&w, t.s("reviewedHead"), &main));
    assert_eq!(at_origin(&w, t.s("branch")), t.t("reviewedHead"), "pushed as soon as it was merged in");
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    settle(&id);
    assert_eq!(merged_at(&w, &id), t.t("reviewedHead")); assert_ne!(t.t("reviewedHead"), reviewed);
}

#[test]
fn main_merged_in_at_the_approval_is_pushed_also_when_merge_pr_was_turned_off_after_the_pull_request_opened() {
    let w = pushed_world(json!({}));
    let id = to_approval(&w, "PR then off");
    assert!(task(&id).b("pr"));
    set_value("merge.pr", &json!(false), &Scope::task(&task(&id))).unwrap();
    approved_after_main_moved(&w, &id, &task(&id).t("reviewedHead"));
}

#[test]
fn main_merged_in_at_the_approval_is_pushed_to_the_pull_request_the_agent_opened_itself() {
    let w = pushed_world(json!({}));
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = w.add("Own PR");
    delegate(&id);
    let url = "https://github.com/o/r/pull/9";
    w.gh_says(json!({ "origin": w.base.join("origin.git").display().to_string(), "headBranch": task(&id).t("branch"), "createFail": "already exists", "prs": [{ "number": 9, "url": url, "state": "OPEN" }] }));
    let t = settle(&id);
    assert_eq!((t.t("stage"), t.t("pr")), ("approval".to_string(), url.to_string()), "{}", t.at("failure"));
    approved_after_main_moved(&w, &id, &t.t("reviewedHead"));
}

#[test]
fn main_that_brought_the_same_change_in_after_the_review_ends_the_task_as_done_not_as_unreviewed_commits() {
    let w = pushed_world(json!({}));
    let id = to_approval(&w, "Same on main");
    main_moves(&w, "f.txt", "v1\n");
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
    assert!(!t.s("msg").contains("not reviewed")); assert!(!gh_log(&w).contains("pr merge"));
}

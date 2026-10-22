//! Main merged into a task's branch by the board itself, before the check and the review and whenever main moves while
//! it waits: the agent hears of it only on a real conflict; a review is repeated only where the update touched the
//! task's own files; a worktree off its own branch is never checked or put up for approval. Under `merge.sync approve`
//! a waiting task is only test-merged: a conflict still goes to its agent, and main is merged after your yes.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::actions::{filing, finish_line, talk};
use sw::json::*;

fn holds(dir: &Path, sha: &str) -> bool { common::worktrees::git(dir, &["merge-base", "--is-ancestor", sha, "HEAD"]).is_ok() }
fn main_moves(w: &World, file: &str, body: &str) -> String { std::fs::write(w.root.join(file), body).unwrap(); git(&w.root, &["add", "-A"]); git(&w.root, &["commit", "-qm", &format!("main: {file}")]); git(&w.root, &["rev-parse", "HEAD"]) }
/// A task whose agent stopped to ask, so main can move before its work is finished.
fn asking(w: &World, title: &str) -> String { let id = w.add(title); delegate(&id); assert_eq!(settle(&id).s("stage"), "asks"); id }
fn tree(t: &sw::json::Obj) -> &Path { Path::new(t.s("worktree")) }
fn pane_prompts(w: &World, id: &str) -> Vec<String> { prompts(w, task(id).at("link").s("pane")) }
fn head(t: &sw::json::Obj) -> String { git(tree(t), &["rev-parse", "HEAD"]) }
fn housekeep(w: &World) { sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes); }
fn checks(w: &World) -> usize { read(&w.base.join("home/checks.log")).lines().count() }
fn sync_jobs() -> usize { sw::runner::JOBS.with(|j| j.borrow().iter().filter(|(job, _, _)| job == "sync").count()) }

#[test]
fn main_is_merged_into_the_tasks_branch_before_the_check_and_the_review_the_agent_is_told_nothing() {
    // the check passes only on the branch as it will land
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "check.command": "test -f g.txt" })), ..Default::default() });
    w.script(json!({ "worker": [{ "note": "Which one?" }, { "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = asking(&w, "Clean sync");
    let target = main_moves(&w, "g.txt", "from main\n");
    talk::tell(&id, "The first one.", false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert!(holds(tree(&t), &target), "the branch holds main");
    assert_eq!(t.s("syncedTo"), target);
    assert_eq!(t.at("review").at("pass"), &json!(true));
    assert_eq!(t.s("reviewedHead"), git(tree(&t), &["rev-parse", "HEAD"]));
    assert!(t.s("stat").starts_with("1 file changed"), "the change is what it adds to main: {}", t.s("stat"));
    assert_eq!(pane_prompts(&w, &id).len(), 2, "its task and your answer: nothing about main");
    assert_eq!(git(&w.root, &["status", "--porcelain"]), "");
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(read(&w.root.join("f.txt")), "v1\n");
}

#[test]
fn when_another_task_merges_the_waiting_ones_are_brought_up_to_date_and_reviewed_again_only_where_their_files_were_touched() {
    let w = world();
    let big = |first: &str, last: &str| { let mut l = vec![first.to_string()]; l.extend((0..18).map(|i| format!("line {}", i + 2))); l.push(last.into()); l.join("\n") + "\n" };
    main_moves(&w, "big.txt", &big("line 1", "line 20"));
    let start = |title: &str, files: serde_json::Value| { w.script(json!({ "worker": [{ "files": files, "commit": title, "note": format!("Did {title}.") }] })); let id = w.add(title); delegate(&id); assert_eq!(settle(&id).s("stage"), "approval"); id };
    let first = start("first", json!({ "big.txt": big("FIRST", "line 20") }));
    let apart = start("apart", json!({ "other.txt": "apart\n" }));
    let near = start("near", json!({ "big.txt": big("line 1", "NEAR") }));
    let clash = start("clash", json!({ "big.txt": big("CLASH", "line 20") }));
    let before = task(&apart);
    w.script(json!({ "worker": [{ "note": null, "status": "working" }] })); // the agent of `clash`, told about its conflict
    finish_line::approve(&first, None, false).unwrap();
    assert_eq!(settle(&first).s("stage"), "done");
    let main = git(&w.root, &["rev-parse", "HEAD"]);
    // another task's files: updated and checked, and the review stands
    let t = settle(&apart);
    assert_eq!(t.s("stage"), "approval"); assert!(holds(tree(&t), &main)); assert_ne!(t.s("reviewedHead"), before.s("reviewedHead"));
    assert_eq!(t.s("reviewedHead"), git(tree(&t), &["rev-parse", "HEAD"])); assert_eq!(t.s("syncedTo"), main);
    assert_eq!(t.i("reviewRound"), 1); assert!(t.at("review").b("keptAt")); assert!(t.s("msg").starts_with("Approve to merge?"), "{}", t.s("msg"));
    assert_eq!(pane_prompts(&w, &apart).len(), 1);
    // the same file, merged cleanly: reviewed again as it will land
    let t = settle(&near);
    assert_eq!(t.s("stage"), "approval"); assert!(holds(tree(&t), &main)); assert_eq!(t.i("reviewRound"), 2); assert!(!t.at("review").b("keptAt"));
    assert_eq!(pane_prompts(&w, &near).len(), 1);
    // a real conflict: it no longer looks ready to approve, and its agent is already on it
    let t = settle(&clash);
    assert_eq!(t.s("stage"), "working"); assert_eq!(t.at("failure").s("title"), "Merge conflict with main"); assert_eq!(t.at("review").at("stale"), &json!(true));
    assert!(pane_prompts(&w, &clash).last().unwrap().contains("Conflicting files:\n- big.txt"));
    assert_eq!(git(tree(&t), &["status", "--porcelain"]), ""); assert!(!holds(tree(&t), &main));
    assert_eq!(row(&clash).s("state"), "working");
    assert!(finish_line::approve(&clash, None, false).unwrap_err().msg.contains("not waiting for an approval"));
    // and the ones still waiting land without a second sync finding anything
    finish_line::approve(&apart, None, false).unwrap();
    assert_eq!(settle(&apart).s("stage"), "done");
    assert_eq!(read(&w.root.join("other.txt")), "apart\n");
}

#[test]
fn main_moving_on_while_a_sync_runs_is_not_marked_as_handled_only_the_commit_that_was_merged_counts_as_seen() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "a.txt": "from the agent\n" }, "commit": "agent", "note": "Changed a.txt." }, { "note": null, "status": "working" }] }));
    let id = w.add("Race");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval");
    let first = main_moves(&w, "g.txt", "from main\n");
    let r = sw::sync::sync_main(&t, t.s("worktree")); // the sync merges this main...
    let second = main_moves(&w, "a.txt", "from main\n"); // ...and main moves again, with a conflict, before it is recorded
    assert_eq!(r.status, "merged"); assert_eq!(r.target, first);
    let patch = sw::sync::seen_patch(&t, Some(&r));
    assert!(patch.s("syncSeen").starts_with(&first));
    assert_ne!(patch.s("syncSeen"), sw::sync::known_target(&t, &mut Default::default()));
    sw::store::tasks::update(&id, |t| { sw::sync::apply(t, patch.clone()); t.put("reviewedHead", r.head.clone()); Ok(sw::store::tasks::Change::Write) }).unwrap();
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    assert_eq!(sync_jobs(), 1, "so the board looks again");
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working"); assert_eq!(t.at("failure").s("title"), "Merge conflict with main");
    assert!(pane_prompts(&w, &id).last().unwrap().contains(&format!("Target commit: {second}")));
}

#[test]
fn a_worktree_that_left_the_tasks_own_branch_is_not_checked_reviewed_or_put_up_for_approval() {
    let w = world();
    w.script(json!({ "worker": [{ "note": "Which one?" }, { "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }, { "note": "Back on my branch." }] }));
    let id = asking(&w, "Strayed");
    let t = task(&id);
    let target = main_moves(&w, "g.txt", "from main\n");
    git(tree(&t), &["checkout", "-q", "-b", "strayed"]);
    talk::tell(&id, "The first one.", false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed"); assert_eq!(t.at("failure").s("title"), "Task is on the wrong branch"); assert!(!t.b("review") && !t.b("reviewedHead"));
    assert!(!holds(tree(&t), &target)); assert_eq!(row(&id).s("state"), "error");
    assert!(finish_line::approve(&id, None, false).unwrap_err().msg.contains("not waiting for an approval"));
    // back on its own branch with the work: synced, checked and reviewed as usual
    git(tree(&t), &["checkout", "-q", t.s("branch")]); git(tree(&t), &["merge", "-q", "strayed"]);
    finish_line::send_failure(&id).unwrap();
    assert!(pane_prompts(&w, &id).last().unwrap().contains("Task is on the wrong branch"));
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(holds(tree(&t), &target)); assert_eq!(t.at("review").at("pass"), &json!(true));
}

#[test]
fn main_renaming_a_file_the_task_edited_is_an_update_to_the_tasks_own_files_the_review_is_repeated() {
    let w = world();
    w.script(json!({ "worker": [{ "files": { "a.txt": "one\nand more from the agent\n" }, "commit": "agent", "note": "Extended a.txt." }] }));
    let id = filing::add(&w.root, "Renamed", Default::default()).unwrap().t("id");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval"); assert_eq!(t.i("reviewRound"), 1);
    git(&w.root, &["mv", "a.txt", "b.txt"]); git(&w.root, &["commit", "-qm", "main: rename"]);
    let target = git(&w.root, &["rev-parse", "HEAD"]);
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(holds(tree(&t), &target));
    assert_eq!(read(&tree(&t).join("b.txt")), "one\nand more from the agent\n", "the edit followed the rename");
    assert_eq!(t.i("reviewRound"), 2); assert!(!t.at("review").b("keptAt"), "what would land changed shape: reviewed again");
}

#[test]
fn with_approve_a_conflicting_main_is_found_by_a_test_merge_and_goes_to_the_agent_as_a_real_conflict_does() {
    let w = world_with(Opts { repo: true, settings: json!({ "merge.sync": "approve" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "a.txt": "from the agent\n" }, "commit": "agent", "note": "Changed a.txt." }, { "note": null, "status": "working" }] }));
    let id = w.add("Clash");
    delegate(&id);
    let before = settle(&id);
    assert_eq!(before.s("stage"), "approval");
    let target = main_moves(&w, "a.txt", "from main\n");
    // the test merge itself: made in memory, it names the conflict and leaves the worktree and the branch as they were
    let r = sw::sync::test_main(&before, before.s("worktree"));
    assert_eq!((r.status, r.files.as_str(), r.target.as_str(), r.restored), ("conflict", "a.txt", target.as_str(), true));
    assert_eq!(head(&before), before.s("reviewedHead")); assert_eq!(git(tree(&before), &["status", "--porcelain"]), "");
    housekeep(&w);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working"); assert_eq!(t.at("failure").s("title"), "Merge conflict with main"); assert!(t.at("failure").s("detail").contains("Conflicting files:\na.txt"));
    assert_eq!(row(&id).s("note"), "Resolving a conflict with main");
    let said = pane_prompts(&w, &id).last().cloned().unwrap();
    assert!(said.contains("Conflicting files:\n- a.txt") && said.contains(&format!("Target commit: {target}")), "{said}");
    // the test merge left nothing behind: no merge under way, the branch where it was
    assert_eq!(head(&t), head(&before)); assert_eq!(git(tree(&t), &["status", "--porcelain"]), "");
    assert!(common::worktrees::git(tree(&t), &["rev-parse", "-q", "--verify", "MERGE_HEAD"]).is_err());
    assert!(finish_line::approve(&id, None, false).unwrap_err().msg.contains("not waiting for an approval"));
}

#[test]
fn with_approve_main_is_merged_and_checked_after_your_yes_and_the_work_lands() {
    let w = world_with(Opts { repo: true, settings: json!({ "merge.sync": "approve", "review.when": "never" }), repo_settings: Some(json!({ "check.command": "echo run >> \"$HOME/checks.log\"" })), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = w.add("Later");
    delegate(&id);
    let before = settle(&id);
    assert_eq!(before.s("stage"), "approval"); assert_eq!(checks(&w), 1);
    let target = main_moves(&w, "g.txt", "from main\n");
    housekeep(&w);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert_eq!(head(&t), head(&before)); assert_eq!(checks(&w), 1, "nothing to check again while it waits");
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(checks(&w), 2, "the branch with main merged in is checked before it lands");
    assert!(holds(&w.root, &target)); assert_eq!(read(&w.root.join("f.txt")), "v1\n"); assert_eq!(read(&w.root.join("g.txt")), "from main\n");
}

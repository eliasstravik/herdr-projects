//! Keeping tasks up to date with a remote's main: main moving there is noticed by the periodic fetch and the updated
//! branch pushed; a fetch that fails is never a sync, and never fails finished work; with pull requests off, your
//! local branch is the main. Under `merge.sync approve` a waiting task is only test-merged: no merge commit, no push.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::actions::{finish_line, talk};
use sw::json::*;
use sw::store::tasks::{update, Change};

fn holds(dir: &Path, sha: &str) -> bool { common::worktrees::git(dir, &["merge-base", "--is-ancestor", sha, "HEAD"]).is_ok() }
fn tree(t: &Obj) -> &Path { Path::new(t.s("worktree")) }
fn sync_jobs() -> usize { sw::runner::JOBS.with(|j| j.borrow().iter().filter(|(job, _, _)| job == "sync").count()) }
fn housekeep(w: &World) { sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes); }
fn clear_jobs() { sw::runner::JOBS.with(|j| j.borrow_mut().clear()); }
fn last_prompt(w: &World, id: &str) -> String { prompts(w, task(id).at("link").s("pane")).last().cloned().unwrap_or_default() }
fn place(t: &Obj) -> String { sw::flow::sidebar::entry_now(t, sw::clock::now()).place }
/// The remote, and a place where no remote is.
fn remote(w: &World) -> (String, String) { (w.base.join("origin.git").display().to_string(), w.base.join("gone.git").display().to_string()) }
fn head(t: &Obj) -> String { git(tree(t), &["rev-parse", "HEAD"]) }
fn pane_prompts(w: &World, id: &str) -> Vec<String> { prompts(w, task(id).at("link").s("pane")) }
/// What the task's branch holds at the remote.
fn pushed(w: &World, t: &Obj) -> String { git(&w.root, &["ls-remote", "--heads", "origin", t.s("branch")]).split('\t').next().unwrap_or("").to_string() }
/// Someone else pushes to main at the remote: what they pushed.
fn they_push(w: &World, file: &str, body: &str) -> String {
    let other = w.base.join("other");
    git(&w.base, &["clone", "-q", &w.base.join("origin.git").display().to_string(), &other.display().to_string()]);
    std::fs::write(other.join(file), body).unwrap(); git(&other, &["add", "-A"]); git(&other, &["commit", "-qm", "theirs"]); git(&other, &["push", "-q", "origin", "main"]);
    git(&other, &["rev-parse", "HEAD"])
}

#[test]
fn with_a_remote_main_moving_there_is_noticed_by_the_periodic_fetch_and_the_updated_branch_is_pushed() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = w.add("Remote");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval"); assert_eq!(t.s("syncSeen"), git(&w.root, &["rev-parse", "origin/main"]));
    let target = they_push(&w, "g.txt", "theirs\n");
    clear_jobs();
    housekeep(&w);
    assert_eq!(sync_jobs(), 0, "nothing known locally yet");
    clear_jobs();
    sw::housekeeping::poll_prs();
    housekeep(&w);
    assert_eq!(sync_jobs(), 1);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert!(holds(tree(&t), &target)); assert_eq!(t.s("syncedTo"), target); assert_eq!(t.s("syncSeen"), target);
    assert_eq!(git(&w.root, &["ls-remote", "--heads", "origin", t.s("branch")]).split('\t').next().unwrap(), git(tree(&t), &["rev-parse", "HEAD"]));
    assert_eq!(prompts(&w, t.at("link").s("pane")).len(), 1);
    clear_jobs();
    housekeep(&w);
    assert_eq!(sync_jobs(), 0, "and it is not asked again");
}

#[test]
fn a_fetch_that_fails_is_not_a_sync_a_finished_turn_is_held_and_a_waiting_task_is_tried_again_once_the_remote_answers() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let (bare, gone) = remote(&w);
    // before review: main cannot be fetched (pushing still works). The finished work is not failed for it: it is
    // checked, then waits as "behind main", never approvable, with what git said, and the board tries again by itself
    w.script(json!({ "worker": [{ "note": "Which one?" }, { "files": { "a.txt": "from the agent\n" }, "commit": "agent", "note": "Changed a.txt." }] }));
    let id = w.add("Offline");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "asks");
    git(&w.root, &["remote", "set-url", "--push", "origin", &bare]);
    git(&w.root, &["remote", "set-url", "origin", &gone]);
    talk::tell(&id, "The first one.", false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(t.at("failure").is_null()); assert!(t.b("reviewedHead"));
    assert!(t.at("syncBlocked").s("why").starts_with("main could not be fetched (origin is not a repository the board can read"), "{}", t.at("syncBlocked"));
    assert!(t.at("syncBlocked").s("detail").contains("does not appear to be a git repository"), "{}", t.at("syncBlocked"));
    assert!(t.s("msg").starts_with("Behind main")); assert_ne!(row(&id).s("state"), "error"); assert_eq!(place(&t), "behind main");
    assert!(finish_line::approve(&id, None, false).unwrap_err().msg.contains("behind main and cannot be approved yet"));
    git(&w.root, &["remote", "set-url", "origin", &bare]);
    update(&id, |t| { t.put("syncRetryAt", sw::clock::now() - 1); Ok(Change::Write) }).unwrap();
    clear_jobs();
    housekeep(&w);
    assert_eq!(sync_jobs(), 1);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(t.at("syncBlocked").is_null(), "{}", t.at("syncBlocked")); assert!(!t.s("msg").starts_with("Behind main"));
    // waiting for approval: main moves at the remote, with a conflict, and is known here; then the remote goes away
    let target = they_push(&w, "a.txt", "theirs\n");
    sw::housekeeping::poll_prs();
    let seen = task(&id).at("syncSeen").clone();
    git(&w.root, &["remote", "set-url", "origin", &gone]);
    clear_jobs();
    housekeep(&w);
    assert_eq!(sync_jobs(), 1);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval"); assert_eq!(t.at("syncSeen"), &seen, "the failed attempt is not marked as seen"); assert!(t.i("syncRetryAt") > sw::clock::now());
    // and it does not look ready: the row says why, and an approval is refused
    assert!(t.at("syncBlocked").s("why").contains("main could not be fetched"), "{}", t.at("syncBlocked")); assert!(t.s("msg").starts_with("Behind main"));
    assert_eq!((row(&id).at("asReview").clone(), place(&t)), (json!(false), "behind main".to_string()));
    assert!(finish_line::approve(&id, None, false).unwrap_err().msg.contains("behind main and cannot be approved yet"));
    clear_jobs();
    housekeep(&w);
    assert_eq!(sync_jobs(), 0, "not hammered: it waits for the pause");
    // the remote is back, and main has not moved again: the retry still comes, and finds the conflict
    git(&w.root, &["remote", "set-url", "origin", &bare]);
    update(&id, |t| { t.put("syncRetryAt", sw::clock::now() - 1); Ok(Change::Write) }).unwrap();
    w.script(json!({ "worker": [{ "note": null, "status": "working" }] }));
    housekeep(&w);
    assert_eq!(sync_jobs(), 1);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working"); assert_eq!(t.at("failure").s("title"), "Merge conflict with main"); assert!(t.at("syncBlocked").is_null());
    assert!(last_prompt(&w, &id).contains(&format!("Target commit: {target}")));
}

#[test]
fn with_a_remote_but_pull_requests_off_tasks_land_in_your_local_branch_the_main_they_are_kept_up_to_date_with() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never", "merge.pr": false }), ..Default::default() });
    let start = |title: &str, file: &str| { w.script(json!({ "worker": [{ "files": { "a.txt": file }, "commit": title, "note": format!("Did {title}.") }] })); let id = w.add(title); delegate(&id); assert_eq!(settle(&id).s("stage"), "approval"); id };
    let first = start("first", "first\n");
    let second = start("second", "second\n");
    assert!(!task(&first).b("pr")); assert_eq!(task(&second).s("syncBranch"), "main");
    w.script(json!({ "worker": [{ "note": null, "status": "working" }] })); // the agent of `second`, told about its conflict
    finish_line::approve(&first, None, false).unwrap();
    assert_eq!(settle(&first).s("stage"), "done");
    let local = git(&w.root, &["rev-parse", "HEAD"]);
    assert_ne!(local, git(&w.root, &["rev-parse", "origin/main"]), "landed here, not at the remote");
    housekeep(&w);
    let t = settle(&second);
    assert_eq!(t.s("stage"), "working"); assert_eq!(t.at("failure").s("title"), "Merge conflict with main");
    assert!(last_prompt(&w, &second).contains(&format!("Target commit: {local} (main)")), "{}", last_prompt(&w, &second));
    assert!(finish_line::approve(&second, None, false).unwrap_err().msg.contains("not waiting for an approval"));
}

#[test]
fn when_the_periodic_fetch_fails_nothing_is_known_about_main_waiting_tasks_are_held_as_behind_main_until_a_fetch_works() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let (bare, gone) = remote(&w);
    w.script(json!({ "worker": [{ "files": { "a.txt": "from the agent\n" }, "commit": "agent", "note": "Changed a.txt." }, { "note": null, "status": "working" }] }));
    let id = w.add("Unknown");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "approval");
    // main moves at the remote with a conflict, and the remote cannot be reached: the local ref still says "nothing new"
    let target = they_push(&w, "a.txt", "theirs\n");
    git(&w.root, &["remote", "set-url", "origin", &gone]);
    clear_jobs();
    sw::housekeeping::poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval"); assert!(t.at("syncBlocked").s("why").contains("main could not be fetched"), "{}", t.at("syncBlocked"));
    assert_eq!((row(&id).at("asReview").clone(), place(&t)), (json!(false), "behind main".to_string()));
    assert!(finish_line::approve(&id, None, false).unwrap_err().msg.contains("behind main and cannot be approved yet"));
    // the remote is back: the retry finds the conflict
    git(&w.root, &["remote", "set-url", "origin", &bare]);
    update(&id, |t| { t.put("syncRetryAt", 0); Ok(Change::Write) }).unwrap();
    housekeep(&w);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working"); assert_eq!(t.at("failure").s("title"), "Merge conflict with main");
    assert!(last_prompt(&w, &id).contains(&format!("Target commit: {target}")));
}

#[test]
fn with_approve_main_moving_at_the_remote_makes_no_merge_and_no_push_while_it_waits_and_always_still_does() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never", "merge.checks": "none" }), ..Default::default() });
    w.gh_says(json!({ "origin": w.base.join("origin.git").display().to_string() }));
    let start = |title: &str, file: &str, how: &str| { w.script(json!({ "worker": [{ "files": { file: format!("{title}\n") }, "commit": title, "note": format!("Did {title}.") }] })); let id = w.add(title); delegate_with(&id, json!({ "merge.sync": how })); assert_eq!(settle(&id).s("stage"), "approval"); id };
    let busy = start("busy", "busy.txt", "always");
    let calm = start("calm", "calm.txt", "approve"); // last, so the one pull request the fake gh knows is its own
    let (before, was) = (task(&calm), pushed(&w, &task(&calm)));
    assert_eq!(was, head(&before), "its branch was pushed once, before it waited");
    let target = they_push(&w, "g.txt", "theirs\n");
    sw::housekeeping::poll_prs();
    clear_jobs();
    housekeep(&w);
    assert_eq!(sync_jobs(), 2, "both look at main");
    // approve: a test merge only. The branch, the worktree and the pull request are as they were
    let t = settle(&calm);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(t.at("syncBlocked").is_null() && t.at("failure").is_null());
    assert_eq!(head(&t), head(&before)); assert!(!holds(tree(&t), &target)); assert_eq!(git(tree(&t), &["status", "--porcelain"]), "");
    assert_eq!(pushed(&w, &t), was, "nothing pushed");
    assert_eq!(t.s("syncSeen"), target, "looked at once"); assert_eq!(t.s("syncedTo"), before.s("syncedTo"), "but not taken in");
    assert_eq!(t.s("msg"), before.s("msg")); assert_eq!(pane_prompts(&w, &calm).len(), 1);
    // always: merged and pushed as before
    let b = settle(&busy);
    assert_eq!(b.s("stage"), "approval", "{}", b.at("failure")); assert!(holds(tree(&b), &target)); assert_eq!(pushed(&w, &b), head(&b)); assert_eq!(b.s("syncedTo"), target);
    clear_jobs();
    housekeep(&w);
    assert_eq!(sync_jobs(), 0, "and neither is asked again until main moves");
    // after your yes: main is merged in, pushed, and the pull request merges at that commit
    finish_line::approve(&calm, None, false).unwrap();
    let t = settle(&calm);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    // merged through its pull request at a commit that holds main, the one pushed after main was merged in
    let at = gh_log(&w).lines().filter_map(|l| l.strip_prefix("pr merge 7 --squash --match-head-commit ")).last().unwrap_or("").to_string();
    assert!(w.gh.merged.get()); assert!(!at.is_empty() && common::worktrees::git(&w.base.join("origin.git"), &["merge-base", "--is-ancestor", &target, &at]).is_ok(), "merged at a commit that holds main: {at}");
}

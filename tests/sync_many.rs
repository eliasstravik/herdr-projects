//! Many tasks of one repo bringing main in at the same moment (one fetch at a time, shared, never "cannot lock ref"),
//! a remote whose main branch is not called main and was never named here, and a remote that cannot be reached
//! before the push or at the merge: the work waits as "behind main", is never failed for it, and goes on by itself.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::actions::{finish_line, talk};
use sw::json::*;
use sw::store::tasks::{update, Change};

fn housekeep(w: &World) { sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes); }
fn holds(dir: &Path, sha: &str) -> bool { common::worktrees::git(dir, &["merge-base", "--is-ancestor", sha, "HEAD"]).is_ok() }
/// Someone else pushes to `branch` at the remote: what they pushed.
fn they_push(w: &World, branch: &str, file: &str) -> String {
    let other = w.base.join("other");
    if !other.exists() { git(&w.base, &["clone", "-q", &w.base.join("origin.git").display().to_string(), &other.display().to_string()]); }
    git(&other, &["fetch", "-q", "origin"]); git(&other, &["checkout", "-q", "-B", branch, &format!("origin/{branch}")]);
    std::fs::write(other.join(file), "theirs\n").unwrap(); git(&other, &["add", "-A"]); git(&other, &["commit", "-qm", "theirs"]); git(&other, &["push", "-q", "origin", branch]);
    git(&other, &["rev-parse", "HEAD"])
}

#[test]
fn six_tasks_of_one_repo_bringing_main_in_at_the_same_moment_all_get_it() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let ids: Vec<String> = (0..6).map(|i| {
        w.script(json!({ "worker": [{ "files": { format!("f{i}.txt"): "mine\n" }, "commit": format!("task {i}"), "note": "Done." }] }));
        let id = w.add(&format!("Task {i}"));
        delegate(&id);
        assert_eq!(settle(&id).s("stage"), "approval", "{}", task(&id).at("failure"));
        id
    }).collect();
    // main moves at the remote, and every task's worktree asks for it at once: each its own thread, as six runners would
    for round in 0..3 {
        let target = they_push(&w, "main", &format!("theirs{round}.txt"));
        let tasks: Vec<Obj> = ids.iter().map(|id| task(id)).collect();
        let found: Vec<sw::sync::Synced> = std::thread::scope(|s| {
            let runs: Vec<_> = tasks.iter().map(|t| s.spawn(move || sw::sync::sync_main(t, t.s("worktree")))).collect();
            runs.into_iter().map(|r| r.join().unwrap()).collect()
        });
        for (t, r) in tasks.iter().zip(&found) {
            assert_eq!(r.status, "merged", "round {round}, {}: {}", t.s("id"), r.detail);
            assert_eq!(r.target, target); assert!(holds(Path::new(t.s("worktree")), &target));
        }
    }
}

#[test]
fn a_remote_main_not_called_main_and_never_named_here_is_asked_of_the_remote_and_kept_up_with() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    // origin's main branch is trunk, and this clone was never told (a remote added by hand: no origin/HEAD)
    let bare = w.base.join("origin.git");
    git(&w.root, &["push", "-q", "origin", "main:trunk"]);
    git(&bare, &["symbolic-ref", "HEAD", "refs/heads/trunk"]);
    git(&w.root, &["push", "-q", "origin", "--delete", "main"]);
    git(&w.root, &["update-ref", "-d", "refs/remotes/origin/main"]);
    assert!(common::worktrees::git(&w.root, &["symbolic-ref", "-q", "refs/remotes/origin/HEAD"]).is_err());
    w.script(json!({ "worker": [{ "note": "Which one?" }, { "files": { "a.txt": "mine\n" }, "commit": "mine", "note": "Done." }] }));
    let id = w.add("Trunk");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "asks");
    // a new task's start asks origin which branch is its main
    assert_eq!(git(&w.root, &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"]), "origin/trunk"); assert_eq!(t.s("from"), "origin/HEAD");
    // one started before that (from your checkout's commit), with origin/HEAD unknown again: the sync asks origin too
    git(&w.root, &["remote", "set-head", "origin", "--delete"]);
    git(&w.root, &["update-ref", "-d", "refs/remotes/origin/trunk"]); // not even a copy of trunk here (renamed at the remote)
    update(&id, |t| { t.put("from", "HEAD"); Ok(Change::Write) }).unwrap();
    let target = they_push(&w, "trunk", "theirs.txt");
    talk::tell(&id, "The first one.", false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(t.at("syncBlocked").is_null(), "{}", t.at("syncBlocked"));
    assert_eq!(t.s("syncedTo"), target); assert!(holds(Path::new(t.s("worktree")), &target));
}

#[test]
fn a_finished_turn_whose_remote_cannot_be_reached_at_all_waits_before_its_push_and_goes_on_by_itself() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let (bare, gone) = (w.base.join("origin.git").display().to_string(), w.base.join("gone.git").display().to_string());
    w.script(json!({ "worker": [{ "note": "Which one?" }, { "files": { "a.txt": "mine\n" }, "commit": "mine", "note": "Done." }] }));
    let id = w.add("Offline");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "asks");
    // neither fetching nor pushing works
    git(&w.root, &["remote", "set-url", "origin", &gone]);
    talk::tell(&id, "The first one.", false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(t.at("failure").is_null(), "{}", t.at("failure")); assert!(!t.b("reviewedHead")); assert!(!t.b("pr"));
    assert!(t.at("syncBlocked").s("why").starts_with("main could not be fetched (origin is not a repository the board can read"), "{}", t.at("syncBlocked"));
    assert!(t.at("syncBlocked").s("why").ends_with("and the branch could not be pushed"), "{}", t.at("syncBlocked"));
    assert!(t.at("syncBlocked").s("detail").contains("does not appear to be a git repository"));
    assert!(t.s("msg").starts_with("Behind main")); assert_ne!(row(&id).s("state"), "error");
    assert!(finish_line::approve(&id, None, false).unwrap_err().msg.contains("behind main and cannot be approved yet"));
    // still unreachable at the next try: it keeps waiting, and still goes on by itself later
    update(&id, |t| { t.put("syncRetryAt", sw::clock::now() - 1); Ok(Change::Write) }).unwrap();
    housekeep(&w);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval"); assert!(t.at("syncBlocked").b("rerun"), "{}", t.at("syncBlocked")); assert!(t.at("failure").is_null());
    // the remote answers: pushed, its pull request opened, and ready to approve
    git(&w.root, &["remote", "set-url", "origin", &bare]);
    update(&id, |t| { t.put("syncRetryAt", sw::clock::now() - 1); Ok(Change::Write) }).unwrap();
    housekeep(&w);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(t.at("syncBlocked").is_null(), "{}", t.at("syncBlocked"));
    assert!(t.b("pr")); assert!(t.b("reviewedHead"));
    assert_eq!(git(&w.root, &["ls-remote", "--heads", "origin", t.s("branch")]).split('\t').next().unwrap(), git(Path::new(t.s("worktree")), &["rev-parse", "HEAD"]));
    finish_line::approve(&id, None, false).unwrap();
    assert_eq!(settle(&id).s("stage"), "done");
}

#[test]
fn a_merge_that_cannot_fetch_main_waits_as_behind_main_and_is_not_failed_and_goes_on_once_the_remote_answers() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let id = to_approval(&w, "Approved");
    // main cannot be fetched now, pushing still works
    let (bare, gone) = (w.base.join("origin.git").display().to_string(), w.base.join("gone.git").display().to_string());
    git(&w.root, &["remote", "set-url", "--push", "origin", &bare]);
    git(&w.root, &["remote", "set-url", "origin", &gone]);
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval"); assert!(t.at("failure").is_null(), "{}", t.at("failure")); assert!(!t.b("mergeFailed")); assert!(t.at("mergeIntent").is_null());
    assert!(t.at("syncBlocked").s("why").starts_with("main could not be fetched (origin is not a repository the board can read"), "{}", t.at("syncBlocked"));
    assert!(t.at("syncBlocked").s("detail").contains("What git said (git fetch origin main):"), "{}", t.at("syncBlocked"));
    assert!(t.s("msg").starts_with("Behind main")); assert_ne!(row(&id).s("state"), "error");
    assert!(!gh_log(&w).contains("pr merge"), "nothing merged on a main it could not see");
    // the remote answers again: the board's own retry clears it, and the approval goes through
    git(&w.root, &["remote", "set-url", "origin", &bare]);
    update(&id, |t| { t.put("syncRetryAt", sw::clock::now() - 1); Ok(Change::Write) }).unwrap();
    housekeep(&w);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval"); assert!(t.at("syncBlocked").is_null(), "{}", t.at("syncBlocked"));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
}

#[test]
fn a_lock_file_left_in_the_way_is_named_in_plain_words_and_the_work_waits_behind_main_until_it_is_gone() {
    let w = world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let id = to_approval(&w, "Locked");
    let lock = w.root.join(".git/refs/remotes/origin/main.lock");
    std::fs::write(&lock, "").unwrap();
    they_push(&w, "main", "theirs.txt");
    let t = task(&id);
    let r = sw::sync::sync_main(&t, t.s("worktree"));
    assert_eq!(r.status, "error");
    assert!(r.detail.starts_with(&format!("git's lock file {} stayed in the way on every try", lock.display())), "{}", r.detail);
    assert!(sw::sync::held_why(&r).starts_with("main could not be fetched (git's lock file"), "{}", sw::sync::held_why(&r));
    std::fs::remove_file(&lock).unwrap();
    assert_eq!(sw::sync::sync_main(&t, t.s("worktree")).status, "merged");
}

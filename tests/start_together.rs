//! Tasks of one repo that start at the same moment (issue #276): a worktree starts from the commit, not from a
//! remote-tracking ref, so git writes no upstream into the repo's shared .git/config; a create refused on a held
//! config lock is tried again, and a start that fails anyway leaves no empty branch behind.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;

fn origin_world() -> World { world_with(Opts { repo: true, origin: true, ..Default::default() }) }
fn creates(w: &World) -> Vec<String> { calls(w).into_iter().filter(|c| c.starts_with("worktree create ")).collect() }
fn work_branches(w: &World) -> String { git(&w.root, &["branch", "--list", "work/*"]) }

#[test]
fn two_tasks_started_together_while_the_config_is_held_both_start_from_the_commit() {
    let w = origin_world();
    // Another git command holds the repo's config (another task's `git push -u`, another start).
    let lock = w.root.join(".git").join("config.lock");
    std::fs::write(&lock, "").unwrap();
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }] }));
    let (a, b) = (w.add("First"), w.add("Second"));
    delegate(&a); delegate(&b);
    let (a, b) = (settle(&a), settle(&b));
    std::fs::remove_file(&lock).unwrap();
    let main = git(&w.root, &["rev-parse", "origin/main"]);
    for t in [&a, &b] {
        assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
        assert!(t.s("from").starts_with("origin/"), "the task still records where it started: {}", t.s("from"));
        assert_eq!(t.s("baseSha"), main);
        assert!(common::worktrees::git(&w.root, &["config", &format!("branch.{}.merge", t.s("branch"))]).is_err(), "no upstream until the push");
    }
    let made = creates(&w);
    assert_eq!(made.len(), 2, "{made:?}");
    assert!(made.iter().all(|c| c.contains(&format!("--base {main}"))), "herdr is handed the commit: {made:?}");
}

#[test]
fn a_create_refused_on_a_held_config_lock_is_tried_again() {
    let w = world();
    sw::clock::set(Some(sw::clock::now()));
    w.patch(|s| s["createLocked"] = json!(1));
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Tried again");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert_eq!(creates(&w).len(), 2);
    assert_eq!(work_branches(&w).trim_start_matches(['*', '+', ' ']), t.s("branch"), "the empty branch of the refused try was taken away: the task keeps its name");
}

#[test]
fn a_start_that_fails_leaves_no_branch_and_the_next_start_takes_the_same_name() {
    let w = world();
    sw::clock::set(Some(sw::clock::now()));
    w.patch(|s| s["createLocked"] = json!(99));
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Held all along");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed");
    assert!(t.at("failure").s("detail").contains("could not lock config file"), "{}", t.at("failure"));
    assert_eq!(creates(&w).len(), 5, "tried five times");
    assert_eq!(work_branches(&w), "", "no empty branch left behind");
    // The lock is gone: delegating again starts, on the branch name the first start would have had.
    w.patch(|s| s["createLocked"] = json!(0));
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert!(!t.s("branch").ends_with("-2"), "{}", t.s("branch"));
}

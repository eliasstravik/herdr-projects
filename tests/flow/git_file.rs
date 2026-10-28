//! A repo whose `.git` is a file (its history kept elsewhere: `--separate-git-dir`, a submodule): its tasks still work
//! in a worktree of their own, and its worktrees still belong to it.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::json::*;
use sw::store::repos::{can_worktree, primary_root};

/// The world's repo, its history moved out of it: `.git` is then a one-line file pointing there.
fn history_elsewhere(w: &World) -> std::path::PathBuf {
    let hist = w.base.join("hist.git");
    git(&w.root, &["init", "-q", "--separate-git-dir", &hist.display().to_string()]);
    assert!(w.root.join(".git").is_file());
    hist
}

#[test]
fn a_repo_whose_git_is_a_file_gives_its_task_a_worktree() {
    let w = world();
    history_elsewhere(&w);
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Add a greeting");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{t:?}");
    assert!(Path::new(t.s("worktree")).exists() && t.s("worktree") != w.root_str(), "{t:?}");
    assert!(!can_worktree(Path::new(t.s("worktree"))), "a linked worktree gets none of its own");
    assert_eq!(primary_root(Path::new(t.s("worktree"))), w.root, "the worktree belongs to the repo");
}

#[test]
fn a_submodule_is_a_main_checkout_and_its_worktrees_belong_to_it() {
    let w = world();
    let sub = w.base.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    git(&sub, &["init", "-q", "-b", "main"]); git(&sub, &["commit", "-q", "--allow-empty", "-m", "init"]);
    git(&w.root, &["-c", "protocol.file.allow=always", "submodule", "add", "-q", &sub.display().to_string(), "lib"]);
    let lib = w.root.join("lib");
    assert!(lib.join(".git").is_file() && can_worktree(&lib));
    assert_eq!(primary_root(&lib.join("nested")), lib);
    let id = sw::store::repos::repo_of(&lib).unwrap().0;
    assert_eq!(sw::store::repos::folder_of(&format!("{id}/1")), lib);
    let tree = w.base.join("lib-tree");
    git(&lib, &["worktree", "add", "-q", "-b", "work/x", &tree.display().to_string()]);
    assert!(!can_worktree(&tree));
    assert_eq!(primary_root(&tree), lib, "found among the known repos");
}

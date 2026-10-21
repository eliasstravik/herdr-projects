//! Where the board is "in" as herdr's plugin: the focused pane's repo, also when it `cd`'d out of the repo the space
//! started in; a task's linked worktree is its main repo; a pane folder that is gone falls back to the space's repo.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;

/// The folder the board takes from this plugin context, as herdr hands it to the plugin's pane.
fn folder(c: serde_json::Value) -> String {
    for (k, v) in [("HERDR_PLUGIN_ID", "herdr-projects".to_string()), ("HERDR_PLUGIN_CONTEXT_JSON", c.to_string())] { unsafe { std::env::set_var(k, v) } }
    let f = sw::cli::context_folder();
    for k in ["HERDR_PLUGIN_ID", "HERDR_PLUGIN_CONTEXT_JSON"] { unsafe { std::env::remove_var(k) } }
    f.display().to_string()
}
fn space(root: &Path, checkout: &Path, pane: &Path) -> serde_json::Value {
    json!({ "worktree": { "repo_root": root.display().to_string(), "checkout_path": checkout.display().to_string() }, "focused_pane_cwd": pane.display().to_string(), "workspace_cwd": checkout.display().to_string() })
}

#[test]
fn the_pane_that_cd_d_into_another_repo_is_in_that_repo_and_a_linked_worktree_is_its_main_repo() {
    let w = world();
    let (a, b) = (w.root.clone(), w.base.join("other"));
    std::fs::create_dir_all(b.join("src")).unwrap();
    git(&b, &["init", "-q", "-b", "main"]); std::fs::write(b.join("b.txt"), "b\n").unwrap(); git(&b, &["add", "-A"]); git(&b, &["commit", "-qm", "b"]);
    let s = |p: &Path| p.display().to_string();
    assert_eq!(folder(space(&a, &a, &a)), s(&a), "a pane still in the space's repo");
    assert_eq!(folder(space(&a, &a, &b)), s(&b), "a pane that cd'd into another repo");
    assert_eq!(folder(space(&a, &a, &b.join("src"))), s(&b.join("src")), "…also deeper in it, as it was");
    // A task's space: herdr's checkout is the linked worktree, its repo_root the main repo.
    let wt = w.base.join("wt");
    git(&a, &["worktree", "add", "-q", "-b", "work/1", &s(&wt)]);
    assert_eq!(folder(space(&a, &wt, &wt)), s(&a), "a task's linked worktree is its main repo");
    // A linked worktree the pane cd'd into, in a space of another repo: still its main repo.
    git(&b, &["worktree", "add", "-q", "-b", "work/2", &s(&w.base.join("wt-b"))]);
    assert_eq!(folder(space(&a, &a, &w.base.join("wt-b"))), s(&b), "a linked worktree of another repo is that repo");
    // A pane folder that is gone: the space's repo, as before; no repo either: the space's folder.
    assert_eq!(folder(space(&a, &wt, &w.base.join("gone"))), s(&a));
    assert_eq!(folder(json!({ "focused_pane_cwd": s(&w.base.join("gone")), "workspace_cwd": s(&b) })), s(&b));
    assert_eq!(folder(json!({ "focused_pane_cwd": s(&b) })), s(&b), "no worktree in the context: the pane's folder");
}

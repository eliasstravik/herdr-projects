//! Worktree destination settings, physical resolution, collision reservations and panel display.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::json::*;
use sw::store::settings::change::set_value;
use sw::store::settings::lookup::Scope;
use sw::worktree_destination::{parent, reserve};

#[test]
fn destination_defaults_to_herdr_and_task_overrides_repo_and_repo_file() {
    let w = world();
    let id = w.add("Default location");
    assert_eq!(parent(&w.root, &task(&id)).unwrap(), None);
    w.script(json!({ "worker": [{ "status": "working" }] }));
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert!(!t.b("worktreeLocation"));
    let calls = w.state();
    let call = calls
        .arr("calls")
        .iter()
        .find(|c| text(c).starts_with("worktree create"))
        .unwrap();
    assert!(!text(call).contains("--path"), "{call}");
    set_value(
        "workspace.destination",
        &json!(".repo-worktrees"),
        &Scope::repo("proj"),
    )
    .unwrap();
    assert_eq!(
        parent(&w.root, &task(&id)).unwrap().unwrap(),
        w.root.join(".repo-worktrees")
    );
    set_value(
        "workspace.destination",
        &json!(w.base.join("absolute").display().to_string()),
        &Scope::task(&task(&id)),
    )
    .unwrap();
    assert_eq!(
        parent(&w.root, &task(&id)).unwrap().unwrap(),
        w.base.join("absolute")
    );
    set_value(
        "workspace.destination",
        &json!("none"),
        &Scope::task(&task(&id)),
    )
    .unwrap();
    assert_eq!(parent(&w.root, &task(&id)).unwrap(), None);
}

#[test]
fn a_relative_repository_destination_resolves_from_the_physical_primary_even_through_a_link() {
    let w = world_with(Opts {
        repo: true,
        repo_settings: Some(json!({ "workspace": { "destination": ".worktrees" } })),
        ..Default::default()
    });
    let linked = w.base.join("linked");
    git(
        &w.root,
        &["worktree", "add", "-b", "linked", linked.to_str().unwrap()],
    );
    let alias = w.base.join("alias");
    std::os::unix::fs::symlink(&linked, &alias).unwrap();
    let id = w.add("Linked origin");
    assert_eq!(
        parent(&alias, &task(&id)).unwrap().unwrap(),
        w.root.join(".worktrees")
    );
    // Add/delegate from the linked checkout still identifies the primary repository.
    assert_eq!(sw::store::repos::repo_of(&alias).unwrap().0, "proj");
    w.script(json!({ "worker": [{ "status": "working" }] }));
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert_eq!(
        Path::new(t.s("worktree")).parent(),
        Some(w.root.join(".worktrees").as_path())
    );
    assert_eq!(t.at("worktreeLocation").s("returned"), t.s("worktree"));
    assert!(w
        .state()
        .arr("calls")
        .iter()
        .any(|c| text(c).contains(&format!("--path {}", t.s("worktree")))));
}

#[test]
fn reservations_skip_files_directories_symlinks_and_missing_registered_worktrees() {
    let w = world();
    let base = w.root.join(".worktrees");
    std::fs::create_dir_all(&base).unwrap();
    std::fs::write(base.join("work-1"), "keep").unwrap();
    std::fs::create_dir(base.join("work-1-2")).unwrap();
    std::os::unix::fs::symlink(w.base.join("absent"), base.join("work-1-3")).unwrap();
    let registered = base.join("work-1-4");
    git(
        &w.root,
        &[
            "worktree",
            "add",
            "-b",
            "occupied",
            registered.to_str().unwrap(),
        ],
    );
    // Simulate a missing folder without deleting a worktree: move this fixture's directory away.
    std::fs::rename(&registered, w.base.join("displaced-fixture")).unwrap();
    let a = reserve(&w.root, &base, "work/1").unwrap();
    let b = reserve(&w.root, &base, "work/1").unwrap();
    assert_eq!(a, base.join("work-1-5"));
    assert_eq!(b, base.join("work-1-6"));
    assert!(
        !a.exists() && !b.exists(),
        "Herdr gets an absent checkout path"
    );
    assert_eq!(read(&base.join("work-1")), "keep");
    assert!(base.join("work-1-3").is_symlink());
}

#[test]
fn the_task_panel_names_a_configured_destination() {
    let w = world();
    let id = w.add("Show destination");
    set_value(
        "workspace.destination",
        &json!(".worktrees"),
        &Scope::repo("proj"),
    )
    .unwrap();
    let mut b = common::board::open(&w.root, 60);
    b.s.sel = id;
    b.load();
    assert!(b.screen(160).iter().any(|l| l.contains("under .worktrees")));
}

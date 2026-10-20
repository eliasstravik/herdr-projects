//! Disk guards include a new destination volume without creating folders, including absent and symlinked parents.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::leftovers::room::places;
use sw::store::settings::change::set_value;
use sw::store::settings::lookup::Scope;

#[test]
fn the_first_task_and_its_attempt_check_an_absent_destination_without_creating_it() {
    let w = world();
    let id = w.add("Check destination volume");
    let dest = w.base.join("other-volume/absent/nested");
    set_value(
        "workspace.destination",
        &json!(dest.to_str().unwrap()),
        &Scope::task(&task(&id)),
    )
    .unwrap();
    let t = task(&id);
    assert!(places(&t).contains(&dest));
    assert!(!w.base.join("other-volume").exists());
    let mut attempt = t.clone();
    attempt.put("id", format!("{id}b"));
    attempt.put("attemptOf", id);
    attempt.put("settings", json!({ "worker.agent": "codex" }));
    attempt.put("n", "1b");
    assert!(places(&attempt).contains(&dest));
    assert!(!w.base.join("other-volume").exists());
}

#[test]
fn a_symlinked_destination_keeps_its_volume_visible_to_the_disk_guard() {
    let w = world();
    let id = w.add("Symlinked volume");
    let volume = w.base.join("volume");
    std::fs::create_dir(&volume).unwrap();
    std::os::unix::fs::symlink(&volume, w.root.join("volume-link")).unwrap();
    set_value(
        "workspace.destination",
        &json!("volume-link/nested"),
        &Scope::task(&task(&id)),
    )
    .unwrap();
    let dest = w.root.join("volume-link/nested");
    assert!(places(&task(&id)).contains(&dest));
    assert_eq!(sw::disk::free_at(&dest), sw::disk::free_at(&volume));
    assert!(!volume.join("nested").exists());
}

#[test]
fn unset_existing_or_non_repository_workspaces_do_not_check_an_unused_destination() {
    let w = world();
    let id = w.add("Existing workspace");
    let dest = w.base.join("unused");
    assert!(!places(&task(&id)).contains(&dest));
    set_value(
        "workspace.destination",
        &json!(dest.to_str().unwrap()),
        &Scope::task(&task(&id)),
    )
    .unwrap();
    let mut t = task(&id);
    t.put("worktree", w.root.to_str().unwrap());
    assert!(!places(&t).contains(&dest));
    t.put("worktree", "");
    t.put("startedOnce", true);
    assert!(!places(&t).contains(&dest));
    drop(w);
    let w = world_with(Opts::default());
    let id = w.add("No repository");
    let mut t = task(&id);
    t.obj_mut("settings")
        .put("workspace.destination", dest.to_str().unwrap());
    assert!(!places(&t).contains(&dest));
    assert!(!dest.exists());
}

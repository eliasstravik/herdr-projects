//! `herdr-projects doctor` on what it could clean (HP-32): the worktree of a task that is done, a worktree on a task
//! branch no task has, one git lists whose folder is gone, and an old warm build; never a clean-up still to come, one
//! you turned off, your own branches or the warm build in use. Each with the command that removes it.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::json::JsMut;
use sw::doctor::stale::{ask, check, Left, Why, SHOWN};
use sw::store::tasks::{update, Change};

fn worktree(w: &World, branch: &str, dir: &str) -> String {
    let p = w.base.join("wt").join(dir);
    git(&w.root, &["worktree", "add", "-q", "-b", branch, &p.display().to_string()]);
    p.display().to_string()
}
fn whys(left: &[Left]) -> Vec<Why> { left.iter().map(|l| l.why.clone()).collect() }
fn aged(p: &Path, secs: u64) { std::fs::File::open(p).unwrap().set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(secs)).unwrap(); }

#[test]
fn a_done_task_s_worktree_is_a_leftover_unless_its_clean_up_is_to_come_or_off() {
    let w = world();
    let id = w.add("one");
    let wt = worktree(&w, "work/1-one", "work-1-one");
    let cache = w.base.join("cache");
    update(&id, |t| { t.put("stage", "progress"); t.put("worktree", wt.as_str()); Ok(Change::Write) }).unwrap();
    assert_eq!(whys(&ask(&cache)), [], "a task at work keeps its worktree");
    update(&id, |t| { t.put("stage", "done"); Ok(Change::Write) }).unwrap();
    let left = ask(&cache);
    assert_eq!(whys(&left), [Why::Finished(format!("{}, which is done", short(&id)))]);
    assert!(left[0].bytes > 0 && !left[0].more);
    let f = check(&left);
    assert!(f[0].problem && f[0].what.starts_with(&format!("The worktree of {}, which is done, is still on disk: ", short(&id))), "{:?}", f[0]);
    assert!(f[0].fix.starts_with("Look at it, then: git -C ") && f[0].fix.contains(" worktree remove "), "{}", f[0].fix);
    // Its clean-up is still to come.
    update(&id, |t| { t.put("tidy", json!({ "quietFrom": 1 })); Ok(Change::Write) }).unwrap();
    assert_eq!(whys(&ask(&cache)), []);
    // You keep done tasks' worktrees.
    update(&id, |t| { t.del("tidy"); Ok(Change::Write) }).unwrap();
    std::fs::write(sw::store::files::config_dir().join("config.json"), json!({ "watcher.agent": "none", "coordinator.agent": "none", "workspace.cleanup": false }).to_string()).unwrap();
    assert_eq!(whys(&ask(&cache)), []);
}

#[test]
fn a_task_branch_no_task_has_and_a_folder_git_lost_are_leftovers() {
    let w = world();
    w.add("one"); // the repo is known to the board
    let cache = w.base.join("cache");
    worktree(&w, "work/9-nobody", "work-9-nobody");
    worktree(&w, "mine", "mine"); // your own branch: not the board's
    let gone = worktree(&w, "work/8-gone", "work-8-gone");
    std::fs::remove_dir_all(&gone).unwrap();
    let mut left = ask(&cache);
    left.sort_by_key(|l| l.path.ends_with("work-8-gone"));
    assert_eq!(whys(&left), [Why::NoTask("work/9-nobody".into()), Why::Gone], "{left:?}");
    let f = check(&left);
    assert!(f[0].what.starts_with("No task has the worktree on work/9-nobody: "), "{:?}", f[0]);
    assert!(f[1].what.contains("whose folder is gone") && f[1].fix.starts_with("Run: git -C ") && f[1].fix.ends_with(" worktree prune"), "{:?}", f[1]);
}

#[test]
fn a_warm_build_no_longer_used_is_a_leftover_an_hour_on() {
    let w = world();
    let cache = w.base.join("cache");
    let set = cache.join("warm/123");
    for s in ["seed-a", "seed-b", "seed-c"] { std::fs::create_dir_all(set.join(s).join("target")).unwrap(); std::fs::write(set.join(s).join("target/f"), "x").unwrap(); }
    std::fs::write(set.join("current"), "seed-b\n").unwrap();
    for s in ["seed-a", "seed-b"] { aged(&set.join(s), 7200); }
    let left = ask(&cache);
    assert_eq!(whys(&left), [Why::OldBuild]);
    assert!(left[0].path.ends_with("seed-a"));
    assert!(check(&left)[0].fix.starts_with("Run: rm -rf "));
}

#[test]
fn nothing_to_clean_is_one_line_and_many_end_in_a_count() {
    let _w = world();
    assert_eq!(check(&[]), [sw::doctor::Found::ok("No leftover worktrees or build caches to clean")]);
    let many: Vec<Left> = (0..SHOWN + 3).map(|i| Left { path: format!("/w/{i}").into(), repo: "/r".into(), why: Why::OldBuild, bytes: 1_000_000_000, more: false }).collect();
    let f = check(&many);
    assert_eq!(f.len(), SHOWN + 1);
    assert_eq!(f[SHOWN].what, "And 3 more leftovers (3.0 GB)");
}

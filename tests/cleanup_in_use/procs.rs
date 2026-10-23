//! A clean-up ends what a task left running from its worktree right before the worktree goes (a dev server handed to
//! init), by a stop's rules: never what runs in another of its panes, which is named on its line for you to end, as is
//! what would not end. Clear does the same; a worktree kept for its work ends nothing; archive ends nothing and names
//! what still runs from the worktree it keeps.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use sw::actions::{archive, filing, finish_line};
use sw::json::*;

fn sleep_in(dir: &Path) -> Child { Command::new("sleep").arg("60").current_dir(dir).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap() }
fn ended(c: &mut Child) -> bool { (0..50).any(|_| { if c.try_wait().unwrap().is_some() { return true; } std::thread::sleep(std::time::Duration::from_millis(20)); false }) }
fn running(c: &mut Child) -> bool { c.try_wait().unwrap().is_none() }
fn done_with(c: &mut Child) { let _ = c.kill(); let _ = c.wait(); }
fn real(p: &str) -> String { std::fs::canonicalize(p).unwrap().display().to_string() }

#[test]
fn a_merged_task_s_dev_server_in_its_worktree_is_ended_before_the_worktree_goes_and_nothing_is_kept() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let id = to_approval(&w, "Work");
    let t = task(&id);
    let mut server = sleep_in(Path::new(t.s("worktree"))); // in none of its panes: handed to init, as far as herdr knows
    let mut yours = sleep_in(&w.root); // yours, in your own checkout
    finish_line::approve(&id, None, false).unwrap();
    let c = settle(&id);
    assert_eq!(c.s("stage"), "done", "{}", c.at("failure"));
    assert!(ended(&mut server), "{}", c.s("msg"));
    assert!(!Path::new(t.s("worktree")).exists() && c.b("cleaned") && !c.b("kept") && !c.b("stillRunning"), "{c:?}");
    assert!(c.s("msg").contains(&format!("Ended what it started: sleep 60 (pid {}).", server.id())) && !c.s("msg").contains("Kept:"), "{}", c.s("msg"));
    assert!(running(&mut yours) && !c.s("msg").contains(&format!("pid {})", yours.id())), "{}", c.s("msg"));
    done_with(&mut yours);
}

#[test]
fn clear_ends_it_too_and_what_runs_in_another_of_its_panes_is_left_and_named() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let id = to_approval(&w, "Cleared");
    let t = task(&id);
    let mut server = sleep_in(Path::new(t.s("worktree")));
    let (_, kept, ends) = archive::clear(&id).unwrap();
    assert!(ended(&mut server) && !Path::new(t.s("worktree")).exists(), "{kept:?} {ends:?}");
    assert_eq!(ends.said(), format!("Ended what it started: sleep 60 (pid {}).", server.id()));
    // a pane you split off in its space, whose shell (this test's process) runs a command in its worktree
    w.script(json!({ "worker": [{ "files": { "g.txt": "x\n" }, "commit": "g", "note": "Did it." }] }));
    let id = to_approval(&w, "Yours beside it");
    let t = task(&id);
    let mine = sw::herdr::Herdr::new(None).pane_split(t.at("link").s("pane"), t.s("worktree"), "right", &[]).unwrap().t("pane_id");
    w.patch(|s| { let p = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.s("pane_id") == mine).unwrap(); p["shell"] = json!(std::process::id()); });
    let (mut theirs, wt) = (sleep_in(Path::new(t.s("worktree"))), real(t.s("worktree")));
    let (_, _, ends) = archive::clear(&id).unwrap();
    assert!(running(&mut theirs), "{:?}", ends);
    assert_eq!(ends.said(), format!("Still running, end it yourself: sleep 60 (pid {}), in {wt}, from another of its panes.", theirs.id()));
    done_with(&mut theirs);
}

#[test]
fn a_worktree_kept_for_its_uncommitted_changes_ends_nothing_and_archive_names_what_still_runs_from_it() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let id = to_approval(&w, "Kept");
    let t = task(&id);
    std::fs::write(Path::new(t.s("worktree")).join("unsaved.txt"), "x\n").unwrap();
    let mut server = sleep_in(Path::new(t.s("worktree")));
    filing::report(&id, filing::Report { state: "done".into(), ..Default::default() }).unwrap();
    let c = settle(&id);
    assert!(c.arr("kept").iter().any(|k| text(k).contains("uncommitted changes")), "{:?}", c.at("kept"));
    assert!(running(&mut server) && !c.s("msg").contains("Ended") && !c.s("msg").contains("Still running"), "{}", c.s("msg"));
    // archive keeps everything: it ends nothing, and its line names what still runs from the worktree
    let a = archive::archive(&id).unwrap();
    let wt = real(t.s("worktree"));
    assert!(running(&mut server), "{}", a.s("msg"));
    assert!(a.s("msg").contains(&format!("Still running, end it yourself: sleep 60 (pid {}), in {wt}.", server.id())) && !a.s("msg").contains("Ended"), "{}", a.s("msg"));
    assert_eq!(a.at("stillRunning"), &json!([format!("sleep 60 (pid {}), in {wt}", server.id())]));
    assert_eq!(finish_line::details(&id).unwrap().at("stillRunning"), a.at("stillRunning"));
    done_with(&mut server);
}

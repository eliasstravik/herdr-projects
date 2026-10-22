//! A start whose runner was lost (#312): ended with the command that delegated it, before it took the task or after,
//! with no pane yet. It is tried again once, then fails with what happened and how to start it again; so does a later
//! step (a check) whose runner never came, while a merge waiting its turn in the line is left to wait. A command run
//! while a keeper is alive hands the start to the keeper instead of starting it itself.
use crate::common;
use common::*;
use serde_json::json;
use sw::housekeeping::{housekeeping, Force};
use sw::json::*;
use sw::store::tasks::{update_wait, Change};

fn jobs() -> Vec<(String, String, Vec<String>)> { sw::runner::JOBS.with(|j| j.borrow().clone()) }
/// The runners asked for are lost: ended before they took the task.
fn lose_jobs() { sw::runner::JOBS.with(|j| j.borrow_mut().clear()); }
fn pass(w: &World) { housekeeping(&w.socket, Force::Yes); }

#[test]
fn a_start_with_a_dead_runner_and_no_pane_is_tried_again_once_then_fails_with_its_words() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Work");
    delegate(&id);
    assert_eq!(jobs(), vec![("delegate".to_string(), id.clone(), vec![])]);
    lose_jobs();
    // it took the task, and was ended at once: its marker's process is gone, and there is no pane yet
    update_wait(&id, 1000, |t| { t.put("runner", json!({ "pid": i32::MAX, "start": "gone", "started": sw::clock::now(), "job": "delegate" })); Ok(Change::Write) }).unwrap();
    pass(&w);
    let t = task(&id);
    assert_eq!((t.t("stage"), t.i("retried"), t.at("runner").clone()), ("starting".to_string(), 1, json!(null)));
    assert_eq!(jobs(), vec![("delegate".to_string(), id.clone(), vec![])], "tried again, once");
    // the second runner is lost too, before it took the task: no marker at all. Within the grace that is a start on its way.
    lose_jobs();
    sw::clock::advance(sw::hook::GRACE_MS - 1000);
    pass(&w);
    assert_eq!(task(&id).s("stage"), "starting");
    assert!(jobs().is_empty());
    sw::clock::advance(2000);
    pass(&w);
    let t = task(&id);
    assert_eq!(t.s("stage"), "failed");
    assert_eq!(t.s("msg"), "Could not start: its start was cut off, twice");
    let f = t.at("failure");
    assert_eq!(f.s("title"), "The start was cut off");
    assert!(f.s("detail").contains("ended before it was done, twice") && f.s("detail").contains("systemd oneshot"), "{f}");
    assert!(f.s("next").starts_with("Delegate again (") && f.s("next").contains("keeper"), "{f}");
    assert!(jobs().is_empty(), "nothing started a third time");
    // delegated again, it is a new start with its one retry of its own
    delegate(&id);
    assert_eq!(task(&id).i("retried"), 0);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
}

#[test]
fn a_check_whose_runner_is_lost_twice_fails_and_a_merge_waiting_in_the_line_is_left_alone() {
    let w = world();
    let (id, queued) = (w.add("Work"), w.add("Waits its turn"));
    let dead = json!({ "pid": i32::MAX, "start": "gone", "started": sw::clock::now(), "job": "pipeline" });
    update_wait(&id, 1000, |t| { t.put("stage", "checking"); t.put("link", json!({ "socket": w.socket })); t.put("runner", dead.clone()); Ok(Change::Write) }).unwrap();
    update_wait(&queued, 1000, |t| { t.put("stage", "merging"); t.put("lineAt", sw::clock::now()); t.put("link", json!({ "socket": w.socket })); Ok(Change::Write) }).unwrap();
    pass(&w);
    let t = task(&id);
    assert_eq!((t.t("stage"), t.i("retried"), t.at("runner").clone()), ("checking".to_string(), 1, json!(null)));
    assert_eq!(jobs().into_iter().filter(|j| j.1 == id).collect::<Vec<_>>(), vec![("pipeline".to_string(), id.clone(), vec![])], "tried again, once");
    // the retry is lost before it took the task: no marker, and past the grace the check fails
    lose_jobs();
    sw::clock::advance(sw::hook::GRACE_MS + 1000);
    pass(&w);
    let t = task(&id);
    assert_eq!(t.s("stage"), "failed");
    assert_eq!(t.at("failure").s("title"), "A workflow step stopped unexpectedly");
    assert!(jobs().iter().all(|j| j.1 != id), "{:?}", jobs());
    // the merge in the line has no runner until the line gives it its turn: never taken for a lost one
    assert_eq!((task(&queued).t("stage"), task(&queued).i("retried")), ("merging".to_string(), 0));
    assert!(!task(&queued).b("failure"));
}

#[test]
fn a_command_hands_its_start_to_a_live_keeper_and_starts_nothing_itself() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let (alone, kept) = (w.add("No keeper"), w.add("Keeper"));
    // no keeper: the command starts it itself, as before
    assert_eq!(sw::cli::main(vec!["delegate".into(), alone.clone()]), 0);
    assert_eq!(jobs(), vec![("delegate".to_string(), alone.clone(), vec![])]);
    assert!(sw::runner::outside::asked().is_empty());
    lose_jobs();
    // a keeper beats on this machine: the job is written down for it, and the command starts nothing
    std::fs::write(sw::store::files::data_dir().join("keeper.json"), "{}").unwrap();
    assert!(sw::keeper::keeper_alive());
    assert_eq!(sw::cli::main(vec!["delegate".into(), kept.clone()]), 0);
    assert!(jobs().is_empty(), "{:?}", jobs());
    assert_eq!(sw::runner::outside::asked(), vec![("delegate".to_string(), kept.clone(), vec![])]);
    assert_eq!(task(&kept).s("stage"), "starting");
    // the keeper's pass starts it, from its own process
    pass(&w);
    assert_eq!(jobs(), vec![("delegate".to_string(), kept.clone(), vec![])]);
    assert!(sw::runner::outside::asked().is_empty());
    let t = settle(&kept);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
}

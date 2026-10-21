//! An autopilot turn: found as a `Tab` turn is (the coordinator's own pane, still in the turn the wake started), held
//! to the allow-list and its settings, ended by a message of yours, and refused every write once autopilot is off.
use crate::common;
use common::pilot::*;
use common::*;
use serde_json::json;
use sw::coordinator::co::Co;
use sw::json::*;

/// On, its wake typed in and its turn still going: what follows runs in its pane, in that turn.
fn in_wake(w: &World, extra: &[&str]) -> Co {
    let co = proj(w);
    w.script(json!({ "coordinator": [{ "status": "working" }, { "status": "working" }, { "status": "working" }] }));
    let mut a = vec!["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.every", "none"];
    a.extend_from_slice(extra);
    ok(&a);
    tick(w);
    assert_eq!(prompts_of(w, &co).len(), 1);
    in_pane(w, &pane_of(&co));
    assert!(sw::autopilot::turn::autopilot_turn(&co));
    co
}

#[test]
fn add_keeps_to_its_area_its_limits_and_what_it_may_set_and_starts_the_task_at_once() {
    let w = pilot_world();
    let co = in_wake(&w, &["--autopilot.tasksPerDay", "2", "--autopilot.mayChange", "review.*"]);
    let said = ok(&["add", "Fix the docs index", "--repo", "proj", "--review.rounds", "5", "--worker.effort", "high"]);
    assert!(said.contains(": added, an agent is on it"), "{said}");
    assert!(said.contains("Not applied (autopilot.mayChange does not allow it): worker.effort."), "{said}");
    let id = sw::store::tasks::list(Default::default()).into_iter().find(|t| t.s("title") == "Fix the docs index").unwrap().t("id");
    let t = task(&id);
    assert_eq!(t.at("settings").at("review.rounds"), &json!(5)); assert!(t.at("settings").at("worker.effort").is_null());
    assert_eq!(t.at("autopilot").s("coordinator"), "project:proj"); assert_eq!(t.at("autopilot").at("repoFileWaits"), &json!(true));
    assert_eq!(record(&co).i("added"), 1);
    // its tasks at once are full (2): the next add of the day is refused, and so are adds elsewhere
    let other = w.base.join("other"); std::fs::create_dir_all(&other).unwrap(); git(&other, &["init", "-q", "-b", "main"]); sw::store::repos::repo_of(&other).unwrap();
    assert!(refused(&["add", "Elsewhere", "--repo", "other"]).contains("adds tasks only in its own area"));
    ok(&["add", "Second", "--repo", "proj"]);
    assert!(refused(&["add", "Third", "--repo", "proj"]).contains("Today's tasks are used: 2 of 2 added"));
    in_pane(&w, "");
}

#[test]
fn tasks_at_once_hold_two_quick_adds_and_without_start_at_once_a_task_waits_to_do() {
    let w = pilot_world();
    let co = in_wake(&w, &["--autopilot.tasksAtOnce", "1"]);
    ok(&["add", "One", "--repo", "proj"]);
    assert!(refused(&["add", "Two", "--repo", "proj"]).contains("tasks at once are full: 1 of 5 added · 1 of 1 under way"));
    in_pane(&w, "");
    ok(&["autopilot", "on", "--project", "proj", "--autopilot.startAtOnce", "no"]);
    tick(&w);
    in_pane(&w, &pane_of(&co));
    let said = ok(&["add", "Later", "--repo", "proj"]);
    assert!(said.ends_with(": added"), "{said}");
    in_pane(&w, "");
}

#[test]
fn delegate_retry_tell_stop_and_archive_keep_to_the_tasks_it_may_look_after() {
    let w = pilot_world();
    let mine_id;
    {
        let co = in_wake(&w, &["--autopilot.startAtOnce", "no", "--autopilot.retries", "1"]);
        in_pane(&w, "");
        let yours = w.add("Yours");
        in_pane(&w, &pane_of(&co));
        assert!(refused(&["delegate", &yours]).contains("It may start only its own tasks"));
        assert!(refused(&["stop", &yours]).contains("only the tasks it added"));
        assert!(refused(&["tell", &yours, "hi"]).contains("tasks it may look after"));
        ok(&["add", "Mine", "--repo", "proj"]);
        mine_id = sw::store::tasks::list(Default::default()).into_iter().find(|t| t.s("title") == "Mine").unwrap().t("id");
        assert!(refused(&["tell", &mine_id, "carry on"]).contains("waits with a question"));
        let mut sc = w.fake.script.borrow().clone(); sc["worker"] = json!([step(1), step(2)]); w.script(sc);
        ok(&["delegate", &mine_id]);
        let t = settle(&mine_id); assert_eq!(t.s("stage"), "approval");
        sw::store::tasks::set(&mine_id, json!({ "stage": "failed", "failure": { "title": "The check failed", "detail": "boom" } })).unwrap();
        ok(&["retry", &mine_id]);
        assert_eq!(sw::autopilot::record::task_rec(&record(&co), &mine_id).unwrap().i("retries"), 1);
        assert!(refused(&["retry", &mine_id]).contains("Its retries of that task are used"));
        sw::store::tasks::set(&mine_id, json!({ "stage": "approval" })).unwrap();
        assert!(refused(&["archive", &mine_id]).contains("waits for the human's yes"));
        sw::store::tasks::set(&mine_id, json!({ "stage": "todo" })).unwrap();
        ok(&["archive", &mine_id]);
        // under `scope all`, your to-do task in its area is one it may start, and then looks after
        in_pane(&w, "");
        ok(&["autopilot", "on", "--project", "proj", "--autopilot.scope", "all"]);
        tick(&w);
        in_pane(&w, &pane_of(&co));
        ok(&["delegate", &yours]);
        assert_eq!(sw::autopilot::record::task_rec(&record(&co), &yours).unwrap().s("by"), "started");
        in_pane(&w, "");
    }
    let _ = mine_id;
}

#[test]
fn approve_needs_approve_yes_a_task_it_looks_after_and_never_anyway_nor_the_repos_file() {
    let w = pilot_world();
    let co = in_wake(&w, &["--autopilot.startAtOnce", "no"]);
    ok(&["add", "Mine", "--repo", "proj"]);
    let id = sw::store::tasks::list(Default::default()).into_iter().find(|t| t.s("title") == "Mine").unwrap().t("id");
    sw::store::tasks::set(&id, json!({ "stage": "approval" })).unwrap();
    assert!(refused(&["approve", &id]).contains("autopilot.approve yes would allow it"));
    assert!(refused(&["approve", &id, "--anyway"]).contains("The gate's override is the human's only"));
    in_pane(&w, "");
    ok(&["autopilot", "on", "--project", "proj", "--autopilot.approve", "yes"]);
    tick(&w);
    in_pane(&w, &pane_of(&co));
    let yours = { in_pane(&w, ""); let y = w.add("Yours"); in_pane(&w, &pane_of(&co)); y };
    sw::store::tasks::set(&yours, json!({ "stage": "approval" })).unwrap();
    assert!(refused(&["approve", &yours]).contains("for a task it may look after"));
    // a change to the repo's own settings file: never its yes
    let tree = w.base.join("tree"); git(&w.root, &["worktree", "add", "-q", "-b", "work/x", &tree.display().to_string()]);
    std::fs::write(tree.join(".herdr-projects.json"), "{}").unwrap(); git(&tree, &["add", "-A"]); git(&tree, &["commit", "-qm", "x"]);
    sw::store::tasks::set(&id, json!({ "worktree": tree.display().to_string() })).unwrap();
    assert!(refused(&["approve", &id]).contains("touches the repo's .herdr-projects.json"));
    in_pane(&w, "");
}

#[test]
fn unlisted_commands_are_refused_with_the_rule_and_off_is_always_allowed() {
    let w = pilot_world();
    let co = in_wake(&w, &[]);
    assert!(refused(&["set", "review.rounds", "1"]).contains("An autopilot turn cannot change settings"));
    assert!(refused(&["coordinator", "tell", "--machine", "hello"]).contains("Messages to another coordinator"));
    assert!(refused(&["autopilot", "on", "--project", "proj", "--autopilot.tasksPerDay", "50"]).contains("cannot widen its own powers"));
    assert!(refused(&["autopilot", "on", "--machine"]).contains("autopilot.switchOthers yes would allow it"));
    assert!(refused(&["rename", "proj/1", "x"]).contains("That command is not one an autopilot wake may run"));
    assert!(refused(&["automations", "add", "x", "--prompt", "y", "--schedule", "every 2h"]).contains("not one an autopilot wake may run"));
    ok(&["list"]); ok(&["autopilot"]);
    // off, from its own turn: allowed; from then on the turn may only read
    ok(&["autopilot", "off", "--project", "proj"]);
    assert!(refused(&["add", "After", "--repo", "proj"]).contains("Autopilot was turned off"));
    ok(&["list"]);
    in_pane(&w, "");
    let _ = co;
}

#[test]
fn switch_others_lets_it_turn_another_on_with_its_saved_goal_only_and_off_all_covers_this_machine() {
    let w = pilot_world();
    ok(&["set", "autopilot.goal", "Keep the machine tidy", "--machine-coordinator"]);
    let co = in_wake(&w, &["--autopilot.switchOthers", "yes"]);
    assert!(refused(&["autopilot", "on", "--machine", "--goal", "Other"]).contains("no --goal"));
    assert!(refused(&["autopilot", "on", "--machine", "--autopilot.every", "1h"]).contains("no values for this time"));
    assert!(ok(&["autopilot", "on", "--machine"]).starts_with("Autopilot on for the"));
    assert!(sw::autopilot::record::is_on(&Co::machine(&w.socket)));
    let said = ok(&["autopilot", "off", "--all"]);
    assert!(said.contains("2 coordinators") && said.contains("this machine"), "{said}");
    assert!(!sw::autopilot::record::is_on(&co));
    in_pane(&w, "");
}

#[test]
fn a_message_of_yours_ends_the_autopilot_turn_and_list_gives_it_its_history() {
    let w = pilot_world();
    let co = in_wake(&w, &[]);
    // its history: what is finished, stopped or archived in its area, who added it and its closing note
    in_pane(&w, "");
    let done = w.add("Old work");
    sw::store::tasks::set(&done, json!({ "stage": "done", "at": sw::clock::now() })).unwrap();
    std::fs::write(sw::flow::task::note_file(&task(&done)), "Did the old work. The index still needs a pass.").unwrap();
    in_pane(&w, &pane_of(&co));
    let said = ok(&["list"]);
    assert!(said.contains("Finished, stopped and archived in your area") && said.contains("added by the human") && said.contains("The index still needs a pass."), "{said}");
    let d = ok(&["details", &done]);
    assert!(d.contains("Did the old work."), "{d}");
    // your message, typed in: the turn is yours
    end_turn(&w, &co);
    sw::coordinator::requests::send(&w.root, "add a task for the changelog", false, "", &co).unwrap(); drain();
    assert!(!sw::autopilot::turn::autopilot_turn(&co));
    assert!(!ok(&["list"]).contains("Finished, stopped and archived"));
    in_pane(&w, "");
}

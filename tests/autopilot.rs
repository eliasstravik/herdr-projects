//! Autopilot on a coordinator: turned on with a goal (saved as that coordinator's own) and values for this time,
//! woken by the board with a message in its own chat, and turned off again, which drops this time's values. And a
//! coordinator's settings, looked up as a task's are: this time -> this coordinator -> you -> built in, its own an
//! entry of its own in config.json set with flags that cannot clash with a repo's name, none read from a repo's file.
use crate::common;
use common::pilot::*;
use common::*;
use serde_json::json;
use sw::coordinator::co::Co;
use sw::json::*;
use sw::store::settings::lookup::lookup;
use sw::store::settings::places::yours;

/// One that never ran: sent as off, and `Tab` from another machine starts it.
mod never_ran;
/// A pane id herdr handed out again: a done task's old pane given to a coordinator is the coordinator's.
mod pane_reuse;

#[test]
fn on_needs_a_goal_saves_it_as_the_coordinators_own_and_wakes_it_with_a_message_in_its_chat() {
    let w = pilot_world();
    let co = proj(&w);
    assert!(refused(&["autopilot", "on", "--project", "proj"]).contains("Tell a coordinator its goal first."));
    let said = ok(&["autopilot", "on", "--project", "proj", "--goal", "Keep the docs current", "--autopilot.every", "2h"]);
    assert!(said.starts_with("Autopilot on for the proj coordinator. Goal: \"Keep the docs current\". This time: autopilot.every 2h."), "{said}");
    let cfg: serde_json::Value = serde_json::from_str(&read(&sw::store::settings::places::config_file())).unwrap();
    assert_eq!(cfg.at("coordinators").at("projects").at("proj"), &json!({ "autopilot": { "goal": "Keep the docs current" } }));
    assert_eq!(record(&co).at("once"), &json!({ "autopilot.every": "2h" }));
    // the first tick sends the wake of turning on: it starts the coordinator, in its own chat, and is typed in
    tick(&w);
    let p = prompts_of(&w, &co);
    assert_eq!(p, vec!["Autopilot woke: autopilot was turned on. Goal: Keep the docs current. Today: 0 of 5 tasks added, 0 of 2 under way. May set: nothing. May touch: own.".to_string()]);
    assert_eq!(record(&co).i("woke"), 1);
    assert!(record(&co).at("wake").b("typedAt"));
    // its turn ended (the scripted one ends at once): the next look sees it answered
    tick(&w);
    let r = record(&co);
    assert!(r.at("wake").is_null(), "{r:?}");
    assert_eq!(r.arr("wakes").len(), 1); assert_eq!(r.arr("wakes")[0].at("answered"), &json!(true));
    // nothing more to say: no second wake
    minutes(30); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 1);
    // off drops this time's values; the goal stays its own
    assert!(ok(&["autopilot", "off", "--project", "proj"]).starts_with("Autopilot off for the proj coordinator."));
    assert_eq!(record(&co).at("once"), &json!({}));
    assert!(!record(&co).b("on"));
    assert_eq!(sw::autopilot::record::words(&co, "autopilot.goal"), "Keep the docs current");
    assert!(ok(&["autopilot", "off", "--project", "proj"]).contains("is not on autopilot"));
}

fn from(co: &Co, name: &str) -> (String, String) { let f = lookup(name, &sw::autopilot::record::scope_of(co), &yours()).unwrap(); (text(&f.value), f.from) }

#[test]
fn this_time_then_this_coordinator_then_you_then_built_in() {
    let w = pilot_world();
    let co = proj(&w);
    assert_eq!(from(&co, "autopilot.tasksPerDay"), ("5".into(), "built in".into()));
    ok(&["set", "autopilot.tasksPerDay", "4"]);
    assert_eq!(from(&co, "autopilot.tasksPerDay"), ("4".into(), "you".into()));
    let said = ok(&["set", "autopilot.tasksPerDay", "3", "--project-coordinator", "proj"]);
    assert!(said.starts_with("autopilot.tasksPerDay for the proj coordinator: 3"), "{said}");
    assert_eq!(from(&co, "autopilot.tasksPerDay"), ("3".into(), "this coordinator".into()));
    // the main Coordinator and the machine's keep their own: yours
    assert_eq!(from(&Co::top(&w.socket), "autopilot.tasksPerDay").0, "4");
    // this time's, while it is on; gone when it goes off
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.tasksPerDay", "2"]);
    assert_eq!(from(&co, "autopilot.tasksPerDay"), ("2".into(), "this time".into()));
    let shown = ok(&["show", "--project-coordinator", "proj"]);
    assert!(shown.contains("autopilot.tasksPerDay") && shown.contains("this time"), "{shown}");
    ok(&["autopilot", "off", "--project", "proj"]);
    assert_eq!(from(&co, "autopilot.tasksPerDay"), ("3".into(), "this coordinator".into()));
    ok(&["unset", "autopilot.tasksPerDay", "--project-coordinator", "proj"]);
    assert_eq!(from(&co, "autopilot.tasksPerDay"), ("4".into(), "you".into()));
    let cfg: serde_json::Value = serde_json::from_str(&read(&sw::store::settings::places::config_file())).unwrap();
    assert_eq!(cfg.at("coordinators").at("projects").at("proj"), &json!({ "autopilot": { "goal": "Docs" } }), "{cfg}");
}

#[test]
fn a_goal_is_one_coordinators_own_and_a_coordinators_own_settings_are_no_values_for_this_time() {
    let _w = pilot_world();
    assert!(refused(&["set", "autopilot.goal", "Everything"]).contains("is said for one coordinator only"));
    assert!(refused(&["autopilot", "on", "--goal", "x", "--autopilot.goal", "y"]).contains("is not a value for this time"));
    assert!(refused(&["autopilot", "on", "--goal", "x", "--coordinator.agent", "codex"]).contains("is read when a coordinator's chat starts"));
    assert!(refused(&["autopilot", "on", "--goal", "x", "--worker.agent", "codex"]).contains("is not one of autopilot's settings"));
    assert!(refused(&["set", "autopilot.every", "2h", "--repo", "proj"]).contains("is a coordinator's setting"));
    assert!(refused(&["set", "review.rounds", "2", "--main-coordinator"]).contains("is not a coordinator's setting"));
    assert!(refused(&["set", "autopilot.mayChange", "merge.by"]).contains("may never set merge.*"));
    assert!(refused(&["set", "autopilot.mayChange", "*"]).contains("a bare *"));
    assert!(refused(&["set", "autopilot.hours", "9-5"]).contains("is not hours from and to"));
    assert_eq!(ok(&["set", "autopilot.when", "added,done"]).lines().next().unwrap(), "autopilot.when: done added  (you)");
}

#[test]
fn a_coordinators_own_agent_starts_its_next_fresh_chat_and_a_one_time_pick_still_wins_for_its_own() {
    let w = pilot_world();
    let co = proj(&w);
    ok(&["set", "coordinator.agent", "codex", "--project-coordinator", "proj"]);
    assert_eq!(sw::coordinator::who::choice_of(&co).unwrap().t("kind"), "codex");
    assert_eq!(sw::coordinator::who::choice_of(&Co::top(&w.socket)).unwrap().t("kind"), "claude");
    sw::coordinator::requests::send(&w.root, "hello", false, "", &co).unwrap(); drain();
    assert_eq!(common::agent_of(&w, &pane_of(&co)).s("kind"), "codex");
    // `coordinator new --agent` picks for that chat only
    sw::coordinator::requests::fresh(sw::coordinator::who::pick_of("pi", "").unwrap(), &co).unwrap(); drain();
    sw::coordinator::requests::send(&w.root, "again", false, "", &co).unwrap(); drain();
    assert_eq!(common::agent_of(&w, &pane_of(&co)).s("kind"), "pi");
    // its instructions add up: yours, then its own
    ok(&["set", "coordinator.instructions", "Be brief."]);
    ok(&["set", "coordinator.instructions", "Mind the docs.", "--project-coordinator", "proj"]);
    let rules = sw::coordinator::rules::rules_of(&co, "board");
    assert!(rules.contains("Be brief.") && rules.contains("Mind the docs."), "{rules}");
    assert!(!sw::coordinator::rules::rules_of(&Co::top(&w.socket), "board").contains("Mind the docs."));
}

#[test]
fn nothing_of_a_coordinators_settings_is_read_from_the_repos_own_file_and_repos_named_main_or_machine_clash_with_nothing() {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude" }), repo_settings: Some(json!({ "autopilot": { "tasksPerDay": 99 }, "coordinator": { "access": "ask" } })), ..Default::default() });
    sw::store::repos::repo_of(&w.root).unwrap();
    let co = Co::project(&w.socket, "proj");
    assert_eq!(from(&co, "autopilot.tasksPerDay").0, "5");
    assert_eq!(from(&co, "coordinator.access").0, "full");
    // repos named main and machine have project coordinators of their own
    for name in ["main", "machine"] {
        let dir = w.base.join(name); std::fs::create_dir_all(&dir).unwrap(); git(&dir, &["init", "-q", "-b", "main"]);
        sw::store::repos::repo_of(&dir).unwrap();
        ok(&["set", "autopilot.gap", "30m", "--project-coordinator", name]);
        assert_eq!(from(&Co::project(&w.socket, name), "autopilot.gap"), ("30m".into(), "this coordinator".into()));
    }
    assert_eq!(from(&Co::top(&w.socket), "autopilot.gap").0, "10m");
    assert_eq!(from(&Co::machine(&w.socket), "autopilot.gap").0, "10m");
    ok(&["set", "autopilot.gap", "20m", "--machine-coordinator"]);
    assert_eq!(from(&Co::machine(&w.socket), "autopilot.gap").0, "20m");
    assert_eq!(from(&Co::project(&w.socket, "machine"), "autopilot.gap").0, "30m");
    assert!(refused(&["set", "autopilot.gap", "20m", "--project-coordinator", "nope"]).contains("No repo \"nope\""));
}

fn quiet_on(w: &World, extra: &[&str]) -> Co {
    let co = proj(w);
    let mut a = vec!["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.every", "none", "--autopilot.gap", "1m", "--autopilot.startAtOnce", "no"];
    a.extend_from_slice(extra);
    ok(&a);
    tick(w); tick(w);
    co
}
fn wakes(w: &World, co: &Co) -> usize { prompts_of(w, co).iter().filter(|p| p.starts_with("Autopilot woke:")).count() }

#[test]
fn with_no_room_only_a_task_that_needs_looking_after_wakes_it() {
    let w = pilot_world();
    let co = quiet_on(&w, &["--autopilot.tasksAtOnce", "1"]);
    let mine = sw::autopilot::add::add(&co, &w.root, "Mine", Default::default(), &[], false).unwrap().task.t("id");
    sw::store::tasks::set(&mine, json!({ "stage": "working" })).unwrap();
    w.add("Yours"); minutes(2); tick(&w);
    assert_eq!(wakes(&w, &co), 1, "its one slot is taken: no room");
    assert_eq!(sw::autopilot::shown::view(&co, sw::clock::now()).s("state"), "at its limit");
    sw::store::tasks::set(&mine, json!({ "stage": "failed" })).unwrap(); minutes(2); tick(&w);
    assert_eq!(wakes(&w, &co), 2, "a task it looks after failed: that needs it");
}

#[test]
fn with_todays_adds_used_scope_own_has_no_room_and_scope_all_has_while_a_to_do_task_of_yours_waits() {
    let w = pilot_world();
    let co = quiet_on(&w, &["--autopilot.tasksPerDay", "0"]);
    w.add("Yours"); minutes(2); tick(&w);
    assert_eq!(wakes(&w, &co), 0, "not even the wake of turning on: it could do nothing");
    assert_eq!(sw::autopilot::shown::view(&co, sw::clock::now()).s("state"), "at its limit");
    ok(&["autopilot", "on", "--project", "proj", "--autopilot.scope", "all"]); tick(&w); tick(&w);
    assert_eq!(wakes(&w, &co), 1);
    w.add("More of yours"); minutes(2); tick(&w);
    assert_eq!(wakes(&w, &co), 2);
}

#[test]
fn under_approve_yes_a_task_waiting_for_its_yes_wakes_it_but_not_one_held_for_yours() {
    let w = pilot_world();
    let co = quiet_on(&w, &["--autopilot.approve", "yes"]);
    let mine = sw::autopilot::add::add(&co, &w.root, "Mine", Default::default(), &[], false).unwrap().task.t("id");
    tick(&w);
    sw::store::tasks::set(&mine, json!({ "stage": "approval" })).unwrap(); minutes(2); tick(&w);
    assert!(prompts_of(&w, &co).last().unwrap().contains(&format!("{} waits for a yes", short(&mine))));
    let held = sw::autopilot::add::add(&co, &w.root, "Held", Default::default(), &[], false).unwrap().task.t("id");
    let tree = w.base.join("tree"); git(&w.root, &["worktree", "add", "-q", "-b", "work/held", &tree.display().to_string()]);
    std::fs::write(tree.join(".herdr-projects.json"), "{}").unwrap(); git(&tree, &["add", "-A"]); git(&tree, &["commit", "-qm", "x"]);
    sw::store::tasks::set(&held, json!({ "worktree": tree.display().to_string() })).unwrap();
    tick(&w); minutes(2); tick(&w);
    let n = wakes(&w, &co);
    sw::store::tasks::set(&held, json!({ "stage": "approval" })).unwrap(); minutes(2); tick(&w);
    assert_eq!(wakes(&w, &co), n);
}

#[test]
fn a_start_that_fails_is_no_wake_counts_toward_no_limit_is_shown_and_waits_for_the_gap() {
    let w = pilot_world();
    let co = proj(&w);
    w.patch(|s| s["missing"] = json!(["claude"]));
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.gap", "10m"]);
    tick(&w); tick(&w);
    let r = record(&co);
    assert!(!r.s("couldNot").is_empty(), "{r:?}");
    assert_eq!((r.i("woke"), r.i("unanswered")), (0, 0));
    let v = sw::autopilot::shown::view(&co, sw::clock::now());
    assert_eq!(v.s("state"), "could not start");
    assert!(v.s("line").starts_with("autopilot could not start: "), "{}", v.s("line"));
    w.patch(|s| s["missing"] = json!([]));
    minutes(5); tick(&w);
    assert!(prompts_of(&w, &co).is_empty(), "no new try before the gap");
    minutes(6); tick(&w);
    assert_eq!(wakes(&w, &co), 1);
    assert!(record(&co).s("couldNot").is_empty());
}

#[test]
fn ask_first_access_refuses_it_stop_says_it_is_still_on_and_a_new_chat_keeps_it() {
    let w = pilot_world();
    ok(&["set", "coordinator.access", "ask", "--project-coordinator", "proj"]);
    assert!(refused(&["autopilot", "on", "--project", "proj", "--goal", "Docs"]).contains("coordinator.access ask"));
    ok(&["unset", "coordinator.access", "--project-coordinator", "proj"]);
    let co = quiet_on(&w, &["--autopilot.tasksPerDay", "3"]);
    ok(&["set", "coordinator.access", "ask", "--project-coordinator", "proj"]);
    assert_eq!(sw::autopilot::shown::view(&co, sw::clock::now()).s("state"), "needs you");
    assert!(refused(&["autopilot", "on", "--project", "proj"]).contains("coordinator.access ask"));
    ok(&["unset", "coordinator.access", "--project-coordinator", "proj"]);
    assert!(ok(&["coordinator", "stop", "--project", "proj"]).contains("its autopilot is still on: its next wake starts it again"));
    ok(&["coordinator", "new", "--project", "proj"]); drain();
    assert!(record(&co).b("on")); assert_eq!(record(&co).at("once").at("autopilot.tasksPerDay"), &json!(3));
}

#[test]
fn every_coordinators_rules_say_what_a_wake_is_and_hold_every_exception_for_one() {
    let w = pilot_world();
    for co in [Co::top(&w.socket), Co::machine(&w.socket), proj(&w)] {
        let rules = sw::coordinator::rules::rules_of(&co, "board");
        assert_eq!(rules.matches(sw::coordinator::rules_autopilot::EXCEPT).count(), 3, "{rules}");
        assert!(rules.contains("An autopilot wake is a message that starts with the exact words \"Autopilot woke:\""));
        assert!(rules.contains("doing nothing is fine, never invent work") && rules.contains("Never add release tasks"));
        assert!(rules.contains("board autopilot off --all, which covers this machine only"));
    }
}

#[test]
fn end_to_end_a_task_it_adds_is_planned_reviewed_twice_and_merges_by_itself_with_no_yes_of_yours() {
    let w = pilot_world_with(json!({ "review.when": "repos", "merge.by": "itself", "worker.agent": "codex" }));
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap") };
    w.patch(|s| s["pids"] = json!(true));
    ok(&["set", "plan.approve", "reviewer", "--repo", "proj"]);
    let work = |n: i64| json!({ "files": { "f.txt": format!("{n}\n") }, "commit": format!("v{n}"), "note": format!("Did v{n}.") });
    let verdict = |findings: serde_json::Value| json!({ "verdict": { "meets": true, "text": "Looked at it.", "findings": findings } });
    w.script(json!({ "coordinator": [{ "status": "working" }, { "status": "working" }, { "status": "working" }],
        "worker": [{ "plan": "# Plan\n\n1. Add f.txt with one line.\n", "note": "Planned." }, work(1), work(2)],
        "reviewer": [verdict(json!([])), verdict(json!([{ "severity": "major", "subject": "The line is wrong", "where": "f.txt", "proof": "read it" }])), verdict(json!([]))] }));
    let co = proj(&w);
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.every", "none"]);
    tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 1);
    in_pane(&w, &pane_of(&co));
    assert!(sw::autopilot::turn::autopilot_turn(&co));
    let said = ok(&["add", "Write f.txt", "--repo", "proj"]);
    assert!(said.contains(": added, an agent is on it"), "{said}");
    let id = sw::store::tasks::list(Default::default()).into_iter().find(|t| t.s("title") == "Write f.txt").unwrap().t("id");
    assert_eq!(task(&id).at("autopilot").s("coordinator"), "project:proj");
    // what follows is the board's own work, not the coordinator's shell
    in_pane(&w, "");
    // settled as `settle` does, with the clock a second on before each job: this world's clock stands still, and a
    // closing note is known as new by its time, after the turn it closes was armed
    let jobs = || sw::runner::drain_jobs(&|job, id, extra| { sw::clock::advance(1000); sw::jobs::run_job(job, id, extra); flush(); });
    let (mut t, mut stages) = (task(&id), vec![]);
    for _ in 0..40 {
        if t.s("stage") == "done" { break; }
        flush(); jobs(); flush();
        if t.at("link").b("pane") { sw::hook::on_pane(&w.socket, t.at("link").s("pane")); }
        jobs();
        t = task(&id);
        stages.push(t.t("stage"));
    }
    assert_eq!(t.s("stage"), "done", "{} {}", t.at("failure"), t.s("msg"));
    assert_eq!(t.at("plan").s("approvedBy"), "reviewer");
    assert_eq!((sw::store::reviews::review_round(&t), t.i("reviewRounds")), (2, 1));
    assert_eq!(read(&w.root.join("f.txt")), "2\n", "merged by itself");
    assert!(t.b("mergedAt") && !stages.iter().any(|s| s == "approval" || s == "plan"), "no yes of yours was waited for: {stages:?}");
    assert_eq!(record(&co).i("added"), 1);
    let today = sw::autopilot::limits::today(&co, &record(&co), &sw::autopilot::limits::board_tasks(), sw::clock::now());
    assert_eq!(today.say(), "1 of 5 added · 0 of 2 under way", "added today 1, and merged it is under way no more");
}

//! A coordinator's settings, looked up as a task's are: this time -> this coordinator -> you -> built in. Its own are
//! an entry of its own in config.json (`coordinators`), set with flags that cannot clash with a repo's name; this time's
//! are autopilot's while it is on. Nothing of them is ever read from a repo's own file.
use crate::common;
use common::pilot::*;
use common::*;
use serde_json::json;
use sw::coordinator::co::Co;
use sw::json::*;
use sw::store::settings::lookup::lookup;
use sw::store::settings::places::yours;

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

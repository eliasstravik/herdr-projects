//! Who added a task stays on it and is said: one a project coordinator adds from its own pane in another repo (the
//! human asked it to) names that coordinator in `details` and in the other repo's autopilot history; each level, and
//! another machine's, by its name.
use crate::common;
use common::pilot::*;
use common::*;
use serde_json::json;
use sw::autopilot::history::{added_by, with_history};
use sw::coordinator::co::Co;
use sw::json::*;

#[test]
fn a_task_a_project_coordinator_adds_in_another_repo_names_that_coordinator_in_details_and_history() {
    let w = pilot_world();
    let other = w.base.join("other"); std::fs::create_dir_all(&other).unwrap(); git(&other, &["init", "-q", "-b", "main"]); sw::store::repos::repo_of(&other).unwrap();
    let co = proj(&w);
    w.script(json!({ "coordinator": [{ "said": "Added it there, as you asked." }] }));
    ok(&["coordinator", "tell", "--project", "proj", "add a task in the other repo for its docs"]);
    drain();
    assert!(!pane_of(&co).is_empty());
    // from its pane, with --repo other: the command cannot tell whether the human asked, so it adds and records who
    in_pane(&w, &pane_of(&co));
    let said = ok(&["add", "Fix the docs", "--repo", "other"]);
    in_pane(&w, "");
    let id = said.split(':').next().unwrap().to_string();
    assert!(id.starts_with("other/"), "{said}");
    assert_eq!(task(&id).s("coordinator"), "project:proj");
    let d = ok(&["details", &id]);
    assert!(d.contains("\"addedBy\": \"the proj coordinator, asked by the human\""), "{d}");
    // finished: the other repo's coordinator reads in its history who added it
    sw::store::tasks::set(&id, json!({ "stage": "done", "at": sw::clock::now() })).unwrap();
    let (j, text) = with_history(&Co::project(&w.socket, "other"), json!([]), String::new());
    assert!(text.contains("Fix the docs  · added by the proj coordinator, asked by the human"), "{text}");
    assert_eq!(j.arr("history")[0].s("addedBy"), "the proj coordinator, asked by the human");
    // one the human added in their own shell says so
    let mine = w.add("Mine");
    assert!(ok(&["details", &mine]).contains("\"addedBy\": \"the human\""));
}

#[test]
fn each_coordinator_is_named_by_its_level_and_another_machines_by_that_machine() {
    let _w = pilot_world();
    let host = sw::remote::check::hostname();
    for (t, want) in [
        (json!({ "coordinator": "main" }), "the Coordinator, asked by the human".to_string()),
        (json!({ "coordinator": "machine" }), format!("the {host} coordinator, asked by the human")),
        (json!({ "coordinator": "project:proj" }), "the proj coordinator, asked by the human".into()),
        (json!({ "coordinator": "machine", "host": "vm-b" }), "the vm-b coordinator, asked by the human".into()),
        (json!({ "coordinator": "project:web", "host": "vm-b" }), "the web coordinator on vm-b, asked by the human".into()),
        (json!({ "coordinator": "project:proj", "autopilot": { "name": "proj coordinator" } }), "the proj coordinator's autopilot".into()),
    ] { assert_eq!(added_by(t.as_object().unwrap()), want, "{t}"); }
}

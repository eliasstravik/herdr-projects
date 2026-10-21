//! Autopilot on a coordinator: turned on with a goal (saved as that coordinator's own) and values for this time,
//! woken by the board with a message in its own chat, and turned off again, which drops this time's values.
use crate::common;
use common::pilot::*;
use common::*;
use serde_json::json;
use sw::json::*;

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

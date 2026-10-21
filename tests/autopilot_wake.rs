//! Waking a coordinator on autopilot: its timer only when the board changed (the free look), the board's events in its
//! area once each, its awake hours, the gap after a turn and the wakes a day, one wake in the queue at most, and no wake
//! while it is mid-turn or a message of yours waits for it.
use crate::common;
use common::pilot::*;
use common::*;
use serde_json::json;
use sw::coordinator::co::Co;
use sw::json::*;

/// On, with its first wake (of turning on) typed in and answered, so what follows starts from a quiet coordinator.
fn on_and_quiet(w: &World, co: &Co, extra: &[&str]) {
    let mut a = vec!["autopilot", "on", "--project", "proj", "--goal", "Docs"];
    a.extend_from_slice(extra);
    ok(&a);
    tick(w); tick(w);
    assert_eq!(prompts_of(w, co).len(), 1, "{:?}", prompts_of(w, co));
    assert_eq!(record(co).arr("wakes")[0].at("answered"), &json!(true));
}
fn last_prompt(w: &World, co: &Co) -> String { prompts_of(w, co).last().cloned().unwrap_or_default() }
fn own_task(co: &Co, w: &World, title: &str) -> String {
    sw::autopilot::add::add(co, &w.root, title, Default::default(), &[], false).unwrap().task.t("id")
}

#[test]
fn the_timer_wakes_only_when_the_board_changed_and_its_own_adds_wake_it_not() {
    let w = pilot_world();
    let co = proj(&w);
    on_and_quiet(&w, &co, &["--autopilot.every", "1h", "--autopilot.when", "none"]);
    // an hour on, nothing changed: no wake, no model, no tokens
    minutes(61); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 1);
    assert!(record(&co).arr("pending").is_empty());
    // the human adds a task: the next timer wakes it
    w.add("Fix the docs index");
    minutes(30); tick(&w); assert_eq!(prompts_of(&w, &co).len(), 1, "not before its timer");
    minutes(31); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 2);
    assert!(last_prompt(&w, &co).starts_with("Autopilot woke: its timer, and the board changed. Goal: Docs."), "{}", last_prompt(&w, &co));
    // what its own turn did (a task it added in it) is in the snapshot taken when that turn ended: no wake for it
    own_task(&co, &w, "Its own");
    tick(&w); // the board sees the turn's end
    minutes(61); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 2);
    w.add("And yours"); minutes(61); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 3);
}

#[test]
fn events_in_its_area_wake_it_once_and_its_own_adds_and_other_repos_do_not() {
    let w = pilot_world();
    let co = proj(&w);
    on_and_quiet(&w, &co, &["--autopilot.every", "none", "--autopilot.gap", "1m"]);
    let other = w.base.join("other"); std::fs::create_dir_all(&other).unwrap(); git(&other, &["init", "-q", "-b", "main"]);
    sw::actions::filing::add(&other, "Elsewhere", Default::default()).unwrap();
    own_task(&co, &w, "Mine");
    minutes(2); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 1, "{:?}", prompts_of(&w, &co));
    let id = w.add("Yours");
    minutes(2); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 2);
    assert!(last_prompt(&w, &co).starts_with(&format!("Autopilot woke: {} was added.", short(&id))), "{}", last_prompt(&w, &co));
    minutes(2); tick(&w); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 2, "once");
    // done: a task in its area is done
    sw::store::tasks::set(&id, json!({ "stage": "done" })).unwrap();
    minutes(2); tick(&w);
    assert!(last_prompt(&w, &co).starts_with(&format!("Autopilot woke: {} is done.", short(&id))), "{}", last_prompt(&w, &co));
}

#[test]
fn a_task_of_its_own_that_starts_to_need_looking_after_wakes_it_once_and_after_its_retries_is_left_to_you() {
    let w = pilot_world();
    let co = proj(&w);
    on_and_quiet(&w, &co, &["--autopilot.every", "none", "--autopilot.gap", "1m", "--autopilot.retries", "1", "--autopilot.startAtOnce", "no"]);
    let id = own_task(&co, &w, "Mine");
    tick(&w);
    sw::store::tasks::set(&id, json!({ "stage": "failed" })).unwrap();
    minutes(2); tick(&w);
    assert!(last_prompt(&w, &co).starts_with(&format!("Autopilot woke: {} failed.", short(&id))), "{}", last_prompt(&w, &co));
    let n = prompts_of(&w, &co).len();
    minutes(2); tick(&w); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), n, "not again while it stays so");
    // asking a question is looking after too; a permission wait is not
    sw::store::tasks::set(&id, json!({ "stage": "asks", "kind": "permission" })).unwrap();
    minutes(2); tick(&w); assert_eq!(prompts_of(&w, &co).len(), n);
    sw::store::tasks::set(&id, json!({ "stage": "asks", "kind": "question" })).unwrap();
    minutes(2); tick(&w);
    assert!(last_prompt(&w, &co).contains(&format!("{} asks a question", short(&id))), "{}", last_prompt(&w, &co));
    // a retry used: failing again leaves it to you, and wakes nothing
    sw::autopilot::record::change(&co, |ap, _| { sw::autopilot::record::with_task(ap, &id, |r| { r.put("retries", 1); }); true }).unwrap();
    sw::store::tasks::set(&id, json!({ "stage": "working" })).unwrap(); minutes(2); tick(&w);
    let n = prompts_of(&w, &co).len();
    sw::store::tasks::set(&id, json!({ "stage": "failed" })).unwrap();
    minutes(2); tick(&w); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), n);
    assert!(sw::autopilot::record::task_rec(&record(&co), &id).unwrap().b("left"));
}

#[test]
fn with_retries_none_a_task_is_left_to_you_at_its_first_failure() {
    let w = pilot_world();
    let co = proj(&w);
    on_and_quiet(&w, &co, &["--autopilot.every", "none", "--autopilot.retries", "0", "--autopilot.startAtOnce", "no"]);
    let id = own_task(&co, &w, "Mine");
    tick(&w);
    sw::store::tasks::set(&id, json!({ "stage": "failed" })).unwrap();
    minutes(20); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 1);
    assert!(sw::autopilot::record::task_rec(&record(&co), &id).unwrap().b("left"));
}

/// The local time `h:m` of the day after the test clock's.
fn at_local(h: u32, m: u32) -> i64 {
    use chrono::{Local, TimeZone};
    let d = Local.timestamp_millis_opt(sw::clock::now()).unwrap().date_naive() + chrono::Duration::days(1);
    Local.from_local_datetime(&d.and_hms_opt(h, m, 0).unwrap()).earliest().unwrap().timestamp_millis()
}

#[test]
fn outside_its_hours_its_reasons_wait_also_across_midnight() {
    let w = pilot_world();
    let co = proj(&w);
    sw::clock::set(Some(at_local(12, 0)));
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.hours", "22:00-06:00"]);
    tick(&w);
    assert!(prompts_of(&w, &co).is_empty(), "asleep at noon");
    assert_eq!(sw::autopilot::shown::view(&co, sw::clock::now()).s("state"), "asleep");
    assert!(sw::autopilot::shown::view(&co, sw::clock::now()).s("line").contains("waits for its hours"));
    sw::clock::set(Some(at_local(23, 0))); tick(&w); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 1, "awake at eleven at night");
    sw::clock::set(Some(at_local(23, 0) + 4 * 3_600_000)); w.add("Night work"); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 2, "and at three in the morning");
    sw::clock::set(Some(at_local(23, 0) + 8 * 3_600_000)); w.add("Morning work"); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 2, "asleep again at seven");
}

#[test]
fn reasons_wait_for_the_gap_merge_into_one_wake_and_wait_while_it_is_mid_turn_or_your_message_waits() {
    let w = pilot_world();
    let co = proj(&w);
    on_and_quiet(&w, &co, &["--autopilot.every", "none", "--autopilot.gap", "10m"]);
    let a = w.add("One"); let b = w.add("Two");
    tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 1, "within the gap");
    minutes(11);
    // mid-turn (the human typed in its own tab): it waits; at rest again, the reasons go as one wake
    busy(&w, &co); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 1);
    end_turn(&w, &co); tick(&w);
    assert!(last_prompt(&w, &co).starts_with(&format!("Autopilot woke: {} was added; {} was added.", short(&a), short(&b))), "{}", last_prompt(&w, &co));
    tick(&w);
    // a turn a message of yours started (through the board): the gap counts from its end
    minutes(20);
    sw::coordinator::requests::send(&w.root, "how is it going?", false, "", &co).unwrap(); drain(); tick(&w);
    assert_eq!(last_prompt(&w, &co), "how is it going?");
    let c = w.add("Three");
    minutes(5); tick(&w);
    assert_eq!(last_prompt(&w, &co), "how is it going?", "within the gap after your turn");
    minutes(6); tick(&w);
    assert!(last_prompt(&w, &co).contains(&format!("{} was added", short(&c))), "{}", last_prompt(&w, &co));
    tick(&w);
    // a message of yours waiting for it holds the wake back (it is blocked behind a dialog here)
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.at("agent").s("name").starts_with("sw-coordinator") { p["agent"]["status"] = json!("blocked"); } } });
    sw::coordinator::requests::send(&w.root, "hold on", false, "", &co).unwrap(); drain();
    let d = w.add("Four");
    minutes(11); tick(&w);
    assert!(!prompts_of(&w, &co).iter().any(|p| p.contains(&short(&d))));
    assert!(record(&co).arr("pending").iter().any(|r| r.s("task") == d));
}

#[test]
fn wakes_a_day_hold_the_rest_for_the_next_day() {
    let w = pilot_world();
    let co = proj(&w);
    sw::clock::set(Some(at_local(9, 0)));
    on_and_quiet(&w, &co, &["--autopilot.every", "none", "--autopilot.gap", "1m", "--autopilot.wakesPerDay", "2"]);
    w.add("One"); minutes(2); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 2);
    tick(&w); w.add("Two"); minutes(2); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 2, "two a day");
    sw::clock::set(Some(at_local(9, 0) + 86_400_000)); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 3, "the next day");
    assert_eq!(record(&co).i("woke"), 1);
}

#[test]
fn a_wake_held_in_the_queue_takes_new_reasons_in_and_after_15_minutes_is_unanswered_once_its_reasons_kept() {
    let w = pilot_world();
    let co = proj(&w);
    on_and_quiet(&w, &co, &["--autopilot.every", "none", "--autopilot.gap", "1m", "--autopilot.needsYouAfter", "2"]);
    // a dialog on its screen: what is queued waits
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.at("agent").s("name").starts_with("sw-coordinator") { p["agent"]["status"] = json!("blocked"); } } });
    let a = w.add("One"); minutes(2);
    // it was at rest when the board looked: the wake goes into the queue, where the dialog holds it
    let idle_tick = || {
        w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.at("agent").s("name").starts_with("sw-coordinator") { p["agent"]["status"] = json!("idle"); } } });
        sw::autopilot::tick::autopilot_tick(&w.socket, sw::clock::now());
        w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.at("agent").s("name").starts_with("sw-coordinator") { p["agent"]["status"] = json!("blocked"); } } });
        tick(&w);
    };
    idle_tick();
    let q = || sw::coordinator::state::get(&co).unwrap().arr("queue").to_vec();
    assert_eq!(q().len(), 1); assert!(q()[0].s("text").contains(&short(&a)));
    let b = w.add("Two"); tick(&w);
    assert_eq!(q().len(), 1, "one wake in the queue at most");
    assert!(q()[0].s("text").contains(&short(&a)) && q()[0].s("text").contains(&short(&b)), "{:?}", q());
    // a message of yours goes first: the wake waits behind it, its reasons kept
    sw::coordinator::requests::send(&w.root, "me first", false, "", &co).unwrap();
    assert_eq!(q().len(), 1); assert_eq!(q()[0].s("said"), "me first");
    assert_eq!(record(&co).arr("pending").len(), 2);
    sw::coordinator::state::change(&co, |c| { c.put("queue", Vec::<serde_json::Value>::new()); true }).unwrap();
    idle_tick();
    assert_eq!(q().len(), 1);
    // 15 minutes held: unanswered, counted once, out of the queue, its reasons waiting again
    minutes(16); sw::autopilot::tick::autopilot_tick(&w.socket, sw::clock::now());
    assert!(q().is_empty());
    assert_eq!(record(&co).i("unanswered"), 1);
    assert_eq!(record(&co).arr("pending").len(), 2);
}

#[test]
fn unanswered_wakes_up_to_needs_you_after_and_turning_on_again_resumes() {
    let w = pilot_world();
    let co = proj(&w);
    w.script(json!({ "coordinator": [{ "status": "working" }, { "status": "working" }, { "status": "working" }] }));
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.every", "none", "--autopilot.gap", "1m", "--autopilot.needsYouAfter", "2"]);
    tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 1);
    let a = w.add("One");
    minutes(16); tick(&w);
    // a typed wake whose turn did not end: unanswered, and its reasons are not put back (its turn may handle them)
    assert_eq!(record(&co).i("unanswered"), 1);
    assert_eq!(record(&co).arr("wakes")[0].at("answered"), &json!(false));
    assert!(!record(&co).arr("pending").iter().any(|r| r.s("why") == "on"));
    end_turn(&w, &co); minutes(2); tick(&w);
    assert!(last_prompt(&w, &co).contains(&short(&a)));
    minutes(16); tick(&w);
    assert_eq!(record(&co).i("unanswered"), 2);
    assert!(record(&co).b("needsYou"));
    assert_eq!(sw::autopilot::shown::view(&co, sw::clock::now()).s("state"), "needs you");
    end_turn(&w, &co); w.add("Two"); minutes(2); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 2, "no more wakes");
    assert!(ok(&["autopilot", "on", "--project", "proj"]).starts_with("Autopilot resumed for the proj coordinator."));
    assert_eq!(record(&co).i("unanswered"), 0);
    tick(&w); assert_eq!(prompts_of(&w, &co).len(), 3);
    tick(&w); end_turn(&w, &co); tick(&w);
    assert_eq!(record(&co).arr("wakes").last().unwrap().at("answered"), &json!(true));
    assert_eq!(record(&co).i("unanswered"), 0);
}

#[test]
fn ask_first_access_holds_its_wakes_while_it_shows_needs_you() {
    let w = pilot_world();
    let co = proj(&w);
    on_and_quiet(&w, &co, &["--autopilot.every", "none", "--autopilot.gap", "1m"]);
    ok(&["set", "coordinator.access", "ask", "--project-coordinator", "proj"]);
    assert_eq!(sw::autopilot::shown::view(&co, sw::clock::now()).s("state"), "needs you");
    w.add("One"); minutes(2); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 1, "no wake at a permission prompt");
    ok(&["unset", "coordinator.access", "--project-coordinator", "proj"]); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 2);
}

#[test]
fn ask_first_access_takes_a_wake_not_typed_in_yet_back_out_of_the_queue() {
    let w = pilot_world();
    let co = proj(&w);
    on_and_quiet(&w, &co, &["--autopilot.every", "none", "--autopilot.gap", "1m"]);
    let blocked = |st: &str| w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.at("agent").s("name").starts_with("sw-coordinator") { p["agent"]["status"] = json!(st); } } });
    w.add("One"); minutes(2);
    sw::autopilot::tick::autopilot_tick(&w.socket, sw::clock::now()); // queued, not typed in yet
    blocked("blocked");
    let q = || sw::coordinator::state::get(&co).unwrap().arr("queue").len();
    assert_eq!(q(), 1);
    ok(&["set", "coordinator.access", "ask", "--project-coordinator", "proj"]);
    tick(&w);
    assert_eq!(q(), 0); assert_eq!(record(&co).arr("pending").len(), 1, "its reasons kept");
    assert_eq!(prompts_of(&w, &co).len(), 1);
}

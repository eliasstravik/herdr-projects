//! What tests/automations*.rs left of the old suite: an automation turned off keeps its firings and its edits and fires
//! nothing, turned on again its schedule counts from then; and on its row the keys of a task change no task, removing
//! it asks with y and n, find and the archive leave out what does not match.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::automations::list::{self as l, Add};
use sw::automations::triggers::{parse_event, parse_schedule};
use sw::json::*;
use sw::store::automations::{automation, firings_of};

const HOUR: i64 = 3600000;
fn tasks() -> Vec<Obj> { let mut v = sw::store::tasks::list(Default::default()); v.sort_by_key(|t| t.s("id").split('/').nth(1).and_then(|n| n.parse::<i64>().ok()).unwrap_or(0)); v }

#[test]
fn off_an_automation_keeps_its_firings_and_edits_and_on_again_its_schedule_counts_from_then() {
    let w = world();
    let t0 = sw::clock::now(); sw::clock::set(Some(t0));
    let mut set = Obj::new(); set.put("reviewer.agent", "claude"); set.put("merge.by", "itself");
    let a = l::add(Add { name: "Look for old dependencies", prompt: "Check what is out of date.\nChange nothing.", trigger: Some(parse_schedule("every 2h").unwrap()), repo: "", folder: &w.root, settings: set, research: Some(true) }).unwrap();
    sw::clock::advance(2 * HOUR + 1000); sw::keeper::tick_once(&w.socket, false);
    let t = tasks().remove(0);
    assert_eq!((t.at("automation").s("id"), t.at("automation").s("name"), t.at("automation").s("cause"), t.at("automation").at("chain")), (a.s("id"), a.s("name"), "schedule", &json!([a.s("id")])));
    l::set_setting(a.s("id"), "merge.by", &json!("you")).unwrap(); l::change(a.s("id"), None, Some("New words."), None, None).unwrap();
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    sw::store::tasks::patch(t.s("id"), |t| t.put("stage", "approval")).unwrap(); // its agent's run is not this test's
    // off: it never fires and keeps everything
    l::turn(a.s("id"), false).unwrap(); sw::clock::advance(9 * HOUR); sw::keeper::tick_once(&w.socket, false);
    assert_eq!(tasks().len(), 1); assert_eq!(firings_of(a.s("id")).len(), 1); assert_eq!(automation(a.s("id")).unwrap().s("prompt"), "New words.");
    // on again, the schedule counts from then: nothing is due at once
    l::turn(a.s("id"), true).unwrap(); assert_eq!(automation(a.s("id")).unwrap().i("nextAt"), sw::clock::now() + 2 * HOUR);
    sw::keeper::tick_once(&w.socket, false); assert_eq!(tasks().len(), 1);
    sw::clock::advance(2 * HOUR); sw::keeper::tick_once(&w.socket, false);
    let all = tasks();
    assert_eq!(all.len(), 2); assert_eq!(all[1].s("description"), "New words.");
    assert_eq!(all[1].at("settings"), &json!({ "reviewer.agent": "claude", "merge.by": "you", "research.only": true }));
}

#[test]
fn on_an_automations_row_a_tasks_keys_change_no_task_removing_asks_and_find_and_the_archive_leave_the_rest_out() {
    let w = world();
    w.add("Mine");
    let mut s = open(&w.root, 44); s.s.side = false;
    let mut set = Obj::new(); set.put("reviewer.agent", "claude");
    let a = l::add(Add { name: "Look for old dependencies", prompt: "Check.", trigger: Some(parse_schedule("every 2h").unwrap()), repo: "", folder: &w.root, settings: set, research: None }).unwrap();
    l::add(Add { name: "Release note after a merge", prompt: "Write it.", trigger: Some(parse_event("merged", true).unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: None }).unwrap();
    l::add(Add { name: "Nightly audit", prompt: "Audit.", trigger: Some(parse_schedule("daily 03:00").unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: None }).unwrap();
    s.load();
    s.press(&["down", "down"]); assert_eq!(s.s.sel, format!("auto:{}", a.s("id")));
    // off, tab only says so: no task is made
    s.press(&["space", "tab"]); assert_eq!(tasks().len(), 1); s.press(&["space"]);
    // a key that acts on a task does nothing here: no task is archived or pinned
    s.press(&["down"]);
    s.press(&["ctrl+a", "ctrl+p", "ctrl+l", "ctrl+r"]);
    assert!(tasks().iter().all(|t| !t.b("archivedAt") && !t.b("pinned")), "{:?}", tasks());
    // ctrl+x asks first, as removing a task does, with the keys that answer
    s.press(&["ctrl+x"]);
    assert!(s.screen(160).join("\n").contains("? Remove the automation \"Release note after a merge\"? The tasks it made stay.  y = yes · n = no"), "{}", s.screen(160).join("\n"));
    s.press(&["n"]);
    // find finds them by name, and nothing else; the archive has none, not even their heading
    s.press(&["ctrl+f"]); s.typed("nightly"); s.press(&["enter"]);
    let found = s.screen(120).join("\n");
    assert!(found.contains("Nightly audit") && !found.contains("Mine") && !found.contains("Look for old"), "{found}");
    s.press(&["esc"]);
    s.press(&["alt+a"]);
    let archive = s.screen(120).join("\n");
    assert!(!archive.contains("automations") && !archive.contains("Nightly"), "{archive}");
}

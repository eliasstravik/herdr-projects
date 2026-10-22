//! Automations: a group of the main list under the tasks, that folds as theirs do (purple, ⟳, on or off where a task has its number), the keys
//! that turn one on or off, fire it and remove it, and a schedule that makes a task and starts it by the keeper's tick.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::automations::list::{self as l, Add};
use sw::automations::triggers::{parse_event, parse_schedule};
use sw::json::*;

const HOUR: i64 = 3600000;
fn three(w: &World) -> Vec<Obj> {
    let mut s = Obj::new(); s.put("reviewer.agent", "claude"); s.put("merge.by", "itself");
    vec![
        l::add(Add { name: "Look for old dependencies", prompt: "Check which dependencies are out of date.\nOpen one pull request per major bump.", trigger: Some(parse_schedule("every 2h").unwrap()), repo: "", folder: &w.root, settings: s, research: None }).unwrap(),
        l::add(Add { name: "Release note after a merge", prompt: "Write the release note.", trigger: Some(parse_event("merged", true).unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: None }).unwrap(),
        l::add(Add { name: "Nightly audit", prompt: "Audit.", trigger: Some(parse_schedule("daily 03:00").unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: Some(true) }).unwrap(),
    ]
}
fn row_id(a: &Obj) -> String { format!("auto:{}", a.s("id")) }

#[test]
fn the_automations_are_a_group_of_the_main_list_with_keys_that_turn_fire_and_remove() {
    let w = world();
    sw::clock::set(Some(sw::clock::now())); // "next in 2h" and "0s ago" stay so however long the drawing takes
    w.add("Mine");
    let mut s = open(&w.root, 44); s.s.side = false;
    assert!(!s.text().contains("automations"), "none: no heading at all");
    let all = three(&w);
    l::turn(all[2].s("id"), false).unwrap(); s.s.sel = "proj/1".into(); s.load();
    let lines = s.screen(120);
    // the top row, this machine's, then its groups a step in
    assert!(regex::Regex::new(r"^ ▾ \S+  1 project · 1 task$").unwrap().is_match(&lines[3]), "{lines:#?}");
    assert_eq!(lines[4..11].to_vec(), vec!["   ▾ to do", "▌  ○ PRJ-1 Mine                                                                                                   0s ago", "", "   ▾ automations",
        "   ⟳ on    Look for old dependencies  every 2h                                                           next in  2h",
        "   ⟳ on    Release note after a merge  when a task is merged, in every repo                                  never fired",
        "   ⟳ off   Nightly audit  daily 03:00                                                                        never fired"]);
    assert_eq!(lines[0], " herdr-projects  proj · PRJ");
    assert!(regex::Regex::new(r"^ +All 1 +Coordinators \d+   Needs you 0   Working 0   Ready 0   To do 1   Done 0   Automations 3$").unwrap().is_match(&lines[1]), "the filter row: {}", lines[1]);
    // the arrows go from the tasks on to the automations' heading and their rows; the hint line shows the keys that act there
    s.press(&["down"]); assert_eq!(s.s.sel, "head:status\t\tautomations"); s.press(&["down"]); assert_eq!(s.s.sel, row_id(&all[0])); assert!(s.s.current().is_none());
    assert!(s.screen(120)[8].starts_with("▌  ⟳ on    Look for old dependencies "));
    assert_eq!(s.screen(140).last().unwrap(), "  enter open this automation · space turn this automation off · ctrl+s fire it now · ctrl+x remove it · ctrl+e its settings · ? all keys");
    // a narrow screen: two lines each, nothing cut
    let narrow = s.screen(60);
    assert_eq!(narrow[7..14].to_vec(), vec!["   ▾ automations", "▌  ⟳ on    Look for old dependencies", "▌          every 2h                               next in 2h", "   ⟳ on    Release note after a merge", "           when a task is merged, in every repo  never fired", "   ⟳ off   Nightly audit", "           daily 03:00                           never fired"]);
    for l in s.screen(30) { assert!(sw::text::width(&l) <= 30, "{l}"); }
    // space: off and on
    s.press(&["space"]); assert_eq!(sw::store::automations::automation(all[0].s("id")).unwrap().at("on"), &json!(false));
    assert!(s.text().contains("\"Look for old dependencies\" is off: it never fires, and keeps everything."));
    s.press(&["ctrl+s"]); assert!(s.text().contains("\"Look for old dependencies\" is off, and off it never fires. space turns it on."), "{}", s.text());
    s.press(&["space"]);
    // tab: fire it now. A task is added and an agent put on it; the selection stays on the automation
    s.press(&["ctrl+s"]);
    // nothing said under the chat when it worked: the task it made is on the list
    assert!(!s.text().contains("fired: PRJ-2 was added"), "{}", s.text());
    assert_eq!(task("proj/2").s("stage"), "starting");
    assert_eq!(s.s.sel, row_id(&all[0]));
    assert!(regex::Regex::new(r"\n[ ▌] *. PRJ-2 Look for old dependencies \[auto\]").unwrap().is_match(&s.text()), "{}", s.text());
    s.press(&["ctrl+s"]); assert!(s.text().contains("did not fire: its last task, PRJ-2, is still being worked on."));
    // a key that acts on a task does nothing here
    s.press(&["down"]); assert_eq!(s.s.sel, row_id(&all[1]));
    s.press(&["ctrl+a", "ctrl+p", "ctrl+l", "ctrl+r"]); assert_eq!(s.s.msg, ""); assert_eq!(s.s.value(), ""); assert_eq!(s.s.sel, row_id(&all[1]));
    // ctrl+x asks first: n keeps it, y removes it
    s.press(&["ctrl+x"]); assert!(s.text().contains("? Remove the automation \"Release note after a merge\"? The tasks it made stay."), "{}", s.text());
    s.press(&["n"]); assert_eq!(sw::store::automations::automations().len(), 3);
    s.press(&["ctrl+x", "y"]);
    assert_eq!(sw::store::automations::automations().iter().map(|a| a.t("name")).collect::<Vec<_>>(), vec!["Look for old dependencies", "Nightly audit"]);
    assert!(!s.text().contains("Removed the automation"), "no confirmation under the chat: {}", s.text());
    // find finds them too, by name; the archive has none
    s.press(&["ctrl+f"]); s.typed("nightly"); s.press(&["enter"]);
    let found = s.text();
    assert!(regex::Regex::new(r"\n +▾ automations\n▌ *⟳ off Nightly audit").unwrap().is_match(&found) && !found.contains("Mine"), "{found}");
    s.press(&["esc"]);
    s.press(&["alt+a"]); assert!(!s.text().contains("Nightly")); s.press(&["alt+a"]); assert!(regex::Regex::new(r"\n +▾ automations\n").unwrap().is_match(&s.text()));
}

#[test]
fn a_schedule_makes_a_task_and_starts_it_with_the_board_closed_and_off_never_fires() {
    let w = world();
    let t0 = sw::clock::now(); sw::clock::set(Some(t0));
    let mut set = Obj::new(); set.put("reviewer.agent", "claude"); set.put("merge.by", "itself");
    let a = l::add(Add { name: "Look for old dependencies", prompt: "Check what is out of date.\nChange nothing.", trigger: Some(parse_schedule("every 2h").unwrap()), repo: "", folder: &w.root, settings: set, research: Some(true) }).unwrap();
    assert_eq!(a.at("settings"), &json!({ "reviewer.agent": "claude", "merge.by": "itself", "research.only": true }));
    assert_eq!((a.s("repo"), a.i("nextAt")), ("proj", t0 + 2 * HOUR));
    sw::keeper::tick_once(&w.socket, false); sw::clock::advance(HOUR); sw::keeper::tick_once(&w.socket, false);
    assert!(sw::store::tasks::list(Default::default()).is_empty(), "not due yet");
    sw::clock::advance(HOUR + 1000); sw::keeper::tick_once(&w.socket, false);
    let all = sw::store::tasks::list(Default::default());
    assert_eq!(all.len(), 1);
    let t = &all[0];
    assert_eq!((t.s("title"), t.s("description")), ("Look for old dependencies", "Check what is out of date.\nChange nothing."));
    assert!(sw::research::is_research(t));
    assert_eq!(t.s("stage"), "starting");
    assert_eq!((t.at("automation").s("id"), t.at("automation").s("cause")), (a.s("id"), "schedule"));
    let rules = sw::delegate::rules::rules(t);
    assert!(rules.contains("- This task was made by the automation \"Look for old dependencies\", not typed just now."), "{rules}");
    let f = sw::store::automations::firings_of(a.s("id"));
    assert_eq!((f[0].s("cause"), f[0].s("what"), f[0].s("task")), ("schedule", "every 2h", t.s("id")));
    // off: it never fires and keeps everything
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    sw::store::tasks::update(t.s("id"), |t| { t.put("stage", "approval"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    l::turn(a.s("id"), false).unwrap(); sw::clock::advance(9 * HOUR); sw::keeper::tick_once(&w.socket, false);
    assert_eq!(sw::store::tasks::list(Default::default()).len(), 1);
    l::turn(a.s("id"), true).unwrap();
    assert_eq!(sw::store::automations::automation(a.s("id")).unwrap().i("nextAt"), sw::clock::now() + 2 * HOUR);
    sw::clock::advance(2 * HOUR); sw::keeper::tick_once(&w.socket, false);
    assert_eq!(sw::store::tasks::list(Default::default()).len(), 2);
}

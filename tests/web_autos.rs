//! The web board's automations (the last group of its list, one of them whole, the buttons that turn it off, fire it
//! and remove it, its settings, a firing that goes to its task) and each task's tag (on its row with the width of the
//! column, on the open task, in the chat's line, and find goes to a task by it).
use crate::common;
use common::board::web;
use common::*;
use serde_json::{json, Value};
use sw::automations::list::{self as l, Add};
use sw::automations::triggers::{parse_event, parse_schedule};
use sw::json::*;

fn rows_of(s: &Value) -> Vec<Value> { s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").to_vec()).collect() }
fn said(r: &Value) -> String { r.at("says").as_array().map(|a| a.iter().map(|p| p.t("t")).collect()).unwrap_or_default() }
fn texts(items: &[Value]) -> Vec<String> { items.iter().filter(|x| x.s("k") == "text").map(|x| x.at("value").as_array().map(|a| a.iter().map(|p| p.t("t")).collect()).unwrap_or_default()).collect() }
fn part<'a>(s: &'a Value, n: usize) -> &'a [Value] { s.at("panel").at("parts")[n].arr("items") }

#[test]
fn the_web_board_has_the_automations_one_whole_with_its_buttons_and_a_firing_that_goes_to_its_task() {
    let w = world();
    sw::clock::set(Some(sw::clock::now())); // "next in 2h" stays 2h however long the board takes
    let s = web(&w, "board", json!({}));
    assert_eq!(s.at("list").arr("groups").iter().map(|g| g.b("machine")).collect::<Vec<_>>(), [true], "the top row alone: this machine's");
    assert!(!s.arr("menu").iter().any(|i| i.s("label").contains("automation")), "none: no group, and no key that shows a list of them");
    let mut set = Obj::new(); set.put("merge.by", "itself");
    let a = l::add(Add { name: "Look for old dependencies", prompt: "Check what is out of date.", trigger: Some(parse_schedule("every 2h").unwrap()), repo: "", folder: &w.root, settings: set, research: None }).unwrap();
    let b = l::add(Add { name: "Release note", prompt: "Write it.", trigger: Some(parse_event("merged", false).unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: None }).unwrap();
    l::turn(b.s("id"), false).unwrap(); w.add("Mine");
    let s = web(&w, "board", json!({}));
    assert_eq!(s.at("list").arr("groups").iter().map(|g| (g.t("name"), g.arr("rows").len(), g.b("automations"))).collect::<Vec<_>>(), vec![(sw::shown::words::clean(&sw::remote::check::hostname()), 0, false), ("to do".into(), 1, false), ("automations".into(), 2, true)]);
    assert_eq!(s.at("list").s("selected"), "head:machine\t", "it opened on the top row, with no task");
    let s = web(&w, "pick", json!({ "id": "proj/1" }));
    assert_eq!(s.at("list").s("selected"), "proj/1"); assert!(s.at("flags").at("automations").is_null()); assert_eq!(s.at("flags").at("onTask"), &json!(true));
    let g = s.at("list").arr("groups")[2].clone();
    let (one, two) = (g.arr("rows")[0].clone(), g.arr("rows")[1].clone());
    // a task row's places: on or off where a task has its number, the trigger where it says something, the firing where it has its time
    assert_eq!((one.s("id"), one.s("title"), one.s("mark"), one.s("color"), one.at("quiet"), one.s("tag"), said(&one), one.s("time")), (format!("auto:{}", a.s("id")).as_str(), "Look for old dependencies", "⟳", "auto", &json!(false), "on", "every 2h".to_string(), "next in 2h"));
    assert_eq!((two.s("title"), two.s("mark"), two.at("markDim"), two.at("quiet"), two.s("tag"), said(&two), two.s("time")), ("Release note", "⟳", &json!(true), &json!(true), "off", "when a task is merged".to_string(), "never fired"), "off: quiet, the same mark dimmed, and the word");
    assert_eq!(g.s("color"), "auto", "the heading has the automations' own colour");
    assert_eq!([one.i("tagWidth"), two.i("tagWidth"), s.at("list").arr("groups")[1].arr("rows")[0].i("tagWidth")], [5, 5, 5], "one column: the names start where the task's title does");
    // on the page its row shows all of what it says, on any width, and its word (on, off) stays when the tasks' numbers give way
    assert_eq!((one.at("automation"), two.at("automation"), s.at("list").arr("groups")[1].arr("rows")[0].at("automation")), (&json!(true), &json!(true), &Value::Null));
    assert!(include_str!("../crates/web/src/webui/page/board.css").contains(".sw-row.is-automation .sw-says { display: block; overflow: visible; -webkit-line-clamp: none; }"));
    // tap one: it opens, whole, with its buttons; a watching device may look
    let s = web(&w, "pick", json!({ "id": one.s("id"), "open": true }));
    assert_eq!((s.at("list").s("selected"), s.at("panel").s("title"), s.at("panel").at("automation"), s.at("panel").at("task"), s.at("flags").at("open"), s.at("canOpen")), (one.s("id"), "Look for old dependencies", &json!(true), &json!(false), &json!(true), &json!(false)));
    assert_eq!(s.at("panel").arr("parts").iter().map(|p| p.t("head")).collect::<Vec<_>>(), vec!["Status", "Prompt", "Settings", "Firings"]);
    assert_eq!(s.at("flags").at("onTask"), &json!(false), "the chat's first line is no switch here: that key is the automation's own");
    assert!(texts(part(&s, 2)).join(" ").contains("Changed with: herdr-projects set <name> <value> --automation \"Look for old dependencies\""), "{:?}", texts(part(&s, 2)));
    assert!(!s.at("panel").to_string().contains("ctrl+"), "no terminal key on the page");
    assert_eq!(part(&s, 0).iter().filter(|x| x.s("k") == "fact").map(|x| x.t("label")).collect::<Vec<_>>(), vec!["State", "Trigger", "Next", "Repo", "Last firing"]);
    assert_eq!(texts(part(&s, 1)), vec!["Check what is out of date."]); assert_eq!(texts(part(&s, 2))[0], "merge.by  itself *");
    assert_eq!(s.at("actions"), &json!([{ "id": "talk", "label": "turn this automation off" }, { "id": "delegate", "label": "fire it now" }, { "id": "stop", "label": "remove it" }]));
    assert!(!s.arr("menu").iter().any(|i| ["archive", "pin", "stop"].contains(&i.s("id"))), "More has no entry that acts on a task");
    assert!(common::board::web_as(&w, "test", "press", json!({ "id": "delegate", "role": "watch" })).unwrap_err().contains("This device can only watch"));
    // fire it now: a task is made and started, and the firing is a link to it
    let s = web(&w, "press", json!({ "id": "delegate" }));
    assert!(!s.at("msg").s("text").contains("fired: PRJ-2 was added"), "no confirmation: the task it made is on the list: {}", s.at("msg"));
    assert_eq!(task("proj/2").s("stage"), "starting"); assert_eq!(s.at("list").s("selected"), one.s("id"));
    sw::automations::fire::fire(a.s("id"), sw::automations::told::Cause::Schedule, sw::clock::now()).unwrap();
    let s = web(&w, "board", json!({}));
    let firings = s.at("panel").arr("parts").iter().find(|p| p.s("head") == "Firings").unwrap().arr("items").to_vec();
    assert_eq!(firings.iter().filter(|x| x.s("k") == "text").map(|x| x.t("go")).collect::<Vec<_>>(), vec!["", "proj/2"]);
    assert!(texts(&firings)[0].contains("skipped: its last task, PRJ-2, is still being worked on"), "{:?}", texts(&firings));
    // off and on: off, the button that fires it is gone
    let s = web(&w, "press", json!({ "id": "talk" }));
    assert_eq!(sw::store::automations::automation(a.s("id")).unwrap().at("on"), &json!(false));
    assert_eq!(s.arr("actions").iter().map(|x| x.t("label")).collect::<Vec<_>>(), vec!["turn this automation on", "remove it"]);
    web(&w, "press", json!({ "id": "talk" })); assert_eq!(sw::store::automations::automation(a.s("id")).unwrap().at("on"), &json!(true));
    // its settings are a part of the page under it (All settings), which the key goes to: every setting its next task
    // will have, changed there as a task's are, and no key is swallowed after
    let s = web(&w, "press", json!({ "id": "settings" }));
    { let c = sw::webui::web::client("test"); assert!(c.lock().unwrap().s.editing.is_none()); }
    assert!(s.at("settings").i("show") > 0);
    let st = s.at("settings");
    assert_eq!((st.s("whose"), st.at("canSet"), st.arr("rows").iter().find(|x| x.s("name") == "merge.by").unwrap().at("own")), ("this automation", &json!(true), &json!(true)));
    let s = web(&w, "set", json!({ "id": st.s("id"), "name": "research.only", "value": "true" }));
    assert_eq!(sw::store::automations::automation(a.s("id")).unwrap().at("settings"), &json!({ "merge.by": "itself", "research.only": true }));
    assert_eq!(s.at("settings").arr("rows").iter().find(|x| x.s("name") == "research.only").unwrap().at("own"), &json!(true));
    web(&w, "set", json!({ "id": st.s("id"), "name": "research.only", "value": null }));
    assert_eq!(sw::store::automations::automation(a.s("id")).unwrap().at("settings"), &json!({ "merge.by": "itself" }));
    // words typed while it is open are for the Coordinator's place, never for a task: the selection is as it was after
    let s = web(&w, "send", json!({ "text": "Another one", "to": { "id": "", "reply": false, "scope": "" } }));
    assert_eq!(s.at("list").s("selected"), one.s("id"));
    assert!(sw::store::tasks::list(Default::default()).iter().any(|t| t.s("title") == "Another one"));
    // the firing's link: to that task, in the same list, open
    let s = web(&w, "pick", json!({ "id": "proj/2", "open": true }));
    assert_eq!((s.at("list").s("selected"), s.at("panel").s("title"), s.at("panel").at("task"), s.at("panel").at("automation")), ("proj/2", "Look for old dependencies", &json!(true), &Value::Null));
    let made = rows_of(&s).into_iter().find(|r| r.s("id") == "proj/2").unwrap();
    assert!(made.s("title").ends_with("[auto]") && made.at("made") == &json!(true), "its tag is drawn in the automations' colour");
    let auto = s.at("list").arr("groups").last().unwrap().arr("rows")[0].clone();
    assert_eq!(auto.at("review").as_array().unwrap().iter().map(|p| p.t("t")).collect::<String>(), "◇ PRJ-2", "the automation's row has the task its last firing made");
    assert!(part(&s, 0).iter().any(|x| x.s("k") == "fact" && x.s("label") == "Made by"));
    // remove: the same question, the same answers
    web(&w, "pick", json!({ "id": "proj/2", "open": false })); web(&w, "pick", json!({ "id": format!("auto:{}", b.s("id")), "open": true }));
    let s = web(&w, "press", json!({ "id": "stop" }));
    assert_eq!(s.at("ask").s("q"), "Remove the automation \"Release note\"? The tasks it made stay.");
    let s = web(&w, "press", json!({ "id": "sayYes" }));
    assert_eq!(sw::store::automations::automations().iter().map(|x| x.t("name")).collect::<Vec<_>>(), vec!["Look for old dependencies"]);
    assert!(s.at("panel").is_null(), "{}", s.at("panel"));
    // closing an open one leaves it selected in the list; the last one removed takes the group with it
    web(&w, "pick", json!({ "id": one.s("id"), "open": true }));
    let s = web(&w, "pick", json!({ "id": one.s("id"), "open": false }));
    assert!(s.at("panel").is_null()); assert_eq!(s.at("flags").at("open"), &json!(false)); assert_eq!(s.at("list").s("selected"), one.s("id"));
    l::remove(a.s("id")).unwrap();
    assert!(!web(&w, "board", json!({})).at("list").arr("groups").iter().any(|g| g.s("name") == "automations"));
}

#[test]
fn the_web_board_has_each_tasks_tag_on_its_row_on_the_open_task_and_in_the_chats_line_and_find_goes_to_it() {
    let w = world();
    let seq = sw::store::files::tasks_dir().join("proj").join("seq");
    w.add("First");
    std::fs::write(&seq, "8").unwrap(); let nine = w.add("Number nine");
    std::fs::write(&seq, "133").unwrap(); let big = w.add("Number one three four");
    assert_eq!((nine.as_str(), big.as_str()), ("proj/9", "proj/134"));
    let tags = |s: &Value, id: &str| { let r = rows_of(s).into_iter().find(|r| r.s("id") == id).unwrap(); (r.t("tag"), r.i("tagWidth")) };
    let s = web(&w, "board", json!({}));
    assert_eq!((tags(&s, &big), tags(&s, &nine)), (("PRJ-134".into(), 7), ("PRJ-9".into(), 7)), "one column, as wide as the widest tag");
    sw::store::settings::change::set_value("repo.key", &json!("hsw"), &sw::store::settings::lookup::Scope::repo("proj")).unwrap();
    let s = web(&w, "board", json!({}));
    assert_eq!((tags(&s, &big), tags(&s, &nine)), (("HSW-134".into(), 7), ("HSW-9".into(), 7)));
    let s = web(&w, "pick", json!({ "id": big, "open": true }));
    assert_eq!((s.at("panel").s("tag"), s.at("panel").s("title")), ("HSW-134", "Number one three four"));
    let s = web(&w, "press", json!({ "id": "talk" }));
    assert_eq!((s.at("chat").s("tag"), s.at("chat").s("name")), ("HSW-134", "Number one three four"));
    // The box's frame is the status colour of whoever gets your words: a task to do (no agent yet) has the base
    // colour, as where no coordinator can run.
    assert_eq!(s.at("chat").s("box"), "text"); assert!(s.at("send").s("place").contains("starts an agent"), "{}", s.at("send"));
    assert_eq!(web(&w, "press", json!({ "id": "talk" })).at("chat").s("box"), "text");
    web(&w, "press", json!({ "id": "talk" }));
    web(&w, "pick", json!({ "id": big, "open": false }));
    assert_eq!(web(&w, "find", json!({ "text": "hsw-9" })).at("list").s("selected"), nine);
    assert_eq!(web(&w, "find", json!({ "text": "#134" })).at("list").s("selected"), big);
    web(&w, "find", json!({ "text": "" }));
    // Tags of very different lengths share the widest's column too, so every title starts in the same place.
    std::fs::write(&seq, "999").unwrap(); let wide = w.add("Number one thousand");
    let s = web(&w, "board", json!({}));
    assert_eq!([tags(&s, "proj/1"), tags(&s, &nine), tags(&s, &wide)], [("HSW-1".into(), 8), ("HSW-9".into(), 8), ("HSW-1000".into(), 8)]);
}

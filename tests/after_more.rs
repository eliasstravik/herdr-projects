//! A waiting task from the board and the command line: its row and panel say what it waits for, the board's key asks
//! before it starts one early, `after` sets, shows and drops a wait, stop stops the waiting, a task with an agent on it
//! cannot be made to wait, and another machine's board starts one early the same way.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use sw::cli::{flags::parse, tasks::command};
use sw::json::*;

fn run(a: &[&str]) -> Result<(Value, String), String> { command(a[0], &parse(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())).map_err(|e| e.msg) }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }

#[test]
fn the_board_shows_what_a_task_waits_for_and_its_key_asks_before_it_starts_one_early() {
    let w = world();
    w.script(json!({ "worker": [step(1)] }));
    let dep = w.add("Change the skill");
    let id = w.add("Take it in");
    run(&["delegate", &id, "--after", &dep]).unwrap();
    let mut b = open(&w.root, 40);
    b.s.sel = id.clone(); b.load();
    let text = b.screen(120).join("\n");
    assert!(has(&text, r"\n▌  ◌ PRJ-2 +Take it in +0s *│[^\n]*\n▌ +Queued after PRJ-1 "), "{text}");
    assert!(has(&text, r"│  State +◌ queued after PRJ-1 · for 0s *\n[^\n]*│  Starts after +PRJ-1 *\n"), "{text}");
    // The key that puts an agent on a task asks first: it starts by itself once the other is done.
    b.press(&["ctrl+s"]);
    assert!(has(&b.text(), r#"\? Start PRJ-2 "Take it in" now\? *\n +It is queued after PRJ-1 and starts by itself once that is done\. *\n +y = start now · n = let it wait"#), "{}", b.text());
    b.press(&["n"]);
    assert_eq!(task(&id).s("stage"), "after");
    assert!(starts(&w).is_empty());
    b.press(&["ctrl+s", "y"]);
    let t = settle(&id);
    assert!(!["after", "queued"].contains(&t.s("stage")), "{}", t.s("stage"));
    assert!(t.b("afterEarly"));
    let told = prompts(&w, t.at("link").s("pane")).join("\n");
    assert!(told.contains("the human started it before they all were") && told.contains("- PRJ-1 \"Change the skill\" (repo proj): not done yet."), "{told}");
    let text = b.screen(120).join("\n");
    assert!(!text.to_lowercase().contains("queued after prj-1") && !text.contains("Starts after"), "{text}");
}

#[test]
fn after_sets_shows_and_drops_a_wait_and_stop_stops_the_waiting() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }] }));
    let dep = w.add("First");
    let id = w.add("Second");
    assert_eq!(run(&["after", &id]).unwrap().1, format!("{id}: waits for nothing"));
    // On a task nobody delegated yet: kept for when it is.
    assert_eq!(run(&["after", &id, &dep]).unwrap().1, format!("{id}: is to wait for {}, from when it is delegated", short(&dep)));
    assert_eq!(task(&id).s("stage"), "todo");
    assert_eq!(run(&["delegate", &id]).unwrap().1, format!("{id}: delegated, it is queued after {} and starts by itself once that is done", short(&dep)));
    // Stop stops the waiting; what it is to wait for stays, for the next delegation.
    sw::actions::talk::stop(&id).unwrap();
    assert_eq!((task(&id).s("stage"), task(&id).at("after").clone()), ("todo".into(), json!([dep])));
    run(&["delegate", &id]).unwrap();
    assert_eq!(task(&id).s("stage"), "after");
    // Dropped: it starts at once.
    assert_eq!(run(&["after", &id, "--clear"]).unwrap().1, format!("{id}: waits for nothing, so it starts now"));
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working");
    assert!(!t.has("after") && !t.has("afterDone"));
    // A task an agent is on cannot be made to wait.
    let e = run(&["after", &id, &dep]).unwrap_err();
    assert!(e.contains("already has an agent on it"), "{e}");
    // Waiting for one that is done already: it starts at once, told what that did.
    sw::store::tasks::set(&dep, json!({ "stage": "done", "summary": "Made the first part." })).unwrap();
    let third = w.add("Third");
    assert_eq!(run(&["delegate", &third, "--after", &dep]).unwrap().1, format!("{third}: started, an agent is on it"));
    let t = settle(&third);
    let told = prompts(&w, t.at("link").s("pane")).join("\n");
    assert!(told.contains("- PRJ-1 \"First\" (repo proj): done. Its closing note:\n    Made the first part."), "{told}");
}

/// The door, as `herdr-projects remote <what> <request>` answers it on this machine (in this process).
fn door(what: &str, req: Value) -> Value { sw::remote::door(what, &req).unwrap_or_else(|e| json!({ "ok": false, "msg": e.msg })) }

#[test]
fn another_machines_board_sees_the_wait_and_starts_one_early_only_when_asked() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let dep = w.add("First");
    let id = w.add("Second");
    run(&["delegate", &id, "--after", &dep]).unwrap();
    let sent = sw::remote::remote_list();
    let r = sent.arr("tasks").iter().find(|t| t.s("id") == id).cloned().unwrap_or_default();
    assert_eq!((r.s("stage"), r.s("note")), ("after", "queued after PRJ-1"));
    let no = door("do", json!({ "action": "delegate", "id": id }));
    assert!(no.s("msg").contains("is still queued after PRJ-1: it starts by itself once that is done"), "{no}");
    assert_eq!(door("do", json!({ "action": "delegate", "id": id, "now": true })), json!({ "ok": true, "msg": format!("{id}: an agent is on it") }));
    assert_ne!(settle(&id).s("stage"), "after");
}

#[test]
fn the_web_board_says_the_same_and_its_button_asks_before_it_starts_one_early() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let dep = w.add("First");
    let id = w.add("Second");
    run(&["delegate", &id, "--after", &dep]).unwrap();
    let said = |p: &Value| p.as_array().map(|a| a.iter().map(|x| x.t("t")).collect::<String>()).unwrap_or_default();
    let s = common::board::web(&w, "pick", json!({ "id": id, "open": true }));
    let row = s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").to_vec()).find(|r| r.s("id") == id).unwrap();
    assert_eq!(said(row.at("says")), "Queued after PRJ-1");
    let fact = s.at("panel").arr("parts").iter().find(|p| p.s("head") == "Status").and_then(|p| p.arr("items").iter().find(|x| x.s("label") == "Starts after").cloned()).unwrap_or_default();
    assert_eq!(said(&fact.at("lines")[0]), "PRJ-1");
    let s = common::board::web(&w, "press", json!({ "id": "delegate" }));
    assert_eq!(s.at("ask").s("q"), "Start PRJ-2 \"Second\" now?");
    assert_eq!(task(&id).s("stage"), "after");
    common::board::web(&w, "press", json!({ "id": "sayYes" }));
    assert_ne!(settle(&id).s("stage"), "after");
}

#[test]
fn stopping_a_task_that_waits_tells_the_tasks_that_wait_for_it_that_it_was_stopped() {
    let w = world();
    let a = w.add("A");
    let b = w.add("B");
    let c = w.add("C");
    run(&["delegate", &b, "--after", &a]).unwrap();
    run(&["delegate", &c, "--after", &b]).unwrap();
    sw::actions::talk::stop(&b).unwrap();
    assert_eq!(task(&b).s("stage"), "todo");
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let t = task(&c);
    assert_eq!((t.s("stage"), t.s("afterBlocked")), ("after", "PRJ-2 was stopped"));
    assert_eq!(row(&c).s("state"), "blocked");
    assert_eq!(sw::notify::event_of(&t), "needs you");
    // Delegated again, it waits once more, and so does the one after it.
    run(&["delegate", &b]).unwrap();
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    assert_eq!((task(&c).s("msg"), task(&c).at("afterBlocked").clone()), ("queued after PRJ-2".into(), Value::Null));
}

#[test]
fn each_attempt_of_a_task_that_waited_is_told_what_came_before_it() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }] }));
    let dep = w.add("First");
    sw::store::tasks::set(&dep, json!({ "stage": "done", "summary": "Made the first part.", "mergeCommit": "abc1234" })).unwrap();
    let id = w.add("Second");
    run(&["delegate", &id, "--after", &dep, "--attempts.by", "2"]).unwrap();
    let t = settle_all(&id);
    assert_eq!(kids(&id).len(), 2, "{}", t.s("stage"));
    for k in kids(&id) {
        assert_eq!(k.at("afterDone"), task(&id).at("afterDone"));
        let told = prompts(&w, k.at("link").s("pane")).join("\n");
        assert!(told.contains("- PRJ-1 \"First\" (repo proj): done, its work merged, merge commit abc1234. Its closing note:\n    Made the first part."), "{}: {told}", k.s("id"));
    }
}

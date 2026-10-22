//! Space on a machine's heading or a repo's heading (grouped by repo) talks to that machine's or that repo's
//! coordinator, as space on a task talks to its agent: the chat names it, what you type goes to it, and space again goes
//! back to the Coordinator. Another machine's are reached there, from what it sends with its tasks.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use sw::board::job::Job;
use sw::board::update::heads::{group_id, machine_id};
use sw::board::update::{update, Effect, Event};
use sw::coordinator::co::Co;
use sw::json::*;

fn coord_world(settings: Value) -> World {
    let mut all = json!({ "coordinator.agent": "claude", "review.when": "never" });
    for (k, v) in settings.as_object().cloned().unwrap_or_default() { all[k] = v; }
    let w = world_with(Opts { repo: true, settings: all, ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn prompts(w: &World, name: &str) -> Vec<String> { w.state().arr("panes").iter().find(|p| p.at("agent").s("name") == name).map(|p| p.at("agent").arr("prompts").iter().map(text).collect()).unwrap_or_default() }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }

#[test]
fn space_on_a_repos_heading_talks_to_its_project_coordinator_and_space_again_goes_back() {
    let w = coord_world(json!({}));
    let id = w.add("Fix the login");
    w.script(json!({ "coordinator": [{ "said": "Proj has one task open." }] }));
    let mut s = open(&w.root, 34);
    s.press(&["alt+s"]); assert!(s.s.by_repo);
    let head = group_id(&s.s, "", "proj");
    s.s.sel = head.clone(); s.load();
    let host = sw::shown::words::clean(&sw::remote::check::hostname());
    assert!(s.text().contains("space talk to proj's coordinator"), "{}", s.text());
    s.press(&["space"]);
    assert!(s.s.reply && s.s.reply_id == head);
    let t = s.text();
    // the box names it, from the top row down; it never ran: its box asks for a first message
    assert!(has(&t, &format!(r"\n╭─ {} ›  proj  · Project coordinator ─+╮\n│ +│\n│  ❯ Ask it something", regex::escape(&host))), "{t}");
    assert!(t.contains("❯ Ask it something, e.g. \"what's left to do here?\"") && t.contains(&format!("space back to {host}'s coordinator")), "{t}");
    s.typed("what is open here?");
    assert!(s.text().contains("enter send to") && s.text().contains("proj's coordinator"), "{}", s.text());
    s.press(&["enter"]); drain(); s.load();
    assert_eq!(s.s.value(), "");
    assert_eq!(prompts(&w, "sw-coordinator-proj"), vec!["what is open here?"]);
    assert!(sw::coordinator::state::get(&w.socket).is_none(), "the Coordinator got nothing");
    let t = s.text();
    // its reply alone under the ladder: no line naming it again, no label over the reply
    assert!(has(&t, r"╭─ [^\n]*›  proj  · Project coordinator ─") && !t.contains("Looks after") && !t.contains("│ proj's coordinator:") && t.contains("Proj has one task open."), "{t}");
    // the tab it is in says whose it is
    assert!(w.state().arr("tabs").iter().any(|x| x.s("label") == "proj coordinator"));
    // the chat follows the selection, as with tasks: to the task's agent and back to the repo's coordinator
    s.press(&["down"]); assert_eq!((s.s.sel.as_str(), s.s.reply_id.as_str()), (id.as_str(), id.as_str()));
    assert!(has(&s.text(), r"╭─ [^\n]* proj ›  PRJ-1  · Agent ─"), "{}", s.text());
    s.press(&["up"]); assert!(has(&s.text(), r"╭─ [^\n]*›  proj  · Project coordinator ─"), "{}", s.text());
    // space again: back to the top row's coordinator (the only machine's, the top-level one)
    s.press(&["space"]); assert!(!s.s.reply);
    assert!(s.text().contains(&format!("╭─  {host}  · Machine coordinator ─")), "{}", s.text());
    // a status group's heading has no coordinator: space says where they are
    s.press(&["alt+s", "alt+s"]); assert!(!s.s.by_repo && !s.s.by_machine); // by machine, then by status
    s.s.sel = group_id(&s.s, "", "to do"); s.load();
    s.press(&["space"]);
    assert!(!s.s.reply && s.s.msg.contains("Space on a machine's heading or a repo's heading talks to its coordinator"), "{}", s.s.msg);
}

#[test]
fn space_on_a_machines_heading_talks_to_that_machines_coordinator_here_or_there() {
    let w = coord_world(json!({ "board.machines": "otherbox" }));
    w.add("Here");
    w.script(json!({ "coordinator": [{ "said": "This machine is busy." }] }));
    let host = sw::remote::check::hostname();
    let mut s = open(&w.root, 34);
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    let now = sw::clock::now();
    let far = |coords: Value| Ok(json!({ "host": "otherbox", "version": "9.0.0", "coordinator": { "state": "none" }, "coordinators": coords,
        "tasks": [{ "id": "proj/1", "title": "Far away", "state": "idle", "stage": "todo", "at": now, "created": now, "snap": { "folder": "~/far" } }] }));
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: far(json!({ "machine": { "state": "idle", "readable": true, "said": "Otherbox is quiet.", "talk": [{ "you": true, "text": "how is it?" }, { "you": false, "text": "Otherbox is quiet." }] } })) });
    while s.s.scope != "*" { s.press(&["alt+m"]); }
    // this machine's heading: its machine coordinator, started here
    s.s.sel = machine_id(""); s.load();
    s.press(&["space"]);
    assert!(s.text().contains(&format!("╭─ All machines ›  {}  · Machine coordinator ─", sw::shown::words::clean(&host))) && s.text().contains("❯ Ask it something"), "{}", s.text());
    s.typed("how busy are you?"); s.press(&["enter"]); drain(); s.load();
    assert_eq!(prompts(&w, "sw-coordinator-machine"), vec!["how busy are you?"]);
    assert!(s.text().contains("This machine is busy."), "{}", s.text());
    assert_eq!(sw::coordinator::look::look(Co::machine(&w.socket)).said, "This machine is busy.");
    // otherbox's heading: what that machine sent of its own machine coordinator; your words go there
    s.s.sel = machine_id("otherbox"); s.load();
    let t = s.text();
    assert!(has(&t, r"\n╭─ All machines ›  otherbox  · Machine coordinator ─+[^\n]*\n") && t.contains("Otherbox is quiet."), "{t}");
    s.typed("anything to do?");
    let fx = s.hold("enter");
    assert!(fx.iter().any(|e| matches!(e, Effect::Run(Job::Dispatch { machine, text, to, .. }) if machine.label == "otherbox" && text == "anything to do?" && to == "machine")), "{fx:?}");
    // a machine on an older version has no machine coordinator: its row is its top-level one
    assert_eq!(s.s.value(), "", "sent: the box is empty again");
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: Ok(json!({ "host": "otherbox", "version": "0.2.0", "coordinator": { "state": "none" },
        "tasks": [{ "id": "proj/1", "title": "Far away", "state": "idle", "stage": "todo", "at": now, "created": now, "snap": { "folder": "~/far" } }] })) });
    s.s.sel = machine_id("otherbox"); s.load();
    let t = s.text();
    // its row talks to the one coordinator it has, its top-level one, named after the machine
    assert!(t.contains("╭─ All machines ›  otherbox  ") && t.contains("❯ Ask it something") && !t.contains("has no machine or project coordinators"), "{t}");
}

#[test]
fn the_web_board_names_a_headings_coordinator_and_sends_it_your_words() {
    let w = coord_world(json!({}));
    w.add("Fix the login");
    w.script(json!({ "coordinator": [{ "said": "Proj is fine." }] }));
    let web = |call: &str, args: Value| common::board::web(&w, call, args);
    web("press", json!({ "id": "byRepo" }));
    let s = web("board", json!({}));
    let head = s.at("list").arr("groups").iter().find(|g| g.s("id").starts_with("head:repo")).map(|g| g.t("id")).unwrap_or_else(|| panic!("{}", s.at("list")));
    // a tap on the heading selects it (and folds it): the switch's other choice is its coordinator
    let s = web("fold", json!({ "id": head }));
    assert_eq!(s.at("chat").s("head"), "proj's coordinator", "{}", s.at("chat"));
    let s = web("press", json!({ "id": "talk" }));
    assert_eq!((s.at("chat").at("toAgent"), s.at("chat").s("name")), (&json!(true), "proj's coordinator"), "{}", s.at("chat"));
    web("send", json!({ "text": "how is it?", "to": { "id": head, "reply": true, "scope": "" } }));
    drain();
    assert_eq!(prompts(&w, "sw-coordinator-proj"), vec!["how is it?"]);
    // what this machine sends another one: its repo's coordinator, by level
    let sent = sw::remote::remote_list();
    assert_eq!(sent.at("coordinators").at("project:proj").s("said"), "Proj is fine.", "{}", sent.at("coordinators"));
}

#[test]
fn words_put_away_for_a_headings_coordinator_come_back_only_to_it() {
    let w = coord_world(json!({}));
    w.add("Fix the login");
    let mut s = open(&w.root, 34);
    s.press(&["alt+s"]);
    let head = group_id(&s.s, "", "proj");
    s.s.sel = head.clone(); s.load();
    s.press(&["space"]); s.typed("half a thought"); s.press(&["esc"]);
    assert_eq!(s.s.value(), ""); assert_eq!(s.s.draft_away.as_ref().map(|d| d.to.clone()), Some(head.clone()));
    s.press(&["space"]); assert!(!s.s.reply);
    s.press(&["ctrl+z"]);
    assert_eq!(s.s.value(), "", "not to the Coordinator"); assert!(s.s.msg.contains("Those words were for") && s.s.msg.contains("proj's coordinator. Select its row and press space"), "{}", s.s.msg);
    s.press(&["space"]); s.press(&["ctrl+z"]);
    assert_eq!(s.s.value(), "half a thought");
}

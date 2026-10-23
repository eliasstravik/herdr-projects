//! The web board, more of it: a draft goes to who it was typed for (a task, a machine's Coordinator) whatever shows
//! meanwhile, a merge has no filled yes, the More menu holds only what applies, the page speaks web (no terminal key);
//! and the marks, links and keys of the terminal's own the page never gets.
use crate::common;
use crate::far;
use common::board::web;
use common::*;
use far::Far;
use serde_json::{json, Value};
use sw::json::*;

fn rows_of(s: &Value) -> Vec<Value> { s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").to_vec()).collect() }
fn client() -> std::sync::Arc<std::sync::Mutex<sw::webui::web::Client>> { sw::webui::web::client("test") }
fn send(w: &World, text: &str, to: Value) -> Value { web(w, "send", json!({ "text": text, "to": to })) }

#[test]
fn a_draft_goes_to_who_it_was_typed_for_a_merge_has_no_filled_yes_and_the_page_speaks_web() {
    let w = world();
    let far = Far::new(&w.base);
    let (one, two) = (w.add("First"), w.add("Second"));
    let bad = w.add("Add an undo");
    sw::store::tasks::patch(&bad, |x| { for (k, v) in [("stage", json!("approval")), ("app", json!("claude")), ("agent", json!("claude")), ("reviewRound", json!(1)), ("summary", json!("Done.")),
        ("review", json!({ "text": "No.", "meets": false, "pass": false, "gate": "major", "findings": [{ "id": "F1", "severity": "critical", "subject": "Loses data", "where": "a.js" }] }))] { x.put(k, v); } }).unwrap();
    // Words typed while the first task was open, for that task; meanwhile another task was tapped. They go to the
    // first all the same (here: they start its agent's description), and the view stays on what you looked at.
    web(&w, "pick", json!({ "id": one, "open": true })); web(&w, "press", json!({ "id": "talk" }));
    { let c = client(); let g = c.lock().unwrap(); assert!(g.s.reply); assert_eq!(g.s.reply_id, one); }
    assert_eq!(web(&w, "pick", json!({ "id": two, "open": true })).at("list").s("selected"), two);
    let before = task(&two).t("description");
    let out = send(&w, "for the first one", json!({ "id": one, "reply": true }));
    assert_eq!(out.at("kept"), &json!(false)); assert_eq!(out.at("list").s("selected"), two, "the view is where you left it");
    assert_eq!(task(&two).t("description"), before, "the task you tapped got nothing");
    assert!(task(&one).s("description").contains("for the first one") || task(&one).s("stage") != "todo", "the task they were for got them");
    // A task that is gone meanwhile: nothing is sent, the words stay on the page.
    let out = send(&w, "too late", json!({ "id": "proj/99", "reply": true }));
    assert_eq!(out.at("kept"), &json!(true)); assert!(out.at("msg").s("text").contains("is gone. Nothing was sent."), "{}", out.at("msg"));
    // For the Coordinator (none here: a new task), whatever task is open: never a reply to it.
    let n = sw::store::tasks::list(Default::default()).len();
    web(&w, "pick", json!({ "id": two, "open": true })); web(&w, "press", json!({ "id": "talk" }));
    let out = send(&w, "A brand new task", json!({ "id": "", "reply": false }));
    assert_eq!(sw::store::tasks::list(Default::default()).len(), n + 1); assert_eq!(out.at("list").s("selected"), two);
    // A draft for one machine's Coordinator goes to that machine, also after the view moved to another one; a machine
    // that is gone meanwhile gets nothing, and nobody gets the words in its place.
    far.answers("vm-a", "dispatch", &json!({ "ok": true, "msg": "ok" }));
    {
        let c = client(); let mut g = c.lock().unwrap();
        for label in ["vm-a", "vm-b"] { g.s.remotes.insert(label.into(), sw::board::data::Remote { machine: sw::remote::Machine { label: label.into(), target: format!("me@{label}"), ..Default::default() }, ok: true, ..Default::default() }); }
        g.s.scope = "vm-b".into(); sw::board::update::list::relist(&mut g.s);
    }
    let out = send(&w, "Words meant for vm-a", json!({ "id": "", "reply": false, "scope": "vm-a" }));
    assert_eq!(out.at("kept"), &json!(false));
    assert_eq!(far.asked().iter().map(|a| (a.0.clone(), a.1.clone(), a.2.t("text"))).collect::<Vec<_>>(), vec![("vm-a".to_string(), "dispatch".to_string(), "Words meant for vm-a".to_string())]);
    assert_eq!(out.at("flags").s("scope"), "vm-b", "the view is where you left it");
    let out = send(&w, "For a machine that left", json!({ "id": "", "reply": false, "scope": "vm-c" }));
    assert_eq!(out.at("kept"), &json!(true)); assert!(out.at("msg").s("text").contains("vm-c is not among the machines you can reach now. Nothing was sent."));
    assert_eq!(far.asked().len(), 1);
    { let c = client(); let mut g = c.lock().unwrap(); g.s.scope.clear(); g.s.remotes.clear(); sw::webui::web::reload(&mut g); }
    // The merge question of work whose review failed: what it is, what failed, and `danger`: neither answer is filled.
    web(&w, "pick", json!({ "id": bad, "open": true }));
    let s = web(&w, "press", json!({ "id": "yes" }));
    let ask = s.at("ask");
    assert_eq!(ask.at("danger"), &json!(true));
    assert!(regex::Regex::new(r#"^Merge PRJ-3 "Add an undo" into \S+\?$"#).unwrap().is_match(ask.s("q")), "{ask}");
    assert!(regex::Regex::new(r"^Its last review failed: 1 critical\. A merge cannot be undone\.$").unwrap().is_match(ask.s("detail")), "{ask}");
    assert_eq!(ask.at("keys"), &json!([{ "id": "sayYes", "label": "merge anyway" }, { "id": "sayNo", "label": "do not merge" }]));
    web(&w, "press", json!({ "id": "sayNo" })); assert_eq!(task(&bad).s("stage"), "approval");
    // More: only what applies, each a button's own words; nothing that names a terminal key, nothing for a key the page has no button for.
    let s = web(&w, "board", json!({}));
    let ids: Vec<String> = s.arr("menu").iter().map(|i| i.t("id")).collect();
    for no in ["yes", "sendBack", "undo", "close", "free"] { assert!(!ids.contains(&no.to_string()), "{ids:?}"); }
    for yes in ["stop", "archive", "pin", "scope", "settings"] { assert!(ids.contains(&yes.to_string()), "{yes}: {ids:?}"); }
    assert_eq!(s.arr("menu").iter().find(|i| i.s("id") == "scope").unwrap().s("label"), "look for other machines (now: this machine)");
    for id in sw::webui::model::side::TERMINAL_ONLY { assert!(!ids.contains(&id.to_string()), "{id}"); }
    let said = s.to_string();
    assert!(!regex::Regex::new(r"ctrl\+|alt\+|\bshift\+").unwrap().is_match(&said), "no terminal key anywhere on the page");
    assert!(said.contains("All settings, below, has every one of them, and changes them") && !said.contains("this task's own, not what it would be"));
    // The header says the repo by its name: no folder path, nothing for the home folder.
    assert_eq!(s.at("list").at("head").s("where"), "proj");
    // The actions row: the review goes back first, and the yes says what it is.
    assert_eq!(s.arr("actions").iter().take(2).map(|b| b.t("label")).collect::<Vec<_>>(), vec!["send the review to this task's agent", "merge this task anyway"]);
}

#[test]
fn the_page_gets_marks_without_colour_links_as_links_and_never_a_key_of_the_terminals_own() {
    let w = world();
    let t = sw::webui::web::theme();
    let at = |stage: &str| sw::webui::ink::first(&sw::board::view::fields::mark(t, &obj(json!({ "state": "working", "stage": stage }))));
    assert_eq!(at("working"), json!({ "t": "●", "c": "yellow" })); assert_eq!(at("reviewing"), json!({ "t": "◑", "c": "review" })); assert_eq!(at("queued"), json!({ "t": "◷", "c": "queue" }));
    for stage in ["starting", "setup", "checking", "pushing", "merging"] { assert_eq!(at(stage).s("t"), "◇", "{stage}"); }
    for kind in ["working", "review", "board", "queue", "blocked", "done", "idle", "error"] { assert_ne!(sw::board::theme::marks::web_mark(kind, ""), sw::board::theme::marks::web_mark("unknown", ""), "{kind}"); }
    let linked = sw::shown::ink::Inked::plain("a ").add(sw::shown::ink::Inked::plain("PR#3").link("https://x.y/z"));
    assert_eq!(sw::webui::ink::pieces(&linked), vec![json!({ "t": "a " }), json!({ "t": "PR#3", "href": "https://x.y/z" })]);
    // a key that is the terminal's own does nothing here
    let id = w.add("Tidy the readme"); w.add("Sort the notes");
    web(&w, "pick", json!({ "id": id, "open": true }));
    let before = web(&w, "board", json!({})).s("mark").to_string();
    web(&w, "press", json!({ "id": "close" })); web(&w, "press", json!({ "id": "menu" }));
    { let c = client(); assert!(c.lock().unwrap().s.menu.is_none()); }
    assert!(!before.is_empty());
    // a task nobody was on has no pane to go to
    assert_eq!(web(&w, "press", json!({ "id": "enter" })).at("canOpen"), &json!(false));
    // typed words that are only spaces add nothing; find narrows the list, and says so
    let n = sw::store::tasks::list(Default::default()).len();
    web(&w, "send", json!({ "text": "   " })); assert_eq!(sw::store::tasks::list(Default::default()).len(), n);
    let s = web(&w, "find", json!({ "text": "readme" }));
    assert_eq!(s.at("flags").s("filter"), "readme"); assert_eq!(rows_of(&s).len(), 1);
    assert_eq!(rows_of(&web(&w, "find", json!({ "text": "" }))).len(), 2);
}

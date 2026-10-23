//! The web Board as data: the same list, groups, marks and words as the terminal board, one task whole as parts, the
//! buttons that press the board's own keys, typed words sent as enter sends them, one board per browser, and what a
//! watching device may not do. Also what the page needs to know of every colour name, mark and setting.
use crate::common;
use common::board::{open, web, web_as};
// The web board in the terminal board's design: its own file, a module of this one (tests/main.rs is at its size).
mod design;
// Open this task's agent on a space that was closed, and the web UI that does not show it yet.
mod open;
use common::*;
use serde_json::json;
use sw::json::*;

fn rows_of(s: &serde_json::Value) -> Vec<serde_json::Value> { s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").to_vec()).collect() }
fn said(p: &serde_json::Value) -> String { p.as_array().map(|a| a.iter().map(|x| x.t("t")).collect()).unwrap_or_default() }
fn make_f(w: &World) -> String {
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }], "reviewer": [{ "verdict": { "meets": true, "text": "Good.", "findings": [{ "severity": "minor", "subject": "f.txt has no title", "where": "f.txt" }] } }] }));
    let id = sw::actions::filing::add(&w.root, "Make f", sw::actions::filing::AddOpts { description: "A file called f.".into(), ..Default::default() }).unwrap().t("id");
    delegate(&id); settle(&id);
    id
}

#[test]
fn the_web_board_shows_the_terminal_boards_list_and_one_task_whole() {
    let w = world();
    let id = make_f(&w);
    let mine = w.add("Write notes");
    sw::actions::filing::report(&mine, sw::actions::filing::Report { state: "working".into(), msg: "Drafting".into(), progress: Some(json!(40)), ..Default::default() }).unwrap();
    w.add("Later");
    let term = open(&w.root, 40);
    let mut want: Vec<Vec<String>> = vec![];
    for r in &term.s.rows { match r { sw::board::state::Row::Head { name, .. } => want.push(vec![name.clone()]), sw::board::state::Row::Rec(o) => want.last_mut().unwrap().push(o.t("id")), _ => {} } }
    let s = web(&w, "board", json!({}));
    let got: Vec<Vec<String>> = s.at("list").arr("groups").iter().map(|g| std::iter::once(g.t("name")).chain(g.arr("rows").iter().map(|r| r.t("id"))).collect()).collect();
    assert_eq!(got, want);
    assert_eq!(s.at("list").arr("groups").iter().map(|g| (g.t("name"), g.t("color"))).collect::<Vec<_>>(), vec![(sw::shown::words::clean(&sw::remote::check::hostname()), "plain".into()), ("ready to approve".into(), "green".into()), ("working".into(), "yellow".into()), ("to do".into(), "todo".into())]); // the top row, this machine's, then its groups
    let rows = rows_of(&s);
    let (ready, working, todo) = (&rows[0], &rows[1], &rows[2]);
    assert_eq!((ready.s("title"), ready.s("mark"), ready.s("color")), ("Make f", "◆", "green"));
    assert_eq!(said(ready.at("says")), "Pass · 1 minor"); assert_eq!(ready.at("says")[0].s("c"), "green");
    assert_eq!(ready.at("review"), &json!([{ "t": "R#1", "c": "green" }]));
    assert!(regex::Regex::new(r"^\d+s ago$").unwrap().is_match(ready.s("time")), "{}", ready.s("time"));
    assert_eq!((working.at("progress"), working.s("percent"), said(working.at("says")), working.at("strong"), working.at("live")), (&json!(40.0), "40%", "Drafting".to_string(), &json!(false), &json!(true))); // a title is never bold: colour and weight are not status
    assert_eq!((todo.at("progress"), todo.at("says"), todo.at("live"), todo.s("mark")), (&json!(null), &json!([]), &json!(false), "○"));
    // a page that already has this is told only that
    let again = web(&w, "board", json!({ "have": s.s("mark") }));
    assert_eq!(again, json!({ "same": true, "mark": s.s("mark"), "ok": true, "role": "drive" }).as_object().map(|o| { let mut o = o.clone(); o.remove("ok"); o.remove("role"); serde_json::Value::Object(o) }).unwrap());
    // one task, whole
    let s = web(&w, "pick", json!({ "id": id, "open": true }));
    assert_eq!((s.at("list").s("selected"), s.at("panel").s("title")), (id.as_str(), "Make f"));
    assert_eq!(s.at("panel").arr("parts").iter().map(|p| p.t("head")).collect::<Vec<_>>(), vec!["Status", "Settings", "Description", "Review", "The agent said", "Session"]);
    let fact = |head: &str, label: &str| s.at("panel").arr("parts").iter().find(|p| p.s("head") == head).and_then(|p| p.arr("items").iter().find(|x| x.s("label") == label).cloned()).unwrap_or_default();
    assert_eq!(said(&fact("Status", "State").at("lines")[0]), format!("◆ ready to approve · {}", ready.s("time")));
    assert!(regex::Regex::new(r"^✓ Pass · 1 minor · \d+s ago · at [0-9a-f]{7}\|Good\.$").unwrap().is_match(&fact("Review", "Review 1").arr("lines").iter().map(said).collect::<Vec<_>>().join("|")), "{}", fact("Review", "Review 1"));
    assert!(regex::Regex::new(r"^minor +f\.txt has no title \(f\.txt\) · not required$").unwrap().is_match(&said(&fact("Review", "Findings").at("lines")[0])));
    assert_eq!(fact("Settings", "Merge").at("lines")[0][0].s("t"), "after your yes");
    assert_eq!(s.at("panel").arr("parts").iter().find(|p| p.s("head") == "Description").unwrap().at("items"), &json!([{ "k": "text", "value": [{ "t": "A file called f." }] }]));
    assert_eq!(s.arr("actions").iter().map(|b| (b.t("id"), b.t("label"))).collect::<Vec<_>>(), vec![("yes".into(), "approve and merge this task".into()), ("sendBack".into(), "send the review to the agent".into()), ("talk".into(), "talk to the agent".into())]);
    assert_eq!(s.at("canOpen"), &json!(true));
    assert!(s.arr("menu").iter().any(|m| m.s("id") == "archive" && m.s("label") == "archive this task") && s.arr("menu").iter().any(|m| m.s("id") == "scope"));
    assert_eq!((s.at("send").t("enter"), s.at("send").t("delegate"), s.at("send").t("place")), ("add it as a task".into(), "add it and put an agent on it".into(), "Add a task".into()));
    // opening the task's agent is a link to the web UI's own view of its pane; each press is a new going
    let t = task(&id);
    let s = web(&w, "press", json!({ "id": "open" }));
    assert_eq!((s.at("go").i("n"), s.at("go").s("pane"), s.at("go").s("workspace")), (1, t.at("link").s("pane"), t.at("link").s("workspace")));
    assert!(["chat", "terminal"].contains(&s.at("go").s("view")));
    assert_eq!(web(&w, "press", json!({ "id": "open" })).at("go").i("n"), 2);
    web(&w, "pick", json!({ "id": mine, "open": true }));
    let none = web(&w, "press", json!({ "id": "open" }));
    assert_eq!(none.at("go").i("n"), 2); assert!(none.at("msg").s("text").contains("No agent has been on this yet"), "{}", none.at("msg"));
}

#[test]
fn a_button_presses_the_boards_own_key_a_merge_asks_first_and_typed_words_are_sent_as_enter_sends_them() {
    let w = world();
    let id = make_f(&w);
    // every key the page can press arrives as that key, and as no other
    for k in sw::board::keys::KEYS.iter().filter(|k| !k.fixed) {
        let p = sw::webui::view::press_of(k.name).unwrap();
        let hits: Vec<&str> = sw::board::keys::KEYS.iter().filter(|x| !x.fixed && sw::board::keys::is(&p, x.id)).map(|x| x.name).collect();
        assert_eq!(hits, vec![k.name], "{}", k.name);
    }
    web_as(&w, "phone", "pick", json!({ "id": id, "open": true })).unwrap();
    let s = web_as(&w, "phone", "press", json!({ "id": "yes" })).unwrap();
    assert_eq!(s.at("ask"), &json!({ "q": "Merge PRJ-1 \"Make f\" into main?", "detail": "A merge cannot be undone.", "danger": true, "keys": [{ "id": "sayYes", "label": "merge" }, { "id": "sayNo", "label": "do not merge" }] }));
    assert_eq!(s.at("actions"), &json!([]));
    let s = web_as(&w, "phone", "press", json!({ "id": "sayNo" })).unwrap();
    assert!(s.at("ask").is_null()); assert_eq!(task(&id).s("stage"), "approval"); assert_eq!(s.s("draft"), "");
    assert!(web_as(&w, "desk", "board", json!({})).unwrap().at("ask").is_null(), "the other browser was never asked");
    web_as(&w, "phone", "press", json!({ "id": "yes" })).unwrap();
    let s = web_as(&w, "phone", "press", json!({ "id": "sayYes" })).unwrap();
    assert!(s.at("ask").is_null()); assert_ne!(s.at("msg").s("kind"), "green"); assert_ne!(task(&id).s("stage"), "approval");
    // a watching device presses only what shows or hides
    assert!(web_as(&w, "phone", "press", json!({ "id": "archive", "role": "watch" })).unwrap_err().contains("only watch"));
    assert!(web_as(&w, "phone", "send", json!({ "text": "x", "role": "watch" })).unwrap_err().contains("only watch"));
    let s = web_as(&w, "phone", "press", json!({ "id": "archiveView", "role": "watch" })).unwrap(); assert_eq!(s.at("flags").at("archive"), &json!(true));
    let s = web_as(&w, "phone", "press", json!({ "id": "archiveView", "role": "watch" })).unwrap(); assert_eq!(s.at("flags").at("archive"), &json!(false));
    // typed words: with no Coordinator they are added as a task, word for word
    web_as(&w, "phone", "pick", json!({ "id": "", "open": false })).unwrap();
    let s = web_as(&w, "phone", "send", json!({ "text": "Tidy the readme\nand its links" })).unwrap();
    assert_eq!(s.at("kept"), &json!(false)); assert!(!s.at("msg").s("text").ends_with("added"), "no confirmation: the task is on the list: {}", s.at("msg"));
    let added = sw::store::tasks::list(Default::default()).into_iter().find(|t| t.s("title") == "Tidy the readme").unwrap();
    assert_eq!(added.s("description"), "and its links");
    assert_eq!(web_as(&w, "phone", "send", json!({ "text": "   " })).unwrap().at("kept"), &json!(false));
    // the board's words name a button, not a key, on the page
    web_as(&w, "phone", "send", json!({ "text": "Sort the notes" })).unwrap();
    let plain = sw::store::tasks::list(Default::default()).into_iter().find(|t| t.s("title") == "Sort the notes").unwrap();
    let s = web_as(&w, "phone", "pick", json!({ "id": plain.s("id"), "open": true })).unwrap();
    let next = s.at("panel").arr("parts")[0].arr("items").iter().find(|x| x.s("label") == "Next").map(|x| said(&x.at("lines")[0])).unwrap_or_default();
    assert_eq!(next, "No agent is on this yet. “Put an agent on this task” puts one on it.");
    assert_eq!(s.arr("actions").iter().map(|b| b.t("id")).collect::<Vec<_>>(), vec!["delegate", "talk"]);
    let s = web_as(&w, "phone", "press", json!({ "id": "stop" })).unwrap(); assert!(s.at("ask").s("q").starts_with("Remove PRJ-"));
    let s = web_as(&w, "phone", "press", json!({ "id": "back" })).unwrap(); assert!(s.at("ask").is_null());
    // find narrows the list
    let s = web_as(&w, "phone", "find", json!({ "text": "readme" })).unwrap();
    assert_eq!(rows_of(&s).iter().map(|r| r.t("title")).collect::<Vec<_>>(), vec!["Tidy the readme"]);
    assert_eq!(rows_of(&web_as(&w, "phone", "find", json!({ "text": "" })).unwrap()).len(), 3);
    // the web board keeps its own choices: the terminal board's are not written
    web_as(&w, "phone", "press", json!({ "id": "byRepo" })).unwrap();
    assert_eq!(sw::store::files::read_obj(&sw::store::files::data_dir().join("web-view.json")).unwrap().at("byRepo"), &json!(true));
    assert_eq!(sw::store::files::read_obj(&sw::store::files::data_dir().join("view.json")).unwrap_or_default().at("byRepo"), &json!(null));
}

#[test]
fn every_colour_name_has_a_colour_on_the_page_every_state_a_mark_and_every_setting_a_row() {
    let css = include_str!("../crates/web/src/webui/page/board.css");
    use sw::shown::ink::Ink::*;
    for ink in [Text, Soft, Muted, Red, Green, Yellow, Teal, Peach, Accent, Review, Check, Queue, Todo, Auto, Rule, Dim] {
        let name = ink.name();
        assert!(regex::Regex::new(&format!(r"\.sw-c-{name}(?:[^a-z0-9_-])[^{{}}]*\{{[^}}]*color:")).unwrap().is_match(css), "the web page has no colour for \"{name}\"");
    }
    let states = sw::board::theme::marks::MARK_STATES;
    let marks: std::collections::BTreeSet<&str> = states.iter().map(|s| sw::board::theme::marks::web_mark(s, "")).collect();
    assert_eq!(marks.len(), states.len(), "one mark each");
    for kind in ["question", "permission", "auth"] { assert_eq!(sw::board::theme::marks::web_mark("blocked", kind), sw::board::theme::marks::mark("blocked", kind, true), "{kind}"); }
    let w = world();
    let id = w.add("A task");
    let s = web(&w, "pick", json!({ "id": id, "open": true }));
    let rows: std::collections::HashMap<String, serde_json::Value> = s.at("settings").arr("rows").iter().map(|r| (r.t("name"), r.clone())).collect();
    let mut left = 0;
    for e in sw::store::settings_list::SETTINGS.iter().filter(|e| e.kind == sw::store::settings_list::Kind::Work) {
        let Some(r) = rows.get(&e.name) else { assert!(e.name.ends_with(".fast"), "no row for \"{}\"", e.name); left += 1; continue };
        assert!(!r.s("label").is_empty() && r.s("label") != e.name, "{}: no plain label", e.name);
        assert!(!r.s("value").is_empty() && r.has("from") && r.has("own"), "{}: no value, or no own mark", e.name);
        assert!(if r.at("words").is_array() { r.arr("words").len() >= 2 && r.arr("words").iter().any(|x| x.at("now") == &json!(true)) } else { !r.s("hint").is_empty() }, "{}: neither words nor a hint", e.name);
    }
    assert!(left <= 3); assert_eq!(s.at("settings").at("canSet"), &json!(true));
}

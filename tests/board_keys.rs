//! The board's keys on a test world: typing adds a task, ctrl+s puts an agent on it, space replies to it, alt+o opens
//! the agent (enter on a task's row does nothing); ctrl+x stops an agent and asks to remove only where none runs; and while the text box holds words it owns the
//! keyboard, so no shortcut fires in the middle of a draft.
use crate::common;
use common::board::{open, open_with};
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::json::*;

/// The filter row on top and the three groupings of alt+s.
mod coord_rows;
mod filters;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }

#[test]
fn typing_adds_a_task_tab_delegates_space_replies_and_enter_opens_the_agent() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_ENV", "1") };
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Editing greet.sh\n  esc to interrupt\n" }] }));
    let mut s = open_with(&w.root, 36, true);
    s.typed("Add a greeting");
    let t = s.text();
    assert!(has(&t, r"\n─[^\n]*\n❯ Add a greeting"), "{t}");
    assert!(t.contains("enter add it as a task · ctrl+s add it and put an agent on it · shift+enter new line · esc put away"), "{t}");
    s.press(&["enter"]);
    assert_eq!(task("proj/1").s("title"), "Add a greeting");
    assert_eq!((s.s.sel.as_str(), s.s.value()), ("proj/1", String::new()));
    assert!(has(&s.text(), r"\n[ ▌] ctrl\+s put an agent on this task · space talk about this task · esc close · \? all keys"), "{}", s.text());
    // enter on a task's row with nothing typed: nothing at all; alt+o goes into its agent, and it has none yet
    let fx = s.hold("enter"); assert!(fx.is_empty() && s.s.msg.is_empty(), "{fx:?} {}", s.s.msg);
    s.press(&["alt+o"]); assert!(s.text().contains("No agent has been on this yet"));
    s.press(&["ctrl+s"]);
    settle("proj/1"); s.load();
    let t = s.text();
    assert!(has(&t, r"\n   ▾ working +[^\n]*\n[ ▌]  ◐ PRJ-\d+ +Add a greeting"), "{t}");
    assert!(!has(&t, "On its screen|Editing greet|esc to interrupt"), "nothing of the program's screen is shown");
    assert!(t.contains(&format!("alt+o open this task's agent · space talk to the agent · {} stop the agent · esc close · ? all keys", key_name("stop"))), "{t}");
    s.press(&["space"]);
    assert!(has(&s.text(), r"│  ❯ Send message to the agent"), "{}", s.text());
    w.script(json!({ "worker": [{ "status": "working" }] }));
    s.typed("also say goodbye");
    assert!(s.text().contains("│  ❯ also say goodbye") && s.text().contains("enter send to PRJ-1's agent · shift+enter new line · esc put away"), "{}", s.text());
    s.press(&["enter"]);
    // What you just sent stands in the box, in place of what the agent had said, until it says something new.
    assert!(has(&s.text(), r"\n╭─[^\n]*╮\n│ +│\n│  also say goodbye +│\n│ +│\n│  ❯ Send message to the agent +│\n│ +│\n╰─+╯\n"), "{}", s.text());
    assert!(!s.text().contains("Sent to the agent"));
    let t1 = task("proj/1");
    assert!(prompts(&w, t1.at("link").s("pane")).last().unwrap().contains("also say goodbye"));
    // esc has nothing else to go back from here: it hides the chat's message, and never changes who you talk to. No
    // coordinator can run here: space goes back to the box that adds tasks.
    assert!(s.text().contains("· space back") && !s.text().contains("esc close"), "{}", s.text());
    s.press(&["esc"]); assert!(!s.s.chat_open && s.s.reply && s.s.ask.is_none() && s.s.closing.is_none() && !s.closed);
    assert!(s.text().contains(&format!("esc close · {} show the latest message", key_name("chatMessage"))), "{}", s.text());
    s.press(&["esc"]); assert!(s.closed, "this board is the popup: the last esc closes it");
    s.closed = false;
    s.press(&["alt+c"]); assert!(s.s.chat_open);
    s.press(&["space"]); assert!(!s.s.reply);
    assert!(has(&s.text(), r"\n─[^\n]*\n❯ Add a task"), "{}", s.text()); // no coordinator here: the box adds tasks
    // alt+o: the agent's real pane by pane id; the popup closes for it
    s.press(&["alt+o"]);
    assert!(s.closed, "the popup closes");
    assert!(w.state().arr("focus").iter().any(|f| f.s("") == "" && text(f) == format!("agent {}", t1.at("link").s("pane"))), "{}", w.state().at("focus"));
    let popup = sw::store::files::read_obj(&sw::store::files::data_dir().join("view-popup.json")).unwrap_or_default();
    assert_eq!(popup.s("sel"), "proj/1");
    assert!(!sw::store::files::data_dir().join("view.json").exists(), "the popup keeps its view in a file of its own");
    // ctrl+x stops the agent, and the selection stays where it was: the task left the working group, which is empty
    // now, so the next group's first task. Only a second ctrl+x on the stopped task asks to remove, and only y would.
    w.add("Another one"); w.add("And one more");
    let mut s2 = open_with(&w.root, 34, true);
    assert_eq!(s2.s.sel, "proj/1");
    s2.press(&["ctrl+x"]);
    assert_eq!(task("proj/1").s("stage"), "stopped");
    assert!(s2.text().contains("Stopped PRJ-1 \"Add a greeting\"."), "{}", s2.text());
    assert!(s2.s.ask.is_none());
    let todo = s2.s.steps.iter().position(|x| x == "head:status\t\tto do").unwrap(); assert_eq!(s2.s.sel, s2.s.steps[todo + 1]);
    s2.s.sel = "proj/1".into(); s2.load();
    s2.press(&["ctrl+x"]);
    assert!(has(&s2.text(), r#"\n[ ▌]\? Remove PRJ-1 "Add a greeting"\?[^?]*\n   y = remove · n = keep"#), "{}", s2.text());
    s2.press(&["enter", "ctrl+x"]); assert!(s2.s.ask.is_some());
    s2.press(&["n"]); assert!(s2.s.ask.is_none());
    assert!(sw::store::tasks::get("proj/1").is_some() && sw::store::tasks::get("proj/3").is_some());
    unsafe { std::env::remove_var("HERDR_ENV") };
}

#[test]
fn while_the_text_box_holds_words_no_shortcut_fires_and_ctrl_c_twice_closes() {
    let w = world();
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }], "reviewer": [{ "verdict": { "meets": true, "text": "Good.", "findings": [{ "severity": "minor", "subject": "f.txt has no title", "where": "f.txt" }] } }] }));
    let id = w.add("Make f"); w.add("Other");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 34);
    s.s.sel = id.clone(); s.load();
    s.typed("half a thought");
    let before = (task(&id).t("stage"), task(&id).b("pinned"), task(&id).b("archivedAt"), sw::store::tasks::list(Default::default()).len());
    for k in ["ctrl+l", "ctrl+r", "ctrl+x", "ctrl+x", "ctrl+p", "ctrl+f", "alt+c", "alt+p", "alt+t", "alt+a", "alt+s", "alt+d", "alt+m", "ctrl+e", "ctrl+g", "ctrl+t", "ctrl+d", "ctrl+b", "ctrl+o", "alt+h", "alt+v", "alt+r", "alt+n", "ctrl+a", "ctrl+e", "?", "space", "up", "down"] { s.press(&[k]); }
    assert_eq!((task(&id).t("stage"), task(&id).b("pinned"), task(&id).b("archivedAt"), sw::store::tasks::list(Default::default()).len()), before, "nothing happened to any task");
    assert!(s.s.sel == id && s.s.ask.is_none() && s.s.menu.is_none() && !s.s.full && !s.s.archive_view && !s.s.all_done && !s.s.by_repo && s.s.chat_open && s.s.msg.is_empty());
    assert!(s.s.value().contains("half a thought") && s.s.value().contains('?'), "\"?\" and space are typed, not acted on: {}", s.s.value());
    // esc puts the draft away, and it can be brought back
    s.press(&["esc"]); assert_eq!(s.s.value(), "");
    assert!(s.text().contains(&format!("Put away. {} brings it back.", key_name("undo"))));
    s.press(&["ctrl+z"]); assert!(s.s.value().contains("half a thought"));
    s.press(&["esc"]);
    let hint = format!("Press {} again to close the board", key_name("close"));
    let esc_hint = format!("{} closes the board", key_name("close"));
    s.press(&["esc"]); assert!(!s.closed && s.s.ask.is_none()); assert!(s.text().contains(&esc_hint), "{}", s.text());
    s.press(&["enter"]); assert!(!s.closed); assert!(!s.text().contains(&esc_hint));
    s.press(&["ctrl+c"]); assert!(s.text().contains(&hint)); s.typed("x"); assert!(!s.text().contains(&hint)); assert_eq!(s.s.value(), "x");
    s.press(&["ctrl+c"]); assert!(!s.closed); assert_eq!(s.s.value(), "x"); assert!(s.text().contains(&hint));
    s.press(&["ctrl+c"]); assert!(s.closed, "the second ctrl+c closes, and the draft is kept");
    assert_eq!(s.s.value(), "x");
    // the other questions keep y and n, and ctrl+c is a no there
    let mut s2 = open(&w.root, 34);
    s2.s.sel = id.clone(); s2.load();
    s2.press(&["ctrl+l"]);
    assert!(has(&s2.text(), r#"\? Merge PRJ-1 "Make f" into \S+\? *\n[ ▌]  A merge cannot be undone\. *\n[ ▌]  y = merge · n = do not merge"#), "{}", s2.text());
    s2.press(&["ctrl+c"]); assert!(s2.s.ask.is_none() && !s2.closed); assert_eq!(task(&id).s("stage"), "approval");
    s2.press(&["ctrl+c", "ctrl+c"]); assert!(s2.closed);
}

#[test]
fn ctrl_x_stops_an_agent_that_waits_for_your_answer_and_asks_to_remove_only_where_none_runs() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let id = w.add("One");
    w.script(json!({ "worker": [{ "status": "blocked", "screen": "Which colour?\n" }] }));
    delegate(&id); settle(&id);
    sw::store::tasks::update(&id, |t| { t.put("stage", "asks"); t.put("kind", "question"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    let mut s = open(&w.root, 34);
    s.s.sel = id.clone(); s.load();
    assert_eq!((s.s.current().unwrap().t("stage"), s.s.current().unwrap().t("state")), ("asks".into(), "blocked".into()));
    let items = sw::board::update::menu::items(&s.s);
    assert_eq!(items.iter().find(|i| i.id == sw::board::keys::K::Stop).map(|i| i.label.clone()).unwrap_or_default(), "stop this task's agent");
    s.press(&["ctrl+x"]); assert!(s.s.ask.is_none());
    assert_eq!(task(&id).s("stage"), "stopped");
    assert!(s.text().contains("Stopped PRJ-1 \"One\"."));
    s.press(&["ctrl+x"]); assert!(has(&s.text(), r#"\n[ ▌]\? Remove PRJ-1 "One"\?"#), "{}", s.text());
    s.press(&["n"]); assert!(sw::store::tasks::get(&id).is_some());
}

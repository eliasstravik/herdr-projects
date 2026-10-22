//! Who said what in a task's chat, as in the Coordinator's: your words and the agent's with no label, and
//! what others sent it under their own name: the board's brief and its own prompts ("Board:"), the Coordinator's
//! ("Coordinator:"). The same in the box, on the web board and for another machine's task.
use crate::common;
use common::*;
use serde_json::json;
use sw::board::view::talk::talk;
use sw::json::*;
use sw::store::said_by::{note_board, note_person, senders};

fn sent_by(r: &Obj) -> Vec<(String, String)> { talk(r).parts.into_iter().filter(|p| p.sent).map(|p| (p.by, p.text.lines().next().unwrap_or("").to_string())).collect() }

#[test]
fn each_thing_sent_to_a_tasks_agent_is_under_the_name_of_who_sent_it() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }, { "status": "working" }, { "status": "working" }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let mut s = common::board::open(&w.root, 34); s.s.side = false;
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    // you, in the board's chat
    s.typed("yes, g too"); s.press(&["enter"]); settle(&id);
    // the Coordinator, with the command it runs
    sw::actions::talk::reply(&id, "and h as well", "coordinator").unwrap(); settle(&id);
    // the board, on its own (what a failed check sends back)
    sw::flow::send::send_own(&id, "The check failed: fix it.", "check", false, Obj::new(), 0).unwrap(); settle(&id);
    // its conversation as Claude Code saves it: each prompt the board typed (just now), each answered
    let t = task(&id);
    let rows: Vec<String> = prompts(&w, t.at("link").s("pane")).iter().enumerate().flat_map(|(n, p)| [json!({ "type": "user", "timestamp": sw::clock::iso(sw::clock::now()), "message": { "role": "user", "content": p } }),
        json!({ "type": "assistant", "timestamp": sw::clock::iso(sw::clock::now()), "message": { "id": format!("m{n}"), "role": "assistant", "content": [{ "type": "text", "text": format!("Done {n}.") }] } })]).map(|v| v.to_string()).collect();
    let file = std::path::PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap()).join("projects").join("x").join(format!("{}.jsonl", t.s("session")));
    std::fs::create_dir_all(file.parent().unwrap()).unwrap(); std::fs::write(&file, rows.join("\n") + "\n").unwrap();
    s.load();
    let r = s.s.current().unwrap().clone();
    let want: Vec<(String, String)> = [("Board", "Work board task proj/1, role: agent."), ("You", "yes, g too"), ("Coordinator", "and h as well"), ("Board", "The check failed: fix it.")].iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    assert_eq!(sent_by(&r), want);
    // the box: each under the line that names who said it, as in the Coordinator's chat
    s.s.just_sent.clear(); // (the agent here says nothing new after your words, which would stand in the box till then)
    let mut b = crate::board_chat::chat(&mut s, 120).boxed;
    for _ in 0..12 { s.press(&["alt+k"]); b.extend(crate::board_chat::chat(&mut s, 120).boxed); }
    for name in ["Board:", "Coordinator:"] { assert!(b.iter().any(|l| l == name), "{name} in {b:?}"); }
    assert!(!b.iter().any(|l| l.starts_with("To the")), "{b:?}");
    // the web board: the same names
    let side = sw::webui::model::side::side_of(&mut s.s, &s.theme.clone());
    let by: Vec<String> = side.at("chat").arr("parts").iter().filter(|p| p.s("by") != "Agent").map(|p| p.t("by")).collect();
    assert_eq!(by, vec!["Board", "You", "Coordinator", "Board"]);
    // another machine's task: its machine says who sent what
    let snap = sw::remote::remote_list().at("tasks").as_array().unwrap().iter().find(|x| x.s("id") == id).unwrap().at("snap").clone();
    let mut far = r.clone(); far.insert("host".into(), json!("vm-a")); far.insert("snap".into(), snap);
    assert_eq!(sent_by(&far).into_iter().map(|p| p.0).collect::<Vec<_>>(), vec!["Board", "You", "Coordinator", "Board"]);
    // a machine that does not say who: all of it is yours, never "To the agent"
    let mut old = r.clone(); old.insert("host".into(), json!("vm-b")); old.insert("snap".into(), json!({ "talk": { "who": "agent", "readable": true, "list": [{ "who": "you", "text": "Do it" }, { "who": "them", "text": "Done." }] } }));
    assert_eq!(sent_by(&old), vec![("You".to_string(), "Do it".to_string())]);
}

fn of(id: &str, sent: &[(&str, i64)]) -> Vec<String> { senders(id).of(sent) }

#[test]
fn what_nobody_noted_is_yours_and_your_words_stay_yours_on_their_way_in() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let id = w.add("Make f");
    sw::clock::set(Some(1_000_000));
    // typed into the program's own box: nobody noted it; the board's brief, sent before the board kept who sent what
    assert_eq!(of(&id, &[("look at the logs", 1_000_000), (&format!("Work board task {id}, role: agent.\n\nMake f"), 0)]), vec!["you", "board"]);
    // your words, then the board typing them in: still yours, also where two went in as one
    note_person(&id, "you", "yes"); note_board(&id, "yes");
    note_person(&id, "you", "and  then\nthat"); note_person(&id, "you", "one more"); note_board(&id, "and then that\n\none more");
    assert_eq!(of(&id, &[("yes", 1_000_500), ("and then that\n\none more", 1_000_500)]), vec!["you", "you"]);
    // the board's own words, even where they start as yours did
    sw::clock::advance(60_000); note_board(&id, "yes, and the check failed");
    assert_eq!(of(&id, &[("yes", 1_000_500), ("yes, and the check failed", 1_060_100)]), vec!["you", "board"]);
    // your words inside the board's own (a plan sent back with them): yours
    let words = sw::plan::back_words(&task(&id), "Please include a rollback step");
    note_person(&id, "you", "Please include a rollback step"); sw::store::said_by::carry(&id, "Please include a rollback step", &id, &words); note_board(&id, &words);
    assert_eq!(of(&id, &[(&words, 1_060_200)]), vec!["you"]);
}

#[test]
fn the_same_words_twice_are_each_their_own_senders_in_the_order_they_were_said() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let (a, b) = (w.add("Make a"), w.add("Make b"));
    sw::clock::set(Some(2_000_000));
    // the Coordinator says "continue", then you do
    for id in [&a, &b] { note_person(id, "coordinator", "continue"); note_board(id, "continue"); }
    sw::clock::advance(120_000);
    note_person(&a, "you", "continue"); note_board(&a, "continue");
    assert_eq!(of(&a, &[("continue", 2_000_400), ("continue", 2_120_300)]), vec!["coordinator", "you"]);
    // only the newer one still in what is read: it is yours
    assert_eq!(of(&a, &[("continue", 2_120_300)]), vec!["you"]);
    // the second typed into the agent's own box: nobody noted it, so it is yours, and the first stays the Coordinator's
    assert_eq!(of(&b, &[("continue", 2_000_400), ("continue", 2_120_300)]), vec!["coordinator", "you"]);
    // a program that says no time: in order
    assert_eq!(of(&a, &[("continue", 0), ("continue", 0)]), vec!["coordinator", "you"]);
    // long after the Coordinator's, the same words typed into the agent's own box: yours, also once the Coordinator's
    // has left what is read
    let c = w.add("Make c");
    sw::clock::set(Some(3_000_000)); note_person(&c, "coordinator", "continue"); note_board(&c, "continue");
    assert_eq!(of(&c, &[("continue", 3_000_100), ("continue", 3_600_000)]), vec!["coordinator", "you"]);
    assert_eq!(of(&c, &[("continue", 3_600_000)]), vec!["you"]);
    // you, then the Coordinator a moment later: in that order, however close the times the conversation has
    let d = w.add("Make d");
    sw::clock::set(Some(3_999_000)); note_person(&d, "you", "continue"); note_board(&d, "continue");
    sw::clock::set(Some(4_000_000)); note_person(&d, "coordinator", "continue"); note_board(&d, "continue");
    for at in [(3_999_500, 4_000_500), (4_000_800, 4_000_900), (0, 0)] { assert_eq!(of(&d, &[("continue", at.0), ("continue", at.1)]), vec!["you", "coordinator"], "{at:?}"); }
    sw::clock::set(Some(2_120_400));
    // two texts alike in their first 200 characters are still two
    let long = "x".repeat(250);
    let (one, two) = (format!("{long} one"), format!("{long} two"));
    note_board(&a, &one); note_person(&a, "you", &two); note_board(&a, &two);
    assert_eq!(of(&a, &[(&one, 2_120_500), (&two, 2_120_500)]), vec!["board", "you"]);
}

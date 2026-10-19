//! What the old box-colour tests checked that tests/board_colours.rs does not: a task's mark in the list is its status
//! colour too (a finished one's made quieter) and its group's; the frame's colour follows the status while words are
//! typed (the prompt stays plain); an agent whose pane is gone takes no message; and the grey words inside the box name
//! who gets the message.
use crate::common;
use common::*;
use serde_json::json;
use sw::board::state::State;
use sw::board::theme::Theme;
use sw::board::view::textbox::{box_ink, box_lines};
use sw::json::*;
use sw::shown::ink::Ink;
use sw::shown::states::{group_ink, group_of, status_ink};

fn statuses() -> Vec<(&'static str, serde_json::Value, Ink)> {
    vec![
        ("working", json!({ "state": "working", "stage": "working", "agent": "claude", "link": { "pane": "p1" } }), Ink::Yellow),
        ("needs you", json!({ "state": "blocked", "kind": "question", "stage": "working", "agent": "claude", "link": { "pane": "p1" } }), Ink::Red),
        ("went wrong", json!({ "state": "error", "stage": "failed", "agent": "claude", "link": { "pane": "p1" } }), Ink::Red),
        ("ready to approve", json!({ "state": "blocked", "kind": "permission", "stage": "approval", "agent": "claude", "link": { "pane": "p1" } }), Ink::Green),
        ("under review", json!({ "state": "working", "stage": "reviewing", "agent": "claude", "link": { "pane": "p1" } }), Ink::Review),
        ("the board's check", json!({ "state": "working", "stage": "checking", "agent": "claude", "link": { "pane": "p1" } }), Ink::Check),
        ("done", json!({ "state": "done", "stage": "merged", "agent": "claude", "link": { "pane": "p1" } }), Ink::Teal),
        ("to do", json!({ "state": "idle", "stage": "todo" }), Ink::Todo),
        ("archived", json!({ "state": "done", "stage": "merged", "agent": "claude", "link": { "pane": "p1" }, "archivedAt": 1 }), Ink::Muted),
    ]
}
fn with(base: &Obj, fields: &serde_json::Value) -> Obj { let mut r = base.clone(); for (k, v) in fields.as_object().unwrap() { r.insert(k.clone(), v.clone()); } r }

#[test]
fn the_mark_is_the_status_colour_too_and_the_colour_follows_the_status_while_typing() {
    let w = world();
    let base = task(&w.add("Make f"));
    let t = Theme::default_dark(false);
    for (name, fields, colour) in statuses() {
        let r = with(&base, &fields);
        // the status's colour is its group's (the reviewer's and the board's check have colours of their own)
        if name != "archived" && ![Ink::Review, Ink::Check].contains(&colour) { assert_eq!(group_ink(group_of(&r)), colour, "{name}"); }
        // the mark in the list is that colour too; a finished task's the same one made quieter
        let mark = sw::board::view::fields::mark(&t, &r);
        assert!(!mark.text().trim().is_empty(), "{name}");
        let p = mark.0.iter().find(|p| !p.t.trim().is_empty()).unwrap();
        assert_eq!(p.c, colour, "{name}: the mark's colour");
        if name == "done" { assert!(p.quiet, "a finished task's mark is quieter"); }
        assert_eq!(status_ink(&r), colour, "{name}");
    }
    // the colour follows the status while something is typed: the same words, three colours
    let mut s = State::new(Default::default(), Default::default());
    s.input.set_text("still typing"); s.reply = true;
    let seen: Vec<_> = ["working", "needs you", "ready to approve"].iter().map(|n| { let f = statuses().into_iter().find(|x| x.0 == *n).unwrap().1; box_lines(&mut s, 60, 3, Some(&with(&base, &f))).0 }).collect();
    assert_eq!(seen.iter().map(|l| l[1].text().trim_end().to_string()).collect::<Vec<_>>(), vec!["❯ still typing"; 3]);
    let inks = |row: usize| seen.iter().map(|l| format!("{:?}", l[row].0.iter().map(|p| p.c).collect::<Vec<_>>())).collect::<std::collections::BTreeSet<String>>();
    assert_eq!(inks(0).len(), 3, "the rule: three colours {seen:?}");
    assert_eq!(inks(1).len(), 1, "the line typed on: one look {seen:?}");
    // an agent whose pane is gone takes no message: enter sends to the Coordinator, in its colour
    s.input.set_text("");
    assert_eq!(box_ink(&s, Some(&with(&base, &json!({ "state": "working", "stage": "working", "agent": "claude" })))), Ink::Text);
}

#[test]
fn the_grey_words_in_the_box_name_who_gets_the_message() {
    let w = world();
    let base = task(&w.add("Make f"));
    let mut s = State::new(Default::default(), Default::default());
    for (name, fields, _) in statuses() {
        let r = with(&base, &fields);
        for reply in [true, false] {
            s.reply = reply;
            let words = box_lines(&mut s, 60, 3, Some(&r)).0[1].text();
            if !reply { assert!(!regex::Regex::new("Agent|starts an agent").unwrap().is_match(&words), "{name}: {words}"); }
            else if name == "to do" { assert!(words.contains("your message starts an agent on it"), "{name}: {words}"); }
            else if name == "archived" { assert!(words.contains("archived: read-only"), "{name}: {words}"); }
            else { assert!(words.starts_with("❯ Send message to "), "{name}: {words}"); }
        }
    }
}

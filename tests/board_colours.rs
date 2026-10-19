//! Who you type to is said by colour, and colour is never the only clue: the text box (its rules and its frame, never
//! the prompt or anything else inside it) is the task's status colour when it sends to that task's agent and herdr's
//! plain text colour for the Coordinator; each status has a colour of its own ("to do" a readable grey of its own); with NO_COLOR the same words
//! stand there with no colour at all.
use crate::common;
use common::*;
use serde_json::json;
use sw::board::state::State;
use sw::board::theme::Theme;
use sw::board::view::textbox::{box_ink, box_lines, chat_box};
use sw::json::*;
use sw::shown::ink::{Ink, Inked};
use sw::shown::states::status_ink;

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
fn starts(l: &Inked, ink: Ink, what: &str) { assert_eq!(l.0.iter().find(|p| !p.t.trim().is_empty()).map(|p| p.c), Some(ink), "{what}: {l:?}"); }
fn ends(l: &Inked, ink: Ink, what: &str) { assert_eq!(l.0.iter().rev().find(|p| !p.t.trim().is_empty()).map(|p| p.c), Some(ink), "{what}: its right edge: {l:?}"); }

#[test]
fn the_box_is_the_tasks_status_colour_for_its_agent_and_plain_text_for_the_coordinator() {
    let w = world();
    let id = w.add("Make f");
    let mut s = State::new(Default::default(), Default::default());
    let base = task(&id);
    for typed in ["", "half a sentence"] {
        s.input.set_text(typed);
        for (name, fields, colour) in statuses() {
            for host in ["", "pc"] {
                let mut r = base.clone();
                for (k, v) in fields.as_object().unwrap() { r.insert(k.clone(), v.clone()); }
                if !host.is_empty() { r.put("host", host); }
                // Another machine's task nobody started has no agent to start from here: its message goes to the Coordinator.
                let want = if !host.is_empty() && name == "to do" { Ink::Text } else { colour };
                let what = format!("{name}{}{}", if host.is_empty() { "" } else { " on another machine" }, if typed.is_empty() { "" } else { ", typing" });
                if host.is_empty() { assert_eq!(status_ink(&r), colour, "{name}"); }
                s.reply = true;
                assert_eq!(box_ink(&s, Some(&r)), want, "{what}");
                let (lines, _) = box_lines(&mut s, 60, 3, Some(&r));
                starts(&lines[0], want, &what); starts(lines.last().unwrap(), want, &what); starts(&lines[1], Ink::Text, &format!("{what}: the prompt, as the Coordinator's"));
                assert!(lines[1].text().starts_with("❯ "), "{what}");
                let (framed, _) = chat_box(&mut s, 60, 3, Some(&r), &[Inked::plain("hello")]);
                for l in &framed { starts(l, want, &what); ends(l, want, &what); }
                // inside the frame, as with the Coordinator: the prompt in plain text, the words as they came
                let inside = |l: &Inked| Inked(l.0.iter().skip(1).take(l.0.len().saturating_sub(2)).cloned().collect());
                let prompt = framed.iter().find(|l| l.text().contains('❯')).unwrap();
                starts(&inside(prompt), Ink::Text, &format!("{what}: the prompt inside the frame"));
                starts(&inside(&framed[1]), Ink::Plain, &format!("{what}: the message inside the frame"));
                s.reply = false;
                assert_eq!(box_ink(&s, Some(&r)), Ink::Text, "{what}: to the Coordinator");
                let (lines, _) = box_lines(&mut s, 60, 3, Some(&r));
                starts(&lines[0], Ink::Text, &what);
            }
        }
    }
    // Each status has a colour of its own, and none is the Coordinator's.
    let t = Theme::default_dark(false);
    let live = [Ink::Yellow, Ink::Red, Ink::Green, Ink::Review, Ink::Check, Ink::Teal, Ink::Todo, Ink::Text];
    let colours: std::collections::BTreeSet<String> = live.iter().map(|i| format!("{:?}", t.ink(*i))).collect();
    assert_eq!(colours.len(), live.len(), "{colours:?}");
    assert_ne!(t.ink(Ink::Todo), t.ink(Ink::Text), "\"to do\" is a grey of its own, never the plain text colour");
    // A task with no agent on it, whose message goes to the Coordinator: the Coordinator's colour. Find is no message at all.
    s.input.set_text(""); s.reply = true;
    let mut stopped = base.clone(); stopped.put("state", "idle"); stopped.put("stage", "stopped");
    assert_eq!(box_ink(&s, Some(&stopped)), Ink::Text);
    s.mode = sw::board::state::Mode::Find;
    let mut working = base.clone(); for (k, v) in statuses()[0].1.as_object().unwrap() { working.insert(k.clone(), v.clone()); }
    assert_eq!(box_ink(&s, Some(&working)), Ink::Text);
}

#[test]
fn with_no_color_the_board_has_the_same_words_and_no_colour() {
    let w = world();
    w.add("Make f");
    let mut b = common::board::open(&w.root, 30);
    let draw = |b: &mut common::board::Board, plain: bool| {
        let t = Theme::default_dark(plain);
        let lines = sw::board::view::screen(&mut b.s, &t, 100, 30).lines;
        let mut buf = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 100, 30));
        sw::board::run::draw::draw(&t, &lines, &mut buf);
        buf
    };
    let coloured = draw(&mut b, false);
    let plain = draw(&mut b, true);
    let text = |buf: &ratatui::buffer::Buffer| (0..30).map(|y| (0..100).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>()).collect::<Vec<_>>();
    assert_eq!(text(&plain), text(&coloured), "the same words");
    let mut painted = 0;
    for y in 0..30 { for x in 0..100 {
        let (p, c) = (&plain[(x, y)], &coloured[(x, y)]);
        assert_eq!(p.fg, ratatui::style::Color::Reset, "no colour at {x},{y}");
        if c.fg != ratatui::style::Color::Reset { painted += 1; }
    } }
    assert!(painted > 0, "with colour on, something is painted");
    let shown = text(&plain).join("\n");
    assert!(regex::Regex::new(r"(?m)^ ▾ to do\s").unwrap().is_match(&shown), "the heading says \"to do\" where the colour would: {shown}");
}

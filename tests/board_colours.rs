//! Colour means status only, and colour is never the only clue: the text box's frame (never the prompt or anything else
//! inside it) is the status colour of the task whose agent it sends to, the base colour for one to do, done or archived,
//! and its top edge names it (view/chat_title.rs); herdr's plain text colour where no one is talked to; each status has a colour of its own ("to do" a readable grey of its own); with NO_COLOR the same words
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
fn the_box_edge_is_the_status_colour_of_the_task_it_talks_to_and_plain_text_with_no_coordinator() {
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
                let what = format!("{name}{}{}", if host.is_empty() { "" } else { " on another machine" }, if typed.is_empty() { "" } else { ", typing" });
                if host.is_empty() { assert_eq!(status_ink(&r), colour, "{name}"); }
                // Talking to a task: the edge is its status colour, as its mark; base for one to do, done or archived.
                s.reply = true;
                let edge = if ["to do", "done", "archived"].contains(&name) { Ink::Text } else { colour };
                assert_eq!(box_ink(&s, Some(&r)), edge, "{what}");
                let (framed, _) = chat_box(&mut s, 60, 3, Some(&r), &[Inked::plain("hello")]);
                for l in &framed { starts(l, edge, &what); ends(l, edge, &what); }
                // its top edge is the ladder; inside, only the message and the line you type on
                assert!(framed[0].text().starts_with("╭─ ") && framed[0].text().contains(" › "), "{what}: {}", framed[0].text());
                // inside the frame: the message as it came, the prompt in plain text
                let inside = |l: &Inked| Inked(l.0.iter().skip(1).take(l.0.len().saturating_sub(2)).cloned().collect());
                let prompt = framed.iter().find(|l| l.text().contains('❯')).unwrap();
                starts(&inside(prompt), Ink::Text, &format!("{what}: the prompt inside the frame"));
                // an empty row under the top edge and over the bottom one, room to breathe
                assert!(framed[1].text().trim_matches(['│', ' ']).is_empty() && framed[framed.len() - 2].text().trim_matches(['│', ' ']).is_empty(), "{what}: {:?}", framed.iter().map(|l| l.text()).collect::<Vec<_>>());
                let message = framed.iter().find(|l| l.text().contains("hello")).unwrap();
                assert!(message.text().starts_with("│  hello"), "{what}: two cells of padding: {}", message.text());
                starts(&inside(message), Ink::Plain, &format!("{what}: the message inside the frame"));
                // No coordinator can run and no task is talked to: plain text, and the box adds tasks.
                s.reply = false;
                assert_eq!(box_ink(&s, Some(&r)), Ink::Text, "{what}: no one to talk to");
                let (lines, _) = box_lines(&mut s, 60, 3, Some(&r));
                starts(&lines[0], Ink::Text, &what);
            }
        }
    }
    // Each status has a colour of its own, and none is plain text.
    let t = Theme::default_dark(false);
    let live = [Ink::Yellow, Ink::Red, Ink::Green, Ink::Review, Ink::Check, Ink::Teal, Ink::Todo, Ink::Text];
    let colours: std::collections::BTreeSet<String> = live.iter().map(|i| format!("{:?}", t.ink(*i))).collect();
    assert_eq!(colours.len(), live.len(), "{colours:?}");
    assert_ne!(t.ink(Ink::Todo), t.ink(Ink::Text), "\"to do\" is a grey of its own, never the plain text colour");
    // Find is no message at all: plain text.
    s.input.set_text(""); s.reply = true;
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
    assert!(regex::Regex::new(r"(?m)^ +▾ to do\s").unwrap().is_match(&shown), "the heading says \"to do\" where the colour would: {shown}");
}

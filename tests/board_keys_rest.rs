//! What the old board-keys tests checked that tests/board_keys.rs does not: the popup says how to get back before it
//! closes for an agent and takes no key meanwhile; which tasks have an agent alive for ctrl+x to stop; and the close
//! hints in full (after esc, after ctrl+c, in the menu, the two seconds it stays), ctrl+y gone, a draft kept on close.
use crate::common;
use common::board::{open, open_with, Board};
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::board::update::{update, Effect, Event};
use sw::json::*;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn tick(b: &mut Board, at: i64) -> Vec<Effect> { update(&mut b.s, Event::Tick(at)) }

#[test]
fn the_popup_says_how_to_get_back_before_it_closes_for_an_agent_and_takes_no_key_meanwhile() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_ENV", "1") };
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Editing\n" }] }));
    let id = w.add("Add a greeting"); delegate(&id); settle(&id);
    let mut v = open_with(&w.root, 34, true);
    v.s.ctx.leave_ms = 30; v.s.ctx.board_key = "prefix+w".into();
    let now = sw::clock::now(); tick(&mut v, now);
    v.s.sel = id.clone(); v.load(); v.press(&["alt+o"]);
    assert!(!v.closed, "not at once: the line shows first");
    assert!(v.text().contains("Opening the agent. prefix+w brings the board back."), "{}", v.text());
    v.press(&["ctrl+x"]); assert_eq!(task(&id).s("stage"), "working", "nothing more is taken from the keyboard"); assert!(v.s.ask.is_none());
    assert!(!tick(&mut v, now + 10).contains(&Effect::Close));
    assert!(tick(&mut v, now + 80).contains(&Effect::Close), "the board closes a moment after the line shows");
    unsafe { std::env::remove_var("HERDR_ENV") };
    // once the agent is stopped, the menu's line for the same key is "remove"
    let mut s = open(&w.root, 34); s.s.sel = id.clone(); s.load(); s.press(&["ctrl+x"]);
    assert_eq!(task(&id).s("stage"), "stopped");
    s.press(&["?"]); assert!(has(&s.text(), &format!("{} +remove this task", regex::escape(&key_name("stop")))), "{}", s.text());
}

#[test]
fn an_agent_is_alive_while_it_works_waits_for_you_or_for_a_slot_and_not_once_it_stopped_or_finished() {
    use sw::board::update::actions::running;
    let r = |v: serde_json::Value| { let mut o = obj(json!({ "id": "proj/1", "title": "One" })); for (k, x) in v.as_object().unwrap() { o.insert(k.clone(), x.clone()); } o };
    for live in [json!({ "agent": "claude", "stage": "working", "state": "working" }), json!({ "agent": "claude", "stage": "asks", "state": "blocked", "kind": "question" }), json!({ "agent": "claude", "stage": "asks", "state": "blocked", "kind": "permission" }),
        json!({ "agent": "claude", "stage": "working", "state": "blocked", "kind": "question" }), json!({ "agent": "claude", "stage": "reviewing", "state": "working" }), json!({ "agent": "claude", "stage": "checking", "state": "working" }), json!({ "agent": "claude", "stage": "starting", "state": "working" }),
        json!({ "stage": "queued", "state": "idle" }), json!({ "stage": "progress", "state": "working" })] { assert!(running(&r(live.clone())), "{live}"); }
    for still in [json!({ "stage": "todo", "state": "idle" }), json!({ "agent": "claude", "stage": "stopped", "state": "idle" }), json!({ "agent": "claude", "stage": "approval", "state": "blocked" }), json!({ "agent": "claude", "stage": "failed", "state": "error" }),
        json!({ "agent": "claude", "stage": "nosummary", "state": "blocked" }), json!({ "agent": "claude", "stage": "done", "state": "done" })] { assert!(!running(&r(still.clone())), "{still}"); }
}

#[test]
fn closing_takes_ctrl_c_twice_and_the_hints_say_so_wherever_you_are() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let id = w.add("Make f"); w.add("Other");
    let mut s = open(&w.root, 34); s.s.sel = id.clone(); s.load();
    let now = sw::clock::now(); tick(&mut s, now);
    let hint = format!(r"\n[ ▌] Press {} again to close the board *$", regex::escape(&key_name("close")));
    let esc_hint = format!(r"\n[ ▌] {} closes the board *$", regex::escape(&key_name("close")));
    s.press(&["esc"]); assert!(has(&s.text(), &esc_hint), "{}", s.text()); assert!(!s.text().contains("Close the board?"));
    // closing is not a question; another esc says it again, and alt+o does what it does
    s.press(&["down", "esc"]); assert!(has(&s.text(), &esc_hint)); s.press(&["up"]);
    s.press(&["alt+o"]); assert!(!s.closed && !has(&s.text(), &esc_hint)); assert!(s.text().contains("No agent has been on this yet"), "{}", s.text());
    // any other key takes the hint away and does its own thing; with words typed ctrl+c is a first press too
    s.press(&["ctrl+c"]); assert!(has(&s.text(), &hint)); s.typed("x"); assert!(!has(&s.text(), &hint)); assert_eq!(s.s.value(), "x");
    s.press(&["ctrl+c"]); assert!(!s.closed); assert_eq!(s.s.value(), "x"); assert!(has(&s.text(), &hint));
    // esc puts the words away, and ctrl+c after it is a first press again
    s.press(&["esc", "ctrl+c"]); assert!(!s.closed); assert!(has(&s.text(), &hint), "{}", s.text());
    // in the menu the line says it, and ctrl+c there closes the menu, not the board
    s.press(&["?"]); assert!(s.s.menu.is_some()); assert!(has(&s.text(), &format!("{} +twice: close the board", regex::escape(&key_name("close")))));
    s.press(&["ctrl+c"]); assert!(s.s.menu.is_none() && !s.closed);
    // two seconds: time enough to read the line; then it goes, and ctrl+c after it only shows it again
    let t0 = s.s.now; s.press(&["ctrl+c"]); assert!(has(&s.text(), &hint));
    tick(&mut s, t0 + 1900); assert!(has(&s.text(), &hint), "two seconds: time enough to read the line");
    tick(&mut s, t0 + 2100); assert!(!has(&s.text(), &hint), "{}", s.text());
    s.press(&["ctrl+c"]); assert!(!s.closed); assert!(has(&s.text(), &hint));
    // esc again is not a yes and not a no: only ctrl+c closes, after esc's hint as after its own
    s.press(&["esc"]); assert!(!s.closed); assert!(has(&s.text(), &esc_hint), "{}", s.text());
    s.press(&["ctrl+c"]); assert!(s.closed);
    // ctrl+y was the yes key once: it asks nothing now, and types nothing
    let mut s2 = open(&w.root, 34); s2.press(&["ctrl+y"]); assert!(s2.s.ask.is_none()); assert_eq!(s2.s.value(), "");
    // ctrl+c twice closes with a draft for the Coordinator or for a task's agent, and the draft is kept
    let mut s3 = open(&w.root, 34); s3.typed("half a thought"); s3.press(&["ctrl+c"]); assert!(!s3.closed); assert!(has(&s3.text(), &hint));
    s3.press(&["ctrl+c"]); assert!(s3.closed); assert_eq!(s3.s.value(), "half a thought");
    let mut s4 = open(&w.root, 34); s4.s.sel = id.clone(); s4.load(); s4.press(&["space"]); assert!(s4.s.reply);
    s4.typed("to the agent"); s4.press(&["ctrl+c", "ctrl+c"]); assert!(s4.closed); assert_eq!(s4.s.value(), "to the agent");
    // a running Coordinator is not closed with the board, and the line says so; with none running it says nothing of it
    let keeps = "the Coordinator keeps running";
    let mut s5 = open(&w.root, 34); s5.s.data.coord = sw::coordinator::look::Look { state: "idle".into(), ..Default::default() };
    s5.press(&["ctrl+c"]); assert!(s5.text().contains(keeps), "{}", s5.text());
    // narrower, in shorter words, but never left out
    for (cols, said) in [(70, format!("{} again closes the board · the Coordinator stays", key_name("close"))), (50, format!("{} again closes · the Coordinator stays", key_name("close")))] {
        let mut n = open(&w.root, 34); n.cols = cols; n.s.data.coord = sw::coordinator::look::Look { state: "idle".into(), ..Default::default() };
        n.press(&["ctrl+c"]); assert!(n.text().contains(&said), "{cols} columns:\n{}", n.text());
    }
    let mut s6 = open(&w.root, 34); s6.s.data.coord = sw::coordinator::look::Look { state: "exited".into(), ..Default::default() };
    s6.press(&["ctrl+c"]); assert!(has(&s6.text(), &hint) && !s6.text().contains(keeps), "{}", s6.text());
}

//! Esc and the chat's latest message: with nothing else to step back from, esc hides the message as alt+c does,
//! before it says how the board closes (or, as a popup, closes it); a draft, find, the whole task and a filter come
//! first.
use crate::common;
use common::board::{open, open_with, Board};
use common::*;
use serde_json::json;
use sw::board::keys::key_name;

fn agent_chat(popup: bool) -> (World, Board) {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f, with one line in it." }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let mut s = if popup { open_with(&w.root, 34, true) } else { open(&w.root, 34) };
    s.s.side = false; s.s.sel = id; s.load(); s.press(&["space"]);
    if !s.s.chat_open { s.press(&["alt+c"]); }
    assert!(crate::board_chat::chat(&mut s, 120).framed, "{}", s.text());
    (w, s)
}
fn closing(s: &mut Board) -> bool { s.text().contains(&format!("{} closes the board", key_name("close"))) }

#[test]
fn esc_hides_the_latest_message_once_nothing_else_is_left_to_step_back_from() {
    let (_w, mut s) = agent_chat(false);
    s.press(&["esc"]);
    let c = crate::board_chat::chat(&mut s, 120);
    assert!(!c.framed && !s.s.chat_open, "{}", s.text());
    assert!(c.hints.contains(&format!("{} show the latest message", key_name("chatMessage"))), "{}", c.hints);
    assert!(!closing(&mut s), "hiding the message is the step: the board says nothing about closing yet");
    s.press(&["esc"]); assert!(closing(&mut s), "{}", s.text());
    assert!(!crate::board_chat::chat(&mut s, 120).framed, "esc never shows the message again");
    s.press(&["alt+c"]); assert!(crate::board_chat::chat(&mut s, 120).framed);
}

#[test]
fn a_draft_find_the_whole_task_and_a_filter_go_before_the_message() {
    let (_w, mut s) = agent_chat(false);
    s.typed("half a thought"); s.press(&["esc"]);
    assert!(crate::board_chat::chat(&mut s, 120).framed && s.s.value().is_empty(), "the draft is put away first: {}", s.text());
    s.press(&["ctrl+f"]); s.typed("Make"); crate::board_chat::chat(&mut s, 120); s.press(&["esc"]);
    assert!(s.s.chat_open && s.s.filter.is_empty(), "find is left first");
    s.press(&["ctrl+f"]); s.typed("Make"); s.press(&["enter"]); assert!(crate::board_chat::chat(&mut s, 120).framed, "{}", s.text());
    s.press(&["alt+t"]); s.text(); assert!(s.s.full); s.press(&["esc"]);
    assert!(!s.s.full && s.s.chat_open, "the whole task closes first");
    crate::board_chat::chat(&mut s, 120); s.press(&["esc"]);
    assert!(s.s.filter.is_empty() && crate::board_chat::chat(&mut s, 120).framed, "then the filter is cleared: {}", s.text());
    s.press(&["esc"]); assert!(!crate::board_chat::chat(&mut s, 120).framed && !closing(&mut s));
}

#[test]
fn as_a_popup_esc_hides_the_message_and_then_closes() {
    let (_w, mut s) = agent_chat(true);
    assert!(!crate::board_chat::chat(&mut s, 120).hints.contains("esc close"), "while the message shows esc does not close: {}", crate::board_chat::chat(&mut s, 120).hints);
    s.press(&["esc"]); assert!(!s.closed && !crate::board_chat::chat(&mut s, 120).framed);
    assert!(crate::board_chat::chat(&mut s, 120).hints.contains("esc close"), "{}", crate::board_chat::chat(&mut s, 120).hints);
    s.press(&["esc"]); assert!(s.closed);
}

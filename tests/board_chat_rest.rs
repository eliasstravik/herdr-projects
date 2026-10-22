//! What the old board-chat and board-reply tests checked that tests/board_chat.rs and tests/board_reply.rs do not:
//! while your just-sent words stand in the chat's box the task panel has the agent's message, and a reply still in the
//! text box when the board is closed from outside comes back with ctrl+z on its task.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;

/// What stands right of the list's bar: the side panel (the chat's box, which starts with its own edge, is not).
fn panel(s: &mut common::board::Board) -> String { s.screen(160).iter().filter(|l| !l.starts_with('│')).map(|l| l.split('│').nth(1).unwrap_or("").trim_end().to_string()).collect::<Vec<_>>().join("\n") }

#[test]
fn while_your_words_stand_in_the_box_the_task_panel_has_the_agents_message() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f, with one line in it." }, { "status": "working" }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 40);
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    assert_eq!(crate::board_chat::chat(&mut s, 160).boxed[0], "Made f, with one line in it.");
    assert!(!panel(&mut s).contains("Made f, with one line in it."), "the chat shows it: the panel does not say it twice:\n{}", panel(&mut s));
    s.typed("yes, g too"); s.press(&["enter"]);
    assert_eq!(crate::board_chat::chat(&mut s, 160).boxed[..1].to_vec(), vec!["yes, g too"]);
    assert!(panel(&mut s).contains("Made f, with one line in it."), "so the task panel has the agent's message meanwhile:\n{}", panel(&mut s));
}

#[test]
fn a_reply_still_in_the_text_box_when_the_board_is_closed_from_outside_comes_back_on_its_task() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Working\n" }] }));
    let id = w.add("Add a greeting"); w.add("Another one");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 34);
    s.s.sel = id.clone(); s.load(); s.press(&["space"]); s.typed("kept across a close");
    sw::board::run::prefs::save(&s.s); // what the board does when it is closed from outside (herdr closes its pane)
    let mut again = open(&w.root, 34);
    assert_eq!(again.s.value(), "", "a reply does not come back as a draft for the Coordinator");
    again.s.sel = id.clone(); again.load(); again.press(&["space", "ctrl+z"]);
    assert!(regex::Regex::new(r"╭─ [^\n]* PRJ-\d+  · Agent ─+╮\n(?:│ +│\n)?(?:[^\n]*\n)*?│ +❯ kept across a close").unwrap().is_match(&again.text()), "{}", again.text());
}

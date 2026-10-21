//! What the old board-layout test checked that tests/board_layout.rs does not: a longer message from the Coordinator
//! grows the chat block upwards into the list, its lines in the one rounded box, and the block stays on the window's
//! last line through a resize.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;

/// The chat block of this screen: the line that says who you talk to, and how many task rows the list above it has.
fn pinned(s: &mut Board, rows: usize, who: &str, what: &str) -> (usize, usize) {
    s.rows = rows;
    let lines = s.screen(120);
    let note = format!("{what}, {rows} rows");
    assert_eq!(lines.len(), rows, "{note}");
    let last = rows - 1;
    assert!(regex::Regex::new(r"^[ ▌] \S.* · ").unwrap().is_match(&lines[last]), "{note}: the hints are the last line: {:?}", lines[last]);
    assert_eq!(lines.iter().rposition(|l| regex::Regex::new(r"^(─+|╰─+╯)$").unwrap().is_match(l)).unwrap(), last - 1, "{note}: the text box sits right on the hints");
    let head = lines.iter().position(|l| l.starts_with(who)).unwrap_or_else(|| panic!("{note}: {}", lines.join("\n")));
    assert!(regex::Regex::new(r"^(─+|╭─.*╮)$").unwrap().is_match(&lines[head + 1]), "{note}: the box is right under that line");
    if rows >= 16 { assert_eq!(lines[head - 1].trim(), "", "{note}: an empty line sets the chat off from the list"); }
    for l in &lines[head..=last] { assert_ne!(l.trim(), "", "{note}: an empty line inside the chat block"); }
    let tasks = lines[..head].iter().filter(|l| l.contains("Task number")).count();
    // (the main Coordinator's row, on top, takes two lines of it: itself and the empty line under it)
    if rows >= 16 { assert!(tasks + 7 >= head, "{note}: {tasks} tasks in {head} lines"); }
    (head, tasks)
}

#[test]
fn a_longer_message_from_the_coordinator_grows_the_chat_block_upwards_and_it_stays_pinned() {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", w.base.join("home-coord")) };
    for i in 1..=230 { w.add(&format!("Task number {i}")); }
    let mut s = open(&w.root, 84); s.s.side = false;
    // The Coordinator, as the loader read it: idle, having said one line; then six.
    let says = |s: &mut Board, said: &str| { s.s.data.can_dispatch = true; s.s.chat_open = true; s.s.data.coord = sw::coordinator::look::Look { state: "idle".into(), readable: true, said: said.into(), ..Default::default() }; };
    says(&mut s, "Fine.");
    let short = pinned(&mut s, 84, " ○ Coordinator", "a one-line message");
    says(&mut s, &(1..=6).map(|i| format!("Line {i} of what it said.")).collect::<Vec<_>>().join("\n"));
    let long = pinned(&mut s, 84, " ○ Coordinator", "long message");
    assert_eq!(long.0, short.0 - 5, "its five more lines, taken from the list");
    let boxed: Vec<String> = s.screen(120)[long.0 + 1..long.0 + 11].iter().map(|l| regex::Regex::new(r" +│$").unwrap().replace(l, "").into_owned()).collect();
    let mut want = vec![format!("╭{}╮", "─".repeat(118)), "│ Coordinator:".to_string()];
    want.extend((1..=6).map(|i| format!("│ Line {i} of what it said.")));
    want.extend(["│".to_string(), "│ ❯ Send message to Coordinator".to_string()]);
    assert_eq!(boxed, want);
    for rows in [200, 30, 84] { pinned(&mut s, rows, " ○ Coordinator", "long message, resized"); }
    assert_eq!(pinned(&mut s, 84, " ○ Coordinator", "long message, back"), long);
}

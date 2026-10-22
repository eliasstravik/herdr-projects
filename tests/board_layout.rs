//! The whole screen: the chat block (the text box, which names who you talk to, what just happened, the hints) ends on the window's
//! last line and the list has every line above it, at any height, through a resize, while the block grows or shrinks.
use crate::common;
use common::board::{open, Board};
use common::*;

fn pinned(s: &mut Board, rows: usize, what: &str) -> (usize, usize) {
    s.rows = rows;
    let lines = s.screen(120);
    let note = format!("{what}, {rows} rows");
    assert_eq!(lines.len(), rows, "{note}");
    let last = lines.len() - 1;
    let said = if s.s.msg.is_empty() { 0 } else { 1 };
    assert!(regex::Regex::new(r"^[ ▌] \S.*( · |all keys)").unwrap().is_match(&lines[last]), "{note}: the hints are the last line: {:?}", lines[last]);
    let rule = lines.iter().rposition(|l| regex::Regex::new(r"^(─+|╰─+╯)$").unwrap().is_match(l)).unwrap();
    assert_eq!(rule, last - 1 - said, "{note}: the text box sits right on the hints");
    // The chat block starts with the text box's top edge.
    let head = lines.iter().position(|l| regex::Regex::new(r"^(─+|╭─.*╮)$").unwrap().is_match(l)).unwrap_or_else(|| panic!("{note}: no text box\n{}", lines.join("\n")));
    if rows >= 16 { assert_eq!(lines[head - 1].trim(), "", "{note}: an empty line sets the chat off from the list"); }
    for l in &lines[head..=last] { assert_ne!(l.trim(), "", "{note}: an empty line inside the chat block"); }
    let tasks = lines[..head].iter().filter(|l| l.contains("Task number")).count();
    if rows >= 16 { assert!(tasks + 7 >= head, "{note}: {tasks} tasks in {head} lines"); } // the header, the filter row, the top row and its group
    (head, tasks)
}

#[test]
fn the_chat_is_pinned_to_the_bottom_tall_resized_and_while_it_grows_or_shrinks() {
    let w = world();
    for i in 1..=230 { w.add(&format!("Task number {i}")); }
    let mut s = open(&w.root, 84); s.s.side = false;
    let tall = pinned(&mut s, 84, "tall");
    assert!(tall.1 > 70);
    for rows in [120, 200, 61, 84, 40, 24, 16, 12, 9, 58, 84] { pinned(&mut s, rows, "resized"); }
    assert_eq!(pinned(&mut s, 84, "back to tall"), tall);
    s.typed(&"word ".repeat(60));
    let draft = pinned(&mut s, 84, "wrapped draft");
    assert!(draft.0 < tall.0, "the box grows upwards");
    for rows in [200, 20, 84] { pinned(&mut s, rows, "wrapped draft, resized"); }
    s.s.flash("Something just happened.", sw::shown::ink::Ink::Yellow);
    pinned(&mut s, 84, "with a message"); pinned(&mut s, 140, "with a message, resized");
    s.s.msg.clear(); s.s.input.set_text("");
    assert_eq!(pinned(&mut s, 84, "shrunk back"), tall);
}

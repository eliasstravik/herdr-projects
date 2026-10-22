//! The text box: a draft wraps between words and the box grows with it up to half the window, then scrolls inside
//! it with the cursor in view; the arrows and editing keys work across wrapped lines; a long word is cut at the edge;
//! a paste keeps its line breaks and nothing in it acts as a key; a reply wraps in the same box under the agent's message.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;

/// A picture pasted: `[Image 1]` in the box, its path in what is sent.
mod images;
/// A picture in what was sent: `[Image 1]` in the chat too.
mod images_sent;

fn words(n: usize) -> String { (1..=n).map(|i| format!("word{i}")).collect::<Vec<_>>().join(" ") }
fn row(c: char) -> String { vec![c.to_string().repeat(4); 7].join(" ") }
/// The draft in the chat's rounded box, which has room to breathe (view/textbox.rs: chat_box): its text two cells in
/// from its edge (one in a box under 30 columns), an empty row under its top edge and over its bottom one.
fn framed_draft(s: &mut common::board::Board, cols: usize) -> common::board::Draft {
    let mut d = s.draft(cols);
    let pad = if cols >= 30 { 2 } else { 1 };
    let inner: Vec<String> = d.lines.iter().map(|l| if l.starts_with('│') && l.ends_with('│') { let c: Vec<char> = l.chars().collect(); c[1 + pad..c.len() - 1 - pad].iter().collect::<String>().trim_end().to_string() } else { l.clone() }).collect();
    let first = inner.iter().position(|l| l.starts_with("❯ ") || l == "❯").unwrap_or(0);
    let last = inner.iter().enumerate().position(|(n, l)| n > first && (l.is_empty() || l.starts_with('╰'))).unwrap_or(inner.len());
    d.rows = inner[first..last].iter().map(|l| l.chars().skip(2).collect::<String>().trim_end().to_string()).collect();
    let hidden = |l: Option<&String>| l.and_then(|l| regex::Regex::new(r"(\d+) more").unwrap().captures(l).map(|c| c[1].parse::<usize>().unwrap_or(0))).unwrap_or(0);
    (d.above, d.below) = (hidden(d.lines.get(first.wrapping_sub(1))), hidden(d.lines.get(last)));
    d.framed = true;
    d
}

#[test]
fn a_long_draft_wraps_between_words_and_the_box_grows_with_it() {
    let w = world(); w.add("Some task");
    let mut s = open(&w.root, 30);
    let all = words(40);
    s.typed(&all);
    for width in [40, 61, 120] {
        let d = s.draft(width);
        assert!(d.rows.len() > 1, "wraps at {width} columns");
        assert_eq!(d.rows.join(" "), all, "the whole draft is in view, cut only between words");
        assert_eq!(d.above + d.below, 0);
        assert_eq!(d.lines.len(), 30);
        assert!(d.lines.iter().all(|l| sw::text::width(l) <= width), "no line runs past the window");
        assert!(d.lines.last().unwrap().contains("enter add it as a task"));
        assert_eq!(d.cursor, (d.rows.len() - 1, d.rows.last().unwrap().chars().count()), "the cursor is after the last word");
    }
    assert_eq!(s.draft(40).rows.len(), 8);
    assert!(s.draft(40).lines[0].contains("herdr-projects"));
    s.press(&["esc"]); assert_eq!(s.draft(40).rows.len(), 1);
}

#[test]
fn the_box_stops_growing_at_half_the_window_and_scrolls_with_the_cursor_in_view() {
    let w = world(); w.add("Some task");
    let all = words(200);
    for (rows, most) in [(30usize, 15usize), (12, 6), (8, 5)] {
        let mut s = open(&w.root, rows);
        s.typed(&all);
        let d = s.draft(40);
        assert!(d.rows.len() <= most && d.rows.len() >= most.min(rows - 6), "{} rows of text in a window of {rows}", d.rows.len());
        assert_eq!(d.lines.len(), rows); assert!(d.lines.last().unwrap().contains("enter add it as a task"));
        assert!(d.above > 0, "the top border says how many lines are above"); assert_eq!(d.below, 0);
        assert!(d.rows.last().unwrap().ends_with("word200")); assert_eq!(d.cursor.0, d.rows.len() - 1);
        for _ in 0..(d.rows.len() + 2) { s.press(&["up"]); }
        let up = s.draft(40);
        assert_eq!((up.cursor.0, up.above, up.below, up.rows.len()), (0, d.above - 3, 3, d.rows.len()));
        for _ in 0..100 { s.press(&["up"]); }
        let top = s.draft(40);
        assert_eq!(top.above, 0); assert!(top.rows[0].starts_with("word1 word2 ")); assert_eq!(top.cursor.0, 0);
        assert_eq!(s.s.value(), all);
    }
}

#[test]
fn the_arrows_and_editing_keys_work_across_the_lines_of_a_wrapped_draft() {
    let w = world(); w.add("Some task");
    let mut s = open(&w.root, 30);
    s.typed(&[row('a'), row('b'), row('c')].join(" "));
    let d = s.draft(40);
    assert_eq!(d.rows, vec![row('a'), row('b'), row('c')]); assert_eq!(d.cursor, (2, 34));
    s.press(&["up"]); assert_eq!(s.draft(40).cursor, (1, 34));
    s.press(&["up"]); assert_eq!(s.draft(40).cursor, (0, 34));
    s.press(&["down"]); assert_eq!(s.draft(40).cursor, (1, 34));
    s.press(&["ctrl+a"]); assert_eq!(s.draft(40).cursor, (0, 0));
    for _ in 0..35 { s.press(&["right"]); }
    assert_eq!(s.draft(40).cursor, (1, 0));
    s.typed("X"); assert_eq!(s.s.value(), [row('a'), format!("X{}", row('b')), row('c')].join(" "));
    s.press(&["backspace", "backspace"]);
    assert_eq!(s.s.value(), [format!("{}{}", row('a'), row('b')), row('c')].join(" "));
    let d = s.draft(40);
    assert_eq!(d.rows, vec![row('a')[..29].to_string(), format!("aaaabbbb{}", &row('b')[4..29]), format!("bbbb {}", &row('c')[..29]), "cccc".to_string()]);
    assert_eq!(d.cursor, (1, 4));
    s.press(&["down", "down", "ctrl+e"]); s.typed(" end"); assert!(s.s.value().ends_with("cccc end"));
    assert_eq!(s.draft(40).rows.last().unwrap(), "cccc end");
    s.press(&["esc"]); s.typed("one"); s.press(&["shift+enter"]); s.typed(&format!("{} {}", row('d'), row('e')));
    assert_eq!(s.s.value(), format!("one\n{} {}", row('d'), row('e')));
    assert_eq!(s.draft(40).rows, vec!["one".to_string(), row('d'), row('e')]);
}

#[test]
fn a_word_longer_than_the_box_is_cut_at_its_edge_and_a_paste_wraps_like_typed_text() {
    let w = world(); w.add("Some task");
    let mut s = open(&w.root, 30);
    let url = format!("https://example.com/{}", "x".repeat(100));
    s.typed(&format!("see {url} now"));
    let d = s.draft(40);
    assert_eq!(d.rows.join("").replace(' ', ""), format!("see{url}now"), "every character of the long word shows");
    assert!(d.lines.iter().all(|l| sw::text::width(l) <= 40)); assert_eq!(d.lines.len(), 30); assert_eq!(d.above + d.below, 0);
    assert_eq!(d.cursor, (d.rows.len() - 1, d.rows.last().unwrap().chars().count()));
    s.press(&["up"]); assert_eq!(s.draft(40).cursor.0, d.rows.len() - 2);
    s.press(&["esc"]);
    // A paste: its own line breaks are kept, each of its lines wraps, a tab is four spaces, and nothing in it acts as a key.
    s.paste(&format!("First line of the paste, long enough to need a second row in the box.\n\tindented\n{}\x01\x18", "y".repeat(90)));
    assert_eq!(s.s.value(), format!("First line of the paste, long enough to need a second row in the box.\n    indented\n{}", "y".repeat(90)));
    let d = s.draft(40);
    assert_eq!(d.rows[..3].to_vec(), vec!["First line of the paste, long enough", "to need a second row in the box.", "    indented"]);
    assert_eq!(d.rows[3..].join(""), "y".repeat(90));
    s.paste(&"line\n".repeat(60));
    let d = s.draft(40);
    assert_eq!(d.rows.len(), 15); assert!(d.above > 40); assert_eq!(d.lines.len(), 30); assert!(d.lines.last().unwrap().contains("enter add it as a task"));
}

#[test]
fn a_reply_to_a_tasks_agent_wraps_in_the_same_box_under_its_latest_message() {
    let w = world();
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }], "reviewer": [{ "verdict": { "meets": true, "text": "Good.", "findings": [{ "severity": "minor", "subject": "f.txt has no title", "where": "f.txt" }] } }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 24);
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    assert!(s.s.reply);
    let all = words(60);
    s.typed(&all);
    let d = framed_draft(&mut s, 50);
    assert_eq!(d.rows.join(" "), all); assert!(d.rows.len() > 5); assert_eq!(d.above + d.below, 0);
    assert_eq!(d.lines.len(), 24); assert!(d.lines.iter().all(|l| sw::text::width(l) <= 50));
    assert!(d.lines.iter().any(|l| regex::Regex::new(r"^╭─ .*PRJ-\d+  · Agent ─*╮$").unwrap().is_match(l)), "who the reply goes to still shows, on the box's edge: {:?}", d.lines);
    assert!(d.framed); assert!(d.lines.iter().any(|l| regex::Regex::new(r"^│ +Made f\. +│$").unwrap().is_match(l)), "the agent's latest message is in the same box: {:?}", d.lines);
    s.typed(&format!(" {all} {all}"));
    let e = framed_draft(&mut s, 50);
    assert_eq!(e.lines.len(), 24); assert!(e.above > 0); assert_eq!(e.below, 0); assert!(e.lines.last().unwrap().contains("enter send to PRJ-1's agent"));
}

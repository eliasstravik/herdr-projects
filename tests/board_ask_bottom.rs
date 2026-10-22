//! The question box (merge, remove, …) ends on the window's last line, the way the chat's box sits on its hints: no
//! empty line under it, at any height, in the popup too; in a short window its detail gives way and its keys stay.
use crate::common;
use common::board::{open_with, Board};
use common::*;
use serde_json::json;

/// The screen at `rows`: the box's closing rule is the last line, and the question and its keys are on screen.
fn pinned(s: &mut Board, rows: usize, q: &str, what: &str) -> Vec<String> {
    s.rows = rows;
    let lines = s.screen(100);
    let note = format!("{what}, {rows} rows:\n{}", lines.join("\n"));
    assert_eq!(lines.len(), rows, "{note}");
    assert!(lines.iter().any(|l| l.contains(q)), "{note}");
    assert!(lines.iter().any(|l| l.contains(" = ")), "{note}: the keys are in view");
    if rows >= 6 { assert!(regex::Regex::new("^─+$").unwrap().is_match(lines.last().unwrap()), "{note}: the box's rule is the last line"); }
    lines
}

#[test]
fn every_question_box_ends_on_the_last_line_at_any_height_and_in_the_popup() {
    let w = world();
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }], "reviewer": [{ "verdict": { "meets": true, "text": "Good.", "findings": [] } }] }));
    let id = w.add("Make f"); w.add("Other");
    delegate(&id); settle(&id);
    for popup in [false, true] {
        let mut s = open_with(&w.root, 30, popup);
        s.s.sel = id.clone(); s.load();
        let before = s.screen(100);
        s.press(&["ctrl+l"]);
        for rows in [30, 60, 16, 10, 8, 6, 5, 3] { pinned(&mut s, rows, "Merge PRJ-1", &format!("merge, popup {popup}")); }
        // The box (five lines) takes the place of the text box and its hints (four): the list gives up one line.
        s.rows = 30;
        let tall = s.screen(100);
        let at = |l: &[String]| l.iter().position(|x| x.starts_with('─')).unwrap();
        assert_eq!((at(&before), at(&tall)), (26, 25), "{}\n{}", before.join("\n"), tall.join("\n"));
        assert_eq!(&tall[tall.len() - 5..], &["─".repeat(100), " ? Merge PRJ-1 \"Make f\" into main?".into(), "   A merge cannot be undone.".into(), "   y = merge · n = do not merge".into(), "─".repeat(100)].map(|l| format!("{l:<100}")).map(|l| l.trim_end().to_string()), "{}", tall.join("\n"));
        // The short window: the detail gives way before the question and its keys.
        let short = pinned(&mut s, 4, "Merge PRJ-1", "merge, short");
        assert!(!short.iter().any(|l| l.contains("cannot be undone")), "{}", short.join("\n"));
        s.press(&["n"]);
        // Answered, the hints are back on the last line.
        s.rows = 30;
        assert!(s.screen(100).last().unwrap().contains("approve and merge this task"));
        // Another question in the same place: removing a task.
        s.s.sel = "proj/2".into(); s.load(); s.press(&["ctrl+x"]);
        assert!(s.s.ask.is_some(), "{}", s.text());
        let q = s.s.ask.as_ref().unwrap().q.chars().take(20).collect::<String>();
        for rows in [30, 12, 8, 6] { pinned(&mut s, rows, &q, &format!("remove, popup {popup}")); }
        s.press(&["n"]);
    }
}

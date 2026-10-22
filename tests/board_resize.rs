//! A window resized through many sizes: the screen is drawn to its last row, the hints last with the one rounded box
//! right above them (who it talks to, the coordinator's message, an empty line, the line you type on), and a working row's columns at
//! the window's right edge however wide it is; under 80 columns they are on the row's third line.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::text::width;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }

/// The old test drove pi-tui's real renderer through a resize; the Rust board's screen is a pure function of its size,
/// so each size is drawn by the same function the terminal loop calls on a resize.
#[test]
fn resizing_keeps_the_chat_on_the_last_line_and_the_working_rows_columns_at_the_right_edge() {
    let w = world();
    let first = w.add("Pin the chat to the bottom");
    for i in 2..=150 { w.add(&format!("Task number {i}")); }
    // One task an agent works on: the bar, the percent, the minutes left, the PR number and the time are its columns.
    sw::actions::filing::report(&first, sw::actions::filing::Report { state: "working".into(), msg: "Checking every window size.".into(), progress: Some(json!(55)), eta: Some(json!(12)), ..Default::default() }).unwrap();
    let at = sw::clock::now() - 6 * 60000;
    sw::store::tasks::update(&first, |t| { t.put("pr", 3); t.put("at", at); t.put("since", at); Ok(sw::store::tasks::Change::Write) }).unwrap();
    let mut s = open(&w.root, 34); s.s.side = false;
    // The Coordinator, as the loader read it: idle, having said this.
    s.s.data.can_dispatch = true; s.s.chat_open = true;
    s.s.data.coord = sw::coordinator::look::Look { state: "idle".into(), readable: true, said: "Tasks 31 and 32 are both waiting.".into(), ..Default::default() };
    for (columns, rows) in [(120, 34), (190, 84), (435, 58), (412, 78), (50, 22), (50, 84), (435, 22), (300, 120), (40, 12), (435, 58), (120, 34)] {
        s.rows = rows; s.cols = columns;
        let lines = s.lines();
        let note = format!("{columns}x{rows}");
        assert_eq!(lines.len(), rows, "{note}");
        for l in &lines { assert!(width(l) <= columns, "{note}: a line is wider than the window: {l:?}"); }
        // The bottom: the hints on the last row, and right above them the one rounded box, as wide as the window.
        assert!(has(&lines[rows - 1], r"^[ ▌] {1}\S") && !lines[rows - 1].trim_end().ends_with('·'), "{note}: {:?}", lines[rows - 1]);
        let in_box = |n: usize| { let l = &lines[n]; assert!(has(l, r"^│ .* │$"), "{note}: {l:?}"); assert_eq!(width(l), columns, "{note}"); let c: Vec<char> = l.chars().collect(); c[2..c.len() - 2].iter().collect::<String>().trim().to_string() };
        // The box's top edge names who you talk to (chat_title.rs), and its first line says who that is.
        let head = lines.iter().position(|l| has(l, r"^╭─.*╮$")).unwrap_or_else(|| panic!("{note}: {}", lines.join("\n")));
        assert_eq!(lines[rows - 2], format!("╰{}╯", "─".repeat(columns - 2)), "{note}\n{}", lines.join("\n"));
        // The line you type on; where the window has the height, an empty row under it and one under the edge.
        let input = if rows < 14 { rows - 3 } else { rows - 4 };
        assert!(in_box(input).starts_with("❯ "), "{note}");
        // Under 14 rows the chat folds to the box's edge (no first line), so the list has the lines. (The old test's
        // stand-in for the chat said it must always show, which only a Coordinator that could not start does.)
        if rows < 14 {
            assert_eq!(head, rows - 4, "{note}\n{}", lines.join("\n"));
        } else {
            assert_eq!(in_box(rows - 3), "", "{note}"); assert_eq!(in_box(rows - 5), "", "{note}"); assert_eq!(in_box(head + 1), "", "{note}");
            // (Under the edge that names it and its empty row, its message alone.)
            let said: Vec<String> = (head + 2..rows - 5).map(in_box).collect();
            assert_eq!(said.join(" ").trim(), "Tasks 31 and 32 are both waiting.", "{note}");
        }
        // Who you talk to: one machine's own coordinator, its name on the box's edge. No keys, no program.
        let me = sw::board::update::heads::label(&s.s, "");
        assert!(lines[head].contains(&format!(" {me} ")) || (columns < 60 && lines[head].starts_with("╭─")), "{note}: {:?}", lines[head]);
        // The working row: its columns sit at the window's right edge, however wide the window is.
        let row = lines.iter().position(|l| l.contains("Pin the chat to the bottom"));
        if rows >= 20 { assert!(row.is_some(), "{note}: the working row is in view"); }
        if let Some(n) = row {
            let l = &lines[n];
            if columns >= 120 {
                // (At 120 the row stands a step in under the machine's row on top, and its time gives way first.)
                assert!(has(l, r"Pin the chat to the bottom +█+░+ +55% 12m left {2}PR#3( {2}6m {4})? *$"), "{note}: {l:?}"); assert_eq!(width(l), columns, "{note}");
                if columns >= 130 { assert!(l.trim_end().ends_with("PR#3  6m"), "{note}: {l:?}"); }
                assert!(has(&lines[n + 1], r"^[ ▌] {10,12}Checking every window size\. *$"), "{note}: {:?}", lines[n + 1]);
            } else {
                // Under 80 columns its first line is the name alone: the columns are on its third line, the bar filling the width.
                assert!(has(l.trim_end(), r"^[ ▌] ◐ PRJ-\d+ +Pin the chat to the bottom$"), "{note}: {l:?}");
                if rows >= 20 { let third = lines[n + 2].trim_end(); assert!(has(third, r"^[ ▌] {9,11}█+░+ 55% 12m left {2}PR#3 {2}6m$"), "{note}: {third:?}"); assert_eq!(width(third), columns, "{note}"); }
            }
        }
        // All spare height is the list's (a narrow screen's working row is one line taller). The old test made the row
        // a working one after the list was grouped, so it stood under "to do": here it is under its own "working"
        // heading, two lines more; the machine's row on top one more, the filter row one more.
        if rows >= 20 { let tasks = lines[..head].iter().filter(|l| has(l, r"Task number \d+")).count(); assert!(tasks + if columns < 80 { 13 } else { 12 } >= head, "{note}: the list fills the space above the chat: {tasks} in {head}\n{}", lines.join("\n")); }
    }
}

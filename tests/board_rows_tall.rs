//! A live task's row on a narrow screen: its third line (the bar, the percent, the minutes left, the review tag, the
//! pull request, the time), the width where it switches, what gives way in a width too small for all of it, and another
//! machine's name taking nothing from it. Its highlight, scrolling and the fold: board_rows_tall_list.rs.
use regex::Regex;
use serde_json::{json, Value};
use std::sync::Arc;
use sw::board::state::{Rec, State};
use sw::board::theme::Theme;
use sw::board::update::list::fold_rows;
use sw::board::view::list::list_lines;
use sw::board::view::row::{row_layout, row_lines, Layout};
use sw::board::view::row_tall::TALL_BELOW;
use sw::json::*;
use sw::shown::ink::{Ink, Inked};
use sw::shown::states::{group_of, GROUPS};

const NOW: i64 = 1_800_000_000_000;
const AT: i64 = NOW - 2 * 3600000; // "2h": the only "…m" on a row is then its minutes left
fn has(text: &str, re: &str) -> bool { Regex::new(re).unwrap().is_match(text) }
fn rec(base: Value, more: Value) -> Rec { let mut o = obj(base); for (k, v) in more.as_object().unwrap() { o.insert(k.clone(), v.clone()); } Arc::new(o) }
fn work(more: Value) -> Rec { rec(json!({ "state": "working", "stage": "working", "agent": "claude", "at": AT }), more) }
fn ready(more: Value) -> Rec { rec(json!({ "state": "blocked", "kind": "permission", "stage": "approval", "agent": "claude", "at": AT }), more) }
fn pass() -> Value { json!({ "meets": true, "gate": "major", "pass": true, "text": "Works.", "findings": [{ "id": "F1", "severity": "minor", "subject": "A name is unclear", "where": "a.js", "proof": "" }] }) }
fn agent() -> Rec { work(json!({ "id": "p/1", "title": "Agent at work", "msg": "Reading the panel and its tests.", "progress": 40, "eta": 12 })) }
fn review() -> Rec { work(json!({ "id": "p/2", "title": "Under review with a name that is long enough to be cut", "stage": "reviewing", "reviewRound": 1, "msg": "Running the tests", "progress": 70, "eta": 3, "pr": 17 })) }
fn waits() -> Rec { ready(json!({ "id": "p/3", "title": "Good work", "review": pass(), "reviewRound": 1, "pr": 18 })) }
fn fresh() -> Rec { work(json!({ "id": "p/4", "title": "Brand new", "msg": "Just started" })) }
fn live() -> Vec<Rec> { vec![agent(), review(), waits(), fresh()] }
fn board(list: &[Rec], sel: &str, scope: &str) -> State {
    sw::clock::set(Some(NOW));
    let mut s = State::new(Default::default(), Default::default());
    s.sel = sel.into(); s.scope = scope.into();
    s.grouped = GROUPS.iter().map(|g| (g.to_string(), list.iter().filter(|r| group_of(r) == *g).cloned().collect())).collect();
    s.rows = fold_rows(&mut s);
    s
}
fn t() -> Theme { Theme::default_dark(false) }
fn lay(s: &mut State, list: &[Rec], width: usize, screen: usize) -> Layout { row_layout(s, &t(), list, width, &[list.to_vec()], screen) }
fn inked(s: &State, r: &Rec, width: usize, l: &Layout) -> Vec<Inked> { row_lines(s, &t(), r, width, l) }
fn plain(s: &State, r: &Rec, width: usize, l: &Layout) -> Vec<String> { inked(s, r, width, l).iter().map(|x| x.text()).collect() }
fn len(l: &str) -> usize { l.chars().count() }
fn idx(l: &str, p: &str) -> usize { l.find(p).map(|b| l[..b].chars().count()).unwrap() }
fn run_of(l: &str, re: &str) -> usize { Regex::new(re).unwrap().find(l).map(|m| m.as_str().chars().count()).unwrap_or(0) }

#[test]
fn narrow_screen_a_live_rows_third_line_has_the_bar_the_percent_the_minutes_left_the_review_tag_the_pull_request_and_the_time() {
    let all = live();
    let mut s = board(&all, "", "");
    for width in [40, 44, 48, 50, 55, 79] {
        let l = lay(&mut s, &all, width, width);
        let mut rows: Vec<Vec<String>> = all.iter().map(|r| plain(&s, r, width, &l)).collect();
        let note = format!("at {width}:\n{}", rows.concat().join("\n"));
        // A name longer than its line wraps onto a second one: that row is one line taller.
        let long = if width < 6 + review().s("title").len() { 1 } else { 0 };
        // A row with nothing for a third line but its time has the time at the end of what it says, and no line for it alone.
        for (n, lines) in rows.iter().enumerate() {
            assert_eq!(lines.len(), if n == 1 { 3 + long } else if n == 3 { 2 } else { 3 }, "{note}");
            assert!(lines.iter().all(|x| len(x) <= width), "{note}"); assert_eq!(len(lines.last().unwrap()), width, "{note}");
        }
        // Line 1 is the mark, the task's number and the name with the whole width: nothing beside it.
        assert_eq!(rows[0][0].trim_end(), " ◐ #1 Agent at work"); assert_eq!(rows[2][0].trim_end(), " ◆ #3 Good work"); assert_eq!(rows[3][0].trim_end(), " ◐ #4 Brand new");
        assert_eq!(rows[1][..1 + long].iter().map(|x| x.trim().trim_start_matches("◑ #2 ").to_string()).collect::<Vec<_>>().join(" "), review().s("title"), "{note}");
        // Then what the task says, as on a wide screen.
        let said: Vec<String> = rows[..3].iter().map(|x| x[x.len() - 2].trim_end().to_string()).collect();
        assert_eq!(said, ["      Reading the panel and its tests.", "      Running the tests", "      Pass · 1 minor"]);
        assert!(has(rows[3].last().unwrap(), r"^ {6}Just started +2h$"), "{note}");
        for x in rows.iter_mut() { let last = x.last().unwrap().clone(); if x.len() > 2 { x[2] = last; } else { x.push(last); } }
        // Line 3, in order. The bar takes the room the others leave, so it is as long on every row.
        let bar = run_of(&rows[0][2], "[█░]+"); assert!(bar >= 4, "{note}");
        assert!(has(&rows[0][2], r"^ {6}█+░+ 40% 12m left +2h$") && has(&rows[1][2], r"^ {6}█+░+ 70% {2}3m left {2}R#1 PR#17 +2h$"), "{note}");
        for n in [0, 1] { assert_eq!(run_of(&rows[n][2], "[█░]+"), bar, "{note}"); }
        assert_eq!(run_of(&rows[0][2], "█+"), (0.4 * bar as f64).round() as usize, "{note}");
        assert!(has(&rows[2][2], r"^ +R#1 PR#18 +2h( ago)?$"), "no progress: the bar's room is empty: {note}");
        assert_eq!(idx(&rows[1][2], "R#1"), idx(&rows[2][2], "R#1"), "the tags start under each other: {note}");
    }
    // Their usual colours: the agent's amber, the reviewer's colour, a passed review's green, the time greyed.
    let l = lay(&mut s, &all, 48, 48);
    let third = |r: &Rec| inked(&s, r, 48, &l).last().unwrap().clone();
    let ink = |x: &Inked, w: &str| x.0.iter().find(|p| p.t.contains(w)).map(|p| p.c);
    let a = third(&agent());
    assert_eq!((ink(&a, "█"), ink(&a, "40%"), ink(&a, "12m")), (Some(Ink::Yellow), Some(Ink::Yellow), Some(Ink::Yellow)));
    let r = third(&review()); assert_eq!((ink(&r, "█"), ink(&r, "R#1")), (Some(Ink::Review), Some(Ink::Review)));
    assert_eq!(ink(&third(&waits()), "R#1"), Some(Ink::Green)); assert!(third(&fresh()).text().ends_with("2h"));
}

#[test]
fn the_third_line_is_there_under_80_columns_of_screen_and_gone_from_80_on_for_every_live_row_at_once() {
    let quiet = vec![rec(json!({ "state": "idle", "stage": "todo", "at": AT, "id": "p/8", "title": "Not begun" }), json!({})), rec(json!({ "state": "done", "stage": "done", "at": AT, "id": "p/9", "title": "Finished", "pr": 3, "reviewRound": 1, "review": pass() }), json!({}))];
    let all: Vec<Rec> = live().into_iter().chain(quiet).collect();
    let mut s = board(&all, "", "");
    assert_eq!(TALL_BELOW, 80);
    let heights = |s: &mut State, width: usize, screen: usize| { let l = lay(s, &all, width, screen); all.iter().map(|r| row_lines(s, &t(), r, width, &l).len()).collect::<Vec<_>>() };
    // (The fourth has only a time for its third line: it goes on the second, and the row is two lines.)
    for width in [60, 79] { assert_eq!(heights(&mut s, width, width), [3, 3, 3, 2, 1, 1], "at {width}"); }
    // Narrower, the names and words that do not fit their line wrap: no live row with columns is under three.
    for width in [30, 40, 55] { let tall = heights(&mut s, width, width); assert!(tall[..3].iter().all(|n| *n >= 3) && tall[3] >= 2, "at {width}"); assert_eq!(tall[4..], [1, 1], "at {width}"); }
    for width in [80, 81, 120, 200] { assert_eq!(heights(&mut s, width, width), [2, 2, 2, 2, 1, 1], "at {width}"); }
    // It is the screen that counts: a list made narrow by the task panel beside it, on a wide screen, is as it was.
    assert_eq!(heights(&mut s, 60, 120), [2, 2, 2, 2, 1, 1]);
    // The list: headings and one-line rows as ever, a one-line row's tags still beside its name. Its time is not: on a
    // narrow screen its name has that room.
    // (Drawn a step wider and that step taken off: the rows stand a step in under the top row, update/depth.rs.)
    let list = |s: &mut State, w: usize| { let k = if w + 2 >= 60 { 2 } else { 1 }; list_lines(s, &t(), w + k, 40, w + k).iter().map(|l| l.text().chars().skip(k).collect::<String>()).collect::<Vec<_>>() };
    let (narrow, wide) = (list(&mut s, 60), list(&mut s, 80));
    let count = |v: &[String]| v.iter().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(count(&narrow) - count(&wide), 3, "one more line for each live row that has columns for it: three of the four");
    let find = |v: &[String], w: &str| v.iter().find(|l| l.contains(w)).cloned().unwrap_or_default();
    assert!(has(&find(&narrow, "Finished"), r"^[ ▌]✓ #\d+ Finished +R#1 PR#3$") && has(&find(&narrow, "Not begun"), r"^[ ▌]○ #\d+ Not begun *$"), "{}", narrow.join("\n"));
    assert!(has(&find(&wide, "Finished"), r"^[ ▌]✓ #\d+ Finished +R#1 PR#3 +2h$") && has(&find(&wide, "Not begun"), r"^[ ▌]○ #\d+ Not begun +2h$"), "{}", wide.join("\n"));
    // At 80 the columns sit beside a name of 40 cells: the widest of each kind, bar and all. The task's number is not
    // counted in the 80: with it that is 83, and at 80 the bar is what gives way.
    let full = work(json!({ "id": "p/1", "title": "A".repeat(40), "msg": "Reading the panel and its tests.", "progress": 40, "eta": 120, "reviewRound": 12, "stage": "reviewing", "pr": 123 }));
    let one = std::slice::from_ref(&full);
    let l = lay(&mut s, one, 88, 88); assert!(has(&plain(&s, &full, 88, &l)[0], r"^[ ▌]◑ #1 A{40} {3}[█░]{8} 40% 120m left {2}R#12 PR#123 {2}2h$"), "{:?}", plain(&s, &full, 88, &l));
    let l = lay(&mut s, one, 80, 80); assert!(has(&plain(&s, &full, 80, &l)[0], r"^[ ▌]◑ #1 A{40} +40% 120m left {2}R#12 PR#123 {2}2h$"), "{:?}", plain(&s, &full, 80, &l));
}

#[test]
fn in_a_width_too_small_for_all_of_the_third_line_the_time_stays_and_nothing_runs_off_the_edge() {
    let all = live();
    let mut s = board(&all, "", "");
    for width in [14, 20, 24, 30, 36] {
        let l = lay(&mut s, &all, width, width);
        for r in &all { let lines = plain(&s, r, width, &l); assert!(lines.iter().all(|x| len(x) <= width), "at {width}"); assert!(has(lines.last().unwrap(), r" 2h( ago)?$"), "at {width}: {:?}", lines.last()); }
    }
    let third = |s: &mut State, width: usize| { let l = lay(s, &all, width, width); plain(s, &review(), width, &l).last().unwrap().clone() };
    assert!(has(&third(&mut s, 40), r"^ {6}[█░]+ 70% {2}3m left {2}R#1 PR#17 {2}2h$"), "the word \"ago\" goes for a bar that can be read: {}", third(&mut s, 40));
    assert!(has(&third(&mut s, 33), r"^ {6}[█░]+ 70% {2}3m left {3}2h$"), "then the tags: {}", third(&mut s, 33));
    assert!(has(&third(&mut s, 26), r"^ +70% {2}3m left {3}2h$"), "then the bar, where it would be too short to read: {}", third(&mut s, 26));
    assert!(has(&third(&mut s, 20), r"^ {3}[█░]+ {3}2h$"), "then the percent and the minutes: {}", third(&mut s, 20));
    assert!(has(&third(&mut s, 14), r"^ {3}[█░]+ {3}2h$"), "{}", third(&mut s, 14));
}

#[test]
fn all_machines_narrow_screen_a_machines_name_however_long_takes_nothing_from_the_third_line() {
    let mut s = board(&[], "", "*");
    let far = work(json!({ "id": "p/1", "title": "Fix login", "stage": "reviewing", "msg": "Running the tests", "progress": 50, "eta": 12, "reviewRound": 12, "pr": 123, "host": "very-long-machine-name-on-the-tailnet" }));
    let near = rec(Value::Object((*far).clone()), json!({ "id": "p/2", "host": "" }));
    let short = rec(Value::Object((*far).clone()), json!({ "id": "p/3", "host": "box" }));
    let list = vec![far, near, short];
    for width in [46, 48, 50, 55, 79] {
        let l = lay(&mut s, &list, width, width);
        let rows: Vec<Vec<String>> = list.iter().map(|r| plain(&s, r, width, &l)).collect();
        let note = format!("at {width}:\n{}", rows.concat().join("\n"));
        for lines in &rows { assert_eq!(lines.len(), 3, "{note}"); assert!(lines.iter().all(|x| len(x) <= width), "{note}"); }
        // Every field is there, the same line on each row whatever its machine: the bar as long, the time at the edge.
        assert!(has(&rows[0][2], r"^ +█+░+ 50% 12m left {2}R#12 PR#123 {2}2h$"), "{note}"); assert_eq!(len(&rows[0][2]), width, "{note}");
        assert_eq!(rows[1][2], rows[0][2], "{note}"); assert_eq!(rows[2][2], rows[0][2], "{note}");
        assert!(rows[0][0].contains("Fix login"), "the name wins over the machine's on line 1: {note}");
    }
}

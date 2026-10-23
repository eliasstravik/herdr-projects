//! One row of the list: its name, what it says, its columns on the right, in what order they give way as the window
//! narrows; the minutes left and a row that never names its machine; and a row with no PR and no bar, whose empty
//! cells are empty (never colour around nothing), so its time sits at the right edge.
use regex::Regex;
use serde_json::{json, Value};
use std::sync::Arc;
use sw::board::state::{Rec, State};
use sw::board::theme::Theme;
use sw::board::view::row::{row, row_layout, row_lines, Layout};
use sw::json::*;
use sw::shown::ink::{Ink, Inked};

/// Each row measured once while a long list folds to its room.
mod measured;

const NOW: i64 = 1_800_000_000_000;
fn has(text: &str, re: &str) -> bool { Regex::new(re).unwrap().is_match(text) }
fn rec(v: Value) -> Rec { Arc::new(obj(v)) }
fn with(r: &Rec, more: Value) -> Rec { let mut o = (**r).clone(); for (k, v) in more.as_object().unwrap() { o.insert(k.clone(), v.clone()); } Arc::new(o) }
fn board() -> (State, Theme) { sw::clock::set(Some(NOW)); (State::new(Default::default(), Default::default()), Theme::default_dark(false)) }
fn lay(s: &mut State, t: &Theme, list: &[Rec], width: usize, screen: usize) -> Layout { row_layout(s, t, list, width, &[list.to_vec()], screen) }
fn idx(line: &str, pat: &str) -> i64 { line.find(pat).map(|b| line[..b].chars().count() as i64).unwrap_or(-1) }
fn len(l: &str) -> usize { l.chars().count() }
/// The ink a word is drawn in on a line.
fn ink_of(l: &Inked, word: &str) -> Option<Ink> { l.0.iter().find(|p| p.t.contains(word)).map(|p| p.c) }

#[test]
fn a_tight_row_keeps_the_tasks_name_then_the_columns_on_the_right_what_a_live_task_says_is_on_a_line_of_its_own() {
    let (mut s, t) = board();
    let at = NOW - 6 * 60000;
    let mut records = vec![
        rec(json!({ "id": "proj/1", "state": "working", "title": "Remove notification pop-ups", "msg": "Removing pop-up calls from plugin, fixing tests, writing closing note.", "progress": 95, "at": at })),
        rec(json!({ "id": "proj/2", "state": "working", "title": "Fix ugly blocked row on the board", "msg": "Fixing layout so approval names keep width.", "progress": 55, "pr": 3, "at": at })),
        rec(json!({ "id": "proj/3", "state": "done", "title": "Truncate long descriptions in sidebar", "pr": 2, "at": at })),
    ];
    let draw = |s: &mut State, records: &[Rec], width: usize| { let l = lay(s, &t, records, width, 200); records.iter().map(|r| row(s, &t, r, width, &l).text()).collect::<Vec<_>>() };
    for width in [200, 120, 90, 72, 64, 50, 40, 30, 20] { assert!(draw(&mut s, &records, width).iter().all(|l| len(l) <= width), "fits {width}"); }
    let rows = draw(&mut s, &records, 200);
    assert!(has(&rows[0], r"Remove notification pop-ups +█+ +95% {8}6m {4}$"), "{rows:?}");
    assert!(has(&rows[1], r"Fix ugly blocked row on the board +█+░+ +55% {2}PR#3 {2}6m {4}$"), "{rows:?}");
    {
        let under = |s: &mut State, width: usize| { let l = lay(s, &t, &records[..1], width, 200); row_lines(s, &t, &records[0], width, &l)[1].text() };
        assert_eq!(under(&mut s, 200), "      Removing pop-up calls from plugin, fixing tests, writing closing note."); assert_eq!(under(&mut s, 80), under(&mut s, 200));
        let u50 = under(&mut s, 50);
        assert!(has(&u50, r"^[ ▌] {5}Removing pop-up calls from plugin, fixing[^…]*…$"), "{u50:?}"); assert_eq!(len(&u50), 50);
        let l = lay(&mut s, &t, &records[2..3], 200, 200);
        assert_eq!(row_lines(&s, &t, &records[2], 200, &l).len(), 1, "a finished task has nothing to say: one line");
    }
    assert!(has(&rows[2], r"Truncate long descriptions in sidebar +PR#2 {2}6m ago$"), "\"ago\" on what has ended, a bare time on what still runs");
    assert_eq!(idx(&rows[1], "PR#3"), idx(&rows[2], "PR#2"), "the PR numbers sit in one column");
    let rows = draw(&mut s, &records, 75);
    assert!(has(&rows[0], r"Remove notification pop-ups +\S.* 95% +6m {4}$") && !has(&rows[0], r"Remov[^ ]* {2}Re"), "{rows:?}");
    assert!(has(&rows[1], r"Fix ugly blocked row on the board +█+░+ +55% {2}PR#3 {2}6m {4}$"), "{rows:?}");
    let rows = draw(&mut s, &records, 59);
    assert!(has(&rows[0], r"Remove notification pop-ups +95% +6m {4}$"), "{rows:?}");
    assert!(has(&rows[1], r"Fix ugly blocked row on the board +55% {2}PR#3 +6m {4}$"), "{rows:?}");
    assert!(has(&rows[2], r"Truncate long descriptions in sidebar +PR#2 +6m ago$"), "{rows:?}");
    {
        let pair = [rec(json!({ "id": "proj/5", "state": "working", "title": "Still going", "at": at })), rec(json!({ "id": "proj/6", "state": "done", "title": "All done", "at": at }))];
        let l = lay(&mut s, &t, &pair, 50, 200);
        let (going, ended) = (row(&s, &t, &pair[0], 50, &l).text(), row(&s, &t, &pair[1], 50, &l).text());
        assert!(has(&going, r"^[ ▌]◐ #\d+ Still going +6m {4}$") && has(&ended, r"^[ ▌]✓ #\d+ All done +6m ago$"), "{going:?} {ended:?}");
    }
    let rows = draw(&mut s, &records, 48);
    assert!(has(&rows[0], r"Remove notification pop-ups +6m$") && !rows[0].contains('%'), "{rows:?}");
    assert!(has(&rows[1], r"Fix ugly blocked row on the board +6m$") && has(&rows[2], r"Truncate long descriptions in sidebar +6m$"), "{rows:?}");
    let rows = draw(&mut s, &records, 43);
    assert!(has(&rows[0], r"^ \S #\d+ Remove notification pop-ups *$") && has(&rows[1], r"^ \S #\d+ Fix ugly blocked row on the board *$") && has(&rows[2], r"^ \S #\d+ Truncate long descriptions in sidebar$"), "{rows:?}");
    let rows = draw(&mut s, &records, 33);
    assert!(has(&rows[0], r"^ \S #\d+ Remove notification pop-ups$") && has(&rows[1], r"^ \S #\d+ Fix ugly blocked row on th…$"), "{rows:?}");
    {
        let eta = vec![with(&records[0], json!({ "eta": 3 })), with(&records[1], json!({ "eta": 130 })), records[2].clone(), rec(json!({ "id": "proj/4", "state": "working", "title": "No guess yet", "progress": 0, "at": at }))];
        for width in [200, 120, 90, 72, 64, 50, 40, 30, 20] { assert!(draw(&mut s, &eta, width).iter().all(|l| len(l) <= width), "fits {width}"); }
        let e = draw(&mut s, &eta, 200);
        assert!(has(&e[0], r"█+ +95% {3}3m left {8}6m {4}$"), "{e:?}"); assert_eq!(idx(&e[0], "3m"), idx(&e[1], "130m") + 2); assert!(has(&e[1], r"█+░+ +55% 130m left {2}PR#3 {2}6m {4}$"), "{e:?}");
        assert!(has(&e[2], r"sidebar +PR#2 {2}6m ago$") && has(&e[3], r"No guess yet +░+ {2}0% {18}6m {4}$"), "no guess: no \"m\", the percent stays in line: {e:?}");
        assert_eq!(idx(&e[0], "95%"), idx(&e[1], "55%"));
        let e = draw(&mut s, &eta, 66);
        assert!(has(&e[0], r"Remove notification pop-ups +95% {3}3m left +6m$") && has(&e[1], r"Fix ugly blocked row on the board +55% 130m left {2}PR#3 +6m$"), "{e:?}");
        assert!(has(&e[2], r"sidebar +PR#2 +6m$"), "the list gives up the word \"ago\" so that a row keeps its percent: {e:?}");
        let e = draw(&mut s, &eta, 55);
        assert!(has(&e[1], r"Fix ugly blocked row on the board +PR#3 +6m$") && has(&e[0], r"pop-ups +6m$") && has(&e[3], r"No guess yet +6m$"), "{e:?}");
        assert_eq!(idx(&e[1], "PR#3"), idx(&e[2], "PR#2")); assert!(e.iter().all(|l| !has(l, r"%|\dm .*\dm")), "{e:?}");
        // The same colour as the percent, whatever the state: the review colour under review.
        let paint = |s: &mut State, r: &Rec| { let l = lay(s, &t, std::slice::from_ref(r), 120, 120); row(s, &t, r, 120, &l) };
        let p = paint(&mut s, &eta[1]);
        assert_eq!((ink_of(&p, "55%"), ink_of(&p, "130m left")), (Some(Ink::Yellow), Some(Ink::Yellow)));
        let under = paint(&mut s, &with(&eta[1], json!({ "stage": "reviewing", "agent": "claude" })));
        assert!(ink_of(&under, "130m").is_some() && ink_of(&under, "130m") == ink_of(&under, "55%") && ink_of(&under, "130m") != Some(Ink::Yellow), "{under:?}");
        // Minutes left with no percent still show.
        let only = rec(json!({ "id": "proj/7", "state": "working", "title": "Only minutes", "eta": 7, "at": at }));
        let one = |s: &mut State, w: usize| { let l = lay(s, &t, std::slice::from_ref(&only), w, 200); row(s, &t, &only, w, &l).text() };
        assert!(has(&one(&mut s, 60), r"^[ ▌]◐ #\d+ Only minutes +7m left +6m$"), "{}", one(&mut s, 60));
        assert!(has(&one(&mut s, 20), r"^[ ▌]◐ Only minutes +6m$"), "{}", one(&mut s, 20));
        let osc = sw::board::update::beat::osc;
        assert!(osc(&eta[1]).contains(":progress=55:eta=130:"), "{}", osc(&eta[1])); assert!(!osc(&eta[3]).contains("eta="));
    }
    // A long name beside a short one: the two rows show the same columns.
    records = vec![
        rec(json!({ "id": "proj/8", "state": "working", "title": "Fix login", "msg": "Tracing why the login form rejects valid passwords.", "progress": 55, "pr": 3, "at": at })),
        rec(json!({ "id": "proj/9", "state": "working", "title": "Fix layout for task names in narrow terminal board", "msg": "Working.", "progress": 20, "at": at })),
    ];
    let rows = draw(&mut s, &records, 103);
    assert!(has(&rows[0], r"Fix login +█+░+ +55% {2}PR#3 +6m$") && has(&rows[1], r"board +█+░+ +20% +6m$"), "{rows:?}");
    let rows = draw(&mut s, &records, 75);
    assert!(has(&rows[0], r"Fix login +55% {2}PR#3 +6m$") && has(&rows[1], r"Fix layout for task names in narrow terminal board +20% +6m$"), "the long name has no room for a bar: neither row has one: {rows:?}");
    // All machines: the machine is the heading's, never in the row's tag (a long machine name takes nothing from it).
    s.scope = "*".into();
    let far = with(&records[0], json!({ "host": "very-long-machine-name-on-the-tailnet" }));
    let one = |s: &mut State, w: usize| { let l = lay(s, &t, std::slice::from_ref(&far), w, 200); row(s, &t, &far, w, &l).text() };
    assert!(has(&one(&mut s, 120), r"^ ◐ #8 Fix login +█"), "{}", one(&mut s, 120));
    assert!(has(&one(&mut s, 53), r"^ ◐ #8 Fix login +(█+░+ +)?55% {2}PR#3 +6m$"), "{}", one(&mut s, 53));
    assert!(has(&one(&mut s, 44), r"^ ◐ (#8 )?Fix login +(█+░+ +)?(55% {2})?PR#3 +6m$"), "{}", one(&mut s, 44));
    assert!(has(&one(&mut s, 29), r"^ \S (#8 )?Fix login +(55% {2})?PR#3 +6m$"), "{}", one(&mut s, 29));
    assert!(has(&one(&mut s, 27), r"^ \S (#8 )?Fix login +PR#3 +6m$"), "{}", one(&mut s, 27));
    assert!(has(&one(&mut s, 18), r"^ \S Fix login +6m$"), "too narrow for the task's number: it went too: {}", one(&mut s, 18));
    for width in [200, 60, 40, 30, 24, 16] { assert!(len(&one(&mut s, width)) <= width, "fits {width}"); }
}

#[test]
fn a_row_with_no_pr_and_no_bar_uses_the_empty_space_on_its_right_also_beside_rows_that_fill_every_column() {
    let (mut s, t) = board();
    let quiz = rec(json!({ "id": "proj/1", "title": "Choose the palette", "state": "blocked", "kind": "question", "stage": "asks", "agent": "claude", "at": NOW, "msg": "Sand or pebbles on the beach, and which palm?" }));
    let full = vec![
        rec(json!({ "id": "proj/2", "title": "Add an undo", "state": "blocked", "kind": "permission", "stage": "approval", "agent": "claude", "at": NOW, "pr": 18, "prState": "passing", "review": { "score": 3, "recommend": false, "text": "No." }, "reviewRound": 2 })),
        rec(json!({ "id": "proj/3", "title": "Tidy", "state": "working", "stage": "reviewing", "agent": "claude", "at": NOW, "pr": 17, "prState": "failing", "progress": 40, "reviewRound": 1, "msg": "Running the tests" })),
        rec(json!({ "id": "proj/4", "title": "Docs", "state": "working", "stage": "working", "agent": "claude", "at": NOW, "progress": 80, "msg": "Writing" })),
    ];
    let all: Vec<Rec> = std::iter::once(quiz.clone()).chain(full.iter().cloned()).collect();
    for width in [80, 100, 150] {
        let l = lay(&mut s, &t, &all, width, 120);
        let lines: Vec<String> = row_lines(&s, &t, &quiz, width, &l).iter().map(|l| l.text()).collect();
        assert!(has(&lines[0], r"^[ ▌]\? #\d+ Choose the palette +0s ago$"), "the time at the right edge at {width}: {:?}", lines[0]);
        assert_eq!(len(&lines[0]), width);
        assert_eq!(lines[1], "      Sand or pebbles on the beach, and which palm?", "whole sentence at {width}");
    }
    // Too narrow for the whole sentence: it is cut at the window's edge, not where the other rows' columns start.
    let l = lay(&mut s, &t, &all, 43, 120);
    let tight = row_lines(&s, &t, &quiz, 43, &l)[1].text();
    assert!(has(&tight, r"^[ ▌] {5}Sand or pebbles on the beach, and wh[^…]*…$"), "{tight:?}"); assert_eq!(len(&tight), 43);
    // The cause, pinned down: a cell a row does not have is nothing at all, never colour around nothing.
    let mut more = all.clone();
    more.push(with(&quiz, json!({ "state": "done", "stage": "done" }))); more.push(with(&quiz, json!({ "state": "idle", "stage": "todo" })));
    for r in &more {
        let f = sw::shown::row_says::row_fields(r, NOW);
        for (name, cell) in [("bar", &f.bar), ("percent", &f.percent), ("eta", &f.eta), ("review", &f.review), ("pr", &f.pr)] {
            if cell.text().is_empty() { assert!(cell.0.is_empty(), "{} {name}: an empty cell carries no colour", r.s("id")); }
        }
    }
}

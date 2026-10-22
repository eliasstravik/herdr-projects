//! More of the fit of a section's rows (board_rows_fit.rs has the rest): what a one-line row says shows on every row of
//! its section or on none; what is shown does not change as percents, minutes left and times tick; minutes left are
//! never wider than nine cells; a machine's name, its heading's, takes no row's words.
use regex::Regex;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use sw::board::state::{Rec, State};
use sw::board::tags::number_cells;
use sw::board::theme::Theme;
use sw::board::view::list::list_lines;
use sw::board::view::row::list_numbering;
use sw::board::view::row::row_layout;
use sw::json::*;

const NOW: i64 = 1_800_000_000_000;
const AT: i64 = NOW - 2 * 3600000; // "2h": the only "…m" on a row is then its minutes left
const GROUPS: [&str; 4] = ["ready to approve", "working", "done", "to do"];
const ORDER: [&str; 6] = ["bar", "percent", "eta", "review", "pr", "time"];
const WIDTHS: [usize; 25] = [200, 160, 130, 118, 110, 104, 96, 90, 84, 80, 76, 72, 68, 64, 63, 60, 56, 52, 48, 46, 44, 40, 36, 30, 24];
fn has(text: &str, re: &str) -> bool { Regex::new(re).unwrap().is_match(text) }
fn rec(base: Value, more: Value) -> Rec { let mut o = obj(base); for (k, v) in more.as_object().unwrap() { o.insert(k.clone(), v.clone()); } Arc::new(o) }
fn work(more: Value) -> Rec { rec(json!({ "state": "working", "stage": "working", "agent": "claude", "at": AT }), more) }
fn ready(more: Value) -> Rec { rec(json!({ "state": "blocked", "kind": "permission", "stage": "approval", "agent": "claude", "at": AT }), more) }
fn done(more: Value) -> Rec { rec(json!({ "state": "done", "stage": "done", "at": AT }), more) }
fn todo(more: Value) -> Rec { rec(json!({ "state": "idle", "stage": "todo", "at": AT }), more) }
fn group(r: &Obj) -> &'static str { if r.b("kind") { GROUPS[0] } else if r.s("state") == "working" { GROUPS[1] } else if r.s("state") == "done" { GROUPS[2] } else { GROUPS[3] } }
fn board() -> State { sw::clock::set(Some(NOW)); State::new(Default::default(), Default::default()) }
fn skip(l: &str, n: usize) -> String { l.chars().skip(n).collect() }
fn at(l: &str, b: usize) -> usize { l[..b].chars().count() }
/// The list as the board draws it under its headings: each task's first line, in the order given, and where a name
/// starts on them (right of the mark and of the column of the tasks' numbers, gone from the narrowest lists).
struct Drawn { rows: Vec<String>, lead: usize }
fn draw(s: &mut State, list: &[Rec], width: usize) -> Drawn {
    s.grouped = GROUPS.iter().map(|g| (g.to_string(), list.iter().filter(|r| group(r) == *g).cloned().collect())).collect();
    s.list_room = 0; // the list is folded again for these rows
    // Drawn a step wider and that step taken off: the rows stand one step in under the top row (update/depth.rs).
    let k = if width + 2 >= 60 { 2 } else { 1 };
    let lines: Vec<String> = list_lines(s, &Theme::default_dark(false), width + k, 80, 200).iter().map(|l| l.text().chars().skip(k).collect()).collect();
    let cells = number_cells(width, list_numbering(s, &[]));
    let lead = 3 + if cells > 0 { cells + 1 } else { 0 };
    let rows = list.iter().map(|r| { let head: String = r.s("title").chars().take(8).collect(); lines.iter().find(|l| skip(l, lead).starts_with(&head)).cloned().unwrap_or_default() }).collect();
    Drawn { rows, lead }
}
/// Where each column sits on a line: the bar and the PR number by their first cell, the right-aligned ones by their
/// last. The review tag is one of those, with its mark: it ends where the pull request starts.
fn places(line: &str, lead: usize) -> BTreeMap<&'static str, usize> {
    let l = skip(line, lead);
    let mut out = BTreeMap::new();
    let first = |re: &str| fancy_regex::Regex::new(re).unwrap().find(&l).unwrap().map(|m| at(&l, m.start()));
    if let Some(n) = first(r"[█░]{8}") { out.insert("bar", n); }
    if let Some(n) = first(r"\d%") { out.insert("percent", n); }
    if let Some(n) = first(r"\d[mh](?= .*\dh)") { out.insert("eta", n); }
    let review = Regex::new(r"(?:^|[^P])R#\d+(?: [✓✗])?( )").unwrap();
    if let Some(c) = review.captures(&l) { let m = c.get(1).unwrap(); if !l[m.end()..].starts_with(['✓', '✗']) { out.insert("review", at(&l, m.start())); } }
    if let Some(n) = first(r"PR#") { out.insert("pr", n); }
    if let Some(n) = first(r"\dh(?=( ago)? *$)") { out.insert("time", n); }
    out
}
fn has_cols(r: &Obj) -> Vec<&'static str> { ORDER.iter().copied().filter(|k| match *k { "bar" | "percent" => r.has("progress"), "eta" => r.has("eta"), "review" => r.i("reviewRound") > 0, "pr" => r.b("pr"), _ => true }).collect() }
fn says(r: &Obj, line: &str) -> bool { r.b("msg") && line.contains(&format!("  {}", r.s("msg").chars().take(6).collect::<String>())) }
/// What each section shows, read off the drawn lines. It fails when two rows of a section differ.
fn shown(list: &[Rec], d: &Drawn, width: usize) -> BTreeMap<&'static str, BTreeMap<&'static str, bool>> {
    let all = d.rows.join("\n");
    GROUPS.iter().map(|g| {
        let mut set: BTreeMap<&'static str, bool> = BTreeMap::new();
        for (n, r) in list.iter().enumerate() {
            if group(r) != *g { continue; }
            let here = places(&d.rows[n], d.lead);
            for key in has_cols(r) { let v = here.contains_key(key); let was = *set.entry(key).or_insert(v); assert_eq!(v, was, "{g}: {key} is on some rows and not on others at {width}:\n{all}"); }
            if r.b("msg") && !["working", "blocked"].contains(&r.s("state")) { let v = says(r, &d.rows[n]); let was = *set.entry("says").or_insert(v); assert_eq!(v, was, "{g}: some rows say what they do and others do not at {width}:\n{all}"); }
        }
        (*g, set)
    }).collect()
}
fn pass() -> Value { json!({ "meets": true, "gate": "major", "pass": true, "text": "Works.", "findings": [{ "id": "F1", "severity": "minor", "subject": "A name is unclear", "where": "a.js", "proof": "" }] }) }
/// The mix on a busy board: short names and long ones, with and without something to say, a percent, minutes left,
/// a pull request, a review.
fn mix() -> Vec<Rec> {
    vec![
        ready(json!({ "id": "p/1", "title": "Add an undo", "pr": 18, "prState": "passing", "review": pass(), "reviewRound": 12 })),
        ready(json!({ "id": "p/2", "title": "Sort the done list by when each task was finished, newest", "pr": 9, "prState": "passing" })),
        work(json!({ "id": "p/3", "title": "Details page crashes: artifactLines is not defined", "msg": "Reading the panel and its tests.", "progress": 72, "eta": 1 })),
        work(json!({ "id": "p/4", "title": "Side panel: Status heading and wrapped Now text", "msg": "Running the tests", "stage": "reviewing", "reviewRound": 1, "progress": 5, "eta": 5, "pr": 63 })),
        work(json!({ "id": "p/5", "title": "Coordinator as a tab in the home space", "progress": 72, "eta": 3 })),
        work(json!({ "id": "p/6", "title": "Fix", "msg": "Opening the pull request now.", "progress": 60, "eta": 16, "pr": 1234, "prState": "failing" })),
        work(json!({ "id": "p/7", "title": "Start", "msg": "Looking around." })),
        done(json!({ "id": "p/8", "title": "An old thing", "pr": 12, "prState": "merged" })),
        done(json!({ "id": "p/9", "title": "Rewrite the whole of the first-run screen and what it asks", "pr": 7, "prState": "merged", "reviewRound": 2, "review": pass() })),
        todo(json!({ "id": "p/10", "title": "Not begun", "msg": "Waits for the first-run screen." })),
    ]
}

#[test]
fn what_a_one_line_row_says_shows_on_every_row_of_its_section_that_says_something_or_on_none() {
    let mut s = board();
    // Only a row of one line says anything beside its name: a task that is to do, with a note.
    let list = vec![
        todo(json!({ "id": "p/1", "title": "Fix login", "msg": "Tracing why the login form rejects valid passwords.", "pr": 3 })),
        todo(json!({ "id": "p/2", "title": "Fix layout for task names in narrow terminal board", "msg": "Writing the tests for every width." })),
        todo(json!({ "id": "p/3", "title": "Quiet" })),
    ];
    let l = draw(&mut s, &list, 99).rows;
    assert!(has(&l[0], r"^[ ▌]○ #\d+ Fix login {2}Tracing why the login form rejects valid passwords\. +PR#3") && has(&l[1], r"board {2}Writing the tests for every w[^…]*… +2h ago$"), "each cut to its own room:\n{}", l.join("\n"));
    let l = draw(&mut s, &list, 85).rows;
    assert!(has(&l[0], r"^[ ▌]○ #\d+ Fix login +PR#3 +2h ago$") && has(&l[1], r"board +2h ago$") && has(&l[2], r"^[ ▌]○ #\d+ Quiet +2h ago$"), "the long name leaves too little to say anything: the short one says nothing either:\n{}", l.join("\n"));
    for width in WIDTHS { let d = draw(&mut s, &list, width); shown(&list, &d, width); }
    // A row with nothing to say does not count: beside it the others still say what they do.
    let two = [list[0].clone(), list[2].clone()];
    let l = draw(&mut s, &two, 85).rows;
    assert!(has(&l[0], r"^[ ▌]○ #\d+ Fix login {2}Tracing why the login form rejects valid pa"), "{}", l[0]);
    // A live task never says anything beside its name, whatever the room.
    let l = draw(&mut s, &[work(json!({ "id": "p/4", "title": "Fix login", "msg": "Tracing why.", "progress": 55 }))], 200).rows;
    assert!(has(&l[0], r"^[ ▌]◐ #\d+ Fix login {3,}█"), "{}", l[0]);
}

#[test]
fn what_is_shown_does_not_change_as_percents_minutes_left_and_times_tick() {
    let mut s = board();
    let t = Theme::default_dark(false);
    let ticks = [(0, 1, AT), (9, 9, AT), (10, 10, AT - 3600000), (55, 99, AT - 9 * 3600000), (100, 100, AT - 20 * 3600000), (99, 240, AT), (50, 999, AT), (50, 1000, AT), (5, 1007, AT), (5, 99999, AT), (3, 7, AT)];
    for width in WIDTHS.iter().copied().chain([95, 94, 93, 92, 91, 89, 88, 87, 86, 85, 75, 74, 73, 71]) {
        // What a row shows is what the layout decided for it (where its section's columns start, whether it says
        // anything, its room, the word "ago"): that is compared at every tick, and the drawn lines at the first and the last.
        let sets: Vec<(String, String)> = ticks.iter().enumerate().map(|(n, (progress, eta, since))| {
            let list: Vec<Rec> = mix().iter().map(|r| { let mut o = (**r).clone(); o.insert("at".into(), json!(since)); if o.has("progress") { o.insert("progress".into(), json!(progress)); } if o.has("eta") { o.insert("eta".into(), json!(eta)); } Arc::new(o) }).collect();
            let sections: Vec<Vec<Rec>> = GROUPS.iter().map(|g| list.iter().filter(|r| group(r) == *g).cloned().collect()).collect();
            let flat: Vec<Rec> = sections.iter().flatten().cloned().collect();
            let lay = row_layout(&mut s, &t, &flat, width, &sections, 200);
            let decided = format!("{:?}", (lay.ago, lay.cols.iter().map(|(k, _)| *k).collect::<Vec<_>>(), sections.iter().map(|recs| recs.iter().map(|r| lay.show.get(r.s("id")).copied()).collect::<Vec<_>>()).collect::<Vec<_>>()));
            let drawn = if n == 0 || n == ticks.len() - 1 { let d = draw(&mut s, &list, width); format!("{:?}", shown(&list, &d, width)) } else { String::new() };
            (decided, drawn)
        }).collect();
        assert!(sets.iter().all(|(d, _)| *d == sets[0].0), "the things shown change with the numbers at {width}");
        assert_eq!(sets.last().unwrap().1, sets[0].1, "the lines drawn change with the numbers at {width}");
    }
}

#[test]
fn minutes_left_are_never_wider_than_nine_cells_and_a_machines_name_beside_a_tasks_takes_no_rows_words() {
    let mut s = board();
    // The reviewer's case: 999m turning 1000m at a fixed width. From a thousand on they read as hours.
    let pair = |eta: i64| vec![work(json!({ "id": "p/1", "title": "A title that is forty cells long, exactly", "progress": 5, "eta": eta })), work(json!({ "id": "p/2", "title": "Fix login", "progress": 5, "eta": eta }))];
    assert_eq!(pair(1)[0].s("title").len(), 41);
    for width in [67, 66, 58, 57, 50] {
        let sets: Vec<String> = [999, 1000, 1007, 999].iter().map(|eta| { let list = pair(*eta); let d = draw(&mut s, &list, width); format!("{:?}", shown(&list, &d, width)) }).collect();
        assert!(sets.iter().all(|x| *x == sets[0]), "999m to 1000m changes what shows at {width}");
    }
    assert!(has(&draw(&mut s, &pair(999), 80).rows[1], r"% 999m left +2h$"));
    assert!(has(&draw(&mut s, &pair(1000), 80).rows[1], r"% 17h left +2h$"));
    assert!(has(&draw(&mut s, &pair(10_000_000), 80).rows[1], r"% 999h left +2h$"));
    // All machines: a machine's name is its heading's, never on a row, so it takes no room the row's words were promised.
    s.scope = "*".into();
    let far = vec![todo(json!({ "id": "p/1", "title": "A title that is forty cells long, exactly", "host": "long-machine-name", "msg": "X", "progress": 50, "eta": 1 })), todo(json!({ "id": "p/2", "title": "Fix login", "host": "long-machine-name", "msg": "X", "progress": 50, "eta": 1 }))];
    for width in 30..=120 {
        draw(&mut s, &far, width); // with a machine's name before it a line does not start with its task's name: found by it anywhere
        let all: Vec<String> = list_lines(&mut s, &Theme::default_dark(false), width, 20, 200).iter().map(|l| l.text()).collect();
        let lines: Vec<String> = far.iter().map(|r| { let head: String = r.s("title").chars().take(6).collect(); all.iter().find(|l| l.contains(&head)).cloned().unwrap_or_default() }).collect();
        let said: Vec<bool> = lines.iter().map(|l| has(l, r" {2}X( |$)")).collect();
        assert_eq!(said[0], said[1], "only one row says X at {width}:\n{}", lines.join("\n"));
        assert!(lines.iter().all(|l| l.chars().count() <= width && !l.contains("long-mach")));
    }
}

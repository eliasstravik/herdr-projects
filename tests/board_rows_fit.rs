//! What the rows of a section show when space is short: the same things on every row, given up together in one order,
//! the columns in line down the whole list; one row with no room for its bar takes the bar off its whole section; a long
//! name is cut a little before its section loses a column. (More of the fit: board_rows_fit_more.rs.)
use regex::Regex;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use sw::board::state::{Rec, State};
use sw::board::tags::number_cells;
use sw::board::theme::Theme;
use sw::board::view::list::list_lines;
use sw::board::view::row::list_numbering;
use sw::board::view::row_fit::NAME_OWED;
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
fn the_rows_of_a_section_show_the_same_things_at_every_width_in_one_order_and_the_columns_line_up_down_the_whole_list() {
    let mut s = board();
    let all = mix();
    let mut seen = std::collections::BTreeSet::new();
    let mut was: BTreeMap<&str, usize> = BTreeMap::new();
    for width in WIDTHS {
        let d = draw(&mut s, &all, width);
        let sets = shown(&all, &d, width);
        let text = d.rows.join("\n");
        let mut place: BTreeMap<&str, usize> = BTreeMap::new();
        for (n, line) in d.rows.iter().enumerate() {
            let r = &all[n];
            let here = places(line, d.lead);
            assert!(!line.is_empty() && line.chars().count() <= width, "{} fits {width}", r.s("id"));
            // A column starts at the same place on every row of the list, whatever its section shows.
            for (k, x) in &here { let p = *place.entry(k).or_insert(*x); assert_eq!(*x, p, "{k} of {} is in line at {width}:\n{text}", r.s("id")); seen.insert(*k); }
            // The name comes before the extras: whole beside a bar, and never under NAME_OWED cells beside any column.
            let rest = skip(line, d.lead);
            let name = Regex::new(r"^(.*?)(…| {2}|$)").unwrap().captures(&rest).unwrap()[1].to_string();
            if here.contains_key("bar") { assert!(rest.starts_with(r.s("title")), "{}: its name is cut beside a bar at {width}", r.s("id")); }
            if !here.is_empty() { assert!(name == r.s("title") || name.chars().count() >= NAME_OWED - 1, "{}: its name is cut to {} beside a column at {width}", r.s("id"), name.chars().count()); }
        }
        for g in GROUPS {
            // A section gives its columns up from the left, the minutes left with their percent: what it shows is the right end.
            let set = &sets[g];
            let kept: Vec<&str> = ORDER.iter().copied().filter(|k| set.get(k) == Some(&true)).collect();
            let could: Vec<&str> = ORDER.iter().copied().filter(|k| set.contains_key(k)).collect();
            assert_eq!(kept, could[could.len() - kept.len()..], "{g} shows the right end of its columns at {width}");
            if set.contains_key("eta") { assert_eq!(set.get("percent"), set.get("eta"), "{g}: percent and minutes go together at {width}"); }
            if set.contains_key("review") && set.contains_key("pr") { assert_eq!(set["review"], set["pr"], "{g}: review tag and pull request go together at {width}"); }
            // Narrower never brings a column back (the bar aside: the word "ago" going can make room for it again).
            let count = kept.iter().filter(|k| **k != "bar").count();
            assert!(count <= *was.get(g).unwrap_or(&count), "{g} shows more columns at {width} than in a wider window"); was.insert(g, count);
        }
        // What the rows say goes before any column: the same list with nothing to say has every column where it was.
        // (The old test made every row say nothing; here the rows lose their words, which is what a row says beside its name.)
        let silent: Vec<Rec> = all.iter().map(|r| { let mut o = (**r).clone(); o.remove("msg"); Arc::new(o) }).collect();
        let quiet = draw(&mut s, &silent, width);
        for (n, line) in quiet.rows.iter().enumerate() { assert_eq!(places(line, quiet.lead), places(&d.rows[n], d.lead), "{}: what it says moved a column at {width}", all[n].s("id")); }
    }
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), { let mut o = ORDER.to_vec(); o.sort(); o }, "every column was looked at");
    // Wide enough, everything shows; narrow enough, only names.
    let wide = { let d = draw(&mut s, &all, 200); shown(&all, &d, 200) };
    assert_eq!(wide["working"], ORDER.iter().map(|k| (*k, true)).collect());
    assert_eq!(wide["to do"], [("time", true), ("says", true)].into_iter().collect());
    let d = draw(&mut s, &all, 24);
    assert!(d.rows.iter().enumerate().all(|(n, line)| group(&all[n]) == "to do" || places(line, d.lead).is_empty()), "{}", d.rows.join("\n"));
}

#[test]
fn one_row_with_no_room_for_its_bar_takes_the_bar_off_every_row_of_its_section_and_off_no_other_section() {
    let mut s = board();
    let five = vec![
        work(json!({ "id": "p/1", "title": "Details page crashes: artifactLines is not defined", "progress": 72, "eta": 1 })),
        work(json!({ "id": "p/2", "title": "Side panel: Status heading and wrapped Now text", "stage": "reviewing", "reviewRound": 1, "progress": 5, "eta": 5, "pr": 63 })),
        work(json!({ "id": "p/3", "title": "Coordinator as a tab in the home space", "progress": 72, "eta": 3 })),
        work(json!({ "id": "p/4", "title": "Board screen glitches when agents start or finish", "progress": 60, "eta": 16 })),
        work(json!({ "id": "p/5", "title": "Board closes when jumping into an agent", "progress": 45, "eta": 32, "pr": 61, "reviewRound": 5 })),
    ];
    let d = draw(&mut s, &five, 90);
    assert!(d.rows.iter().all(|l| !has(l, "[█░]")), "no bar on any row, the short names too:\n{}", d.rows.join("\n"));
    assert!(d.rows.iter().enumerate().all(|(n, l)| skip(l, d.lead).starts_with(five[n].s("title")) && has(l, r"\d%") && has(l, r"\dm left ")), "and nothing else went:\n{}", d.rows.join("\n"));
    let d = draw(&mut s, &five, 98);
    assert!(d.rows.iter().all(|l| has(l, "[█░]{8}")), "room for the longest name's bar: every row has its bar:\n{}", d.rows.join("\n"));
    // The scope is the section: a long name under another heading leaves the bars of `working` alone.
    let two = vec![five[2].clone(), five[4].clone(), ready(json!({ "id": "p/8", "title": "Rewrite the whole of the first-run screen and what it asks", "progress": 100, "pr": 7 })), ready(json!({ "id": "p/9", "title": "Add an undo", "progress": 100, "pr": 12 }))];
    let l = draw(&mut s, &two, 99).rows;
    assert!(has(&l[0], r"^[ ▌]◐ #\d+ Coordinator as a tab in the home space +█{6}░░ {2}72% {2}3m left {13}2h {4}$") && has(&l[1], "[█░]{8}"), "{}", l.join("\n"));
    assert!(has(&l[2], r"^[ ▌]◆ #\d+ Rewrite the whole of the first-run screen and what it asks +100% +PR#7 +2h ago$") && has(&l[3], r"^[ ▌]◆ #\d+ Add an undo +100% +PR#12 +2h ago$"), "its bar went with its neighbour's:\n{}", l.join("\n"));
    let idx = |l: &str, p: &str| l.find(p).map(|b| at(l, b)).unwrap();
    assert_eq!(idx(&l[1], "PR#61"), idx(&l[2], "PR#7")); assert_eq!(idx(&l[2], "PR#7"), idx(&l[3], "PR#12")); assert_eq!(idx(&l[0], "72%") + 3, idx(&l[3], "100%") + 4);
}

#[test]
fn a_long_name_is_cut_a_little_before_its_section_loses_a_column_never_for_a_bar_and_never_under_its_first_cells() {
    let mut s = board();
    let long = work(json!({ "id": "p/1", "title": "Rewrite the whole of the first-run screen and what it asks", "progress": 40, "eta": 12, "pr": 7 }));
    let list = vec![long.clone(), work(json!({ "id": "p/2", "title": "Fix login", "progress": 55, "eta": 3, "pr": 35 }))];
    // 6: the mark, and the task's number (`#1`) with the cell after it; then the gap, the percent, the minutes (at their
    // widest), the gap between the groups, PR#35, the time.
    let columns = 2 + 4 + 1 + 9 + 2 + 5 + 1 + 3;
    let room = |width: usize| width - 6 - columns;
    assert_eq!(long.s("title").len(), 58);
    let l = draw(&mut s, &list, 6 + 58 + columns).rows;
    assert!(has(&l[0], r"^[ ▌]◐ #\d+ Rewrite the whole of the first-run screen and what it asks +40% 12m left {2}PR#7 +2h$") && has(&l[1], r"^[ ▌]◐ #\d+ Fix login +55% {2}3m left {2}PR#35 +2h$"), "the whole name beside every column but the bar:\n{}", l.join("\n"));
    let l = draw(&mut s, &list, 6 + 58 + columns + 8).rows;
    assert!(l.iter().all(|x| !has(x, "[█░]")) && l[0].contains(long.s("title")), "not yet room for a bar: the name is not cut for one");
    assert!(draw(&mut s, &list, 6 + 58 + columns + 9).rows.iter().all(|x| has(x, "[█░]{8}")));
    for width in [6 + 56 + columns, 73, 6 + NAME_OWED + columns] {
        let d = draw(&mut s, &list, width);
        let l = &d.rows;
        assert!(has(&l[0], r"^[ ▌]◐ #\d+ Rewrite the whole of the first-run scre[^…]*… +40% 12m left {2}PR#7 +2h$") && has(&l[1], r"^[ ▌]◐ #\d+ Fix login +55% {2}3m left {2}PR#35 +2h$"), "cut at {width}:\n{}", l.join("\n"));
        let rest = skip(&l[0], d.lead);
        assert!(rest.find('…').map(|b| at(&rest, b)).unwrap() + 1 >= NAME_OWED.min(room(width)), "the name keeps its first {NAME_OWED} cells at {width}");
    }
    let l = draw(&mut s, &list, 6 + NAME_OWED + columns - 1).rows;
    assert!(has(&l[0], r"^[ ▌]◐ #\d+ Rewrite the whole of the first-run screen [^…]*… +PR#7 +2h$") && has(&l[1], r"^[ ▌]◐ #\d+ Fix login +PR#35 +2h$"), "under that the name wins: the percent and the minutes go, on both rows:\n{}", l.join("\n"));
    // A short name is never cut for a column: it is whole wherever one shows.
    for width in WIDTHS { let d = draw(&mut s, &list, width); if !places(&d.rows[1], d.lead).is_empty() { assert!(skip(&d.rows[1], d.lead).starts_with("Fix login "), "at {width}: {}", d.rows[1]); } }
}

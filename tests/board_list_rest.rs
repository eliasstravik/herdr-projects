//! What the old board-list tests checked that tests/board_list.rs does not: the ? menu's legend under its keys, reached
//! with ↓ in a short window and whole in a narrow one; the side panel closed on a board's first opening, and the list's
//! whole width without it; looking at a done task changes nothing about it; and every machine's tasks: whose they are
//! in the header, never cut, a heading per machine, find by number, and where a far task's merge lands; a
//! click on no row is not the board's.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::update::{update, Event};
use sw::json::*;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
const LEGEND: &str = r"\n[ ▌]  R#2 +the review it is on or last had, coloured by where it stands: passed, failed, the reviewer working";

#[test]
fn the_menus_legend_is_reached_with_the_arrows_in_a_short_window_and_whole_in_a_narrow_one() {
    let w = world();
    w.add("Write the docs");
    let b = w.add("My own task");
    sw::actions::filing::report(&b, sw::actions::filing::Report { state: "working".into(), msg: "half way".into(), progress: Some(json!(50)), eta: Some(json!(12)), ..Default::default() }).unwrap();
    let mut s = open(&w.root, 34);
    s.press(&["?"]);
    let n = sw::board::update::menu::items(&s.s).len() as i64;
    s.press(&["down"]); assert_eq!(s.s.menu.as_ref().unwrap().at, 0, "the menu opens with no line picked: the first ↓ picks the first");
    for rows in [70, 24, 14] {
        s.rows = rows;
        let top = s.text();
        if rows == 70 {
            assert!(has(&top, LEGEND), "{top}");
            assert!(has(&top, r"\n on a row *\n[ ▌]  ◐ +an agent works *\n[ ▌]  ◑ +its reviewer works *\n"), "{top}");
            assert!(has(&top, r"\n[ ▌]  ◆ +ready to approve *\n") && has(&top, r"\n[ ▌]  65% 2m left +how far it is, and the time left"), "{top}");
        } else { assert!(!has(&top, LEGEND), "{rows} rows: {top}"); }
        let mut downs = 0; while !has(&s.text(), LEGEND) && downs < 60 { s.press(&["down"]); downs += 1; }
        assert!(has(&s.text(), LEGEND), "reached with {rows} lines"); assert!(s.text().starts_with(" Menu"), "the menu's first line stays: {rows} {downs}\n{}", s.text());
        for _ in 0..60 { s.press(&["down"]); }
        let end = s.text(); assert!(has(&end, LEGEND));
        s.press(&["down", "down"]); assert_eq!(s.text(), end, "nothing under the end"); assert_eq!(s.s.menu.as_ref().unwrap().at, n - 1);
        for _ in 0..80 { s.press(&["up"]); }
        // ↑ on the first line scrolls on up to what is above the list (the plain keys), and ↓ comes back from there
        assert_eq!(s.s.menu.as_ref().unwrap().at, 0);
        assert!(has(&s.text(), r"^ Menu[^\n]*\n plain keys *\n[ ▌]  ↑ ↓ "), "the top with {rows} lines: {}", s.text());
        for _ in 0..80 { if s.text() == top { break; } s.press(&["down"]); }
        assert_eq!(s.s.menu.as_ref().unwrap().at, 0); assert_eq!(s.text(), top, "↓ goes back the way ↑ came");
    }
    // narrow: the line wraps, so every state it names can be read, and nothing is cut
    for (cols, rows) in [(60, 18), (44, 18), (36, 24), (80, 14)] {
        s.rows = rows;
        let flat = |s: &mut common::board::Board| regex::Regex::new(r"\s+").unwrap().replace_all(&s.screen(cols).join("\n"), " ").into_owned();
        let mut seen = String::new();
        for _ in 0..120 { s.press(&["down"]); seen = flat(&mut s); if seen.contains("the reviewer working") { break; } }
        assert!(seen.contains("R#2 the review it is on or last had, coloured by where it stands: passed, failed, the reviewer working"), "whole at {cols}×{rows}: {seen}");
        assert!(!has(&s.screen(cols).join("\n"), "R#2[^\n]*…"), "not cut");
        for l in s.screen(cols) { assert!(sw::text::width(&l) <= cols, "fits {cols}: {l}"); }
        for _ in 0..125 { s.press(&["up"]); }
    }
}

#[test]
fn the_side_panel_is_closed_on_a_first_opening_and_the_list_has_the_whole_width_without_it() {
    let w = world();
    let b = w.add("My own task");
    sw::actions::filing::report(&b, sw::actions::filing::Report { state: "working".into(), msg: "half way".into(), progress: Some(json!(50)), ..Default::default() }).unwrap();
    let _ = std::fs::remove_file(sw::board::run::prefs::file(false));
    let mut first = sw::board::state::State::new(Default::default(), Default::default());
    sw::board::run::prefs::apply(&mut first, &sw::board::run::prefs::read(false));
    assert!(!first.side, "a board opened for the first time has it closed");
    let mut s = open(&w.root, 34); s.press(&["alt+p"]); assert!(!s.s.side);
    assert!(has(&s.screen(120).join("\n"), r"◐ PRJ-1 +My own task {70,}█[^\n]*\n[ ▌] {10}half way *\n"), "{}", s.screen(120).join("\n"));
}

#[test]
fn looking_at_a_done_task_changes_nothing_about_it() {
    let w = world();
    let ids: Vec<String> = ["oldest", "middle", "newest"].iter().map(|t| w.add(t)).collect();
    for (n, id) in ids.iter().enumerate() { sw::store::tasks::update(id, |t| { t.put("stage", "done"); t.put("seen", n != 0); Ok(sw::store::tasks::Change::Write) }).unwrap(); }
    let mut s = open(&w.root, 34);
    s.press(&["enter", "esc"]);
    for (n, id) in ids.iter().enumerate() { assert_eq!(task(id).b("seen"), n != 0, "{id}"); }
}

#[test]
fn every_machines_tasks_say_whose_they_are_and_a_far_merge_says_where_it_lands() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "otherbox" }), ..Default::default() });
    for n in 1..=3 { w.add(&format!("Task number {n}")); }
    let mut s = open(&w.root, 24); s.s.side = false;
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    let now = sw::clock::now();
    let far = |items: serde_json::Value| Ok(json!({ "host": "otherbox", "version": "0.2.0", "tasks": items }));
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: far(json!([{ "id": "proj/1", "title": "Far away", "state": "idle", "stage": "todo", "at": now, "created": now, "snap": { "folder": "~/far" } }])) });
    // Alt+M: every machine comes first in its round (this machine where herdr's sidebar has it).
    while s.s.scope != "*" { s.press(&["alt+m"]); }
    // whose board shows is the filter row's first item, never cut
    for width in [100, 40, 24] { assert!(s.screen(width)[1].starts_with(" All machines │"), "at {width}: {}", s.screen(width)[1]); }
    s.s.sel = "otherbox:proj/1".into(); s.load();
    let t = s.screen(100).join("\n");
    // Each machine a step in under All machines, its status groups a step in under that: the row's tag is the number alone.
    assert!(has(&t, r"\n   (○ |  )?▾ otherbox *\n     ▾ to do\n▌    ○ #1 +Far away "), "{t}"); assert!(!t.contains("otherbox#1") && !t.contains("otherbox Far away"));
    s.press(&["ctrl+f"]); s.typed("1");
    assert!(s.s.rows.iter().any(|r| matches!(r, sw::board::state::Row::Rec(o) if o.s("host") == "otherbox")), "find takes the number alone: no machine in it");
    s.press(&["esc"]);
    // another machine's work: the merge question says where it lands there; an older one that does not say still gets one
    let item = |lands: Option<&str>| { let mut snap = json!({ "folder": "~/far" }); if let Some(l) = lands { snap["landsOn"] = json!(l); } json!([{ "id": "proj/7", "title": "Remote merge", "state": "blocked", "kind": "permission", "stage": "approval", "agent": "claude", "app": "claude", "at": now, "created": now, "snap": snap }]) };
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: far(item(Some("trunk"))) });
    s.s.sel = "otherbox:proj/7".into(); s.load();
    s.press(&["ctrl+l"]);
    let ask = s.s.ask.clone().unwrap_or_else(|| panic!("{}", s.text()));
    assert_eq!(ask.q, "Merge otherbox#7 \"Remote merge\" into trunk on otherbox?"); assert!(ask.detail.contains("A merge cannot be undone."), "{}", ask.detail);
    s.press(&["n"]);
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: far(item(None)) });
    s.load(); s.press(&["ctrl+l"]);
    assert_eq!(s.s.ask.as_ref().map(|a| a.q.clone()).unwrap_or_default(), "Merge otherbox#7 \"Remote merge\" into its main branch on otherbox?");
    s.press(&["n"]);
}

#[test]
fn machine_names_too_long_for_a_line_are_shown_whole_never_cut() {
    let long = ["a-very-long-machine-name-on-the-tailnet", "another-extremely-long-machine-name-there"];
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": long.join(", ") }), ..Default::default() });
    w.add("Mine");
    let mut s = open(&w.root, 24); s.s.side = false;
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    for m in long { update(&mut s.s, Event::Remote { label: m.into(), answer: Ok(json!({ "host": m, "version": "0.2.0", "tasks": [] })) }); }
    while s.s.scope != "*" { s.press(&["alt+m"]); }
    // every machine: each under a heading of its own, its name whole
    for width in [80, 50] {
        let t = s.screen(width).join("\n");
        for name in long { assert!(t.lines().any(|l| l.contains(&format!("▾ {name}")) || l.contains(&format!("▸ {name}"))), "{name} whole at {width}: {t}"); }
    }
    // one machine by itself: its name is the filter row's first item, whole
    for name in long {
        while s.s.scope != name { s.press(&["alt+m"]); }
        let l = s.screen(50)[1].clone();
        assert!(l.starts_with(&format!(" {name} │")), "{l}");
    }
}

#[test]
fn a_click_that_is_on_no_row_is_not_the_boards() {
    let w = world();
    for n in 1..=5 { w.add(&format!("Task number {n}")); }
    let mut s = open(&w.root, 20); s.s.side = false; s.screen(120);
    let (sel, top) = (s.s.sel.clone(), s.s.top);
    s.click(0); assert_eq!((s.s.sel.clone(), s.s.top, s.s.msg.clone()), (sel, top, String::new()), "the header is no row");
}

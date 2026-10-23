//! Pins and the Pinned tab. A pin moves nothing: the task, project or machine keeps its place and gets a pin mark. The
//! Pinned tab shows only what is pinned, each under its machine's and its project's rows, a project with all its tasks
//! and a machine with its projects; it has no filter row, keeps its own folds, and the board remembers which tab shows.
//! `pin` / `unpin` take a task, a machine or a project from the shell, and `list --pinned` lists what is pinned.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::board::state::Row;
use sw::board::update::heads::{group_id, machine_id};
use sw::board::update::list::relist;
use sw::cli::flags::parse;
use sw::json::*;

/// Another machine's pins, sent with its tasks, and its machine and projects pinned from here.
mod far_pins;

/// Tasks added a second apart, so the list's order (the newest first) is never left to the same millisecond.
fn add(w: &World, titles: &[&str]) -> Vec<String> {
    sw::clock::set(Some(sw::clock::now()));
    titles.iter().map(|t| { sw::clock::advance(1000); w.add(t) }).collect()
}
fn board(w: &World) -> Board { let mut s = open(&w.root, 40); s.s.side = false; s }
fn on(s: &mut Board, id: &str) { s.s.sel = id.into(); s.load(); relist(&mut s.s); }
/// Every row the arrows stop on, top to bottom: headings by their id, tasks by theirs.
fn ids(s: &Board) -> Vec<String> { s.s.rows.iter().filter_map(|r| match r { Row::Head { id, .. } => Some(id.clone()), Row::Rec(o) => Some(o.t("id")), _ => None }).collect() }
fn tasks_shown(s: &Board) -> Vec<String> { s.s.rows.iter().filter_map(|r| match r { Row::Rec(o) => Some(o.t("id")), _ => None }).collect() }
fn line_with(s: &mut Board, words: &str) -> String { s.lines().into_iter().find(|l| l.contains(words)).unwrap_or_else(|| panic!("no line with {words}")) }
fn mark(id: &str, fields: serde_json::Value) { sw::store::tasks::set(id, fields).unwrap(); }
fn run(w: &World, args: &[&str]) -> Result<(serde_json::Value, String), String> {
    let a = parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let was = std::env::current_dir().unwrap();
    std::env::set_current_dir(&w.root).unwrap();
    let r = sw::cli::flags::refuse_unknown(&a.pos[0], &a).and_then(|_| sw::cli::tasks::command(&a.pos[0], &a));
    std::env::set_current_dir(was).unwrap();
    r.map_err(|e| e.msg)
}

#[test]
fn a_pin_moves_nothing_and_marks_the_row() {
    let w = world();
    let [_, b, _] = <[String; 3]>::try_from(add(&w, &["One", "Two", "Three"])).unwrap();
    let mut s = board(&w);
    on(&mut s, &b);
    let before = ids(&s);
    s.press(&["ctrl+p"]);
    assert!(task(&b).b("pinned"));
    assert_eq!(ids(&s), before, "the task keeps its place under to do");
    assert_eq!(s.s.sel, b, "and the selection stays on it");
    assert!(line_with(&mut s, "Two").contains("Two ⚑"), "{}", s.text());
    s.press(&["?"]);
    assert!(s.text().contains("unpin this task"), "{}", s.text());
    s.press(&["esc"]);
    s.press(&["ctrl+p"]);
    assert!(!task(&b).b("pinned")); assert!(!s.text().contains('⚑'), "{}", s.text());
}

#[test]
fn the_pinned_tab_shows_only_what_is_pinned_under_its_machine_and_project_and_is_remembered() {
    let w = world();
    let [a, b, _] = <[String; 3]>::try_from(add(&w, &["One", "Two", "Three"])).unwrap();
    sw::actions::filing::set_pin(&b, true).unwrap();
    let mut s = board(&w);
    // The board's own view: grouped by repo, which the Pinned tab sets aside and gives back.
    s.press(&["alt+s"]); assert!(s.s.by_repo); s.press(&["alt+s"]); assert!(s.s.by_machine);
    let head = s.lines()[0].clone();
    assert!(head.contains(" Board ") && head.contains(" Pinned "), "{head}");
    s.press(&["alt+i"]);
    assert!(s.s.pinned_view);
    assert_eq!(tasks_shown(&s), vec![b.clone()], "{}", s.text());
    let rows = ids(&s);
    assert!(rows.contains(&machine_id("")) && rows.contains(&group_id(&s.s, "", "proj")), "under its machine and its project: {rows:?}");
    assert!(!s.text().contains("Needs you"), "no filter row: {}", s.text());
    // Its grouping and its filter are fixed: the keys say so and change nothing.
    s.press(&["alt+s"]); assert!(s.s.by_repo && !s.s.by_machine); assert!(s.s.msg.contains("always by machine and project"), "{}", s.s.msg);
    s.press(&["tab"]); assert_eq!(s.s.show, 0);
    // Remembered: a board opened again shows the Pinned tab, and the board's own grouping comes back with the key.
    let mut t = board(&w);
    assert!(t.s.pinned_view); assert_eq!(tasks_shown(&t), vec![b.clone()]);
    t.press(&["alt+i"]);
    assert!(!t.s.pinned_view && t.s.by_machine && !t.s.by_repo);
    assert!(tasks_shown(&t).contains(&a));
    // Unpinned there, the tab is empty and says how to pin something.
    on(&mut s, &b);
    s.press(&["ctrl+p"]);
    assert!(!task(&b).b("pinned"));
    assert!(tasks_shown(&s).is_empty());
    assert!(s.text().contains(&format!("Nothing pinned yet. {} on a task, a project or a machine pins it here.", key_name("pin"))), "{}", s.text());
}

#[test]
fn a_project_and_a_machine_pin_from_their_rows_and_bring_their_tasks_into_the_tab() {
    let w = world();
    let [a, b] = <[String; 2]>::try_from(add(&w, &["One", "Two"])).unwrap();
    let mut s = board(&w);
    s.press(&["alt+s"]); assert!(s.s.by_repo);
    let proj = group_id(&s.s, "", "proj");
    on(&mut s, &proj);
    s.press(&["?"]); assert!(s.text().contains("pin this project"), "{}", s.text()); s.press(&["esc"]);
    s.press(&["ctrl+p"]);
    assert_eq!(sw::store::pins::pins().repos, vec!["proj".to_string()]);
    assert_eq!(s.s.sel, proj, "nothing moved");
    assert!(s.lines().iter().any(|l| l.contains("proj") && l.contains('⚑')), "{}", s.text());
    // In the tab: the project with every task of it.
    s.press(&["alt+i"]);
    let mut shown = tasks_shown(&s); shown.sort();
    let mut all = vec![a.clone(), b.clone()]; all.sort();
    assert_eq!(shown, all);
    // The machine's row (the top row on this machine's board): pinned too; the project unpinned, its tasks stay
    // under the pinned machine.
    on(&mut s, &machine_id(""));
    s.press(&["ctrl+p"]);
    assert!(sw::store::pins::pins().machine);
    on(&mut s, &proj);
    s.press(&["ctrl+p"]);
    assert!(sw::store::pins::pins().repos.is_empty());
    assert_eq!(tasks_shown(&s).len(), 2, "{}", s.text());
    assert!(s.lines().iter().any(|l| l.contains('⚑')), "the machine's mark: {}", s.text());
    // A heading that is no machine's or project's pins nothing.
    s.press(&["alt+i"]);
    while s.s.by_repo || s.s.by_machine { s.press(&["alt+s"]); }
    let todo = group_id(&s.s, "", "to do");
    on(&mut s, &todo);
    s.press(&["ctrl+p"]);
    assert!(s.s.msg.contains("pins a task, a project's row or a machine's row"), "{}", s.s.msg);
}

#[test]
fn the_tabs_say_what_needs_you_in_each() {
    let w = world();
    let [a, b] = <[String; 2]>::try_from(add(&w, &["One", "Two"])).unwrap();
    mark(&a, json!({ "stage": "asks", "msg": "Which one?" }));
    let mut s = board(&w);
    assert_eq!(s.s.tab_needs, (1, 0));
    sw::actions::filing::set_pin(&a, true).unwrap(); s.load();
    assert_eq!(s.s.tab_needs, (1, 1), "pinned, it needs you in both tabs");
    let head = s.lines()[0].clone();
    assert!(head.contains(" Board ") && head.contains("Pinned 1"), "the tab shown says it with the filter row, the other by itself: {head}");
    // In the Pinned tab, what needs you elsewhere is still counted on the board's tab.
    sw::actions::filing::set_pin(&a, false).unwrap(); sw::actions::filing::set_pin(&b, true).unwrap(); s.load();
    s.press(&["alt+i"]);
    assert_eq!(tasks_shown(&s), vec![b.clone()]);
    assert_eq!(s.s.tab_needs, (1, 0));
    assert!(s.lines()[0].contains("Board 1"), "{}", s.lines()[0]);
}

#[test]
fn pin_and_unpin_take_a_task_a_machine_or_a_project_and_list_pinned_lists_them() {
    let w = world();
    let [a, _] = <[String; 2]>::try_from(add(&w, &["One", "Two"])).unwrap();
    let host = sw::remote::check::hostname();
    assert_eq!(run(&w, &["pin", &a]).unwrap().1, format!("{a}: pinned"));
    assert_eq!(run(&w, &["pin", &a]).unwrap().1, format!("{a}: pinned"), "pin pins, whatever it was");
    assert_eq!(run(&w, &["pin", "proj"]).unwrap().1, "proj: pinned");
    assert_eq!(run(&w, &["pin", &host]).unwrap().1, format!("{host}: pinned"));
    let (j, text) = run(&w, &["list", "--pinned"]).unwrap();
    assert_eq!(text.lines().count(), 3, "{text}");
    assert!(text.starts_with(&format!("machine   {host}\nproject   proj\n")), "{text}");
    assert_eq!((j.s("machine"), j.at("projects").clone(), j.arr("tasks").len()), (host.as_str(), json!(["proj"]), 1));
    assert_eq!(run(&w, &["unpin", &a]).unwrap().1, format!("{a}: unpinned"));
    assert_eq!(run(&w, &["unpin", "proj"]).unwrap().1, "proj: unpinned");
    assert_eq!(run(&w, &["unpin", &host]).unwrap().1, format!("{host}: unpinned"));
    assert_eq!(run(&w, &["list", "--pinned"]).unwrap().1, "Nothing pinned.");
    let e = run(&w, &["pin", "nowhere"]).unwrap_err();
    assert!(e.starts_with("\"nowhere\" is no task, machine or project the board knows."), "{e}");
    assert!(run(&w, &["pin"]).unwrap_err().contains("pin needs a task's id"));
    assert!(run(&w, &["pin", "proj", "--on"]).unwrap_err().contains("--on needs a machine's name"));
}

#[test]
fn the_web_board_has_the_pinned_tab_its_chip_and_the_pin_mark() {
    let w = world();
    let [a, b] = <[String; 2]>::try_from(add(&w, &["One", "Two"])).unwrap();
    sw::actions::filing::set_pin(&b, true).unwrap();
    mark(&a, json!({ "stage": "asks", "msg": "Which one?" })); // it needs you, and it is not pinned
    let s = common::board::web(&w, "board", json!({}));
    assert!(!s.at("flags").b("pinned") && s.at("list").at("shows").is_object());
    assert_eq!(s.at("list").at("head").at("tabs"), &json!({ "pinned": false, "board": 1, "pinnedNeeds": 0 }));
    let titles = |s: &serde_json::Value| s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").iter().map(|r| (r.t("id"), r.t("title"))).collect::<Vec<_>>()).collect::<Vec<_>>();
    assert!(titles(&s).contains(&(b.clone(), "Two ⚑".into())) && titles(&s).contains(&(a.clone(), "One".into())), "{:?}", titles(&s));
    let s = common::board::web(&w, "press", json!({ "id": "pinnedView" }));
    assert!(s.at("flags").b("pinned") && !s.at("flags").b("byRepo"), "no By repo chip in the tab: {}", s.at("flags"));
    assert!(s.at("list").at("shows").is_null(), "no filter row");
    // What needs you on the board is still counted, for the page's pin button and its chip (board.js).
    assert_eq!(s.at("list").at("head").at("tabs"), &json!({ "pinned": true, "board": 1, "pinnedNeeds": 0 }));
    assert_eq!(titles(&s), vec![(b.clone(), "Two ⚑".into())]);
    // Its filter and its grouping stay as they are.
    let s = common::board::web(&w, "group", json!({})); assert!(s.at("flags").b("pinned")); assert_eq!(titles(&s).len(), 1);
    let s = common::board::web(&w, "press", json!({ "id": "pinnedView" }));
    assert!(!s.at("flags").b("pinned")); assert_eq!(titles(&s).len(), 2);
}

//! Pins and the Pinned filter. A pin moves nothing: the task, project or machine keeps its place and gets a pin mark.
//! Pinned, the last filter on the filter row, shows only what is pinned in its places and the board's grouping, a
//! project with all its tasks and a machine with its projects, as every filter does; there is no row of tabs.
//! `pin` / `unpin` take a task, a machine or a project from the shell, and `list --pinned` lists what is pinned.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::board::state::Row;
use sw::board::update::heads::{group_id, machine_id, HEAD};
use sw::board::update::list::relist;
use sw::board::update::shows::{NEEDS, PINNED};
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
/// The Pinned filter shown, as tab gets there.
fn pinned(s: &mut Board) { while s.s.show != PINNED { s.press(&["tab"]); } }
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
fn the_pinned_filter_shows_only_what_is_pinned_in_its_places_and_is_remembered() {
    let w = world();
    let [a, b, _] = <[String; 3]>::try_from(add(&w, &["One", "Two", "Three"])).unwrap();
    sw::actions::filing::set_pin(&b, true).unwrap();
    let mut s = board(&w);
    // No row of tabs: the header is the board's name and its repo, Pinned the last filter on the filter row.
    let lines = s.lines();
    assert!(!lines[0].contains("Board") && !lines[0].contains("Pinned"), "{}", lines[0]);
    let row = lines.iter().find(|l| l.contains("Automations")).unwrap_or_else(|| panic!("{}", s.text())).clone();
    assert!(row.contains("Pinned 1") && row.find("Automations") < row.find("Pinned"), "{row}");
    // The board's grouping stays: by machine here.
    s.press(&["alt+s"]); s.press(&["alt+s"]); assert!(s.s.by_machine);
    s.press(&["shift+tab"]);
    assert_eq!(s.s.show, PINNED, "shift+tab from All: the last filter");
    assert_eq!(tasks_shown(&s), vec![b.clone()], "{}", s.text());
    assert!(s.s.by_machine && ids(&s).contains(&machine_id("")), "{:?}", ids(&s));
    assert!(s.text().contains("Needs you 0") && s.s.show_n[0] == 3, "the filter row stays, its counts the board's: {}", s.text());
    // Tab goes round to All, as from any last filter.
    s.press(&["tab"]); assert_eq!(s.s.show, 0); assert!(tasks_shown(&s).contains(&a));
    pinned(&mut s);
    // Remembered as any filter: a board opened again shows it, with the board's grouping.
    let t = board(&w);
    assert_eq!(t.s.show, PINNED); assert!(t.s.by_machine); assert_eq!(tasks_shown(&t), vec![b.clone()]);
    // Unpinned there, the list is empty and says how to pin something.
    on(&mut s, &b);
    s.press(&["ctrl+p"]);
    assert!(!task(&b).b("pinned"));
    assert!(tasks_shown(&s).is_empty());
    assert!(s.text().contains(&format!("Nothing pinned yet. {} on a task, a project or a machine pins it here.", key_name("pin"))), "{}", s.text());
    assert!(s.text().contains("Pinned 0"), "{}", s.text());
}

#[test]
fn a_project_and_a_machine_pin_from_their_rows_and_show_with_their_tasks_under_pinned() {
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
    // Under Pinned by repo: the project's heading with every task of it under it.
    pinned(&mut s);
    let mut shown = tasks_shown(&s); shown.sort();
    let mut all = vec![a.clone(), b.clone()]; all.sort();
    assert_eq!(shown, all);
    assert!(ids(&s).contains(&proj), "{:?}", ids(&s));
    assert_eq!(s.s.show_n[PINNED], 1, "one pin, the project's: its tasks add nothing");
    // By status: the project's row stands above its tasks, as a project that needs you does under Needs you.
    while s.s.by_repo || s.s.by_machine { s.press(&["alt+s"]); }
    let row = format!("{HEAD}repo\t\tproj");
    let at = ids(&s).iter().position(|i| *i == row).unwrap_or_else(|| panic!("{:?}", ids(&s)));
    assert!(ids(&s).iter().position(|i| *i == a).unwrap() > at, "{:?}", ids(&s));
    assert!(s.lines().iter().any(|l| l.contains("proj") && l.contains('⚑')), "{}", s.text());
    // The machine's row (the top row on this machine's board): pinned too; the project unpinned, its tasks stay
    // under the pinned machine.
    on(&mut s, &machine_id(""));
    s.press(&["ctrl+p"]);
    assert!(sw::store::pins::pins().machine);
    on(&mut s, &row);
    s.press(&["ctrl+p"]);
    assert!(sw::store::pins::pins().repos.is_empty());
    assert_eq!(tasks_shown(&s).len(), 2, "{}", s.text());
    assert!(!ids(&s).contains(&row), "no project pinned, no project's row: {:?}", ids(&s));
    assert!(s.lines().iter().any(|l| l.contains('⚑')), "the machine's mark: {}", s.text());
    // A heading that is no machine's or project's pins nothing.
    s.press(&["tab"]);
    let todo = group_id(&s.s, "", "to do");
    on(&mut s, &todo);
    s.press(&["ctrl+p"]);
    assert!(s.s.msg.contains("pins a task, a project's row or a machine's row"), "{}", s.s.msg);
}

#[test]
fn pinneds_count_is_each_pin_once_not_the_tasks_under_a_pinned_project() {
    let w = world();
    let [a, b, c] = <[String; 3]>::try_from(add(&w, &["Done in it", "Pinned in it", "Another"])).unwrap();
    mark(&a, json!({ "stage": "done" }));
    sw::store::pins::set(Some("proj"), true).unwrap();
    sw::actions::filing::set_pin(&b, true).unwrap();
    let mut s = board(&w);
    pinned(&mut s);
    // The project and the task pinned in it: two pins. Its done task (folded away) and its other task add nothing.
    assert!(tasks_shown(&s).contains(&c) && !tasks_shown(&s).contains(&a), "{}", s.text());
    let text = s.text();
    assert_eq!(s.s.show_n[PINNED], 2, "{text}");
    assert!(text.contains("Pinned 2"), "{text}");
    let (j, _) = run(&w, &["list", "--pinned"]).unwrap();
    assert_eq!(j.at("projects").as_array().unwrap().len() + j.arr("tasks").len(), 2, "as list --pinned says: {j}");
    // The web board's filter row says the same.
    let web = common::board::web(&w, "board", json!({}));
    assert_eq!(web.at("list").at("shows").arr("items")[PINNED].i("count"), 2);
}

#[test]
fn what_needs_you_keeps_its_count_on_the_filter_row_under_pinned() {
    let w = world();
    let [a, b] = <[String; 2]>::try_from(add(&w, &["One", "Two"])).unwrap();
    mark(&a, json!({ "stage": "asks", "msg": "Which one?" }));
    sw::actions::filing::set_pin(&b, true).unwrap();
    let mut s = board(&w);
    pinned(&mut s);
    assert_eq!(tasks_shown(&s), vec![b.clone()]);
    assert_eq!((s.s.show_n[NEEDS], s.s.show_n[PINNED]), (1, 1));
    assert!(s.text().contains("Needs you 1"), "{}", s.text());
    // Pinned, a task that needs you shows under both.
    sw::actions::filing::set_pin(&a, true).unwrap(); s.load();
    assert_eq!(s.s.show_n[PINNED], 2);
    assert!(tasks_shown(&s).contains(&a));
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
fn the_web_board_has_pinned_on_its_filter_row_and_the_pin_mark() {
    let w = world();
    let [a, b] = <[String; 2]>::try_from(add(&w, &["One", "Two"])).unwrap();
    sw::actions::filing::set_pin(&b, true).unwrap();
    let s = common::board::web(&w, "board", json!({}));
    assert!(s.at("list").at("head").at("tabs").is_null() && !s.at("flags").has("pinned"), "no tabs: {}", s.at("flags"));
    let items = s.at("list").at("shows").arr("items").clone();
    let last = items.last().unwrap();
    assert_eq!((last.i("n"), last.s("name"), last.i("count")), (PINNED as i64, "Pinned", 1));
    assert_eq!(items[items.len() - 2].s("name"), "Automations");
    let titles = |s: &serde_json::Value| s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").iter().map(|r| (r.t("id"), r.t("title"))).collect::<Vec<_>>()).collect::<Vec<_>>();
    assert!(titles(&s).contains(&(b.clone(), "Two ⚑".into())) && titles(&s).contains(&(a.clone(), "One".into())), "{:?}", titles(&s));
    // A tap on it shows it, as any filter: only what is pinned, the filter row still there.
    let s = common::board::web(&w, "show", json!({ "n": PINNED }));
    assert!(s.at("list").at("shows").arr("items")[PINNED].b("lit"));
    assert_eq!(titles(&s), vec![(b.clone(), "Two ⚑".into())]);
    let s = common::board::web(&w, "show", json!({ "n": 0 }));
    assert_eq!(titles(&s).len(), 2);
}

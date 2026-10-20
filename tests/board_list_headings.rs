//! The list's headings: with every machine shown a heading per machine and its repos (or its status groups) under it,
//! rows that never name their machine; the arrows stop on every heading, enter folds it to one line that says how many
//! tasks it holds and unfolds it, at both levels, grouped by repo and by status; a fold outlasts a refresh and the board
//! closing, gives way to a filter and to a task the selection is taken to; the hint line, the ? menu and the web board.
//! A repo's heading is its name in plain text, bold, as loud as its tasks' titles, its key and caret muted.
use crate::common;
use common::board::{open, web, Board};
use common::*;
use serde_json::{json, Value};
use sw::board::update::{update, Event};
use sw::shown::ink::Ink;
use sw::json::*;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
/// A board of this machine's three tasks in `proj`, and otherbox's two: one in `proj`, one in `crm`.
fn two_machines() -> (World, Board) {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "otherbox" }), ..Default::default() });
    for n in 1..=3 { w.add(&format!("Mine {n}")); }
    let mut s = open(&w.root, 40); s.s.side = false;
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    let now = sw::clock::now();
    let task = |id: &str, title: &str| json!({ "id": id, "title": title, "state": "idle", "stage": "todo", "at": now, "created": now });
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: Ok(json!({ "host": "otherbox", "version": "0.2.0", "tasks": [task("proj/151", "Theirs in proj"), task("crm/4", "Theirs in crm")] })) });
    (w, s)
}
fn head(name: &str) -> String { format!(r"\n ▾ {name} ─+\n") }
/// Arrow down until the selection is on this heading (or on this task).
fn go(s: &mut Board, id: &str) { for _ in 0..40 { if s.s.sel == id { return; } s.press(&["down"]); } for _ in 0..40 { if s.s.sel == id { return; } s.press(&["up"]); } panic!("{id} is not reached with the arrows"); }
fn machine(host: &str) -> String { format!("head:machine\t{host}") }
fn repo(host: &str, name: &str) -> String { format!("head:repo\t{host}\t{name}") }

#[test]
fn every_machine_a_heading_its_repos_under_it_and_rows_with_the_number_alone() {
    let (_w, mut s) = two_machines();
    s.press(&["alt+m", "alt+s"]); assert!(s.s.scope == "*" && s.s.by_repo);
    let t = s.screen(100).join("\n");
    let here = sw::shown::words::clean(&sw::remote::check::hostname());
    assert!(has(&t, &format!(r"{} ▾ proj · PRJ\n[ ▌]○ PRJ-\d+ +Mine", head(&regex::escape(&here)))), "this machine first, under its own name: {t}");
    // The repo the board is in comes first on every machine, as on one.
    assert!(has(&t, &format!(r"{} ▾ proj\n[ ▌]○ #151 +Theirs in proj[^\n]*\n\n ▾ crm\n[ ▌]○ #4 +Theirs in crm", head("otherbox"))), "{t}");
    assert!(!t.contains("otherbox#") && !t.contains("otherbox:"), "a row never names its machine, nor a heading the repo's machine: {t}");
    // By status: the same machines, their status groups under them.
    s.press(&["alt+s"]);
    let t = s.screen(100).join("\n");
    assert!(has(&t, &format!(r"{} ▾ to do\n[ ▌]○ #\d+ +Theirs in (crm|proj)[^\n]*\n[ ▌]○ #\d+ +Theirs", head("otherbox"))), "{t}");
    // One machine's board has no machine heading, by repo or by status.
    s.press(&["alt+m"]); s.s.scope.clear(); sw::board::update::list::relist(&mut s.s);
    let t = s.screen(100).join("\n");
    assert!(!t.contains(" ─────") && t.contains("\n ▾ to do\n"), "{t}");
}

#[test]
fn enter_on_a_heading_folds_it_to_one_line_with_how_many_it_holds_and_unfolds_it_at_both_levels() {
    let (_w, mut s) = two_machines();
    s.press(&["alt+m", "alt+s"]);
    // The arrows stop on each heading, which is selected as a row is.
    go(&mut s, &repo("otherbox", "crm"));
    assert!(s.screen(100).iter().any(|l| l.starts_with("▌▾ crm")), "{}", s.text());
    assert!(s.text().contains("enter fold this repo"), "the hint line says what enter does there: {}", s.text());
    s.press(&["enter"]);
    let t = s.screen(100).join("\n");
    assert!(has(&t, r"Theirs in proj[^\n]*\n\n▌▸ crm  1 task"), "{t}"); assert!(!t.contains("Theirs in crm") && t.contains("Theirs in proj"));
    assert_eq!(s.s.sel, repo("otherbox", "crm"), "the selection stays on the heading");
    assert!(s.text().contains("enter unfold this repo"));
    // A machine folds whole; unfolded, its repos are as they were.
    go(&mut s, &machine("otherbox"));
    s.press(&["enter"]);
    let t = s.screen(100).join("\n");
    assert!(has(&t, r"\n▌▸ otherbox  2 tasks ─+\n") || has(&t, r"\n▌▸ otherbox  2 tasks"), "{t}"); assert!(!t.contains("Theirs") && !t.contains("▾ proj\n ○ #151"));
    assert!(t.contains("Mine 1"), "the other machine's tasks stay: {t}");
    s.press(&["enter"]);
    let t = s.screen(100).join("\n");
    assert!(has(&t, r"\n ▸ crm  1 task\n") && t.contains("Theirs in proj"), "{t}");
    // By status, and on one machine: its status headings fold the same way.
    s.press(&["alt+s", "alt+m"]); s.s.scope.clear(); sw::board::update::list::relist(&mut s.s);
    let todo = "head:status\t\tto do".to_string();
    go(&mut s, &todo);
    assert!(s.text().contains("enter fold this group"));
    s.press(&["enter"]);
    let t = s.screen(100).join("\n");
    assert!(has(&t, r"\n▌▸ to do  3 tasks"), "{t}"); assert!(!t.contains("Mine 1"));
    s.press(&["enter"]); assert!(s.text().contains("Mine 1"));
    // The ? menu says it too.
    s.press(&["?"]); assert!(s.text().contains("or fold a heading"), "{}", s.text());
}

#[test]
fn a_fold_outlasts_a_refresh_and_the_board_closing_a_filter_and_a_task_taken_to_open_it() {
    let w = world();
    for n in 1..=3 { w.add(&format!("Task {n}")); }
    let mut s = open(&w.root, 30); s.s.side = false;
    let todo = "head:status\t\tto do".to_string();
    go(&mut s, &todo); s.press(&["enter"]);
    s.load(); s.load();
    assert!(!s.text().contains("Task 1") && s.text().contains("▸ to do  3 tasks"), "a refresh keeps it folded: {}", s.text());
    let saved = sw::store::files::read_obj(&sw::store::files::data_dir().join("view.json")).unwrap();
    assert_eq!(saved.at("folded"), &json!([todo]), "kept with the view");
    let mut again = open(&w.root, 30); again.s.side = false;
    assert!(again.text().contains("▸ to do  3 tasks") && !again.text().contains("Task 1"), "and after the board closed: {}", again.text());
    // A filter shows every task that matches, folded or not; the fold is back after it.
    again.press(&["ctrl+f"]); again.typed("Task 2");
    assert!(again.text().contains("Task 2"), "{}", again.text());
    again.press(&["esc"]); assert!(!again.text().contains("Task 2"));
    // A task the selection is taken to (a jump from elsewhere: herdr's approve action, the popup's task) opens its heading.
    let two = again.s.items.iter().find(|r| r.s("title") == "Task 2").unwrap().t("id");
    again.s.sel = two.clone(); again.load();
    let t = again.text();
    assert!(t.contains("Task 2") && t.contains("Task 1") && !t.contains("▸ to do") && again.s.sel == two, "{t}");
}

#[test]
fn a_click_on_a_heading_folds_it_and_the_web_board_has_the_same_headings_a_tap_folds() {
    let w = world();
    for n in 1..=2 { w.add(&format!("Task {n}")); }
    let mut s = open(&w.root, 30); s.s.side = false;
    s.screen(120);
    let y = s.s.list_ids.iter().position(|id| id.starts_with("head:")).unwrap();
    s.click((s.s.list_at.unwrap().0 + y) as u16);
    assert!(s.text().contains("▸ to do  2 tasks"), "{}", s.text());
    // The web board: the headings by their ids, a tap on one folds it, and it says how many it holds.
    let groups = |v: &Value| v.at("list").arr("groups").to_vec();
    let b = web(&w, "board", json!({}));
    let g = groups(&b).into_iter().find(|g| g.s("name") == "to do").unwrap();
    assert_eq!((g.s("id"), g.b("folded"), g.arr("rows").len()), ("head:status\t\tto do", false, 2));
    let b = web(&w, "fold", json!({ "id": g.s("id"), "role": "watch" }));
    let g = groups(&b).into_iter().find(|g| g.s("name") == "to do").unwrap();
    assert!(g.b("folded") && g.arr("rows").is_empty() && g.i("count") == 2, "{g:?}");
    let b = web(&w, "fold", json!({ "id": g.s("id") }));
    assert_eq!(groups(&b).into_iter().find(|g| g.s("name") == "to do").unwrap().arr("rows").len(), 2);
}

#[test]
fn a_repos_heading_is_its_name_in_plain_bold_text_as_its_titles_and_its_key_muted() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    w.add("Mine 1");
    let mut s = open(&w.root, 30); s.s.side = false;
    if !s.s.by_repo { s.press(&["alt+s"]); }
    let lines = s.inked(100);
    // Each piece of the first line with `on`, with its ink and boldness: blanks left out.
    let pieces = |on: &str| { let l = lines.iter().find(|l| l.text().contains(on)).unwrap_or_else(|| panic!("no {on:?}")); l.0.iter().filter(|p| !p.t.trim().is_empty()).map(|p| (p.t.trim().to_string(), p.c, p.bold)).collect::<Vec<_>>() };
    let head = pieces("▾ proj");
    assert_eq!(head, vec![("▾".into(), Ink::Muted, false), ("proj".into(), Ink::Plain, true), ("· PRJ".into(), Ink::Muted, false)]);
    let title = pieces("Mine 1").into_iter().find(|p| p.0.contains("Mine 1")).unwrap();
    assert_eq!(title.1, Ink::Plain, "the heading is as loud as the titles under it: {title:?}");
    // The web board: the heading in plain text too (board.css: .sw-c-plain), its key on its own, dimmed there.
    let mut b = web(&w, "board", json!({}));
    if !b.at("flags").b("byRepo") { b = web(&w, "press", json!({ "id": "byRepo" })); }
    let g = b.at("list").arr("groups").iter().find(|g| g.s("name") == "proj").cloned().unwrap_or_else(|| panic!("{}", b.at("list")));
    assert_eq!((g.s("color"), g.s("key")), ("plain", "PRJ"), "{g:?}");
}

//! Autopilot on the board: a machine's or a project's row keeps its coordinator's own mark, and while on has the
//! board's own patterns only (a dimmed `[autopilot]` after its name, a grey status line under it, `1 on autopilot` in its
//! machine's summary; nothing while off), `Ctrl+O`, its panel, its settings page, the web board's row, a frame.
//! Other machines: autopilot_board/far.rs.
use crate::common;
use common::board::{open, web, Board};
use common::pilot::*;
use common::*;
use serde_json::json;
use sw::board::state::Whose;
use sw::json::*;
use sw::shown::ink::Ink;

mod far;

fn board(w: &World) -> Board { let mut b = open(&w.root, 40); b.s.side = false; b.load(); b }
fn line_with<'a>(lines: &'a [String], needle: &str) -> Option<&'a String> { lines.iter().skip(2).find(|l| l.contains(needle)) } // under the header and its filter row

#[test]
fn on_one_machine_the_top_rows_coordinator_is_the_top_level_one_and_its_autopilot_is_its_mark() {
    let w = pilot_world();
    let id = w.add("Fix the docs index");
    let mut b = board(&w);
    let top = sw::board::update::heads::machine_id(""); // this machine's: the only one
    let lines = b.screen(140);
    assert!(!lines.iter().skip(2).take_while(|l| !l.contains("╭")).any(|l| l.trim_start_matches(['▌', ' ']).starts_with("Coordinator")), "no row of a main Coordinator's: {lines:#?}");
    // On a task's row the autopilot key is no coordinator's; on the top row it is the top-level one's.
    b.s.sel = id.clone(); b.load();
    b.press(&["ctrl+o"]); assert!(b.text().contains("select a machine's or a project's row"), "{}", b.text());
    b.s.sel = top.clone(); b.load();
    b.press(&["ctrl+o"]);
    assert!(b.text().contains("Tell a coordinator its goal first."), "{}", b.text());
    ok(&["set", "autopilot.goal", "Keep the docs current", "--main-coordinator"]); b.s.msg.clear();
    b.press(&["ctrl+o"]);
    assert!(sw::autopilot::record::is_on(&sw::coordinator::co::Co::top(&w.socket)) && b.s.msg.is_empty(), "on says nothing: its row shows it ({})", b.s.msg);
    b.load();
    // the top row keeps its coordinator's mark; its tag after its name, its count in its summary, its status line under it
    let lines = b.screen(140);
    let at = lines.iter().position(|l| l.starts_with("▌○ ▾ ")).unwrap_or_else(|| panic!("{lines:#?}"));
    assert!(lines[at].contains(" [autopilot]  1 project · 1 task · 1 on autopilot"), "{lines:#?}");
    assert!(lines[at + 1].trim_start_matches(['▌', ' ']).starts_with("Autopilot · "), "{lines:#?}");
    assert!(!lines.iter().any(|l| l.contains("∞")), "no icon of its own: {lines:#?}");
    b.press(&["ctrl+o"]); b.load(); // the autopilot key again turns it off
    assert!(!sw::autopilot::record::is_on(&sw::coordinator::co::Co::top(&w.socket)));
    let off = b.screen(140).join("\n");
    assert!(!off.contains("[autopilot]") && !off.contains("on autopilot") && !off.contains("Autopilot ·"), "off: nothing of autopilot: {off}");
}

#[test]
fn a_projects_row_on_autopilot_keeps_its_mark_and_says_what_autopilot_does_under_it() {
    let w = pilot_world();
    w.add("One");
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.every", "1h"]);
    let mut b = board(&w);
    b.press(&["alt+s"]); b.load();
    let lines = b.screen(140);
    // its coordinator's own mark in the marks' column, its tag dimmed after its name, its status line under it
    let at = lines.iter().position(|l| l.contains("▾ proj")).unwrap_or_else(|| panic!("{lines:#?}"));
    assert_eq!(lines[at].trim(), "○ ▾ proj · PRJ [autopilot]");
    assert!(lines[at + 1].trim().starts_with("Autopilot · next check-in in ") && lines[at + 1].trim().ends_with("0 of 5 today"), "{lines:#?}");
    let inked = b.inked(140);
    let tag = inked.iter().flat_map(|l| l.0.iter()).find(|p| p.t.contains("[autopilot]")).unwrap_or_else(|| panic!("{inked:?}"));
    assert_eq!(tag.c, Ink::Muted, "dimmed, as a task's [research]");
    let under = inked.iter().flat_map(|l| l.0.iter()).find(|p| p.t.starts_with("Autopilot · ")).unwrap();
    assert_eq!(under.c, Ink::Soft, "in the grey of a task's status line");
    // at `needs you`: its status line says so; its mark stays its coordinator's
    sw::autopilot::record::change(&proj(&w), |ap, _| { ap.put("needsYou", true); true }).unwrap();
    b.load();
    let lines = b.screen(140);
    let at = lines.iter().position(|l| l.contains("▾ proj")).unwrap();
    assert!(lines[at].trim().starts_with("○ ") && lines[at + 1].trim() == "Needs you · its last check-ins went unanswered", "{lines:#?}");
    // one of its tasks under way: what it works on
    sw::autopilot::record::change(&proj(&w), |ap, _| { ap.put("needsYou", false); true }).unwrap();
    let mine = w.add("Its own");
    sw::autopilot::record::change(&proj(&w), |ap, _| { sw::autopilot::record::own_task(ap, &mine, "added", sw::clock::now()); true }).unwrap();
    sw::store::tasks::set(&mine, json!({ "stage": "working" })).unwrap();
    b.load();
    let lines = b.screen(140);
    let at = lines.iter().position(|l| l.contains("▾ proj")).unwrap();
    assert_eq!(lines[at + 1].trim(), format!("Working on {} · 0 of 5 today", short(&mine)), "{lines:#?}");
    // off: idle's mark
    ok(&["autopilot", "off", "--project", "proj"]); b.load();
    let lines = b.screen(140);
    assert_eq!(line_with(&lines, "▾ proj").unwrap().trim_end(), "   ○ ▾ proj · PRJ", "off and never started: idle's mark, as a task's: {lines:#?}");
}

#[test]
fn on_an_empty_board_the_top_row_has_the_top_level_coordinators_autopilot() {
    let w = pilot_world();
    ok(&["autopilot", "on", "--goal", "Docs", "--autopilot.hours", "00:00-00:01"]);
    let mut b = board(&w); assert_eq!(b.s.sel, sw::board::update::heads::machine_id(""), "no task: the top row is the selected row");
    let lines = b.screen(140);
    assert!(lines.iter().any(|l| l.starts_with("▌○ ▸ ") && l.contains("[autopilot]")), "asleep: its own mark, and its tag: {lines:#?}");
    assert!(lines.iter().any(|l| l.contains("Autopilot · asleep outside 00:00-00:01")), "{lines:#?}");
    // on the top row the autopilot key is its coordinator's
    b.press(&["ctrl+o"]); assert!(!sw::autopilot::record::is_on(&sw::coordinator::co::Co::top(&w.socket)));
}

#[test]
fn a_frame_of_the_board_with_coordinators_on_autopilot() {
    let w = pilot_world();
    sw::clock::set(Some(1_800_000_000_000));
    let id = w.add("Fix the docs index");
    ok(&["autopilot", "on", "--goal", "Keep the docs current", "--autopilot.every", "1h"]);
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.when", "done"]);
    tick(&w); tick(&w);
    // The repo's coordinator added the task: its last task's column has that task's mark and tag
    sw::autopilot::record::change(&proj(&w), |ap, _| { ap.put("tasks", vec![json!({ "id": id, "by": "added", "at": sw::clock::now() })]); ap.put("added", 1); true }).unwrap();
    let mut b = open(&w.root, 16); b.s.side = false; b.press(&["alt+s"]); b.load(); // wide, and narrow
    // The hostname differs by machine, and so does the filler after it on a line of the box: that filler goes.
    let host = sw::remote::check::hostname();
    // On the narrow screen the top row's summary is cut where the hostname's length says: it is left out there.
    let canon = |x: &String, narrow: bool| { let x = x.replace(&host, "HOST");
        if let Some(at) = x.find("HOST [autopilot]  ").filter(|_| narrow) { return format!("{}HOST [autopilot]  …", &x[..at]); }
        if !x.starts_with(['╭', '│']) || !x.contains("HOST") { return x; } let key = if x.contains("HOST's coordinator") { "HOST's coordinator" } else { "HOST" }; format!("{}{key} {}", &x[..x.find(key).unwrap()], if x.starts_with('╭') { "╮" } else { "│" }) };
    let got = [(b.screen(100), false), (b.screen(56), true)].map(|(l, n)| l.iter().map(|x| canon(x, n)).collect::<Vec<_>>().join("\n")).join("\n\n");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/frames/autopilot.txt");
    let want = std::fs::read_to_string(&path).unwrap_or_default();
    if want.is_empty() || std::env::var("UPDATE_FRAMES").is_ok() { std::fs::write(&path, format!("{got}\n")).unwrap(); return; }
    assert_eq!(got, want.trim_end_matches('\n'), "the frame changed: UPDATE_FRAMES=1 npm run check autopilot writes it again");
}

#[test]
fn a_coordinators_panel_beside_the_list_and_whole_and_nothing_of_autopilot_while_off() {
    let w = pilot_world();
    w.add("One");
    let mut b = open(&w.root, 50); b.s.side = true; b.press(&["alt+s"]); b.load();
    b.s.sel = sw::board::update::heads::group_id(&b.s, "", "proj"); b.load();
    let panel = |b: &mut Board| b.screen(160).iter().map(|l| l.split('│').nth(1).unwrap_or("").to_string()).collect::<Vec<_>>().join("\n");
    let t = panel(&mut b);
    assert!(t.contains("proj's coordinator") && t.contains("Status") && t.contains("Settings") && !t.contains("autopilot"), "off: nothing of autopilot: {t}");
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs stay current", "--autopilot.tasksPerDay", "3"]);
    tick(&w); tick(&w); b.load(); let t = b.screen(160).join("\n");
    assert!(t.contains("proj's coordinator [autopilot]") && t.contains("Goal") && t.contains("Docs stay current") && t.contains("0 of 3 added · 0 of 2 under way"), "{t}");
    assert!(t.contains("3 tasks a day") && t.contains("* this time only"), "this time's value is marked: {t}");
    b.press(&["alt+t"]); let t = b.screen(160).join("\n");
    assert!(t.contains("Wakes") && t.contains("Turned on") && t.contains("answered"), "whole, with its last wakes: {t}");
    b.press(&["esc"]); assert!(!b.s.full);
}

#[test]
fn the_settings_page_of_a_coordinators_row_saves_for_this_time_while_on_and_as_its_own_while_off() {
    let w = pilot_world();
    w.add("One");
    let mut b = board(&w); b.press(&["alt+s"]);
    b.s.sel = sw::board::update::heads::group_id(&b.s, "", "proj"); b.load();
    b.press(&["ctrl+e"]);
    assert_eq!(b.s.editing.as_ref().map(|e| e.whose.clone()), Some(Whose::Coord("project:proj".into(), "repo:proj".into())));
    b.rows = 120; let t = b.screen(140).join("\n");
    assert!(t.contains("This coordinator") && t.contains("autopilot.tasksPerDay"), "the project's row: its own rows first: {t}");
    let pick = |b: &mut Board, name: &str| { let at = sw::board::update::edit::rows(&b.s).iter().position(|r| r.s("name") == name).unwrap(); b.s.editing.as_mut().unwrap().at = at; };
    // off: a value is its own
    pick(&mut b, "autopilot.scope"); b.press(&["enter"]);
    let co = proj(&w); assert_eq!(sw::autopilot::record::words(&co, "autopilot.scope"), "all");
    let cfg: serde_json::Value = serde_json::from_str(&read(&sw::store::settings::places::config_file())).unwrap();
    assert_eq!(cfg.at("coordinators").at("projects").at("proj").at("autopilot").at("scope"), &json!("all"));
    // on: this time's, marked, and gone at off
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs"]); b.load();
    pick(&mut b, "autopilot.approve"); b.press(&["enter"]); b.load();
    assert_eq!(record(&co).at("once").at("autopilot.approve"), &json!("yes"));
    assert!(b.screen(140).join("\n").contains("saved for this time"));
    pick(&mut b, "autopilot.approve"); b.press(&["ctrl+u"]); assert!(record(&co).at("once").at("autopilot.approve").is_null());
    pick(&mut b, "autopilot.approve"); b.press(&["enter"]); ok(&["autopilot", "off", "--project", "proj"]);
    assert_eq!(sw::autopilot::record::words(&co, "autopilot.approve"), "no");
}

#[test]
fn the_web_board_has_the_same_marks_and_buttons() {
    let w = pilot_world();
    w.add("One");
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs"]);
    web(&w, "press", json!({ "id": "byRepo" }));
    let snap = web(&w, "board", json!({}));
    let groups = snap.at("list").arr("groups");
    assert!(!groups.iter().any(|g| g.b("main") || g.s("name") == "Coordinator"), "no row of the main Coordinator's: {snap}");
    let g = groups.iter().find(|g| g.s("name") == "proj").cloned().unwrap_or_else(|| panic!("{snap}"));
    let p = g.at("pilot"); assert!(p.arr("mark").iter().map(|x| x.t("t")).collect::<String>() == "○" && p.s("label") == "idle" && p.arr("cols").is_empty(), "its coordinator's own mark: {g}");
    assert!(p.arr("under").iter().map(|x| x.t("t")).collect::<String>().starts_with("Autopilot · "), "its status line: {g}");
    assert!(g.arr("words").iter().any(|x| x.s("t").contains("[autopilot]") && x.s("c") == "overlay0"), "its tag, dimmed: {g}");
    let snap = web(&w, "pick", json!({ "id": g.s("id"), "open": true }));
    assert!(snap.at("panel").b("coordinator"), "{}", snap.at("panel"));
    let labels: Vec<String> = snap.arr("actions").iter().map(|a| a.t("label")).collect();
    assert!(labels.contains(&"start".to_string()) && labels.contains(&"stop".to_string()) && labels.contains(&"autopilot off".to_string()), "{labels:?}");
    web(&w, "press", json!({ "id": "autopilot" }));
    assert!(!sw::autopilot::record::is_on(&proj(&w)));
}

#[test]
fn a_wake_in_the_chat_stands_under_autopilots_name_as_the_boards_words_do_and_says_only_why() {
    let said = |you: bool, text: &str| sw::agents::talk::Said { you, text: text.into(), at: 0 };
    let talk = vec![said(true, "Autopilot: AFS-4 finished, AFS-5 is stuck"), said(false, "Looked: nothing to do."), said(true, "Autopilot woke: autopilot was turned on. Goal: Docs.")];
    let v = sw::coordinator::look::Look { state: "idle".into(), status: Some("idle".into()), readable: true, talk, ..Default::default() };
    let parts = sw::board::view::talk::coord_talk(&v, "").0;
    assert_eq!(parts.iter().map(|p| format!("{}: {}", p.by, p.text)).collect::<Vec<_>>(), ["Autopilot: AFS-4 finished, AFS-5 is stuck", "Coordinator: Looked: nothing to do.", "Autopilot: autopilot was turned on. Goal: Docs."]);
    assert!(parts.iter().all(|p| p.tone.is_empty()), "no colour of its own: {parts:?}");
}

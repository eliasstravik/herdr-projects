//! Autopilot on the board: the main Coordinator's row on top (what it sums up of the others), a coordinator's
//! autopilot line only while it is on, `Tab` and `Ctrl+X` on a coordinator's row, its panel beside the list and whole
//! (with its last wakes), its settings page (this time's values while on), the web board's same row, and a frame of it.
use crate::common;
use common::board::{open, web, Board};
use common::pilot::*;
use common::*;
use serde_json::json;
use sw::board::state::Whose;
use sw::board::update::pilot::main_id;
use sw::json::*;

fn board(w: &World) -> Board { let mut b = open(&w.root, 40); b.s.side = false; b.load(); b }
fn line_with<'a>(lines: &'a [String], needle: &str) -> Option<&'a String> { lines.iter().find(|l| l.contains(needle)) }

#[test]
fn the_main_coordinators_row_is_on_top_and_shows_autopilot_only_while_it_is_on() {
    let w = pilot_world();
    let id = w.add("Fix the docs index");
    let mut b = board(&w);
    assert_eq!(b.s.sel, id, "the list still opens on its first task");
    let lines = b.screen(140);
    assert_eq!(lines[2].trim(), "Coordinator", "{lines:#?}");
    assert!(!b.text().contains("autopilot"), "off: nothing of autopilot shows");
    b.press(&["up", "up"]);
    assert_eq!(b.s.sel, main_id(""), "the top row is reached with ↑");
    // Tab with no goal saved: refused, and said
    b.press(&["tab"]);
    assert!(b.text().contains("Tell a coordinator its goal first."), "{}", b.text());
    ok(&["set", "autopilot.goal", "Keep the docs current", "--main-coordinator"]);
    b.press(&["tab"]);
    assert!(sw::autopilot::record::is_on(&sw::coordinator::co::Co::top(&w.socket)));
    b.load();
    let lines = b.screen(140);
    let top = line_with(&lines, "Coordinator").unwrap();
    assert!(top.contains("Coordinator   autopilot on · every 6h + events"), "beside its name where it fits: {top}");
    let narrow = b.screen(40);
    let bare = |l: &String| l.trim_start_matches(['▌', ' ']).to_string();
    let at = narrow.iter().position(|l| bare(l) == "Coordinator").unwrap_or_else(|| panic!("{narrow:#?}"));
    assert!(bare(&narrow[at + 1]).starts_with("autopilot on"), "on the line under it where it does not: {narrow:#?}");
    // Tab again says it is on; ctrl+x turns it off; again says it is off
    b.press(&["tab"]); assert!(b.text().contains("is on autopilot already"), "{}", b.text());
    b.press(&["ctrl+x"]); b.load();
    assert!(!sw::autopilot::record::is_on(&sw::coordinator::co::Co::top(&w.socket)));
    let lines = b.screen(140);
    assert_eq!(line_with(&lines, "Coordinator").unwrap().trim_start_matches(['▌', ' ']), "Coordinator", "off: no line, no mark");
    b.press(&["ctrl+x"]); assert!(b.text().contains("is not on autopilot"), "{}", b.text());
    let hints = b.screen(140).last().cloned().unwrap();
    assert!(hints.contains("tab autopilot on") && hints.contains("ctrl+x off"), "{hints}");
}

#[test]
fn the_main_row_sums_up_the_others_and_a_repos_heading_has_its_own_line() {
    let w = pilot_world();
    w.add("One");
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.every", "1h"]);
    let mut b = board(&w);
    let lines = b.screen(140);
    assert!(line_with(&lines, "Coordinator").unwrap().contains("Coordinator · 1 other on"), "{lines:#?}");
    b.press(&["alt+s"]); b.load();
    let lines = b.screen(140);
    let head = line_with(&lines, "▾ proj").unwrap();
    assert!(head.contains("autopilot on · every 1h + events"), "{head}");
    // at `needs you`, the main row says so
    sw::autopilot::record::change(&proj(&w), |ap, _| { ap.put("needsYou", true); true }).unwrap();
    b.load();
    assert!(line_with(&b.screen(140), "Coordinator").unwrap().contains("1 other on · 1 needs you"));
    assert!(line_with(&b.screen(140), "▾ proj").unwrap().contains("autopilot needs you"));
}

#[test]
fn a_coordinators_panel_beside_the_list_and_whole_and_nothing_of_autopilot_while_off() {
    let w = pilot_world();
    w.add("One");
    let mut b = open(&w.root, 50); b.s.side = true; b.press(&["alt+s"]); b.load();
    b.s.sel = sw::board::update::heads::group_id(&b.s, "", "proj"); b.load();
    let panel = |b: &mut Board| b.screen(160).iter().map(|l| l.split('│').nth(1).unwrap_or("").to_string()).collect::<Vec<_>>().join("\n");
    let t = panel(&mut b);
    assert!(t.contains("proj coordinator") && t.contains("Status") && t.contains("Settings") && !t.contains("autopilot"), "off: nothing of autopilot: {t}");
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs stay current", "--autopilot.tasksPerDay", "3"]);
    tick(&w); tick(&w);
    b.load();
    let t = b.screen(160).join("\n");
    assert!(t.contains("· autopilot on") && t.contains("Goal") && t.contains("Docs stay current") && t.contains("0 of 3 added · 0 of 2 under way"), "{t}");
    assert!(t.contains("3 tasks a day") && t.contains("* this time only"), "this time's value is marked: {t}");
    b.press(&["alt+t"]);
    let t = b.screen(160).join("\n");
    assert!(t.contains("Wakes") && t.contains("autopilot was turned on") && t.contains("answered"), "whole, with its last wakes: {t}");
    b.press(&["esc"]);
    assert!(!b.s.full);
}

#[test]
fn the_settings_page_of_a_coordinators_row_saves_for_this_time_while_on_and_as_its_own_while_off() {
    let w = pilot_world();
    let mut b = board(&w);
    b.s.sel = main_id(""); b.load();
    b.press(&["ctrl+e"]);
    assert_eq!(b.s.editing.as_ref().map(|e| e.whose.clone()), Some(Whose::Coord("main".into(), String::new())));
    b.rows = 120;
    let t = b.screen(140).join("\n");
    assert!(t.contains("This coordinator") && t.contains("autopilot.tasksPerDay") && !t.contains("merge.method"), "the main row: its own rows and nothing more: {t}");
    let pick = |b: &mut Board, name: &str| { let at = sw::board::update::edit::rows(&b.s).iter().position(|r| r.s("name") == name).unwrap(); b.s.editing.as_mut().unwrap().at = at; };
    // off: a value is its own
    pick(&mut b, "autopilot.scope"); b.press(&["enter"]);
    let co = sw::coordinator::co::Co::top(&w.socket);
    assert_eq!(sw::autopilot::record::words(&co, "autopilot.scope"), "all");
    let cfg: serde_json::Value = serde_json::from_str(&read(&sw::store::settings::places::config_file())).unwrap();
    assert_eq!(cfg.at("coordinators").at("main").at("autopilot").at("scope"), &json!("all"));
    // on: this time's, marked, and gone at off
    ok(&["autopilot", "on", "--goal", "Docs"]); b.load();
    pick(&mut b, "autopilot.approve"); b.press(&["enter"]); b.load();
    assert_eq!(record(&co).at("once").at("autopilot.approve"), &json!("yes"));
    assert!(b.screen(140).join("\n").contains("saved for this time"));
    pick(&mut b, "autopilot.approve"); b.press(&["ctrl+u"]);
    assert!(record(&co).at("once").at("autopilot.approve").is_null());
    pick(&mut b, "autopilot.approve"); b.press(&["enter"]);
    ok(&["autopilot", "off"]);
    assert_eq!(sw::autopilot::record::words(&co, "autopilot.approve"), "no");
}

#[test]
fn the_web_board_has_the_same_row_line_and_buttons() {
    let w = pilot_world();
    w.add("One");
    ok(&["autopilot", "on", "--goal", "Docs"]);
    let snap = web(&w, "board", json!({}));
    let main = snap.at("list").arr("groups").iter().find(|g| g.b("main")).cloned().unwrap_or_else(|| panic!("{snap}"));
    assert_eq!(main.s("name"), "Coordinator");
    assert!(main.arr("pilot").iter().map(|p| p.t("t")).collect::<String>().starts_with("autopilot on"), "{main}");
    let snap = web(&w, "pick", json!({ "id": main_id(""), "open": true }));
    assert!(snap.at("panel").b("coordinator"), "{}", snap.at("panel"));
    let labels: Vec<String> = snap.arr("actions").iter().map(|a| a.t("label")).collect();
    assert!(labels.contains(&"autopilot on".to_string()) && labels.contains(&"off".to_string()), "{labels:?}");
    web(&w, "press", json!({ "id": "stop" }));
    assert!(!sw::autopilot::record::is_on(&sw::coordinator::co::Co::top(&w.socket)));
}

#[test]
fn a_frame_of_the_board_with_coordinators_on_autopilot() {
    let w = pilot_world();
    sw::clock::set(Some(1_800_000_000_000));
    w.add("Fix the docs index");
    ok(&["autopilot", "on", "--goal", "Keep the docs current", "--autopilot.every", "1h"]);
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.when", "done"]);
    tick(&w); tick(&w);
    let mut b = open(&w.root, 24); b.s.side = false; b.press(&["alt+s"]); b.load();
    let got = b.screen(100).join("\n").replace(&sw::remote::check::hostname(), "HOST");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/frames/autopilot.txt");
    let want = std::fs::read_to_string(&path).unwrap_or_default();
    if want.is_empty() || std::env::var("UPDATE_FRAMES").is_ok() { std::fs::write(&path, format!("{got}\n")).unwrap(); return; }
    assert_eq!(got, want.trim_end_matches('\n'), "the frame changed: UPDATE_FRAMES=1 npm run check autopilot writes it again");
}

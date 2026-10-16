//! Where an automation stands in the list: in the part of the machine that has it, as its tasks do. By status, a
//! machine's automations are a group of their own after its tasks' groups, folding as they do; by repo, each stands
//! under its repo's heading after that repo's tasks. A folded heading says what it holds (`1 task, 1 automation`), and
//! the web board has the same groups and the same words.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::{json, Value};
use sw::automations::list::{self as l, Add};
use sw::automations::triggers::parse_schedule;
use sw::board::update::heads::{group_id, machine_id};
use sw::board::update::{update, Event};
use sw::json::*;

fn t(s: &mut Board, width: usize) -> String { s.screen(width).join("\n") }
fn like(text: &str, re: &str) { assert!(regex::Regex::new(re).unwrap().is_match(text), "{re}\n{text}"); }
/// vm-b answers the board's "list" with one task of its repo `far` and one automation there.
fn poll(s: &mut Board) {
    let now = sw::clock::now();
    let auto = json!({ "id": "0a1b2c3d", "name": "Theirs nightly", "prompt": "Audit it.", "trigger": parse_schedule("every 5m").unwrap(), "repo": "far", "root": "/home/x/far", "settings": {}, "on": true, "created": now, "since": now, "nextAt": now + 4 * 60000 + 500, "shown": { "firings": [], "own": [], "settings": [] } });
    let j = json!({ "host": "vm-b", "version": "0.2.0", "tasks": [{ "id": "far/7", "title": "Their task", "stage": "todo", "state": "idle", "agent": "", "app": "", "msg": "", "note": "", "at": now, "created": now, "snap": { "folder": "~/far" } }], "automations": [auto] });
    let _ = update(&mut s.s, Event::Remote { label: "vm-b".into(), answer: Ok(j) });
}
/// The web board's groups as (name, count, its rows' ids).
fn groups(s: &Board) -> Vec<(String, Value, Vec<String>)> {
    let l = sw::webui::model::list::list_of(&s.s, &s.theme);
    l.arr("groups").iter().map(|g| (g.t("name"), g.at("count").clone(), g.arr("rows").iter().map(|r| r.t("id")).collect())).collect()
}

#[test]
fn each_machines_automations_stand_in_its_own_part_and_fold_with_it_by_status_and_by_repo() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-b" }), ..Default::default() });
    sw::clock::set(Some(sw::clock::now()));
    w.add("Mine");
    let mine = l::add(Add { name: "Look for old dependencies", prompt: "Check.", trigger: Some(parse_schedule("every 2h").unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: None }).unwrap();
    let row = format!("auto:{}", mine.s("id"));
    let mut s = open(&w.root, 44); s.s.side = false;
    let _ = update(&mut s.s, Event::Machines(sw::remote::machines()));
    poll(&mut s);
    s.press(&["alt+m"]); assert_eq!(s.s.scope, "*");
    let here = sw::board::update::heads::label(&s.s, "");
    // By status: each machine's automations after its own tasks, under a heading of their own; no machine on a row.
    let text = t(&mut s, 120);
    like(&text, &format!(r"\n ▾ {} ─+\n ▾ to do\n[ ▌]○ PRJ-1 Mine +0s ago\n\n ▾ automations\n ⟳ on {{4}}Look for old dependencies {{2}}every 2h +next in {{2}}2h\n\n ▾ vm-b ─+\n ▾ to do\n ○ #7 +Their task +0s ago\n\n ▾ automations\n ⟳ on {{4}}Theirs nightly {{2}}every 5m · far +next in {{2}}4m\n", regex::escape(&here)));
    assert!(!text.contains("on vm-b"), "{text}");
    // The web board: the same groups, in the same order.
    assert_eq!(groups(&s).iter().map(|g| (g.0.clone(), g.2.clone())).collect::<Vec<_>>(), vec![(here.clone(), vec![]), ("to do".into(), vec!["proj/1".into()]), ("automations".into(), vec![row.clone()]),
        ("vm-b".into(), vec![]), ("to do".into(), vec!["vm-b:far/7".into()]), ("automations".into(), vec!["vm-b:auto:0a1b2c3d".into()])]);
    // The arrows go through them where they stand: this machine's automation, then vm-b's heading.
    s.s.sel = "proj/1".into(); s.load();
    s.press(&["down"]); assert_eq!(s.s.sel, group_id(&s.s, "", "automations"));
    s.press(&["down"]); assert_eq!(s.s.sel, row);
    s.press(&["down"]); assert_eq!(s.s.sel, machine_id("vm-b"));
    // Its group folded: one line that says how many; the selection on it, its keys still act once it is open again.
    let g = group_id(&s.s, "", "automations");
    s.s.sel = g.clone(); s.load(); s.press(&["enter"]);
    let text = t(&mut s, 120);
    like(&text, r"\n▌▸ automations {2}1 automation\n"); assert!(!text.contains("Look for old"), "{text}");
    assert_eq!(groups(&s)[2].1, json!("1 automation"));
    s.press(&["enter", "down", "space"]); assert_eq!(s.s.sel, row);
    assert_eq!(sw::store::automations::automation(mine.s("id")).unwrap().at("on"), &json!(false), "space on its row in its machine's part turns it off");
    s.press(&["space"]);
    // This machine folded: its one line counts the tasks and the automations it hides.
    s.s.sel = machine_id(""); s.load(); s.press(&["enter"]);
    like(&t(&mut s, 120), &format!(r"\n▌▸ {} {{2}}1 task, 1 automation ─+\n", regex::escape(&here)));
    assert_eq!(groups(&s)[0].1, json!("1 task, 1 automation"));
    s.press(&["enter"]);
    // Taken to an automation in a folded group (a jump, a firing's link): its group unfolds.
    s.s.sel = g.clone(); s.load(); s.press(&["enter"]); s.s.sel = row.clone(); s.load();
    assert!(t(&mut s, 120).contains("Look for old dependencies"));
    // By repo: under its repo's heading, after that repo's tasks; no heading of their own, and the repo is not said again.
    s.press(&["alt+s"]); assert!(s.s.by_repo);
    let text = t(&mut s, 120);
    like(&text, r"\n ▾ proj · PRJ\n[ ▌]○ PRJ-1 Mine +0s ago\n[ ▌]⟳ on {4}Look for old dependencies {2}every 2h +next in {2}2h\n");
    like(&text, r"\n ▾ far\n ○ #7 +Their task +0s ago\n ⟳ on {4}Theirs nightly {2}every 5m +next in {2}4m\n");
    assert!(!text.contains(" automations\n"), "{text}");
    // A repo folded says what it holds too.
    s.s.sel = group_id(&s.s, "vm-b", "far"); s.load(); s.press(&["enter"]);
    like(&t(&mut s, 120), r"\n▌▸ far {2}1 task, 1 automation\n");
    assert_eq!(groups(&s).iter().find(|g| g.0 == "far").unwrap().1, json!("1 task, 1 automation"));
    // On one machine's own board, its automations stand with its tasks there.
    s.press(&["enter", "alt+s", "alt+m"]); assert_eq!(s.s.scope, "vm-b");
    like(&t(&mut s, 120), r"\n ▾ to do\n[ ▌]○ #7 +Their task +0s ago\n\n ▾ automations\n[ ▌]⟳ on {2}Theirs nightly {2}every 5m · far +next in {2}4m\n");
}

#[test]
fn another_machines_repo_with_only_automations_never_takes_this_machines_key() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-b" }), ..Default::default() });
    sw::clock::set(Some(sw::clock::now()));
    w.add("Mine");
    let mut s = open(&w.root, 44); s.s.side = false;
    let _ = update(&mut s.s, Event::Machines(sw::remote::machines()));
    // vm-b has one task in `far`, and one automation in a repo named as this machine's (here `proj`, whose key is PRJ).
    let now = sw::clock::now();
    let auto = json!({ "id": "0a1b2c3d", "name": "Theirs nightly", "prompt": "Audit it.", "trigger": parse_schedule("every 5m").unwrap(), "repo": "proj", "root": "/home/x/proj", "settings": {}, "on": true, "created": now, "since": now, "nextAt": now + 4 * 60000 + 500, "shown": { "firings": [], "own": [], "settings": [] } });
    let _ = update(&mut s.s, Event::Remote { label: "vm-b".into(), answer: Ok(json!({ "host": "vm-b", "version": "0.2.0", "tasks": [{ "id": "far/3", "key": "FAR", "title": "Their task", "stage": "todo", "state": "idle", "agent": "", "app": "", "msg": "", "note": "", "at": now, "created": now, "snap": { "folder": "~/far" } }], "automations": [auto] })) });
    s.press(&["alt+m", "alt+s"]); assert!(s.s.scope == "*" && s.s.by_repo);
    let text = t(&mut s, 120);
    like(&text, r"\n ▾ proj · PRJ\n[ ▌]○ PRJ-1 Mine"); // this machine's own repo keeps its key
    like(&text, r"\n ▾ vm-b ─+\n ▾ proj\n ⟳ on {4}Theirs nightly"); // vm-b's sent none: no key, never this machine's
    let keys = |s: &Board| sw::webui::model::list::list_of(&s.s, &s.theme).arr("groups").iter().filter(|g| g.s("name") == "proj").map(|g| g.t("key")).collect::<Vec<_>>();
    assert_eq!(keys(&s), ["PRJ", ""]);
    // Folded, with no row under it, the same: no key.
    s.s.sel = group_id(&s.s, "vm-b", "proj"); s.load(); s.press(&["enter"]);
    like(&t(&mut s, 120), r"\n ▾ vm-b ─+\n▌▸ proj {2}1 automation\n");
    assert_eq!(keys(&s), ["PRJ", ""]);
    // A repo of theirs that has tasks keeps the key they came with, folded too.
    s.s.sel = group_id(&s.s, "vm-b", "far"); s.load(); s.press(&["enter"]);
    like(&t(&mut s, 120), r"\n▌▸ far · FAR {2}1 task\n");
}

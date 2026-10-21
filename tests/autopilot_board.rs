//! Autopilot on the board: a coordinator's row drawn as a task's while on (mark, columns, its line under it, the theme's
//! own inks; nothing while off), the main Coordinator's on top, `Tab` and `Ctrl+X` there, its panel, its settings page,
//! the web board's row, a frame. Other machines: their coordinators' rows and keys, what the main Coordinator sees of
//! them (only what an open board heard), its adds there, their slot taken here and given back when refused.
use crate::common;
use common::board::{open, web, Board};
use common::pilot::*;
use common::*;
use serde_json::json;
use sw::board::state::Whose;
use sw::board::update::pilot::main_id;
use sw::json::*;
use sw::shown::ink::Ink;

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
    ok(&["set", "autopilot.goal", "Keep the docs current", "--main-coordinator"]); b.s.msg.clear();
    b.press(&["tab"]);
    assert!(sw::autopilot::record::is_on(&sw::coordinator::co::Co::top(&w.socket)) && b.s.msg.is_empty(), "on says nothing: its row shows it ({})", b.s.msg);
    b.load();
    let lines = b.screen(140);
    // A task's row: autopilot's mark in the marks' column, the timing in the columns at the right edge, what it does under it
    let at = lines.iter().position(|l| l.contains("◎   Coordinator")).unwrap_or_else(|| panic!("{lines:#?}"));
    assert!(lines[at].ends_with("0 of 5 added  next in 10m") && lines[at].chars().count() == 140 && lines[at + 1].trim_start_matches(['▌', ' ']).starts_with("On · wakes every 6h + events"), "{lines:#?}");
    // Narrow: the columns give way, the least needed first; when it next wakes goes to the end of the line under it
    let (narrow, narrower) = (b.screen(40), b.screen(26));
    assert!(line_with(&narrow, "◎   Coordinator").map(|l| l.ends_with(" next in 10m") && !l.contains("added")).unwrap_or(false), "{narrow:#?}");
    let at = narrower.iter().position(|l| l.contains("◎   Coordinator")).unwrap();
    assert!(!narrower[at].contains("next") && narrower[at + 1].contains("On · wakes every") && narrower[at + 1].chars().count() <= 26, "{narrower:#?}");
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
    let head = line_with(&lines, "◎ ▾ proj").unwrap_or_else(|| panic!("{lines:#?}"));
    assert!(head.contains("◎ ▾ proj · PRJ") && head.trim_end().ends_with("0 of 5 added  next in 10m"), "{head}");
    // at `needs you`, the main row says so
    sw::autopilot::record::change(&proj(&w), |ap, _| { ap.put("needsYou", true); true }).unwrap();
    b.load();
    assert!(line_with(&b.screen(140), "Coordinator").unwrap().contains("1 other on · 1 needs you"));
    // Its mark and its state word in red, as a task's that needs you; nothing on the list in a colour of its own
    let inked = b.inked(140);
    let has = |what: &str, bold: bool| inked.iter().any(|l| l.0.iter().any(|p| p.t == what && p.c == Ink::Red && p.bold == bold));
    assert!(has("◎", true) && has("Needs you", false) && b.text().contains("Needs you · its last wakes went unanswered"), "{inked:?}");
    assert!(inked.iter().all(|l| l.0.iter().all(|p| p.c != Ink::Pilot)), "the theme's own inks only");
}

#[test]
fn on_an_empty_board_the_main_coordinators_row_and_its_line_show() {
    let w = pilot_world();
    ok(&["autopilot", "on", "--goal", "Docs", "--autopilot.hours", "00:00-00:01"]);
    let mut b = board(&w); assert!(b.s.sel.is_empty(), "nothing selected");
    let lines = b.screen(140); let at = lines.iter().position(|l| l.contains("◎   Coordinator")).unwrap_or_else(|| panic!("{lines:#?}"));
    assert!(lines[at].ends_with("waits for its hours") && lines[at + 1].trim() == "Asleep · its hours are 00:00-00:01", "{lines:#?}");
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
    let got = [b.screen(100), b.screen(56)].map(|l| l.join("\n")).join("\n\n").replace(&sw::remote::check::hostname(), "HOST");
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
    assert!(t.contains("proj coordinator") && t.contains("Status") && t.contains("Settings") && !t.contains("autopilot"), "off: nothing of autopilot: {t}");
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs stay current", "--autopilot.tasksPerDay", "3"]);
    tick(&w); tick(&w); b.load(); let t = b.screen(160).join("\n");
    assert!(t.contains("· ◎ autopilot on") && t.contains("Goal") && t.contains("Docs stay current") && t.contains("0 of 3 added · 0 of 2 under way"), "{t}");
    assert!(t.contains("3 tasks a day") && t.contains("* this time only"), "this time's value is marked: {t}");
    b.press(&["alt+t"]); let t = b.screen(160).join("\n");
    assert!(t.contains("Wakes") && t.contains("autopilot was turned on") && t.contains("answered"), "whole, with its last wakes: {t}");
    b.press(&["esc"]); assert!(!b.s.full);
}

#[test]
fn the_settings_page_of_a_coordinators_row_saves_for_this_time_while_on_and_as_its_own_while_off() {
    let w = pilot_world();
    let mut b = board(&w);
    b.s.sel = main_id(""); b.load();
    b.press(&["ctrl+e"]);
    assert_eq!(b.s.editing.as_ref().map(|e| e.whose.clone()), Some(Whose::Coord("main".into(), String::new())));
    b.rows = 120; let t = b.screen(140).join("\n");
    assert!(t.contains("This coordinator") && t.contains("autopilot.tasksPerDay") && !t.contains("merge.method"), "the main row: its own rows and nothing more: {t}");
    let pick = |b: &mut Board, name: &str| { let at = sw::board::update::edit::rows(&b.s).iter().position(|r| r.s("name") == name).unwrap(); b.s.editing.as_mut().unwrap().at = at; };
    // off: a value is its own
    pick(&mut b, "autopilot.scope"); b.press(&["enter"]);
    let co = sw::coordinator::co::Co::top(&w.socket); assert_eq!(sw::autopilot::record::words(&co, "autopilot.scope"), "all");
    let cfg: serde_json::Value = serde_json::from_str(&read(&sw::store::settings::places::config_file())).unwrap();
    assert_eq!(cfg.at("coordinators").at("main").at("autopilot").at("scope"), &json!("all"));
    // on: this time's, marked, and gone at off
    ok(&["autopilot", "on", "--goal", "Docs"]); b.load();
    pick(&mut b, "autopilot.approve"); b.press(&["enter"]); b.load();
    assert_eq!(record(&co).at("once").at("autopilot.approve"), &json!("yes"));
    assert!(b.screen(140).join("\n").contains("saved for this time"));
    pick(&mut b, "autopilot.approve"); b.press(&["ctrl+u"]); assert!(record(&co).at("once").at("autopilot.approve").is_null());
    pick(&mut b, "autopilot.approve"); b.press(&["enter"]); ok(&["autopilot", "off"]);
    assert_eq!(sw::autopilot::record::words(&co, "autopilot.approve"), "no");
}

#[test]
fn the_web_board_has_the_same_row_line_and_buttons() {
    let w = pilot_world();
    w.add("One");
    ok(&["autopilot", "on", "--goal", "Docs"]);
    let snap = web(&w, "board", json!({}));
    let main = snap.at("list").arr("groups").iter().find(|g| g.b("main")).cloned().unwrap_or_else(|| panic!("{snap}"));
    assert_eq!(main.s("name"), "Coordinator"); let p = main.at("pilot"); assert!(p.arr("mark").iter().map(|x| x.t("t")).collect::<String>() == "◎" && p.arr("under").iter().map(|x| x.t("t")).collect::<String>().starts_with("On · wakes every 6h"), "{main}");
    assert_eq!(p.arr("cols").len(), 2, "today's adds and when it next wakes: {main}");
    let snap = web(&w, "pick", json!({ "id": main_id(""), "open": true }));
    assert!(snap.at("panel").b("coordinator"), "{}", snap.at("panel"));
    let labels: Vec<String> = snap.arr("actions").iter().map(|a| a.t("label")).collect();
    assert!(labels.contains(&"autopilot on".to_string()) && labels.contains(&"off".to_string()), "{labels:?}");
    web(&w, "press", json!({ "id": "stop" }));
    assert!(!sw::autopilot::record::is_on(&sw::coordinator::co::Co::top(&w.socket)));
}

fn far_world() -> (World, crate::far::Far) { let w = pilot_world_with(json!({ "board.machines": "vm-b" })); let f = crate::far::Far::new(&w.base); (w, f) }
fn far_list(f: &crate::far::Far, pilots: serde_json::Value, tasks: serde_json::Value) {
    f.answers("vm-b", "list", &json!({ "host": "vm-b", "version": "9.9.9", "coordinator": { "state": "idle" }, "coordinators": { "machine": { "state": "idle" } }, "autopilot": pilots, "tasks": tasks }));
}
/// vm-b's list, as an open board asks for it (what the main Coordinator's autopilot sees of it).
fn heard(f: &crate::far::Far, tasks: serde_json::Value) { far_list(f, json!({}), tasks); let _ = sw::remote::ask_list(&sw::remote::machines()[0], 20000); }
/// The main Coordinator on autopilot, in its wake's turn, in its pane.
fn main_in_wake(w: &World, extra: &[&str]) -> sw::coordinator::co::Co {
    let main = sw::coordinator::co::Co::top(&w.socket);
    w.script(json!({ "coordinator": [{ "status": "working" }] }));
    let mut a = vec!["autopilot", "on", "--goal", "Docs", "--autopilot.every", "none"]; a.extend_from_slice(extra); ok(&a);
    tick(w); in_pane(w, &pane_of(&main));
    main
}

#[test]
fn another_machines_coordinator_shows_its_line_and_takes_its_keys_there_and_an_older_one_says_to_update() {
    let (w, f) = far_world();
    far_list(&f, json!({ "machine": { "on": true, "state": "working", "line": "autopilot working · every 2h + events", "activity": "Working · woke for its timer", "added": "1 of 5 added", "next": "", "last": "#3", "lastId": "web/3" } }), json!([]));
    let mut b = board(&w);
    sw::board::update::update(&mut b.s, sw::board::update::Event::Machines(sw::remote::machines()));
    for m in sw::remote::machines() { let a = sw::remote::ask_list(&m, 20000); sw::board::update::update(&mut b.s, sw::board::update::Event::Remote { label: m.label.clone(), answer: a }); }
    b.press(&["alt+m"]); assert_eq!(b.s.scope, "*");
    let lines = b.screen(160); let at = lines.iter().position(|l| l.contains("◎ ▸ vm-b")).unwrap_or_else(|| panic!("{lines:#?}"));
    assert!(lines[at].contains("──") && lines[at].ends_with("1 of 5 added  last #3"), "its rule, then its columns: {lines:#?}");
    assert!(lines[at + 1].contains("Working · woke for its timer"), "{lines:#?}");
    assert!(line_with(&lines, "Coordinator").unwrap().contains("1 other on"), "the main row sums up every machine's: {lines:#?}");
    f.answers("vm-b", "do", &json!({ "ok": true, "msg": "Autopilot off for the vm-b coordinator." }));
    b.s.sel = sw::board::update::heads::machine_id("vm-b"); b.press(&["ctrl+x"]);
    let (who, what, req) = f.asked().pop().unwrap();
    assert_eq!((who.as_str(), what.as_str(), req.s("action"), req.s("to")), ("vm-b", "do", "autopilot-off", "machine"));
    // a machine on an older version sends no autopilot: the keys say to update it
    sw::board::update::update(&mut b.s, sw::board::update::Event::Remote { label: "vm-b".into(), answer: Ok(json!({ "host": "vm-b", "version": "1.0.0", "tasks": [] })) });
    b.press(&["tab"]); assert!(b.text().contains("runs an older version that has no autopilot: update herdr-projects there first"), "{}", b.text());
    // the door there: Tab and Ctrl+X from another machine's board
    ok(&["set", "autopilot.goal", "Tidy", "--machine-coordinator"]);
    let a = sw::remote::door("do", &json!({ "action": "autopilot-on", "id": "", "to": "machine" })).unwrap();
    assert!(a.b("ok") && sw::autopilot::record::is_on(&sw::coordinator::co::Co::machine(&w.socket)), "{a}");
}

#[test]
fn the_main_coordinator_sees_other_machines_only_as_an_open_board_last_heard_them() {
    let (w, f) = far_world();
    let main = sw::coordinator::co::Co::top(&w.socket);
    ok(&["autopilot", "on", "--goal", "Docs", "--autopilot.every", "none", "--autopilot.gap", "1m"]);
    tick(&w); tick(&w);
    assert!(sw::autopilot::limits::board_tasks().iter().all(|t| !t.b("host")), "no board heard vm-b: its tasks are not seen");
    heard(&f, json!([{ "id": "web/3", "title": "Theirs", "stage": "todo", "created": sw::clock::now() + 1 }])); // as an open board asks
    assert!(sw::autopilot::limits::board_tasks().iter().any(|t| t.s("id") == "vm-b:web/3" && t.s("host") == "vm-b"));
    minutes(2); tick(&w);
    assert!(prompts_of(&w, &main).last().unwrap().contains("vm-b:web/3 was added") || prompts_of(&w, &main).last().unwrap().contains("web/3 was added"), "{:?}", prompts_of(&w, &main));
    assert!(sw::autopilot::events::look(&proj(&w), &mut Default::default(), &sw::autopilot::limits::board_tasks(), "added", false, 1).is_empty(), "never a project coordinator's");
    minutes(4); assert!(sw::autopilot::limits::board_tasks().iter().all(|t| !t.b("host")), "not heard for three minutes: not seen");
}

#[test]
fn an_add_on_another_machine_takes_its_slot_here_carries_the_record_and_gives_the_slot_back_when_refused() {
    let (w, f) = far_world();
    let main = main_in_wake(&w, &[]);
    f.answers("vm-b", "add", &json!({ "ok": true, "id": "web/7", "msg": "web/7: added on vm-b, an agent is on it" }));
    let said = ok(&["add", "Fix their docs", "--repo", "web", "--on", "vm-b"]); assert!(said.contains("web/7: added on vm-b"), "{said}");
    let (_, what, req) = f.asked().pop().unwrap(); assert_eq!(what, "add");
    assert_eq!((req.at("autopilot").s("coordinator"), req.at("autopilot").s("machine"), req.at("autopilot").at("repoFileWaits"), req.b("delegate")), ("main", sw::remote::check::hostname().as_str(), &json!(true), true));
    assert!(sw::autopilot::record::task_rec(&record(&main), "vm-b:web/7").is_some()); assert_eq!(record(&main).i("added"), 1);
    f.answers("vm-b", "add", &json!({ "ok": false, "msg": "Unknown request add" })); // refused there (an older version): the slot and the day's add come back
    assert!(refused(&["add", "More", "--repo", "web", "--on", "vm-b"]).contains("runs an older version of herdr-projects that cannot add a task from another machine"));
    assert_eq!(record(&main).i("added"), 1); assert_eq!(record(&main).arr("tasks").len(), 1);
    in_pane(&w, ""); // the door's side: the task is added here with its record, naming the machine that added it
    let a = sw::remote::door("add", &json!({ "title": "From afar", "repo": "proj", "own": {}, "delegate": false, "from": "laptop", "autopilot": { "coordinator": "main", "name": "Coordinator", "repoFileWaits": false }, "settings": {} })).unwrap();
    let t = task(a.s("id"));
    assert_eq!((t.at("autopilot").s("machine"), t.at("autopilot").at("repoFileWaits"), t.at("carried").s("from")), ("laptop", &json!(false), "laptop"));
}

#[test]
fn a_task_it_added_on_another_machine_keeps_its_slot_until_seen_over_and_is_looked_after_there() {
    let (w, f) = far_world();
    let main = main_in_wake(&w, &["--autopilot.tasksAtOnce", "1"]);
    f.answers("vm-b", "add", &json!({ "ok": true, "id": "web/7", "msg": "web/7: added on vm-b, an agent is on it" }));
    ok(&["add", "Fix their docs", "--repo", "web", "--on", "vm-b"]);
    assert!(refused(&["add", "And more", "--repo", "web", "--on", "vm-b"]).contains("tasks at once are full"), "its slot stands while vm-b is not heard");
    // heard asking a question: answered there, through vm-b's door
    heard(&f, json!([{ "id": "web/7", "title": "Fix their docs", "stage": "asks", "kind": "question", "created": sw::clock::now(), "autopilot": { "coordinator": "main", "machine": sw::remote::check::hostname() } }]));
    f.answers("vm-b", "do", &json!({ "ok": true, "msg": "Sent to the agent" }));
    assert!(ok(&["tell", "vm-b:web/7", "Use the existing index"]).contains("vm-b: Sent to the agent"));
    let (_, what, req) = f.asked().pop().unwrap(); assert_eq!((what.as_str(), req.s("action"), req.s("id"), req.s("text")), ("do", "tell", "web/7", "Use the existing index"));
    // what its agent asks is read there: `details` goes to vm-b's door, which answers with the task whole and its note
    f.answers("vm-b", "do", &json!({ "ok": true, "msg": "", "details": { "task": { "id": "web/7", "msg": "Which index do you mean?" }, "closingNote": "" } }));
    assert!(ok(&["details", "vm-b:web/7"]).contains("Which index do you mean?")); assert_eq!(f.asked().pop().unwrap().2.s("action"), "details");
    // seen merged there: its slot is free, also once vm-b is heard no more
    heard(&f, json!([{ "id": "web/7", "title": "Fix their docs", "stage": "done", "mergedAt": sw::clock::now(), "created": sw::clock::now(), "autopilot": { "coordinator": "main", "machine": sw::remote::check::hostname() } }]));
    in_pane(&w, ""); minutes(1); tick(&w); minutes(4); in_pane(&w, &pane_of(&main));
    assert_eq!(sw::autopilot::limits::today(&main, &record(&main), &sw::autopilot::limits::board_tasks(), sw::clock::now()).under_way, 0);
    in_pane(&w, "");
}

#[test]
fn the_door_answers_details_with_the_closing_note_and_a_task_delegated_later_on_another_machine_keeps_its_slot() {
    let (w, f) = far_world();
    let id = w.add("Here");
    std::fs::write(sw::flow::task::note_file(&task(&id)), "Asked which index.").unwrap();
    let a = sw::remote::door("do", &json!({ "action": "details", "id": id })).unwrap(); assert_eq!(a.at("details").s("closingNote"), "Asked which index.");
    let main = main_in_wake(&w, &["--autopilot.startAtOnce", "no"]);
    f.answers("vm-b", "add", &json!({ "ok": true, "id": "web/8", "msg": "web/8: added on vm-b" })); ok(&["add", "Later", "--repo", "web", "--on", "vm-b"]);
    let under = || sw::autopilot::limits::today(&main, &record(&main), &sw::autopilot::limits::board_tasks(), sw::clock::now()).under_way;
    assert_eq!(under(), 0, "added, not started");
    f.answers("vm-b", "do", &json!({ "ok": true, "msg": "web/8: an agent is on it" })); ok(&["delegate", "vm-b:web/8"]);
    assert_eq!(under(), 1, "started there: its slot stands while vm-b is not heard");
    in_pane(&w, "");
}

#[test]
fn an_autopilot_turns_yes_for_another_machines_task_is_checked_there_and_never_for_the_repos_settings_file() {
    let (w, f) = far_world();
    // the door's side: a task here whose change touches .herdr-projects.json, asked for a yes by an autopilot turn there
    let id = w.add("Theirs");
    let tree = w.base.join("tree"); git(&w.root, &["worktree", "add", "-q", "-b", "work/x", &tree.display().to_string()]);
    std::fs::write(tree.join(".herdr-projects.json"), "{}").unwrap(); git(&tree, &["add", "-A"]); git(&tree, &["commit", "-qm", "x"]);
    sw::store::tasks::set(&id, json!({ "stage": "approval", "worktree": tree.display().to_string() })).unwrap();
    let a = sw::remote::door("do", &json!({ "action": "pilot-approve", "id": id })).unwrap(); assert!(!a.b("ok") && a.s("msg").contains(".herdr-projects.json"), "{a}");
    assert_eq!(task(&id).s("stage"), "approval");
    // the asking side: an autopilot turn's yes for a task of a machine that cannot check it is refused here
    let main = main_in_wake(&w, &["--autopilot.approve", "yes"]);
    sw::autopilot::record::change(&main, |ap, _| { sw::autopilot::record::own_task(ap, "vm-b:web/9", "added", sw::clock::now()); true }).unwrap();
    let theirs = json!([{ "id": "web/9", "title": "Theirs", "stage": "approval", "created": sw::clock::now(), "autopilot": { "coordinator": "main", "machine": sw::remote::check::hostname() } }]);
    f.answers("vm-b", "list", &json!({ "host": "vm-b", "version": "1.0.0", "tasks": theirs })); let _ = sw::remote::ask_list(&sw::remote::machines()[0], 20000);
    assert!(refused(&["approve", "vm-b:web/9"]).contains("cannot check an autopilot turn's yes"));
    // one that can: the yes goes there marked as an autopilot turn's, and that machine refuses what it must
    heard(&f, theirs); f.answers("vm-b", "do", &json!({ "ok": false, "msg": "web/9: refused in an autopilot wake: its change touches the repo's .herdr-projects.json" }));
    assert!(refused(&["approve", "vm-b:web/9"]).contains(".herdr-projects.json"));
    assert_eq!(f.asked().pop().unwrap().2.s("action"), "pilot-approve", "an action of its own, which only a machine that checks it knows");
    // one that sends autopilot but has no such check (an earlier version) answers that it knows no such action: refused
    f.answers("vm-b", "do", &json!({ "ok": false, "msg": "Unknown action pilot-approve" }));
    assert!(refused(&["approve", "vm-b:web/9"]).contains("cannot check an autopilot turn's yes for its task"));
    in_pane(&w, "");
}

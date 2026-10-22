//! Autopilot on other machines: their coordinators' rows and keys, what the main Coordinator sees of them (only what an
//! open board heard), its adds there, their slot taken here and given back when refused.
use crate::common;
use common::board::open;
use common::pilot::*;
use common::*;
use serde_json::json;
use sw::json::*;

fn board(w: &World) -> common::board::Board { let mut b = open(&w.root, 40); b.s.side = false; b.load(); b }
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
    far_list(&f, json!({ "machine": { "on": true, "state": "working", "line": "autopilot working · every 2h + events", "status": "Checking in · Regular check-in", "added": "1 of 5 added", "next": "", "last": "#3", "lastId": "web/3" } }), json!([]));
    let mut b = board(&w);
    sw::board::update::update(&mut b.s, sw::board::update::Event::Machines(sw::remote::machines()));
    for m in sw::remote::machines() { let a = sw::remote::ask_list(&m, 20000); sw::board::update::update(&mut b.s, sw::board::update::Event::Remote { label: m.label.clone(), answer: a }); }
    while b.s.scope != "*" { b.press(&["alt+m"]); }
    let lines = b.screen(160); let at = lines.iter().position(|l| l.contains("▸ vm-b [autopilot]")).unwrap_or_else(|| panic!("{lines:#?}"));
    assert!(!lines[at].contains("──") && !lines[at].contains("added") && lines[at].contains("0 tasks, 1 on autopilot"), "its tag and its count, no rule, no columns: {lines:#?}");
    assert!(lines[at + 1].trim_start_matches(['▌', ' ']) == "Checking in · Regular check-in", "what it does, under it: {lines:#?}");
    f.answers("vm-b", "do", &json!({ "ok": true, "msg": "Autopilot off for the vm-b coordinator." }));
    b.s.sel = sw::board::update::heads::machine_id("vm-b"); b.press(&["ctrl+o"]);
    let (who, what, req) = f.asked().pop().unwrap();
    assert_eq!((who.as_str(), what.as_str(), req.s("action"), req.s("to")), ("vm-b", "do", "autopilot-off", "machine"));
    b.press(&["ctrl+x"]); let (_, _, req) = f.asked().pop().unwrap(); assert_eq!((req.s("action"), req.s("to")), ("coordinator-stop", "machine"), "ctrl+x stops it there");
    // a machine on an older version sends no autopilot: the keys say to update it
    sw::board::update::update(&mut b.s, sw::board::update::Event::Remote { label: "vm-b".into(), answer: Ok(json!({ "host": "vm-b", "version": "1.0.0", "tasks": [] })) });
    b.press(&["ctrl+o"]); assert!(b.text().contains("runs an older version that has no autopilot: update herdr-projects there first"), "{}", b.text());
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

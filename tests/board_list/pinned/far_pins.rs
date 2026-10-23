//! Another machine's pins come with its tasks: its rows carry the pin mark and the Pinned tab shows what it pinned.
//! Its machine's row and its project's row pin there, through its door (`do`, `pin-machine` / `pin-project`), and so do
//! `pin <machine>` and `pin <project> --on <machine>` from the shell; this machine's door pins here, and what it
//! sends with its tasks says what is pinned.
use crate::common;
use crate::far;
use common::board::{open, Board};
use common::*;
use far::{theirs, Far};
use serde_json::{json, Value};
use sw::board::state::Row;
use sw::board::update::heads::{group_id, machine_id};
use sw::board::update::{update, Event};
use sw::json::*;

fn far_world() -> (World, Far) {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-b" }), ..Default::default() });
    let far = Far::new(&w.base);
    (w, far)
}
/// vm-b answers with these tasks and pins.
fn answer(b: &mut Board, tasks: Value, pins: Value) { update(&mut b.s, Event::Remote { label: "vm-b".into(), answer: Ok(json!({ "host": "vm-b", "version": "9.9.9", "tasks": tasks, "archived": [], "pins": pins })) }); }
fn every_machine(w: &World, tasks: Value, pins: Value) -> Board {
    let mut s = open(&w.root, 40); s.s.side = false;
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    answer(&mut s, tasks, pins);
    while s.s.scope != "*" { s.press(&["alt+m"]); }
    s
}
fn tasks_shown(s: &Board) -> Vec<String> { let mut v: Vec<String> = s.s.rows.iter().filter_map(|r| match r { Row::Rec(o) => Some(o.t("id")), _ => None }).collect(); v.sort(); v }
fn heads(s: &Board) -> Vec<String> { s.s.rows.iter().filter_map(|r| match r { Row::Head { id, .. } => Some(id.clone()), _ => None }).collect() }
/// The last action vm-b was asked for, and what came with it.
fn asked(far: &Far) -> Option<(String, Value)> { far.asked().into_iter().rev().find(|a| a.1 == "do").map(|a| (a.2.t("action"), a.2)) }
fn door(req: Value) -> Value { sw::remote::door("do", &req).unwrap_or_else(|e| json!({ "ok": false, "msg": e.msg })) }
fn run(args: &[&str]) -> Result<(Value, String), String> {
    let a = sw::cli::flags::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    sw::cli::flags::refuse_unknown(&a.pos[0], &a).and_then(|_| sw::cli::tasks::command(&a.pos[0], &a)).map_err(|e| e.msg)
}

#[test]
fn another_machines_pins_come_with_its_tasks_and_its_rows_pin_there() {
    let (w, far) = far_world();
    w.add("Mine");
    let mut s = every_machine(&w, json!([theirs(1), theirs(2)]), json!({ "machine": false, "repos": ["far"] }));
    s.press(&["alt+i"]);
    assert_eq!(tasks_shown(&s), vec!["vm-b:far/1".to_string(), "vm-b:far/2".into()], "its pinned project's tasks, not this machine's: {}", s.text());
    let far_row = group_id(&s.s, "vm-b", "far");
    assert!(heads(&s).contains(&machine_id("vm-b")) && heads(&s).contains(&far_row), "{:?}", heads(&s));
    assert!(s.lines().iter().any(|l| l.contains("far") && l.contains('⚑')), "{}", s.text());
    // Its project's row: unpinned there.
    far.answers("vm-b", "do.pin-project", &json!({ "ok": true, "pin": true, "msg": "far: unpinned", "name": "far" }));
    s.s.sel = far_row; s.load();
    s.press(&["ctrl+p"]);
    let (action, req) = asked(&far).expect("vm-b was asked");
    assert_eq!((action.as_str(), req.t("repo").as_str(), req.b("on")), ("pin-project", "far", false));
    // Its machine's row: pinned there.
    far.answers("vm-b", "do.pin-machine", &json!({ "ok": true, "pin": true, "msg": "vm-b: pinned", "name": "vm-b" }));
    s.s.sel = machine_id("vm-b"); s.load();
    s.press(&["ctrl+p"]);
    let (action, req) = asked(&far).unwrap();
    assert_eq!((action.as_str(), req.b("on")), ("pin-machine", true));
    // vm-b says so with its tasks: its row has the mark, and every task of it is in the tab.
    answer(&mut s, json!([theirs(1), theirs(2), theirs(3)]), json!({ "machine": true, "repos": [] }));
    assert_eq!(tasks_shown(&s).len(), 3, "{}", s.text());
    // A machine on an older version cannot: it says to update it there.
    far.ends("vm-b", "do.pin-machine", "{\"ok\":false,\"msg\":\"No task \\\"\\\".\"}\n", "", 0);
    s.s.pending.clear();
    s.press(&["ctrl+p"]);
    assert!(s.s.msg.contains("older version of herdr-projects that cannot pin"), "{}", s.s.msg);
}

#[test]
fn the_door_pins_this_machine_and_its_projects_and_says_so_with_its_tasks() {
    let w = world();
    let id = w.add("Theirs to pin");
    let j = door(json!({ "action": "pin-project", "id": "", "repo": "proj", "on": true }));
    assert_eq!((j.b("ok"), j.b("pin"), j.s("msg")), (true, true, "proj: pinned"));
    let j = door(json!({ "action": "pin-machine", "id": "", "repo": "", "on": true }));
    assert!(j.b("ok") && sw::store::pins::pins().machine);
    assert_eq!(sw::remote::remote_list().at("pins"), &json!({ "machine": true, "repos": ["proj"] }));
    let j = door(json!({ "action": "pin-project", "id": "", "repo": "nope", "on": true }));
    assert!(!j.b("ok") && j.b("pin") && j.s("msg").contains("No repo \"nope\""), "{j}");
    // A task's pin from another machine's shell: `on` sets it, whatever it was; without it, the pin key's way round.
    for _ in 0..2 { assert!(door(json!({ "action": "pin", "id": id, "on": true })).b("ok")); assert!(task(&id).b("pinned")); }
    door(json!({ "action": "pin", "id": id })); assert!(!task(&id).b("pinned"));
}

#[test]
fn the_shell_pins_another_machine_and_its_project_there() {
    let (_w, far) = far_world();
    far.answers("vm-b", "do.pin-machine", &json!({ "ok": true, "pin": true, "msg": "vm-b: pinned", "name": "vm-b" }));
    let (j, text) = run(&["pin", "vm-b"]).unwrap();
    assert_eq!(text, "vm-b: pinned"); assert_eq!(j.s("machine"), "vm-b");
    far.answers("vm-b", "do.pin-project", &json!({ "ok": true, "pin": true, "msg": "far: unpinned", "name": "far" }));
    assert_eq!(run(&["unpin", "far", "--on", "VM-B"]).unwrap().1, "far on vm-b: unpinned");
    let (action, req) = asked(&far).unwrap();
    assert_eq!((action.as_str(), req.t("repo").as_str(), req.b("on")), ("pin-project", "far", false));
    assert!(run(&["pin", "far", "--on", "vm-z"]).unwrap_err().starts_with("vm-z is not a machine the board knows"));
}

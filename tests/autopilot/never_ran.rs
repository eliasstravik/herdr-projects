//! Autopilot on a coordinator that never ran: this machine sends it as off (its machine's and every repo's it knows),
//! so another machine's board offers `Tab` there and does not take it for an older version; `Tab` from there starts
//! it here as a first message would. A machine that sends no autopilot at all still says to update it.
use crate::common;
use common::board::{open, Board};
use common::pilot::*;
use common::*;
use serde_json::json;
use sw::coordinator::co::Co;
use sw::json::*;

fn far_world() -> (World, crate::far::Far) { let w = pilot_world_with(json!({ "board.machines": "vm-b" })); let f = crate::far::Far::new(&w.base); (w, f) }
/// A board that heard vm-b's list, every machine shown, grouped by repo.
fn far_board(w: &World, f: &crate::far::Far, list: serde_json::Value) -> Board {
    f.answers("vm-b", "list", &list);
    let mut b = open(&w.root, 40); b.s.side = false; b.load();
    sw::board::update::update(&mut b.s, sw::board::update::Event::Machines(sw::remote::machines()));
    for m in sw::remote::machines() { let a = sw::remote::ask_list(&m, 20000); sw::board::update::update(&mut b.s, sw::board::update::Event::Remote { label: m.label.clone(), answer: a }); }
    b.press(&["alt+m"]); b.press(&["alt+s"]); b.load();
    b
}
fn their_task() -> serde_json::Value { json!([{ "id": "web/1", "repo": "web", "title": "Their task", "stage": "todo", "created": sw::clock::now() }]) }

#[test]
fn a_coordinator_that_never_ran_is_sent_as_off() {
    let w = pilot_world();
    let proj = proj(&w);
    assert!(sw::coordinator::state::get(&proj).is_none() && sw::coordinator::state::get(Co::machine(&w.socket)).is_none(), "neither ever ran");
    let sent = sw::remote::remote_list();
    assert_eq!(sent.at("autopilot").at("project:proj"), &json!({ "on": false }), "{sent}");
    assert_eq!(sent.at("autopilot").at("machine"), &json!({ "on": false }), "{sent}");
    assert_eq!(sent.at("autopilot").at(""), &json!({ "on": false }));
    // one that is on is sent as it shows
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs"]);
    assert!(sw::remote::remote_list().at("autopilot").at("project:proj").b("on"));
}

#[test]
fn tab_on_another_machines_repo_whose_coordinator_never_ran_sends_autopilot_on_and_it_starts_there() {
    let (w, f) = far_world();
    // vm-b sends its autopilot, with nothing of its web repo's coordinator (one older than this fix leaves it out)
    let mut b = far_board(&w, &f, json!({ "host": "vm-b", "version": "9.9.9", "autopilot": { "": { "on": false } }, "tasks": their_task() }));
    let head = sw::board::update::heads::group_id(&b.s, "vm-b", "web");
    assert!(sw::board::update::coords::exists(&b.s, &head), "{}", b.text());
    b.s.sel = head; b.load();
    b.press(&["ctrl+x"]);
    assert!(b.text().contains("is not on autopilot"), "off, not an older version: {}", b.text());
    f.answers("vm-b", "do", &json!({ "ok": true, "msg": "Autopilot on for the web coordinator." }));
    b.press(&["tab"]);
    assert!(!b.text().contains("older version"), "{}", b.text());
    let (who, what, req) = f.asked().pop().unwrap();
    assert_eq!((who.as_str(), what.as_str(), req.s("action"), req.s("to")), ("vm-b", "do", "autopilot-on", "project:web"));
    // there, the door turns it on and the keeper starts it, as a first message would
    let proj = proj(&w);
    assert!(sw::coordinator::state::get(&proj).is_none(), "it never ran");
    ok(&["set", "autopilot.goal", "Tidy", "--project-coordinator", "proj"]);
    let a = sw::remote::door("do", &json!({ "action": "autopilot-on", "id": "", "to": "project:proj" })).unwrap();
    assert!(a.b("ok") && sw::autopilot::record::is_on(&proj), "{a}");
    tick(&w);
    assert!(!pane_of(&proj).is_empty(), "its coordinator started: {:?}", sw::coordinator::state::get(&proj));
    assert!(prompts_of(&w, &proj).iter().any(|p| p.contains("Tidy")), "{:?}", prompts_of(&w, &proj));
}

#[test]
fn a_machine_that_sends_no_autopilot_at_all_still_says_to_update_it() {
    let (w, f) = far_world();
    let mut b = far_board(&w, &f, json!({ "host": "vm-b", "version": "1.0.0", "tasks": their_task() }));
    b.s.sel = sw::board::update::heads::group_id(&b.s, "vm-b", "web"); b.load();
    b.press(&["tab"]);
    assert!(b.text().contains("vm-b runs an older version that has no autopilot: update herdr-projects there first."), "{}", b.text());
    b.press(&["ctrl+x"]);
    assert!(b.text().contains("runs an older version"), "{}", b.text());
    assert!(f.asked().iter().all(|(_, what, _)| what != "do"), "nothing was sent");
}

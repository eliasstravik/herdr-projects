//! Other machines on the web board: their tasks show with the machine's name, an action goes to the machine that owns
//! the task, and another machine's agent opens in the web UI's view of that machine's pane, or what is missing is said.
use crate::common;
use crate::far;
use common::*;
use far::Far;
use serde_json::{json, Value};
use sw::json::*;
use sw::webui::pcs::Pc;

fn call(what: &str, args: Value, pcs: Option<Vec<Pc>>) -> Value { let mut a = args; if a.get("role").is_none() { a["role"] = json!("drive"); } sw::webui::web::call("t", what, a, "", pcs).unwrap_or_else(|e| panic!("{what}: {e}")) }
fn rows_of(s: &Value) -> Vec<Value> { s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").to_vec()).collect() }
/// (Its door sends its settings: a task with none at all is drawn "Reading its settings…" by the Rust board.)
fn theirs() -> Value { let now = sw::clock::now(); json!({ "id": "far/1", "title": "Theirs", "stage": "working", "state": "working", "agent": "claude", "app": "claude", "msg": "", "note": "Building", "at": now, "created": now, "link": { "pane": "w1:p1" }, "snap": { "folder": "~/far", "choices": {}, "settings": [] } }) }
fn pc(id: &str, name: &str, to: &str, local: bool) -> Pc { Pc { id: id.into(), name: name.into(), to: to.into(), local, connected: true, state: "connected".into(), ..Default::default() } }
fn web_world() -> (World, Far) {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-a" }), ..Default::default() });
    let far = Far::new(&w.base);
    std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str() }).to_string()).unwrap();
    (w, far)
}

#[test]
fn other_machines_tasks_show_with_the_machines_name_and_an_action_goes_to_the_machine_that_owns_the_task() {
    let (_w, far) = web_world();
    far.tasks("vm-a", "0.2.0", json!([theirs()]));
    far.answers("vm-a", "do", &json!({ "ok": true, "msg": "far/1: archived." }));
    let s = call("press", json!({ "id": "scope", "role": "watch" }), None); // whose board you see only shows: a watching device may
    // (no other machine known yet: herdr is asked for them first, and every machine shows once there is another one)
    assert_eq!(s.at("flags").s("scope"), "");
    assert!(s.at("msg").s("text").contains("Looking for other machines"), "{}", s.at("msg"));
    far::answered("t", 1);
    let s = call("board", json!({}), None);
    assert_eq!(s.at("flags").s("scope"), "*");
    assert!(s.at("msg").s("text").contains("Every machine you can reach"), "{}", s.at("msg"));
    assert_eq!(s.at("list").at("head").s("machines"), "vm-a");
    let f = rows_of(&s).into_iter().find(|r| r.s("host") == "vm-a").unwrap();
    assert_eq!((f.s("id"), f.s("title")), ("vm-a:far/1", "Theirs"));
    assert_eq!(f.at("says").as_array().unwrap().iter().map(|p| p.t("t")).collect::<String>(), "Building");
    let s = call("pick", json!({ "id": f.s("id"), "open": true }), None);
    let machine = s.at("panel").arr("parts").iter().find(|p| p.s("head") == "Settings").and_then(|p| p.arr("items").iter().find(|x| x.s("label") == "Machine").cloned()).unwrap();
    assert_eq!(machine.at("lines")[0][0].s("t").trim(), "vm-a");
    assert_eq!(s.at("canOpen"), &json!(true), "its agent opens in the web UI's view of that machine");
    let s = call("press", json!({ "id": "archive" }), None);
    let did = far.asked().into_iter().filter(|a| a.1 == "do").last().unwrap();
    assert_eq!((did.0.as_str(), did.2.s("action"), did.2.s("id")), ("vm-a", "archive", "far/1"));
    assert!(!s.at("msg").s("text").contains("archived."), "no confirmation: {}", s.at("msg"));
}

#[test]
fn another_machines_agent_opens_in_the_web_uis_view_of_that_machines_pane_and_what_is_missing_is_said_plainly() {
    let (_w, far) = web_world();
    let now = sw::clock::now();
    let todo = json!({ "id": "far/2", "title": "Not started", "stage": "todo", "state": "idle", "msg": "", "at": now, "created": now, "snap": { "folder": "~/far", "choices": {} } });
    far.tasks("vm-a", "0.2.0", json!([theirs(), todo]));
    far.answers("vm-a", "do", &json!({ "ok": true, "msg": "" }));
    let reopen = |j: Value| far.answers("vm-a", "do.reopen", &j);
    reopen(json!({ "ok": true, "msg": "", "pane": "w3:p2", "workspace": "w3", "hasAgent": true }));
    call("press", json!({ "id": "scope" }), None);
    far::answered("t", 1);
    let open = |pcs: Vec<Pc>| { call("pick", json!({ "id": "vm-a:far/1", "open": true }), None); call("press", json!({ "id": "open" }), Some(pcs)) };
    let mine = pc("pc-9", "vm-a", "vm-a", false);
    // the web UI has that machine as a PC: the machine reopens its own task's space, and the page goes to that pane there
    let s = open(vec![pc("local", "home", "", true), mine.clone()]);
    let last = far.asked().last().cloned().unwrap();
    assert_eq!((last.0.as_str(), last.1.as_str(), &last.2), ("vm-a", "do", &json!({ "action": "reopen", "id": "far/1" })));
    assert_eq!(s.at("go"), &json!({ "n": 1, "machine": "pc-9", "pane": "w3:p2", "workspace": "w3", "view": "chat", "space": "vm-a: far/1" }));
    assert_eq!(s.at("canOpen"), &json!(true));
    // its agent has exited and could not be restarted: the pane's terminal, as on this machine
    reopen(json!({ "ok": true, "msg": "", "pane": "w3:p2", "workspace": "w3", "hasAgent": false }));
    let s = open(vec![mine.clone()]); assert_eq!((s.at("go").i("n"), s.at("go").s("view")), (2, "terminal"));
    // an answer from before the workspace was sent along: a pane's id starts with its space's
    reopen(json!({ "ok": true, "msg": "", "pane": "w7:p1", "hasAgent": true }));
    let s = open(vec![mine.clone()]); assert_eq!((s.at("go").s("pane"), s.at("go").s("workspace")), ("w7:p1", "w7"));
    // the machine refuses, or does not answer: said, and the page goes nowhere
    reopen(json!({ "ok": false, "msg": "far/1: its worktree is gone" }));
    let s = open(vec![mine.clone()]); assert_eq!((s.at("go").i("n"), s.at("msg").s("text")), (3, "far/1: its worktree is gone"));
    // (a real ssh call always ends some way: here it answered nothing, and that is said after "did not answer")
    far.ends("vm-a", "do.reopen", "", "", 0);
    let s = open(vec![mine.clone()]); assert_eq!(s.at("go").i("n"), 3); assert!(s.at("msg").s("text").starts_with("vm-a did not answer"), "{}", s.at("msg"));
    reopen(json!({ "ok": true, "msg": "", "pane": "w3:p2", "workspace": "w3", "hasAgent": true }));
    // the web UI has no such PC, has it but not connected, or could not say what it has: each said as what it is
    let before = far.asked().len();
    let s = open(vec![pc("local", "home", "", true), pc("pc-1", "far", "elsewhere", false)]);
    assert!(regex::Regex::new(r"^The web UI has no PC for vm-a \(.*@vm-a\)\. Add it there \(Settings → Remote PCs → Add PC\)").unwrap().is_match(s.at("msg").s("text")), "{}", s.at("msg"));
    let s = open(vec![Pc { connected: false, state: "reconnecting".into(), ..mine.clone() }]);
    assert_eq!(s.at("msg").s("text"), "vm-a is not connected in the web UI (reconnecting). Connect it there, then open the agent again.");
    { let c = sw::webui::web::client("t"); c.lock().unwrap().s.web.pcs = None; }
    call("pick", json!({ "id": "vm-a:far/1", "open": true }), None);
    let s = call("press", json!({ "id": "open" }), None);
    assert!(s.at("msg").s("text").starts_with("The web UI did not say which PCs it has"), "{}", s.at("msg"));
    assert_eq!(far.asked().len() - before, 0, "the other machine is not asked to open anything");
    assert_eq!(s.at("go").i("n"), 3);
    // a task there that no agent has been on has nothing to open
    call("pick", json!({ "id": "vm-a:far/2", "open": true }), None);
    let s = call("press", json!({ "id": "open" }), Some(vec![mine.clone()]));
    assert_eq!(s.at("canOpen"), &json!(false)); assert!(s.at("msg").s("text").starts_with("No agent has been on this yet"), "{}", s.at("msg"));
    // a machine on an older plugin cannot reopen a space: the pane it last named is opened, and the board says to update
    far.tasks("vm-a", "", json!([theirs(), todo]));
    far::ask_again("t", 1);
    let s = open(vec![mine]);
    assert_eq!(s.at("go"), &json!({ "machine": "pc-9", "pane": "w1:p1", "workspace": "w1", "view": "chat", "n": 4 }));
    assert_eq!(s.at("msg").s("text"), "Update herdr-projects on vm-a: it cannot reopen a closed space yet.");
    assert_ne!(far.asked().last().unwrap().2.s("action"), "reopen");
}

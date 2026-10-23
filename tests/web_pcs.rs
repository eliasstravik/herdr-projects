//! The web UI's PCs and the board's machines: which is which (by herdr's id, or by where they connect), what the web UI
//! answers read down to what the board needs, and the web board going with the web UI to the PC it is on and back.
use crate::common;
use crate::far;
use common::*;
use far::Far;
use serde_json::{json, Value};
use sw::json::*;
use sw::remote::Machine;
use sw::webui::pcs::{address, machine_of, pc_of, read_pcs, same, Pc};

fn pc(f: impl FnOnce(&mut Pc)) -> Pc { let mut p = Pc { id: "pc-1".into(), name: "far".into(), connected: true, state: "connected".into(), ..Default::default() }; f(&mut p); p }
fn to(t: &str, port: u16) -> Pc { pc(|p| { p.to = t.into(); p.port = port; }) }

#[test]
fn a_machine_of_the_board_and_a_pc_of_the_web_ui_are_the_same_one_by_herdrs_id_or_by_where_they_connect() {
    assert_eq!(address("ssh://me@Far.example:2200", 0), ("me".into(), "far.example".into(), 2200));
    assert_eq!(address("me@far.example", 2200), ("me".into(), "far.example".into(), 2200));
    assert_eq!(address("far", 0), (String::new(), "far".into(), 0), "an SSH alias");
    assert_eq!(address("ssh://[fd7a::1]:22", 0), (String::new(), "[fd7a::1]".into(), 22));
    // herdr's own way to write a saved machine, and the web UI's way to write the PC it was added as
    let m = Machine { id: "79cd".into(), label: "far".into(), target: "ssh://me@far.example:2200".into(), session: "default".into() };
    let plain = Machine { id: String::new(), ..m.clone() };
    assert!(same(&plain, &to("me@far.example", 2200)));
    assert!(same(&plain, &to("far.example", 2200)), "no user said on one side");
    assert!(!same(&plain, &to("you@far.example", 2200)));
    assert!(!same(&plain, &to("me@far.example", 0)), "a port left out is not taken for another");
    assert!(!same(&plain, &pc(|p| { p.to = "me@far.example".into(); p.port = 2200; p.session = "work".into(); })), "another herdr on that machine");
    assert!(same(&Machine { session: "work".into(), ..plain.clone() }, &pc(|p| { p.to = "me@far.example".into(); p.port = 2200; p.session = "work".into(); })));
    assert!(!same(&plain, &pc(|p| { p.local = true; p.to = "me@far.example".into(); p.port = 2200; })));
    // a PC the web UI took from herdr's own list names herdr's id: that decides, whatever the addresses say
    assert!(same(&m, &pc(|p| { p.profile = "79cd".into(); p.to = "alias".into(); })));
    assert!(!same(&m, &pc(|p| { p.profile = "other".into(); p.to = "me@far.example".into(); p.port = 2200; })));
    // a machine named in board.machines has no id and no port
    let named = Machine { target: "me@vm-a".into(), label: "vm-a".into(), ..Default::default() };
    assert_eq!(pc_of(&named, &[pc(|p| { p.id = "local".into(); p.local = true; }), pc(|p| { p.id = "x".into(); p.to = "me@vm-b".into(); }), pc(|p| { p.id = "y".into(); p.to = "vm-a".into(); })]).map(|p| p.id), Some("y".into()));
    assert_eq!(pc_of(&named, &[to("me@vm-b", 0)]), None);
    assert_eq!(machine_of(&to("vm-a", 0), &[m.clone(), named.clone()]), Some(named.clone()));
    assert_eq!(machine_of(&to("nowhere", 0), &[m, named]), None);
    // what the web UI answers, down to what the board needs (the panes it knows there: none said, none known); an answer
    // that is no list of PCs is none
    let list = read_pcs(&json!({ "machines": [{ "id": "local", "name": "home", "kind": "local", "enabled": true, "state": "connected", "snapshot": { "panes": [{ "pane_id": "w1:p1" }, { "pane_id": "w3:p1" }, 2] } }, { "id": "p", "name": "far", "kind": "ssh", "herdr_profile_id": "79cd", "target": { "destination": "me@far", "port": 22, "session": "work" }, "enabled": false, "state": "disconnected" }, { "name": "no id" }] }).to_string());
    assert_eq!(list, Some(vec![Pc { id: "local".into(), name: "home".into(), local: true, connected: true, state: "connected".into(), panes: Some(vec!["w1:p1".into(), "w3:p1".into()]), ..Default::default() },
        Pc { id: "p".into(), name: "far".into(), local: false, profile: "79cd".into(), to: "me@far".into(), port: 22, session: "work".into(), connected: false, state: "turned off".into(), panes: None }]));
    for bad in ["", "not json", "{}", r#"{"machines":{}}"#, r#"{"error":{"code":"bridge_only"}}"#] { assert_eq!(read_pcs(bad), None, "{bad}"); }
}

fn call(c: &str, what: &str, args: Value, machine: &str, pcs: Option<Vec<Pc>>) -> Value { let mut a = args; a["role"] = json!("drive"); sw::webui::web::call(c, what, a, machine, pcs).unwrap_or_else(|e| panic!("{what}: {e}")) }
fn client(c: &str) -> std::sync::Arc<std::sync::Mutex<sw::webui::web::Client>> { sw::webui::web::client(c) }

#[test]
fn the_web_board_goes_with_the_web_ui_on_another_pc_it_is_that_machines_board_and_back_here_what_it_was() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-a vm-b" }), ..Default::default() });
    let far = Far::new(&w.base);
    for m in ["vm-a", "vm-b", "nowhere"] { far.tasks(m, "0.2.0", json!([])); }
    std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str() }).to_string()).unwrap();
    let pcs = vec![pc(|p| { p.id = "local".into(); p.name = "home".into(); p.local = true; }), pc(|p| { p.id = "pc-a".into(); p.name = "vm-a".into(); p.to = "vm-a".into(); }), pc(|p| { p.id = "pc-x".into(); p.name = "stranger".into(); p.to = "nowhere".into(); })];
    let board = |m: &str, list: Option<Vec<Pc>>| call("c", "board", json!({}), m, list);
    let scope = |s: &Value| s.at("flags").t("scope");
    // this machine's page: nothing changes, whatever the board showed
    let s = board("", Some(pcs.clone())); assert_eq!(scope(&s), ""); assert!(s.at("msg").is_null());
    // the web UI goes to vm-a: that machine's board by itself, as the terminal board's own key shows it
    let s = board("pc-a", Some(pcs.clone()));
    assert_eq!((scope(&s), s.at("list").at("head").s("only"), s.at("msg").s("text")), ("vm-a".into(), "vm-a", "Only vm-a. What you type goes to vm-a."));
    // what a page opened later starts with is what this one showed before it went there, not where the web UI was
    assert_eq!(sw::store::files::read_obj(&sw::store::files::data_dir().join("web-view.json")).unwrap().at("ownScope"), &json!(null), "the terminal board's, followed");
    // while it stays there, a choice of your own stands
    let chosen = scope(&call("c", "press", json!({ "id": "scope" }), "pc-a", Some(pcs.clone())));
    assert_eq!(scope(&board("pc-a", Some(pcs.clone()))), chosen);
    // back on this machine: what the board showed before it went
    assert_eq!(scope(&board("", Some(pcs.clone()))), "");
    // a pick of every machine, of your own (the scope key's round would go on to vm-a after this machine)
    assert_eq!(scope(&call("c", "scope", json!({ "to": "*" }), "", Some(pcs.clone()))), "*");
    assert_eq!(scope(&board("pc-a", Some(pcs.clone()))), "vm-a");
    assert_eq!(scope(&board("local", Some(pcs.clone()))), "*");
    // a PC the board has no machine for: said, and the board stays as it is
    let s = board("pc-x", Some(pcs.clone()));
    assert_eq!(scope(&s), "*");
    assert!(s.at("msg").s("text").starts_with("The web UI is on stranger, which the board knows no machine for: add it with herdr machine add"), "{}", s.at("msg"));
    // the machine is added while the web UI stays on that PC: said once, looked for again after a few seconds, and the
    // board then goes there by itself
    let n = board("pc-x", Some(pcs.clone())).at("msg").i("n");
    assert_eq!(board("pc-x", Some(pcs.clone())).at("msg").i("n"), n, "said once");
    sw::store::settings::change::set_value("board.machines", &json!("vm-a vm-b nowhere"), &sw::store::settings::lookup::Scope::you()).unwrap();
    assert_eq!(scope(&board("pc-x", Some(pcs.clone()))), "*", "not asked again at once");
    { let c = client("c"); c.lock().unwrap().s.web.stranger_tried -= 11000; }
    let s = board("pc-x", Some(pcs.clone()));
    assert_eq!((scope(&s), s.at("msg").s("text")), ("nowhere".into(), "Only nowhere. What you type goes to nowhere."));
    sw::store::settings::change::set_value("board.machines", &json!("vm-a vm-b"), &sw::store::settings::lookup::Scope::you()).unwrap();
    assert_eq!(scope(&board("", Some(pcs.clone()))), "*");
    // also straight from another PC's board: it does not go on showing that one under the new PC's name
    assert_eq!(scope(&board("pc-a", Some(pcs.clone()))), "vm-a");
    let s = board("pc-x", Some(pcs.clone()));
    assert_eq!(scope(&s), "*"); assert!(s.at("msg").s("text").ends_with("The board shows what it showed before."), "{}", s.at("msg"));
    assert_eq!(scope(&board("", Some(pcs.clone()))), "*");
    // the web UI's list has not come (or it has no such PC): nothing happens until it has
    { let c = client("c"); c.lock().unwrap().s.web.pcs = None; }
    assert_eq!(scope(&board("pc-a", None)), "*");
    assert_eq!(scope(&board("pc-a", Some(pcs))), "vm-a");
}

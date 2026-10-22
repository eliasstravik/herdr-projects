//! The Boards section of the web UI's sidebar: one entry for all machines at the top, then one per machine with a board,
//! named as the web UI names it, each with a dot in the board's status colours and a short line of how it stands; a
//! machine that cannot be reached says so; picking an entry shows that machine's board, and the board and the section
//! stay in step.
use crate::common;
use crate::far;
use common::*;
use far::Far;
use serde_json::{json, Value};
use sw::json::*;
use sw::webui::pcs::Pc;

fn call(what: &str, args: Value, pcs: Option<Vec<Pc>>) -> Result<Value, String> { let mut a = args; if a.get("role").is_none() { a["role"] = json!("watch"); } sw::webui::web::call("t", what, a, "", pcs) }
fn boards(pcs: Option<Vec<Pc>>) -> Value { call("boards", json!({}), pcs).unwrap() }
fn pc(id: &str, name: &str, to: &str, local: bool) -> Pc { Pc { id: id.into(), name: name.into(), to: to.into(), local, connected: true, state: "connected".into(), ..Default::default() } }
fn pcs() -> Option<Vec<Pc>> { Some(vec![pc("local", "Mac", "", true), pc("pc-9", "vm-box", "vm-a", false)]) }
/// Each entry as (whose board, its name, its dot, its line).
fn entries(b: &Value) -> Vec<(String, String, String, String)> { b.arr("boards").iter().map(|e| (e.t("scope"), e.t("name"), e.t("dot"), e.t("line"))).collect() }
fn e(scope: &str, name: &str, dot: &str, line: &str) -> (String, String, String, String) { (scope.into(), name.into(), dot.into(), line.into()) }
fn working(id: &str) { sw::actions::filing::report(id, sw::actions::filing::Report { state: "working".into(), msg: "On it".into(), ..Default::default() }).unwrap(); }
fn web_world() -> (World, Far) {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-a" }), ..Default::default() });
    let far = Far::new(&w.base);
    std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str() }).to_string()).unwrap();
    (w, far)
}

#[test]
fn each_machine_has_its_board_in_the_section_named_as_the_web_ui_names_it_with_how_it_stands_and_all_machines_add_up() {
    let (w, far) = web_world();
    w.add("Later");
    working(&w.add("Write notes"));
    let mut busy = far::theirs(1); busy["state"] = json!("working"); busy["stage"] = json!("working");
    let mut stuck = far::theirs(3); stuck["state"] = json!("blocked");
    far.tasks("vm-a", "0.2.0", json!([busy, far::theirs(2), stuck]));
    // asked before the other machine answered: it is being asked, and there is nothing to add up yet
    // (and before herdr said which machines there are, no all machines: it would be this one again)
    let first = boards(pcs());
    assert_eq!(first.arr("boards").iter().find(|e| e.s("scope") == "").map(|e| e.t("name")).as_deref(), Some("Mac"), "this machine as the web UI names it");
    far::answered("t", 1);
    let b = boards(None);
    assert_eq!(entries(&b), vec![
        e("*", "All machines", "red", "1 needs you · 2 working · 2 to do"),
        e("", "Mac", "yellow", "1 working · 1 to do"),
        e("vm-a", "vm-box", "red", "1 needs you · 1 working · 1 to do"),
    ]);
    assert_eq!(b.s("scope"), "", "the board shows this machine");
}

#[test]
fn a_board_with_nothing_going_on_is_all_quiet_its_archive_left_out_and_with_no_other_machine_there_is_no_all_machines() {
    let w = world();
    let far = Far::new(&w.base);
    far.tasks("vm-a", "0.2.0", json!([]));
    std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str(), "scope": "*" }).to_string()).unwrap();
    sw::actions::archive::archive(&w.add("Old")).unwrap();
    // Which machines there are is only what the test tells: the list a new board asks herdr for on a thread of its own
    // could come in at any moment (after the second machine, taking it away again), so it goes nowhere.
    { let c = sw::webui::web::client("t"); c.lock().unwrap().inbox = std::sync::Arc::new(std::sync::Mutex::new(vec![])); }
    // a web UI that names this machine only by its id, or none: the board's own words for it; this machine is the only
    // one, so all machines would be this one again: it is not there
    let told = |fx| { let c = sw::webui::web::client("t"); let mut g = c.lock().unwrap(); let fx = sw::board::update::update(&mut g.s, sw::board::update::Event::Machines(fx)); sw::webui::web::effects(&mut g, fx, false); };
    told(vec![]);
    let b = boards(Some(vec![pc("local", "local", "", true)]));
    assert_eq!(b.at("boards"), &json!([
        { "scope": "", "name": "This machine", "dot": "quiet", "line": "all quiet", "title": "This machine's board: all quiet" },
    ]));
    // the terminal board showed every machine: with no other one, the web shows this machine's own, and cannot pick it
    assert_eq!(b.s("scope"), "", "{b}");
    assert_eq!(call("scope", json!({ "to": "*" }), None).unwrap_err(), "There is no other machine: every machine's board is this one's.");
    // a second machine: all machines is there again, at the top
    let m = sw::remote::Machine { label: "vm-a".into(), target: "me@vm-a".into(), ..Default::default() };
    told(vec![m]);
    let names: Vec<String> = boards(None).arr("boards").iter().map(|e| e.t("name")).collect();
    assert_eq!(names, vec!["All machines", "This machine", "vm-a"]);
}

#[test]
fn a_machine_that_cannot_be_reached_says_so_on_its_entry_and_a_machine_without_a_pc_keeps_the_boards_name() {
    let (_w, far) = web_world();
    far.ends("vm-a", "list", "", "ssh: connect to host vm-a port 22: Connection refused\n", 255);
    boards(Some(vec![pc("local", "Mac", "", true)]));
    far::answered("t", 1);
    let b = boards(None);
    let them = &b.arr("boards")[2];
    assert_eq!((them.s("name"), them.s("line"), them.at("off"), them.s("dot")), ("vm-a", "not reachable", &json!(true), ""));
    assert!(them.s("title").starts_with("vm-a is not reachable: "), "{}", them.s("title"));
    assert_eq!(b.arr("boards")[0].s("line"), "all quiet · 1 not reachable");
}

#[test]
fn all_machines_stands_first_then_the_machines_in_the_order_of_herdrs_sidebar_not_in_the_web_uis_order_of_its_pcs() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-a, vm-b" }), ..Default::default() });
    let far = Far::new(&w.base);
    std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str() }).to_string()).unwrap();
    // vm-b's herdr window shows this machine: its sidebar has vm-a first, then this machine
    let mine = json!({ "label": "Mac", "target": format!("me@{}", sw::remote::check::hostname()), "session": "default", "enabled": true, "selected": true });
    far.answers("vm-b", "list", &json!({ "host": "vm-b", "version": "0.3.0", "tasks": [], "sidebar": [{ "label": "vm-a", "target": "me@vm-a", "session": "default", "enabled": true }, mine] }));
    far.tasks("vm-a", "0.3.0", json!([]));
    let them = Some(vec![pc("local", "Mac", "", true), pc("pc-a", "box-a", "vm-a", false), pc("pc-b", "box-b", "vm-b", false)]);
    boards(them.clone());
    far::answered("t", 2);
    let order: Vec<String> = boards(them).arr("boards").iter().map(|e| e.t("name")).collect();
    assert_eq!(order, vec!["All machines", "box-b", "box-a", "Mac"], "this machine where herdr's sidebar puts it");
}

#[test]
fn picking_an_entry_shows_that_machines_board_as_other_machines_does_and_the_two_stay_in_step() {
    let (_w, far) = web_world();
    far.tasks("vm-a", "0.2.0", json!([far::theirs(1)]));
    boards(pcs());
    far::answered("t", 1);
    // a device that only watches may pick: it only changes what this browser looks at
    let s = call("scope", json!({ "to": "vm-a" }), None).unwrap();
    assert_eq!(s.at("flags").s("scope"), "vm-a");
    assert_eq!(s.at("list").at("head").s("only"), "vm-a");
    assert!(s.at("msg").is_null(), "a board you picked is not confirmed by a line: the filter row says whose it is: {}", s.at("msg"));
    assert_eq!(boards(None).s("scope"), "vm-a", "the section highlights what the board shows");
    // the other way round: the board's own Other machines moves the section's highlight with it
    call("press", json!({ "id": "scope" }), None).unwrap();
    assert_eq!(boards(None).s("scope"), "*", "after the last machine the round is back at every machine");
    call("scope", json!({ "to": "" }), None).unwrap();
    assert_eq!(boards(None).s("scope"), "");
    call("scope", json!({ "to": "*" }), None).unwrap();
    assert_eq!(boards(None).s("scope"), "*");
    assert!(call("scope", json!({ "to": "nowhere" }), None).unwrap_err().contains("nowhere"));
    assert_eq!(boards(None).s("scope"), "*", "nothing changed");
}

#[test]
fn a_new_page_shows_whose_board_the_terminal_board_shows_until_a_board_is_picked_on_the_web() {
    // The terminal board shows every machine; the tasks are all on another machine; the web board was last left on
    // this machine only (as every web board was before it took the terminal board's choice): the page showed nothing.
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-a" }), ..Default::default() });
    let far = Far::new(&w.base);
    let view = |scope: &str| std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str(), "scope": scope }).to_string()).unwrap();
    let kept = || sw::store::files::read_obj(&sw::store::files::data_dir().join("web-view.json")).unwrap_or_default();
    view("*");
    std::fs::write(sw::store::files::data_dir().join("web-view.json"), json!({ "scope": "" }).to_string()).unwrap();
    far.tasks("vm-a", "0.2.0", json!([far::theirs(1), far::theirs(2)]));
    boards(pcs());
    far::answered("t", 1);
    let s = call("board", json!({}), None).unwrap();
    let rows: Vec<String> = s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").iter().map(|r| r.t("title"))).collect();
    assert_eq!((s.at("flags").s("scope"), rows.clone()), ("*", vec!["Theirs 1".to_string(), "Theirs 2".into()]), "the terminal board's tasks");
    assert_eq!(s.at("list").s("empty"), "");
    // a board picked on the web is the web's own: a page opened later starts there, whatever the terminal board shows
    call("scope", json!({ "to": "" }), None).unwrap();
    assert_eq!(kept().s("ownScope"), "");
    sw::webui::web::forget();
    assert_eq!(call("board", json!({}), None).unwrap().at("flags").s("scope"), "");
    // picked back to what the terminal board shows: the web follows the terminal board again
    call("scope", json!({ "to": "*" }), None).unwrap();
    assert_eq!(kept().at("ownScope"), &json!(null));
    view("");
    sw::webui::web::forget();
    assert_eq!(call("board", json!({}), None).unwrap().at("flags").s("scope"), "");
}

#[test]
fn only_a_board_picked_on_the_web_is_kept_another_change_leaves_the_web_following_the_terminal_board() {
    // A page opened while the terminal board showed this machine; the terminal board then went to every machine; a
    // change of the grouping on the page saved the web's choices, and that was taken for a pick of this machine.
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-a" }), ..Default::default() });
    let far = Far::new(&w.base);
    let view = |scope: &str| std::fs::write(sw::store::files::data_dir().join("view.json"), json!({ "cwd": w.root_str(), "scope": scope }).to_string()).unwrap();
    let kept = || sw::store::files::read_obj(&sw::store::files::data_dir().join("web-view.json")).unwrap_or_default();
    far.tasks("vm-a", "0.2.0", json!([far::theirs(1)]));
    view("");
    assert_eq!(call("board", json!({}), None).unwrap().at("flags").s("scope"), "");
    view("*");
    call("press", json!({ "id": "byRepo", "role": "drive" }), None).unwrap();
    assert_eq!(kept().b("byRepo"), true, "the grouping is kept");
    assert_eq!(kept().at("ownScope"), &json!(null), "no board was picked");
    sw::webui::web::forget();
    boards(pcs());
    far::answered("t", 1);
    let s = call("board", json!({}), None).unwrap();
    assert_eq!(s.at("flags").s("scope"), "*");
    assert!(s.at("list").arr("groups").iter().any(|g| g.arr("rows").iter().any(|r| r.s("title") == "Theirs 1")), "{}", s.at("list"));
    // the scope key pressed on the web is a pick, as the sidebar's entry is
    let s = call("press", json!({ "id": "scope" }), None).unwrap();
    assert_ne!(s.at("flags").s("scope"), "*");
    assert_eq!(kept().at("ownScope"), s.at("flags").at("scope"));
}

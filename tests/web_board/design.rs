//! The web board in the terminal board's design (HP-127): one filter row (whose board first, then each filter with its
//! count, the one shown lit in its colour) that a tap changes and the page keeps; every row as far in as the terminal
//! sets it, machine and project rows their coordinators' with task marks and no second line; the chat's switch of the
//! top row's coordinator and the selected row's agent, named as the terminal's box names them (the label, the kind of
//! agent, the status colour); the grouping put back to by status; a project archived from its row; and a watching
//! device that cannot go into an agent.
use crate::common;
use common::board::{web, web_as};
use common::*;
use serde_json::{json, Value};
use sw::json::*;

fn coord_world() -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn group<'a>(s: &'a Value, f: &dyn Fn(&Value) -> bool) -> Value { s.at("list").arr("groups").iter().find(|g| f(g)).cloned().unwrap_or_else(|| panic!("{}", s.at("list"))) }
fn said(p: &Value) -> String { p.as_array().map(|a| a.iter().map(|x| x.t("t")).collect()).unwrap_or_default() }

#[test]
fn the_filter_row_counts_lights_the_one_shown_and_a_tap_shows_another() {
    let w = world();
    let a = w.add("Write notes");
    sw::actions::filing::report(&a, sw::actions::filing::Report { state: "working".into(), msg: "Drafting".into(), ..Default::default() }).unwrap();
    w.add("Later"); w.add("Much later");
    let s = web(&w, "board", json!({}));
    let shows = s.at("list").at("shows");
    let items: Vec<(String, u64, bool, String)> = shows.arr("items").iter().map(|x| (x.t("name"), x.at("count").as_u64().unwrap(), x.b("lit"), x.t("color"))).collect();
    assert_eq!(items[0], ("All".into(), 3, true, "text".into()));
    assert_eq!(items.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["All", "Coordinators", "Needs you", "Working", "Ready", "To do", "Done", "Automations", "Pinned"]);
    assert_eq!((items[3].1, items[3].3.as_str(), items[5].1, items[2].3.as_str(), items[7].3.as_str()), (1, "yellow", 2, "red", "auto"));
    assert!(shows.at("machine").is_null(), "no other machine: no machine picker");
    // a tap on To do: only its tasks, To do lit, and the next page shows the same
    let s = web(&w, "show", json!({ "n": 5 }));
    assert!(s.at("list").at("shows").arr("items")[5].b("lit") && !s.at("list").at("shows").arr("items")[0].b("lit"));
    let ids: Vec<String> = s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").iter().map(|r| r.t("title")).collect::<Vec<_>>()).collect();
    assert_eq!(ids.len(), 2, "{ids:?}"); assert!(ids.iter().all(|t| t.contains("ater")));
    sw::webui::web::forget();
    assert!(web(&w, "board", json!({})).at("list").at("shows").arr("items")[5].b("lit"), "kept for the next page");
    // a watching device may change what it looks at; the archive has no filter row
    assert!(web_as(&w, "other", "show", json!({ "n": 0, "role": "watch" })).is_ok());
    let s = web(&w, "press", json!({ "id": "archiveView" }));
    assert!(s.at("list").at("shows").is_null());
    // the page's own control: no Next filter button
    web(&w, "press", json!({ "id": "archiveView" }));
    let s = web(&w, "pick", json!({ "id": a, "open": true }));
    assert!(!s.arr("actions").iter().any(|b| b.s("id") == "filter"), "{}", s.at("actions"));
}

#[test]
fn rows_stand_as_far_in_as_the_terminal_sets_them_and_coordinator_rows_read_as_task_rows() {
    let w = coord_world();
    let a = w.add("Fix the login");
    let s = web(&w, "press", json!({ "id": "byRepo" }));
    let host = sw::shown::words::clean(&sw::remote::check::hostname());
    // the top row (this machine, the only one): bold in the base colour, what it holds dimmed, its coordinator's mark
    let top = group(&s, &|g| g.b("top"));
    assert_eq!((top.t("name"), top.at("depth"), top.t("color")), (host.clone(), &json!(0), "plain".to_string()));
    assert_eq!(top.at("words")[0], json!({ "t": host, "b": true }));
    assert_eq!(said(&top.at("words")), format!("{host}  1 project · 1 task"));
    assert_eq!(top.at("words")[1].s("c"), "overlay0");
    assert_eq!((said(&top.at("pilot").at("mark")), top.at("pilot").s("label"), top.at("pilot").arr("under").len()), ("○".to_string(), "idle", 0), "never a second line");
    // the project's row, one step in, its name in normal weight with its key dimmed; its task one step further
    let proj = group(&s, &|g| g.s("id").starts_with("head:repo"));
    assert_eq!((proj.t("name"), proj.at("depth"), proj.b("repo"), proj.b("coord")), ("proj".to_string(), &json!(1), true, true));
    assert!(!proj.at("words")[0].b("b"), "{}", proj.at("words"));
    assert_eq!(proj.arr("rows").iter().map(|r| (r.t("id"), r.at("depth").clone())).collect::<Vec<_>>(), vec![(a.clone(), json!(2))]);
    // by status: the status groups one step in, in their colours, their tasks where they stand
    web(&w, "group", json!({}));
    let s = web(&w, "board", json!({}));
    let todo = group(&s, &|g| g.s("name") == "to do");
    assert_eq!((todo.at("depth"), todo.t("color"), todo.at("words")[0].s("c"), todo.b("coord")), (&json!(1), "todo".to_string(), "todo", false));
    assert_eq!(todo.arr("rows")[0].at("depth"), &json!(1));
}

#[test]
fn the_chats_switch_names_the_top_rows_coordinator_and_the_selected_rows_agent_as_the_terminals_box_does() {
    let w = coord_world();
    let a = w.add("Fix the login");
    let host = sw::shown::words::clean(&sw::remote::check::hostname());
    let s = web(&w, "pick", json!({ "id": a }));
    let c = s.at("chat");
    let tag = c.t("task");
    // the top row's coordinator by default, never a main "Coordinator": its label, its kind, the base colour (never ran)
    assert_eq!((c.t("top"), c.t("row"), c.at("toAgent")), (host.clone(), tag.clone(), &json!(false)), "{c}");
    assert_eq!((c.at("title").at("steps"), c.at("title").s("kind"), c.at("title").s("edge"), c.at("title").at("fresh")), (&json!([host]), "Machine coordinator", "text", &json!(true)), "{c}");
    assert_eq!(c.s("box"), "text");
    assert_eq!(c.at("mark").s("t"), "○", "its mark is its row's, as the list has it: {c}");
    // space (the other choice): the task's agent, named by its tag, `Agent`, its status colour (to do: the base colour)
    let s = web(&w, "press", json!({ "id": "talk" }));
    let t = s.at("chat").at("title");
    assert_eq!((t.arr("steps").last().and_then(|x| x.as_str()).unwrap_or_default().to_string(), t.s("kind"), s.at("chat").at("toAgent")), (tag.clone(), "Agent", &json!(true)), "{}", s.at("chat"));
    sw::actions::filing::report(&a, sw::actions::filing::Report { state: "blocked".into(), kind: "question".into(), msg: "Which login?".into(), ..Default::default() }).unwrap();
    let s = web(&w, "board", json!({}));
    assert_eq!((s.at("chat").at("title").s("edge"), s.at("chat").s("box")), ("red", "red"), "needs you: its frame and name in red");
    // a project's row: its coordinator is the other choice, by the project's name, `Project coordinator`
    web(&w, "press", json!({ "id": "talk" }));
    web(&w, "press", json!({ "id": "byRepo" }));
    let head = format!("{}repo\t\tproj", sw::board::update::heads::HEAD);
    let s = web(&w, "pick", json!({ "id": head }));
    assert_eq!((s.at("chat").t("row"), s.at("chat").at("toAgent")), ("proj".to_string(), &json!(false)), "{}", s.at("chat"));
    let s = web(&w, "press", json!({ "id": "talk" }));
    assert_eq!((s.at("chat").at("title").at("steps"), s.at("chat").at("title").s("kind")), (&json!([host, "proj"]), "Project coordinator"), "{}", s.at("chat"));
    // on the top row there is nobody else to talk to
    let s = web(&w, "pick", json!({ "id": sw::board::update::top::top_id(&common::board::open(&w.root, 40).s) }));
    assert_eq!(s.at("chat").t("row"), "");
}

#[test]
fn a_chip_puts_the_grouping_back_to_by_status_and_the_page_keeps_by_machine() {
    let w = world();
    w.add("Later");
    web(&w, "press", json!({ "id": "byRepo" }));
    let s = web(&w, "press", json!({ "id": "byRepo" }));
    assert_eq!((s.at("flags").at("byRepo"), s.at("flags").at("byMachine")), (&json!(false), &json!(true)));
    sw::webui::web::forget();
    let s = web(&w, "board", json!({}));
    assert_eq!(s.at("flags").at("byMachine"), &json!(true), "kept for the next page");
    let s = web(&w, "group", json!({}));
    assert_eq!((s.at("flags").at("byRepo"), s.at("flags").at("byMachine")), (&json!(false), &json!(false)));
}

#[test]
fn a_projects_row_archives_it_from_the_more_menu_and_a_watching_device_cannot_go_into_an_agent() {
    let w = world();
    let id = w.add("To do");
    web(&w, "press", json!({ "id": "byRepo" }));
    let head = format!("{}repo\t\tproj", sw::board::update::heads::HEAD);
    let s = web(&w, "pick", json!({ "id": head }));
    assert!(s.arr("menu").iter().any(|m| m.s("id") == "archive" && m.s("label").starts_with("archive this project")), "{}", s.at("menu"));
    assert!(!s.arr("actions").iter().any(|b| b.s("id") == "enter"), "its caret folds it on the page: {}", s.at("actions"));
    // in the archive the archived project's row has its restore (the page draws it under the row)
    web(&w, "press", json!({ "id": "archive" })); web(&w, "press", json!({ "id": "sayYes" }));
    web(&w, "press", json!({ "id": "archiveView" }));
    let s = web(&w, "pick", json!({ "id": head }));
    assert!(s.arr("actions").iter().any(|b| b.s("id") == "archive" && b.s("label") == "restore this project"), "{}", s.at("actions"));
    assert!(group(&s, &|g| g.s("id") == head).b("repo"));
    web(&w, "press", json!({ "id": "archive" }));
    assert!(!sw::store::tasks::list(Default::default()).iter().any(|t| t.s("id") == id && t.b("archivedAt")));
    // going into an agent starts or reopens it: not for a device that only watches
    assert_eq!(web_as(&w, "watcher", "press", json!({ "id": "open", "role": "watch" })).unwrap_err(), "This device can only watch.");
}

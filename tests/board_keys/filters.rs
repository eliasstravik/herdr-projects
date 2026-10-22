//! The filter row on top: <machine> │ All · Coordinators · Needs you · Working · Ready · To do · Done · Automations,
//! each with its count, the one shown lit in its own colour. Tab and shift+tab go round it, a click picks one, the list
//! shows only that kind of row; a machine's row and a project's row stand for their coordinators (space talks to
//! one), and Coordinators leaves only those and the main Coordinator's. Alt+S groups by status, repo, machine.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::state::Row;
use sw::board::update::heads::{group_id, machine_id};
use sw::board::update::{update, Event};
use sw::json::*;
use sw::shown::ink::Ink;

fn coord_world(machines: &str) -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never", "board.machines": machines }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn tasks_shown(s: &common::board::Board) -> Vec<String> { s.s.rows.iter().filter_map(|r| match r { Row::Rec(o) => Some(o.t("title")), _ => None }).collect() }
fn heads(s: &common::board::Board) -> Vec<String> { s.s.rows.iter().filter_map(|r| match r { Row::Head { name, .. } => Some(name.clone()), _ => None }).collect() }
fn to_all(s: &mut common::board::Board) { for _ in 0..4 { if s.s.scope == "*" { return; } s.press(&["alt+m"]); } panic!("never every machine"); }
/// The filter row's piece that says `name`, with its ink and whether it is reversed.
fn piece(s: &mut common::board::Board, name: &str) -> (Ink, bool) {
    let l = s.inked(140);
    let p = l[1].0.iter().find(|p| p.t.contains(name)).unwrap_or_else(|| panic!("{name}: {:?}", l[1].text()));
    (p.c, p.inv)
}

#[test]
fn tab_goes_round_the_filters_lit_in_their_own_colours_and_the_list_shows_only_that_kind() {
    let w = coord_world("none");
    let asks = w.add("Asks you"); sw::actions::filing::report(&asks, sw::actions::filing::Report { state: "blocked".into(), msg: "which one?".into(), ..Default::default() }).unwrap();
    w.add("Plain to do");
    for n in 0..7 { let id = w.add(&format!("Done {n}")); sw::store::tasks::update(&id, |t| { t.put("stage", "done"); Ok(sw::store::tasks::Change::Write) }).unwrap(); }
    let mut s = open(&w.root, 40); s.s.side = false;
    let l = s.screen(120);
    assert_eq!(l[0], " herdr-projects  proj · PRJ");
    assert!(regex::Regex::new(r"^ +All 9 +Coordinators 1   Needs you 1   Working 0   Ready 0   To do 1   Done 7   Automations 0$").unwrap().is_match(&l[1]), "{l:?}");
    assert!(!l.iter().any(|x| x.starts_with(" ") && x.contains("──")), "no heading has a rule: {l:?}");
    // All is lit in the plain text colour, reversed
    assert_eq!(piece(&mut s, "All 9"), (Ink::Text, true));
    // each item keeps a cell of room on each side, lit or not: lighting another moves nothing
    s.lines(); let before = s.s.show_hits.clone();
    s.press(&["tab"]); s.lines(); assert_eq!(s.s.show_hits, before, "the row does not move");
    assert_eq!(piece(&mut s, "Coordinators 1"), (Ink::Accent, true));
    assert!(s.inked(140)[1].0.iter().any(|p| p.inv && p.t == " Coordinators 1 "), "a cell of highlight on each side");
    s.press(&["shift+tab"]);
    // tab: Coordinators, lit in the accent colour; then needs you only, in red
    s.press(&["tab"]); assert_eq!(s.s.show, 1); assert_eq!(piece(&mut s, "Coordinators 1"), (Ink::Accent, true));
    assert!(tasks_shown(&s).is_empty());
    s.press(&["tab"]); assert_eq!(s.s.show, 2); assert_eq!(piece(&mut s, "Needs you 1"), (Ink::Red, true));
    assert_eq!(tasks_shown(&s), vec!["Asks you"]);
    s.press(&["tab"]); assert_eq!(piece(&mut s, "Working 0"), (Ink::Yellow, true));
    s.press(&["tab"]); assert_eq!(piece(&mut s, "Ready 0"), (Ink::Green, true));
    s.press(&["tab"]); assert_eq!(s.s.show, 5); assert_eq!(tasks_shown(&s), vec!["Plain to do"]);
    // done: every done task, not the latest five
    s.press(&["tab"]); assert_eq!(tasks_shown(&s).len(), 7, "{:?}", tasks_shown(&s));
    s.press(&["tab"]); assert_eq!(s.s.show, 7); assert_eq!(piece(&mut s, "Automations 0"), (Ink::Auto, true));
    assert!(tasks_shown(&s).is_empty());
    // shift+tab: back; round again
    s.press(&["shift+tab"]); assert_eq!(s.s.show, 6);
    s.press(&["tab", "tab"]); assert_eq!(s.s.show, 0);
    // a click on a filter picks it
    s.lines();
    let (y, x, _, _) = *s.s.show_hits.iter().find(|h| h.3 == 2).unwrap();
    let fx = update(&mut s.s, Event::Click { x: x as u16, y: y as u16 }); s.run(fx);
    assert_eq!(s.s.show, 2);
    // tab never puts an agent on a task any more
    assert!(sw::store::tasks::list(Default::default()).iter().all(|t| t.s("stage") != "progress"));
    let view: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(sw::store::files::data_dir().join("view.json")).unwrap()).unwrap();
    assert_eq!(view.at("show"), &json!(2));
    // narrow: shorter names, then only the one shown
    assert!(s.screen(76)[1].contains("Coords 1 ") && s.screen(76)[1].contains("Needs 1"), "{:?}", s.screen(76));
    assert_eq!(s.screen(30)[1].trim(), "Needs you 1  tab next");
}

#[test]
fn space_on_a_machines_row_or_a_projects_row_talks_to_its_coordinator_and_coordinators_leaves_only_those() {
    let w = coord_world("otherbox");
    w.add("Fix the login");
    w.script(json!({ "coordinator": [{ "said": "Proj is quiet." }] }));
    let host = sw::shown::words::clean(&sw::remote::check::hostname());
    let mut s = open(&w.root, 40); s.s.side = false;
    // by repo: the project's row (its heading) is its coordinator's: space talks to it
    s.press(&["alt+s"]);
    s.s.sel = group_id(&s.s, "", "proj"); s.load();
    s.press(&["space"]);
    assert!(s.s.reply && s.text().contains("Send message to proj coordinator"), "{}", s.text());
    s.typed("hello"); s.press(&["enter"]); drain(); s.load();
    let t = s.text();
    // one line; idle, its coordinator has no mark (the column blank), no time, nothing under it: the chat has it
    assert!(regex::Regex::new(r"\n[ ▌]  ▾ proj · PRJ *\n[ ▌] +○ PRJ-1").unwrap().is_match(&t) && !t.contains("\n     Proj is quiet.") && !t.contains("◈"), "{t}");
    s.press(&["space", "alt+s", "alt+s"]); assert!(!s.s.by_repo && !s.s.by_machine);
    // every machine: space on a machine's row talks to its machine coordinator
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    let answer = json!({ "host": "otherbox", "version": "9.0.0", "coordinator": { "state": "working", "readable": true, "said": "Far main is busy." },
        "coordinators": { "machine": { "state": "idle", "readable": true, "said": "Far machine is quiet." }, "project:far": { "state": "idle", "said": "Far project." } }, "tasks": [] });
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: Ok(answer) });
    to_all(&mut s);
    s.s.sel = machine_id("otherbox"); s.load();
    s.press(&["space"]);
    assert!(s.s.reply && s.text().contains("Send message to otherbox coordinator") && s.text().contains("Far machine is quiet."), "{}", s.text());
    s.press(&["space"]);
    // Coordinators: each machine's row and its projects' rows; no task, and no row of a main Coordinator's
    s.press(&["tab"]); assert_eq!(s.s.show, 1);
    let mut want = vec![host.clone(), "proj".into(), "otherbox".into(), "far".into()];
    if sw::board::update::remotes::place(&s.s, "otherbox") < sw::board::update::remotes::place(&s.s, "") { want = vec!["otherbox".into(), "far".into(), host.clone(), "proj".into()]; }
    assert_eq!(heads(&s), want);
    assert!(tasks_shown(&s).is_empty());
    assert!(regex::Regex::new(r"Coordinators 4 ").unwrap().is_match(&s.screen(160)[1]), "{:?}", s.screen(160));
    assert!(!s.text().contains("Coordinator on otherbox"), "{}", s.text());
    // a project's row folds nothing there: enter talks to it, as space does
    s.s.sel = format!("head:repo\totherbox\tfar"); s.load();
    assert!(s.text().contains("enter talk to it"), "{}", s.text());
    s.press(&["enter"]); assert!(s.s.reply && s.text().contains("Send message to far coordinator"), "{}", s.text());
    s.press(&["space"]);
    // a machine's row with its projects' rows under it folds them
    s.s.sel = machine_id("otherbox"); s.load();
    s.press(&["enter"]); assert!(s.s.folded.contains(&machine_id("otherbox")) && !s.s.reply, "{}", s.text());
    s.press(&["enter"]);
    // one machine shown: its own only, and the count is its
    while s.s.scope != "otherbox" { s.press(&["alt+m"]); }
    assert_eq!(heads(&s), vec!["otherbox".to_string(), "far".into()], "that machine's row and its project's; its own Coordinator has none");
    assert!(s.screen(160)[1].starts_with(" otherbox ▾ │ ") && regex::Regex::new(r"Coordinators 2 ").unwrap().is_match(&s.screen(160)[1]), "{:?}", s.screen(160));
    // a click on the machine item goes round as alt+m does
    s.lines();
    let (y, x, _, _) = *s.s.show_hits.iter().find(|h| h.3 == sw::board::view::shows_row::MACHINE).unwrap();
    let fx = update(&mut s.s, Event::Click { x: x as u16, y: y as u16 }); s.run(fx);
    assert_ne!(s.s.scope, "otherbox");
}

#[test]
fn automations_is_a_filter_with_only_the_automations_rows_and_its_count() {
    use sw::automations::list::{self as l, Add};
    let w = coord_world("none");
    w.add("A task");
    l::add(Add { name: "Nightly audit", prompt: "Audit.", trigger: Some(sw::automations::triggers::parse_schedule("daily 03:00").unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: Some(true) }).unwrap();
    let mut s = open(&w.root, 40); s.s.side = false;
    assert!(s.screen(140)[1].ends_with("Automations 1"), "{:?}", s.screen(140));
    s.press(&["shift+tab"]); assert_eq!(s.s.show, 7);
    assert!(tasks_shown(&s).is_empty() && s.s.rows.iter().any(|r| matches!(r, Row::Auto(_))), "{}", s.text());
    assert!(s.text().contains("Nightly audit") && !s.text().contains("A task"), "{}", s.text());
}

#[test]
fn alt_s_groups_by_status_by_repo_by_machine_and_round_again() {
    let w = coord_world("none");
    w.add("One"); w.add("Two");
    let host = sw::shown::words::clean(&sw::remote::check::hostname());
    let mut s = open(&w.root, 40); s.s.side = false;
    let plain = |s: &common::board::Board| heads(s).into_iter().filter(|h| h != "Coordinator").collect::<Vec<_>>();
    assert_eq!(plain(&s), vec!["to do"]);
    s.press(&["alt+s"]); assert!(s.s.by_repo); assert_eq!(plain(&s), vec!["proj"]);
    s.press(&["alt+s"]); assert!(s.s.by_machine && !s.s.by_repo);
    assert_eq!(plain(&s), vec![host.clone()], "the machine's row alone, no status heading under it");
    assert_eq!(tasks_shown(&s).len(), 2);
    s.press(&["alt+s"]); assert!(!s.s.by_machine && !s.s.by_repo); assert_eq!(plain(&s), vec!["to do"]);
}

#[test]
fn the_menus_start_line_puts_an_agent_on_the_task() {
    let w = coord_world("none");
    let id = w.add("Start me from the menu");
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Working\n" }] }));
    let mut s = open(&w.root, 60); s.s.side = false;
    s.s.sel = id.clone(); s.load();
    s.press(&["?"]);
    let at = sw::board::update::menu::items(&s.s).iter().position(|i| i.id == sw::board::keys::K::Delegate).expect("a start line");
    for _ in 0..=at { s.press(&["down"]); }
    s.press(&["enter"]); settle(&id); s.load();
    assert_ne!(task(&id).s("stage"), "todo", "{:?}", task(&id));
}

/// Enter on a heading (and a click on it) folds it to one line that says what it holds, and unfolds it.
fn folds(s: &mut common::board::Board, id: &str, holds: &str) {
    s.s.sel = id.to_string(); s.load();
    let name = match s.s.rows.iter().find(|r| matches!(r, Row::Head { id: h, .. } if h == id)) { Some(Row::Head { name, .. }) => sw::shown::words::repo_label(name), _ => panic!("no heading {id}: {}", s.text()) };
    s.press(&["enter"]);
    assert!(s.s.folded.contains(id) && regex::Regex::new(&format!(r"\n[ ▌]\s*▸ {}( · \w+)?  {holds}", regex::escape(&name))).unwrap().is_match(&s.text()), "{id} folds to {holds}: {}", s.text());
    s.press(&["enter"]);
    assert!(!s.s.folded.contains(id), "{id} unfolds");
    // a click does the same
    s.lines();
    let y = s.s.list_at.unwrap().0 + s.s.list_ids.iter().position(|x| x == id).unwrap();
    let fx = update(&mut s.s, Event::Click { x: 3, y: y as u16 }); s.run(fx);
    assert!(s.s.folded.contains(id), "a click folds {id}");
    let fx = update(&mut s.s, Event::Click { x: 3, y: y as u16 }); s.run(fx);
    assert!(!s.s.folded.contains(id), "a click unfolds {id}");
}

#[test]
fn enter_folds_every_kind_of_heading_under_every_filter_and_grouping_and_space_talks() {
    let w = coord_world("otherbox");
    w.add("Here one");
    let asks = w.add("Asks you"); sw::actions::filing::report(&asks, sw::actions::filing::Report { state: "blocked".into(), msg: "which one?".into(), ..Default::default() }).unwrap();
    w.script(json!({ "coordinator": [{ "said": "Proj is quiet." }] }));
    let host = sw::shown::words::clean(&sw::remote::check::hostname());
    let mut s = open(&w.root, 60); s.s.side = false;
    // this machine, by status: a status group's heading
    { let g = group_id(&s.s, "", "to do"); folds(&mut s, &g, "1 task"); }
    // the filter Needs you: its group's heading, and no empty heading of anything else
    s.press(&["tab", "tab"]); assert_eq!(s.s.show, 2);
    { let g = group_id(&s.s, "", "needs you"); folds(&mut s, &g, "1 task"); }
    s.press(&["shift+tab", "shift+tab"]); assert_eq!(s.s.show, 0);
    // by repo: a repo's row, which is its project coordinator's (space talks to it)
    s.press(&["alt+s"]);
    { let g = group_id(&s.s, "", "proj"); folds(&mut s, &g, "2 tasks"); }
    s.s.sel = group_id(&s.s, "", "proj"); s.load();
    s.press(&["space"]); assert!(s.text().contains("Send message to proj coordinator"), "{}", s.text());
    s.typed("hello"); s.press(&["enter"]); drain(); s.press(&["space"]); s.load();
    { let g = group_id(&s.s, "", "proj"); folds(&mut s, &g, "2 tasks"); }
    // by machine: this machine's row
    s.press(&["alt+s"]); assert!(s.s.by_machine);
    folds(&mut s, &machine_id(""), "2 tasks");
    s.press(&["alt+s"]);
    // every machine: this machine's row and the other one's, by status and by repo
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    let now = sw::clock::now();
    let answer = json!({ "host": "otherbox", "version": "9.0.0", "coordinator": { "state": "idle" }, "coordinators": { "machine": { "state": "idle", "said": "Far machine is quiet." } },
        "tasks": [{ "id": "far/1", "title": "Far away", "state": "idle", "stage": "todo", "at": now, "created": now, "snap": { "folder": "~/far" } }] });
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: Ok(answer) });
    to_all(&mut s);
    folds(&mut s, &machine_id(""), "2 tasks");
    folds(&mut s, &machine_id("otherbox"), "1 task");
    { let g = group_id(&s.s, "otherbox", "to do"); folds(&mut s, &g, "1 task"); }
    s.press(&["alt+s"]);
    { let g = group_id(&s.s, "otherbox", "far"); folds(&mut s, &g, "1 task"); }
    s.s.sel = machine_id("otherbox"); s.load();
    s.press(&["space"]); assert!(s.text().contains("Send message to otherbox coordinator"), "{}", s.text());
    s.press(&["space", "alt+s", "alt+s"]);
    // Coordinators: a machine's row folds its projects' rows
    s.press(&["tab"]); assert_eq!(s.s.show, 1);
    folds(&mut s, &machine_id(""), "1 project coordinator");
    let _ = host;
}

#[test]
fn ctrl_s_starts_a_coordinator_ctrl_x_stops_it_and_ctrl_o_is_its_autopilot() {
    let w = coord_world("otherbox");
    w.add("Fix the login");
    let mut s = open(&w.root, 40); s.s.side = false;
    let jobs = |fx: Vec<sw::board::update::Effect>| fx.iter().filter_map(|e| match e { sw::board::update::Effect::Run(j) => Some(format!("{j:?}")), _ => None }).collect::<Vec<_>>().join(" | ");
    s.press(&["alt+s"]);
    s.s.sel = group_id(&s.s, "", "proj"); s.load();
    assert!(s.text().contains("ctrl+s start · ctrl+x stop · ctrl+o autopilot ·"), "{}", s.text());
    let j = jobs(s.hold("ctrl+s")); assert!(j.contains("CoordRun") && j.contains("project:proj") && j.contains("on: true"), "{j}");
    let j = jobs(s.hold("ctrl+x")); assert!(j.contains("CoordRun") && j.contains("on: false"), "{j}");
    let j = jobs(s.hold("ctrl+o")); assert!(j.contains("Pilot") && j.contains("project:proj") && j.contains("on: true"), "{j}");
    // the main Coordinator, which has no row: on a status group's heading, as the chat talks to it there
    s.press(&["alt+s", "alt+s"]); s.s.sel = group_id(&s.s, "", "to do"); s.load();
    let j = jobs(s.hold("ctrl+s")); assert!(j.contains("CoordRun") && j.contains("to: \"\""), "{j}");
    s.press(&["alt+s"]);
    // another machine's: on that machine
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: Ok(json!({ "host": "otherbox", "version": "9.0.0", "coordinator": { "state": "idle" }, "coordinators": {}, "autopilot": {}, "tasks": [] })) });
    to_all(&mut s);
    s.s.sel = machine_id("otherbox"); s.load();
    let j = jobs(s.hold("ctrl+s")); assert!(j.contains("coordinator-start") && j.contains("otherbox"), "{j}");
    let j = jobs(s.hold("ctrl+x")); assert!(j.contains("coordinator-stop"), "{j}");
    // on a task, ctrl+o says what it is for
    s.press(&["alt+s"]);
    s.s.sel = "proj/1".into(); s.load();
    s.press(&["ctrl+o"]); assert!(s.s.msg.contains("turns a coordinator's autopilot on and off"), "{}", s.s.msg);
}

/// A machine's or a project's row: its coordinator's state as a task's mark, only where it says something; idle,
/// stopped or never started, the column blank. No ◈ and no main Coordinator's row anywhere.
#[test]
fn machine_and_project_rows_mark_their_coordinators_state_with_a_tasks_marks_or_none() {
    let w = coord_world("otherbox");
    w.add("Fix the login");
    let mut s = open(&w.root, 40); s.s.side = false;
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    let answer = json!({ "host": "otherbox", "version": "9.0.0", "coordinator": { "state": "working" }, "tasks": [],
        "coordinators": { "machine": { "state": "working" }, "project:asks": { "state": "blocked" }, "project:broke": { "state": "none", "error": "no claude" }, "project:quiet": { "state": "idle" }, "project:pilot": { "state": "idle" } },
        "autopilot": { "project:pilot": { "on": true, "state": "working" } } });
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: Ok(answer) });
    to_all(&mut s); s.press(&["tab"]); assert_eq!(s.s.show, 1);
    // every row shows, though none holds anything under it
    let l = s.screen(120);
    let row = |name: &str| l.iter().find(|x| x.trim_start_matches(['▌', ' ', '◐', '!', '✗', '∞']).trim_start().starts_with(&format!("▾ {name}")) || x.contains(&format!("▸ {name}"))).cloned().unwrap_or_else(|| panic!("no {name}: {l:#?}"));
    assert!(row("otherbox").contains("◐ ▾ otherbox"), "working: {l:#?}");
    assert!(row("asks").contains("! ▾ asks") && row("broke").contains("✗ ▾ broke") && row("pilot").contains("∞ ▾ pilot"), "{l:#?}");
    assert!(row("quiet").contains("    ▾ quiet"), "idle: no mark, the column blank: {l:#?}");
    let text = l.join("\n");
    assert!(!text.contains("◈") && !text.contains("◎") && !s.s.rows.iter().any(|r| matches!(r, Row::Head { name, .. } if name == "Coordinator")), "{text}");
    let red = s.inked(120).iter().flat_map(|x| x.0.clone()).filter(|p| p.t == "!" || p.t == "✗").all(|p| p.c == Ink::Red && p.bold);
    assert!(red, "what needs you and what went wrong: red, bold");
}

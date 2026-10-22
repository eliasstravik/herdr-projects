//! The rows of coordinators, read as tasks' rows: the top row (All machines, or the one machine) the board opens on,
//! a task's marks for where each coordinator stands, its time on the right and its live second line; enter goes into
//! its coordinator, ← and → fold, ← on a task goes to its heading; one that asks you stands under Needs you.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::state::Row;
use sw::board::update::heads::{group_id, machine_id};
use sw::board::update::{update, Event};
use sw::shown::ink::Ink;

fn coord_world(machines: &str) -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never", "board.machines": machines }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn heads(s: &common::board::Board) -> Vec<String> { s.s.rows.iter().filter_map(|r| match r { Row::Head { name, .. } => Some(name.clone()), _ => None }).collect() }
fn to_all(s: &mut common::board::Board) { for _ in 0..4 { if s.s.scope == "*" { return; } s.press(&["alt+m"]); } panic!("never every machine"); }
fn jobs(fx: Vec<sw::board::update::Effect>) -> String { fx.iter().filter_map(|e| match e { sw::board::update::Effect::Run(j) => Some(format!("{j:?}")), _ => None }).collect::<Vec<_>>().join(" | ") }
/// A board of this machine (one task in `proj`) and otherbox, with what otherbox says of its coordinators.
fn far_board(coordinators: serde_json::Value, autopilot: serde_json::Value, tasks: serde_json::Value) -> (World, common::board::Board) {
    let w = coord_world("otherbox");
    w.add("Fix the login");
    let mut s = open(&w.root, 40); s.s.side = false;
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    let answer = json!({ "host": "otherbox", "version": "9.0.0", "coordinator": { "state": "idle" }, "tasks": tasks, "coordinators": coordinators, "autopilot": autopilot });
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: Ok(answer) });
    to_all(&mut s);
    (w, s)
}

#[test]
fn the_board_opens_on_its_top_row_and_enter_goes_into_a_rows_coordinator() {
    let w = coord_world("none");
    let id = w.add("Fix the login");
    let host = sw::shown::words::clean(&sw::remote::check::hostname());
    let ctx = sw::board::state::Ctx { cwd: w.root.display().to_string(), can_reload: true, ..Default::default() };
    let mut s = common::board::Board { s: sw::board::run::start::view(ctx, &w.root), rows: 40, cols: 120, closed: false, theme: sw::board::theme::Theme::default_dark(true) };
    s.s.side = false; s.load();
    // one machine: its row is the top row, the board opens on it, and the chat talks to its coordinator
    assert_eq!(s.s.sel, machine_id(""));
    assert!(s.text().contains(&format!("{host}'s coordinator.")), "{}", s.text());
    assert!(s.text().contains("enter go into it") && s.text().contains("← → fold"), "{}", s.text());
    // enter: into its coordinator, here; on a project's row, into that one's
    let j = jobs(s.hold("enter")); assert!(j.contains("CoordOpen") && j.contains("to: \"\""), "{j}");
    s.press(&["alt+s"]); s.s.sel = group_id(&s.s, "", "proj"); s.load();
    let j = jobs(s.hold("enter")); assert!(j.contains("CoordOpen") && j.contains("project:proj"), "{j}");
    // a status group's heading has no agent: enter says how it folds, and starts nothing
    s.press(&["alt+s", "alt+s"]); s.s.sel = group_id(&s.s, "", "to do"); s.load();
    let j = jobs(s.hold("enter")); assert!(j.is_empty(), "{j}");
    assert!(s.s.msg.contains("has no agent to go into"), "{}", s.s.msg);
    // ← on a task goes to the heading it stands under; ← there folds it, → unfolds it
    s.s.sel = id.clone(); s.load();
    s.press(&["left"]); assert_eq!(s.s.sel, group_id(&s.s, "", "to do"));
    s.press(&["left"]); assert!(s.s.folded.contains(&s.s.sel));
    s.press(&["right"]); assert!(!s.s.folded.contains(&s.s.sel));
    // with words typed, ← and → move in them and fold nothing
    s.typed("ab"); s.press(&["left"]); assert!(!s.s.folded.contains(&s.s.sel)); s.press(&["esc"]);
}

#[test]
fn another_machines_coordinator_is_gone_into_there() {
    let (_w, mut s) = far_board(json!({ "machine": { "state": "idle" } }), json!({}), json!([]));
    s.s.sel = machine_id("otherbox"); s.load();
    let j = jobs(s.hold("enter")); assert!(j.contains("coordinator-open") && j.contains("otherbox") && j.contains("\"to\": String(\"machine\")"), "{j}");
    // All machines: the coordinator of every machine, this machine's top-level one
    s.s.sel = sw::board::update::top::all_id(); s.load();
    let j = jobs(s.hold("enter")); assert!(j.contains("CoordOpen") && j.contains("to: \"\""), "{j}");
}

#[test]
fn a_coordinator_that_asks_you_stands_under_needs_you_with_its_question_and_is_counted() {
    let now = sw::clock::now();
    let (_w, mut s) = far_board(json!({ "machine": { "state": "working", "doing": "Cleaning old worktrees", "began": now - 120000 },
        "project:gtm": { "state": "idle", "talk": [{ "you": false, "text": "Two PRs wait on CI. Want me to retry #11?" }], "at": now - 60000 } }), json!({}), json!([]));
    let l = s.screen(120).join("\n");
    // the right side as on tasks: how long it has worked, how long ago it spoke; the live line under it
    assert!(regex::Regex::new(r"\n[ ▌]  ◐ ▸ otherbox  0 tasks +2m *\n[ ▌] +Cleaning old worktrees").unwrap().is_match(&l), "{l}");
    s.press(&["tab", "tab"]); assert_eq!(s.s.show, 2);
    let l = s.screen(120);
    assert!(regex::Regex::new(r"Needs you 1 ").unwrap().is_match(&l[1]), "{:?}", l);
    let t = l.join("\n");
    assert!(regex::Regex::new(r"\n[ ▌]    \? ▾ gtm +1m ago *\n[ ▌] +Two PRs wait on CI\. Want me to retry #11\?").unwrap().is_match(&t), "{t}");
    let q = s.inked(120).iter().flat_map(|x| x.0.clone()).find(|p| p.t == "?").unwrap();
    assert!(q.c == Ink::Red && q.bold, "asks you: red, bold");
}

/// A machine's or a project's row: its coordinator's state as a task's mark (idle, stopped or never started: ○). No ◈,
/// no ◎ and no main Coordinator's row anywhere.
#[test]
fn machine_and_project_rows_mark_their_coordinators_state_with_a_tasks_marks_or_none() {
    let w = coord_world("otherbox");
    w.add("Fix the login");
    let mut s = open(&w.root, 40); s.s.side = false;
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    let answer = json!({ "host": "otherbox", "version": "9.0.0", "coordinator": { "state": "working" }, "tasks": [],
        "coordinators": { "machine": { "state": "working" }, "project:asks": { "state": "blocked" }, "project:broke": { "state": "none", "error": "no claude" }, "project:quiet": { "state": "idle" }, "project:pilot": { "state": "idle" }, "project:stopped": { "state": "exited" }, "project:never": { "state": "none" } },
        "autopilot": { "project:pilot": { "on": true, "state": "working" } } });
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: Ok(answer) });
    to_all(&mut s); s.press(&["tab"]); assert_eq!(s.s.show, 1);
    // every row shows, though none holds anything under it
    let l = s.screen(120);
    let row = |name: &str| l.iter().find(|x| x.trim_start_matches(['▌', ' ', '◐', '!', '✗', '∞', '○']).trim_start().starts_with(&format!("▾ {name}")) || x.contains(&format!("▸ {name}"))).cloned().unwrap_or_else(|| panic!("no {name}: {l:#?}"));
    assert!(row("otherbox").contains("◐ ▾ otherbox"), "working: {l:#?}");
    assert!(row("asks").contains("! ▾ asks") && row("broke").contains("✗ ▾ broke") && row("pilot").contains("∞ ▾ pilot"), "{l:#?}");
    assert!(row("quiet").contains("○ ▾ quiet"), "idle: a task's idle mark: {l:#?}");
    // only those that run: a stopped or never started one is not there, nor in the count (this machine's: none run)
    let text = l.join("\n");
    assert!(!heads(&s).iter().any(|h| h == "stopped" || h == "never") && text.contains("Coordinators 5 "), "{text}");
    assert!(!text.contains("◈") && !text.contains("◎") && !s.s.rows.iter().any(|r| matches!(r, Row::Head { name, .. } if name == "Coordinator")), "{text}");
    let red = s.inked(120).iter().flat_map(|x| x.0.clone()).filter(|p| p.t == "!" || p.t == "✗").all(|p| p.c == Ink::Red && p.bold);
    assert!(red, "what needs you and what went wrong: red, bold");
}

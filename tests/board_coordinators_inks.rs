//! The chat's box edge follows the status of whoever it talks to (view/chat_title.rs), as their mark: base for one idle
//! or never started (a coordinator, a task to do), amber while it works, red when it needs you or failed; its ladder
//! names whose it is, the lit step in that colour too. Inside the box (the prompt, the reply, your words) all looks the
//! same whoever it talks to. Names have no colour: a machine's and a repo's heading are in the base colour (a machine's
//! bold, a repo's not), in a light theme and a dark one.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;
use sw::board::theme::{resolve, Seen, Theme};
use sw::board::update::coords::{MACHINE_INK, REPO_HEAD_INK};
use sw::board::update::heads::{group_id, machine_id};
use sw::board::update::{update, Event};
use sw::board::view::textbox::box_ink;
use sw::json::*;
use sw::shown::ink::{Ink, Inked};

fn coord_world() -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never", "board.machines": "otherbox" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
/// Herdr's theme of that name, as the board paints with it.
fn theme(name: &str) -> Theme { let r = resolve(&Default::default(), None, name, Seen::default()); Theme { pal: r.pal, ground: r.ground, symbols: true, plain: false, name: r.name, web: false } }
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
/// The inks (and boldness) of the pieces under `needle` on the first line that has it (and `on`, where it is not empty).
fn inks(lines: &[Inked], needle: &str) -> Vec<(Ink, bool)> { inks_on(lines, "", needle) }
fn inks_on(lines: &[Inked], on: &str, needle: &str) -> Vec<(Ink, bool)> {
    let l = lines.iter().find(|l| l.text().contains(needle) && l.text().contains(on)).unwrap_or_else(|| panic!("no line has {needle:?}: {:#?}", lines.iter().map(|l| l.text()).collect::<Vec<_>>()));
    let from = l.text().find(needle).unwrap();
    let (to, mut at, mut out) = (from + needle.len(), 0, vec![]);
    for p in &l.0 { let end = at + p.t.len(); if end > from && at < to && !p.t.trim().is_empty() { out.push((p.c, p.bold)); } at = end; }
    out
}
fn all(b: &mut Board, needle: &str, ink: Ink, bold: bool) {
    let got = inks(&b.inked(100), needle);
    assert!(!got.is_empty() && got.iter().all(|x| *x == (ink, bold)), "{needle:?}: {got:?}, not {ink:?}");
}

#[test]
fn the_boxs_edge_follows_the_status_of_whoever_it_talks_to_and_names_are_in_the_base_colour() {
    let w = coord_world();
    let id = w.add("Fix the login");
    w.script(json!({ "coordinator": [{ "said": "Proj has one task open." }] }));
    let host = sw::remote::check::hostname();
    // as much of its name as its heading always has room for: a long one (GitHub's macOS runners') is cut there
    let head: String = host.chars().take(24).collect();
    for (name, light) in [("catppuccin", false), ("catppuccin-latte", true)] { // dark and light
        let mut s = open(&w.root, 34);
        s.theme = theme(name);
        if !s.s.by_repo { s.press(&["alt+s"]); }
        s.s.sel = group_id(&s.s, "", "proj"); s.load();
        // the repo's heading: the base colour, normal weight, as the task titles under it
        assert_eq!(inks_on(&s.inked(100), "▾ proj", "proj"), vec![(REPO_HEAD_INK, false)]);
        assert_eq!(REPO_HEAD_INK, Ink::Plain);
        s.press(&["space"]);
        // never started: the base colour, the edge and its lit step alike; no line inside says whose it is again
        assert_eq!(box_ink(&s.s, None), Ink::Text);
        all(&mut s, "╭─", Ink::Text, false);
        assert!(inks_on(&s.inked(100), "╭─", " proj ").iter().all(|x| x.0 == Ink::Text), "{:?}", inks_on(&s.inked(100), "╭─", " proj "));
        assert!(!s.text().contains("Looks after"), "{}", s.text());
        all(&mut s, "❯", Ink::Text, false); // the prompt: plain, whoever it talks to
        if !light {
            s.typed("what is open here?");
            assert!(s.text().contains("send to") && s.text().contains("proj's coordinator"), "{}", s.text());
            s.press(&["enter"]); drain(); s.load();
            // its reply has no label over it (the ladder names it); idle again: the base colour
            assert!(!s.text().contains("proj's coordinator:"), "{}", s.text());
            s.press(&[&sw::board::keys::key_name("chatScrollUp")]); // the conversation, your words too
            // your words with no label, in the colour you typed them in
            assert!(!s.text().contains("You:"), "{}", s.text());
            all(&mut s, "what is open here?", Ink::Plain, false);
            all(&mut s, "╰", Ink::Text, false);
        }
        // a task to do: the base colour; at work: amber; waiting for you: red; failed: red
        s.s.sel = id.clone(); s.load();
        assert_eq!(box_ink(&s.s, s.s.current()), Ink::Text);
        all(&mut s, "╭─", Ink::Text, false);
        let report = |state: &str| sw::actions::filing::report(&id, sw::actions::filing::Report { state: state.into(), msg: "now".into(), ..Default::default() }).unwrap();
        for (state, ink) in [("working", Ink::Yellow), ("blocked", Ink::Red), ("error", Ink::Red)] {
            report(state); s.load();
            assert_eq!(box_ink(&s.s, s.s.current()), ink, "{state}");
            all(&mut s, "╭─", ink, false);
        }
        report("idle"); s.load();
        // this machine's row: its name in the base colour, bold
        s.press(&["space"]); s.press(&["alt+s", "alt+s"]); // by machine, then by status
        update(&mut s.s, Event::Machines(sw::remote::machines()));
        while s.s.scope != "*" { s.press(&["alt+m"]); }
        s.s.sel = machine_id(""); s.load();
        assert_eq!(inks_on(&s.inked(100), &format!("▾ {head}"), &head), vec![(MACHINE_INK, true)]); // its row, not the filter row's ▾ beside the panel
        assert_eq!(MACHINE_INK, Ink::Plain);
        assert_eq!(inks_on(&s.inked(100), "▾ All machines", "All machines"), vec![(Ink::Plain, true)]);
        s.press(&["space"]);
        all(&mut s, "❯", Ink::Text, false);
        s.press(&["space"]); assert!(!s.s.reply);
        assert!(s.text().contains("╭─  All machines  · Machines coordinator ─"), "{}", s.text());
    }
}

#[test]
fn the_boxs_edge_names_the_kind_of_agent_dimmed_after_its_lit_step_and_the_empty_box_says_it_too() {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never", "board.machines": "otherbox" }), ..Default::default() });
    let home = w.base.join("home-coord"); std::fs::create_dir_all(&home).unwrap(); unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }], "coordinator": [{ "said": "Proj is quiet." }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 34); s.s.side = false;
    // the kind after the lit step, dimmed: never in a colour of its own
    let dim = |s: &mut Board, kind: &str| { assert_eq!(crate::board_chat::chat_kind(s, 100), kind, "{}", s.text()); assert!(inks_on(&s.inked(100), "╭─", &format!("· {kind}")).iter().all(|x| x.0 == Ink::Muted), "{kind}: {:?}", inks_on(&s.inked(100), "╭─", &format!("· {kind}"))); };
    // the top row of one machine: its machine coordinator
    dim(&mut s, "Machine coordinator");
    // a task's agent
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    dim(&mut s, "Agent");
    assert!(s.text().contains("❯ Send message to the agent"), "{}", s.text());
    s.press(&["space"]);
    // a project's coordinator, once it has said something
    s.press(&["alt+s"]); s.s.sel = group_id(&s.s, "", "proj"); s.load(); s.press(&["space"]);
    s.typed("anything?"); s.press(&["enter"]); drain(); s.load();
    dim(&mut s, "Project coordinator");
    assert!(s.text().contains("❯ Send message to the project coordinator"), "{}", s.text());
    s.press(&["space", "alt+s", "alt+s"]);
    // every machine shown: All machines' coordinator, and another machine's
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: Ok(json!({ "host": "otherbox", "version": "9.0.0", "coordinator": { "state": "idle" }, "coordinators": { "machine": { "state": "idle", "talk": [{ "you": false, "text": "Quiet." }] } }, "tasks": [] })) });
    while s.s.scope != "*" { s.press(&["alt+m"]); }
    s.s.sel = sw::board::update::top::all_id(); s.load();
    dim(&mut s, "Machines coordinator");
    s.s.sel = machine_id("otherbox"); s.load(); s.press(&["space"]);
    dim(&mut s, "Machine coordinator");
    assert!(s.text().contains("❯ Send message to the machine coordinator"), "{}", s.text());
}

#[test]
fn another_machines_coordinators_box_is_amber_while_it_works_and_red_when_it_asks_or_failed() {
    let w = coord_world();
    w.add("Fix the login");
    let mut s = open(&w.root, 40); s.s.side = false;
    update(&mut s.s, Event::Machines(sw::remote::machines()));
    let answer = json!({ "host": "otherbox", "version": "9.0.0", "coordinator": { "state": "idle" }, "tasks": [], "autopilot": {},
        "coordinators": { "machine": { "state": "working" }, "project:gtm": { "state": "idle", "talk": [{ "you": false, "text": "Retry #11?" }] }, "project:crm": { "state": "none", "error": "no claude" }, "project:quiet": { "state": "idle" } } });
    update(&mut s.s, Event::Remote { label: "otherbox".into(), answer: Ok(answer) });
    while s.s.scope != "*" { s.press(&["alt+m"]); }
    s.press(&["tab"]); assert_eq!(s.s.show, 1); // Coordinators: the project rows show
    for (id, ink) in [(machine_id("otherbox"), Ink::Yellow), ("head:repo\totherbox\tgtm".to_string(), Ink::Red), ("head:repo\totherbox\tcrm".to_string(), Ink::Red), ("head:repo\totherbox\tquiet".to_string(), Ink::Text)] {
        s.s.sel = id.clone(); s.load(); s.press(&["space"]);
        assert!(s.s.reply && s.s.reply_id == id, "{id}");
        assert_eq!(box_ink(&s.s, None), ink, "{id}");
        all(&mut s, "╭─", ink, false);
        s.press(&["space"]);
    }
}

#[test]
fn the_web_board_draws_the_box_in_the_status_colour_and_names_in_the_base_colour() {
    let w = coord_world();
    w.add("Fix the login");
    let web = |call: &str, args: serde_json::Value| common::board::web(&w, call, args);
    let s = web("board", json!({}));
    assert_eq!(s.at("chat").s("box"), "text", "the top row's coordinator, never started: {}", s.at("chat"));
    // this machine's group: the base colour, no accent
    assert!(s.at("list").arr("groups").iter().filter(|g| g.b("machine")).all(|g| g.s("color") == "plain"), "{}", s.at("list"));
    web("press", json!({ "id": "byRepo" }));
    let s = web("board", json!({}));
    let head = s.at("list").arr("groups").iter().find(|g| g.s("id").starts_with("head:repo")).map(|g| g.t("id")).unwrap();
    web("fold", json!({ "id": head }));
    let s = web("press", json!({ "id": "talk" }));
    assert_eq!(s.at("chat").s("box"), "text", "{}", s.at("chat"));
    // its heading on the page is in plain text, as on the terminal board; no task title is bold
    assert!(s.at("list").arr("groups").iter().any(|g| g.s("id") == head && g.s("color") == "plain"), "{}", s.at("list"));
    assert!(s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").to_vec()).all(|r| !r.b("strong")), "{}", s.at("list"));
}

#[test]
fn in_every_theme_with_colours_of_its_own_idle_working_and_needs_you_are_three_colours() {
    let _w = common::world();
    // ("terminal" has the terminal's own colours: there the box's ladder and the marks tell them apart.)
    for name in sw::herdr::palette::THEME_NAMES.iter().filter(|n| **n != "terminal") {
        let t = theme(name);
        let (idle, work, need) = (t.ink(Ink::Text), t.ink(Ink::Yellow), t.ink(Ink::Red));
        assert!(idle != work && work != need && idle != need, "{name}: idle {idle:?}, working {work:?}, needs you {need:?}");
    }
}

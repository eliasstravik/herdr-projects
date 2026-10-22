//! The chat's box is in its kind's colour (view/chat_title.rs): amber for every coordinator (a machine's, a project's,
//! all machines'), violet for a task's agent; its ladder names whose it is, the lit step in that colour too, and so
//! does the hint. Inside the box (the prompt, the reply, your words) all looks the same whoever it talks to. A repo's heading stays in plain text and a machine's in its accent, in a light theme and a dark one.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;
use sw::board::theme::{resolve, Seen, Theme};
use sw::board::update::coords::{MACHINE_INK, REPO_HEAD_INK};
use sw::board::view::chat_title::{AGENT_EDGE, COORD_EDGE};
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
fn a_coordinators_box_is_amber_and_names_it_from_the_top_row_down_and_a_tasks_agent_is_violet() {
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
        // the heading in plain text, bold; its coordinator's name in the hint in a coordinator's amber
        assert_eq!(inks_on(&s.inked(100), "▾ proj", "proj"), vec![(REPO_HEAD_INK, true)]);
        assert_eq!(inks_on(&s.inked(100), "talk to proj's coordinator", "proj's coordinator"), vec![(COORD_EDGE, false)]);
        s.press(&["space"]);
        assert_eq!(box_ink(&s.s, None), COORD_EDGE);
        // its top edge: the ladder, the step it is lit in amber; no line inside says whose it is again
        all(&mut s, "╭─", COORD_EDGE, false);
        assert!(inks_on(&s.inked(100), "╭─", " proj ").iter().all(|x| x.0 == COORD_EDGE), "{:?}", inks_on(&s.inked(100), "╭─", " proj "));
        assert!(!s.text().contains("Looks after"), "{}", s.text());
        all(&mut s, "❯", Ink::Text, false); // the prompt: plain, whoever it talks to
        if !light {
            s.typed("what is open here?");
            assert!(s.text().contains("send to") && s.text().contains("proj's coordinator"), "{}", s.text());
            s.press(&["enter"]); drain(); s.load();
            // its reply has no label over it (the ladder names it); only the frame is in the coordinator's colour
            assert!(!s.text().contains("proj's coordinator:"), "{}", s.text());
            s.press(&[&sw::board::keys::key_name("chatScrollUp")]); // the conversation, your words too
            all(&mut s, "You:", Ink::Muted, false);
            all(&mut s, "what is open here?", Ink::Soft, false);
            all(&mut s, "╰", COORD_EDGE, false);
        }
        // a task's agent: violet
        s.s.sel = id.clone(); s.load();
        assert_eq!(box_ink(&s.s, s.s.current()), AGENT_EDGE);
        all(&mut s, "╭─", AGENT_EDGE, false);
        // this machine's coordinator, and the one of all machines: amber as every coordinator
        s.press(&["space"]); s.press(&["alt+s", "alt+s"]); // by machine, then by status
        update(&mut s.s, Event::Machines(sw::remote::machines()));
        while s.s.scope != "*" { s.press(&["alt+m"]); }
        s.s.sel = machine_id(""); s.load();
        assert_eq!(inks_on(&s.inked(100), &format!("▾ {head}"), &head), vec![(MACHINE_INK, true)]); // its row, not the filter row's ▾ beside the panel
        s.press(&["space"]);
        assert_eq!(box_ink(&s.s, None), COORD_EDGE);
        all(&mut s, "❯", Ink::Text, false);
        s.press(&["space"]); assert!(!s.s.reply);
        assert_eq!(box_ink(&s.s, None), COORD_EDGE);
        assert!(s.text().contains("╭─  All machines  ─"), "{}", s.text());
    }
}

#[test]
fn the_web_board_draws_a_coordinators_box_amber() {
    let w = coord_world();
    w.add("Fix the login");
    let web = |call: &str, args: serde_json::Value| common::board::web(&w, call, args);
    let s = web("board", json!({}));
    assert_eq!(s.at("chat").s("box"), "yellow", "the top row's coordinator: {}", s.at("chat"));
    web("press", json!({ "id": "byRepo" }));
    let s = web("board", json!({}));
    let head = s.at("list").arr("groups").iter().find(|g| g.s("id").starts_with("head:repo")).map(|g| g.t("id")).unwrap();
    web("fold", json!({ "id": head }));
    let s = web("press", json!({ "id": "talk" }));
    assert_eq!(s.at("chat").s("box"), "yellow", "{}", s.at("chat"));
    // its heading on the page is in plain text, as on the terminal board
    assert!(s.at("list").arr("groups").iter().any(|g| g.s("id") == head && g.s("color") == "plain"), "{}", s.at("list"));
}

#[test]
fn in_every_theme_with_colours_of_its_own_a_coordinator_and_a_tasks_agent_are_two_colours() {
    let _w = common::world();
    // ("terminal" has the terminal's own colours: there the box's ladder and its placeholder tell them apart.)
    for name in sw::herdr::palette::THEME_NAMES.iter().filter(|n| **n != "terminal") {
        let t = theme(name);
        let (c, a, p) = (t.ink(COORD_EDGE), t.ink(AGENT_EDGE), t.ink(Ink::Text));
        assert!(c != a && a != p && c != p, "{name}: coordinator {c:?}, agent {a:?}, text {p:?}");
    }
}

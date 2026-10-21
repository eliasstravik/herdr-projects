//! A machine's coordinator is drawn in its machine heading's colour, a repo's coordinator in the brighter grey (its
//! repo's heading is in plain text, as loud as its tasks): the chat's frame, its name over the box, its name in the hints, and the web board's chat. Inside the box (the prompt,
//! who said what) all looks as with the top-level Coordinator, which keeps its plain look, so the three are told apart
//! by their frames, in a light theme and a dark one.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::json;
use sw::board::theme::{resolve, Seen, Theme};
use sw::board::update::coords::{MACHINE_INK, REPO_HEAD_INK, REPO_INK};
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
fn a_repos_coordinator_is_drawn_in_the_brighter_grey_and_a_machines_in_its_headings_accent() {
    let w = coord_world();
    w.add("Fix the login");
    w.script(json!({ "coordinator": [{ "said": "Proj has one task open." }] }));
    let host = sw::remote::check::hostname();
    // as much of its name as its heading always has room for: a long one (GitHub's macOS runners') is cut there
    let head: String = host.chars().take(24).collect();
    for (name, light) in [("catppuccin", false), ("catppuccin-latte", true)] { // dark and light
        let mut s = open(&w.root, 34);
        s.theme = theme(name);
        if !s.s.by_repo { s.press(&["alt+s"]); }
        s.s.sel = group_id(&s.s, "", "proj"); s.load();
        // the heading in plain text, bold; its coordinator's name in the hint in the brighter grey
        assert_eq!(inks_on(&s.inked(100), "▾ proj", "proj"), vec![(REPO_HEAD_INK, true)]);
        assert_eq!(inks_on(&s.inked(100), "talk to the proj coordinator", "proj coordinator"), vec![(REPO_INK, false)]);
        s.press(&["space"]);
        assert_eq!(box_ink(&s.s, None), REPO_INK);
        // (the second time round it has run: the board reads the selected heading's coordinator before space)
        let state = if light { "idle" } else { "not started" };
        assert_eq!(inks_on(&s.inked(100), &format!("proj coordinator  {state}"), "proj coordinator"), vec![(REPO_INK, true)]); // the name, bold
        assert_eq!(inks_on(&s.inked(100), "proj coordinator  ", state), vec![(Ink::Muted, false)]);
        all(&mut s, "❯", Ink::Text, false); // the prompt: the Coordinator's
        if !light {
            s.typed("what is open here?");
            assert_eq!(inks_on(&s.inked(100), "send to the proj coordinator", "proj coordinator"), vec![(REPO_INK, false)]);
            s.press(&["enter"]); drain(); s.load();
            // who said what: grey, as with the Coordinator; only the frame is in the heading's colour
            all(&mut s, "proj coordinator:", Ink::Muted, false);
            s.press(&[&sw::board::keys::key_name("chatScrollUp")]); // the conversation, your words too
            all(&mut s, "You:", Ink::Muted, false);
            all(&mut s, "what is open here?", Ink::Soft, false);
            all(&mut s, "╭", REPO_INK, false);
            all(&mut s, "╰", REPO_INK, false);
        }
        // this machine's coordinator: its heading's accent
        s.press(&["space"]); s.press(&["alt+s"]);
        update(&mut s.s, Event::Machines(sw::remote::machines()));
        if s.s.scope != "*" { s.press(&["alt+m"]); }
        s.s.sel = machine_id(""); s.load();
        assert_eq!(inks_on(&s.inked(100), "▾", &head), vec![(MACHINE_INK, true)]);
        s.press(&["space"]);
        assert_eq!(box_ink(&s.s, None), MACHINE_INK);
        assert_eq!(inks_on(&s.inked(100), "coordinator  not started", &format!("{host} coordinator")), vec![(MACHINE_INK, true)]);
        all(&mut s, "❯", Ink::Text, false);
        // the Coordinator keeps its plain look
        s.press(&["space"]); assert!(!s.s.reply);
        assert_eq!(box_ink(&s.s, None), Ink::Text);
        assert_eq!(inks_on(&s.inked(100), "Coordinator  not started", "Coordinator"), vec![(Ink::Plain, true)]);
        all(&mut s, "❯", Ink::Text, false);
    }
}

#[test]
fn the_web_board_draws_a_headings_coordinator_in_its_headings_colour() {
    let w = coord_world();
    w.add("Fix the login");
    let web = |call: &str, args: serde_json::Value| common::board::web(&w, call, args);
    let s = web("board", json!({}));
    assert_eq!((s.at("chat").s("tint"), s.at("chat").s("box")), ("", "text"), "the Coordinator: {}", s.at("chat"));
    web("press", json!({ "id": "byRepo" }));
    let s = web("board", json!({}));
    let head = s.at("list").arr("groups").iter().find(|g| g.s("id").starts_with("head:repo")).map(|g| g.t("id")).unwrap();
    web("fold", json!({ "id": head }));
    let s = web("press", json!({ "id": "talk" }));
    assert_eq!((s.at("chat").s("tint"), s.at("chat").s("box")), ("subtext0", "subtext0"), "{}", s.at("chat"));
    // its heading on the page is in plain text, as on the terminal board
    assert!(s.at("list").arr("groups").iter().any(|g| g.s("id") == head && g.s("color") == "plain"), "{}", s.at("list"));
    // on the page only the name over the box takes it: who said what keeps the Coordinator's look (chat.css)
    let css = include_str!("../crates/web/src/webui/page/chat.css");
    let tinted: Vec<&str> = css.lines().filter(|l| l.contains("data-tint")).collect();
    assert!(!tinted.is_empty() && tinted.iter().all(|l| l.contains(".sw-who-name") && !l.contains(".sw-by")), "{tinted:?}");
}

#[test]
fn in_every_theme_with_colours_of_its_own_the_three_are_three_colours() {
    let _w = common::world();
    // ("terminal" has the terminal's own text colour for both greys: there the names and their heading tell them apart.)
    for name in sw::herdr::palette::THEME_NAMES.iter().filter(|n| **n != "terminal") {
        let t = theme(name);
        let (m, r, c) = (t.ink(MACHINE_INK), t.ink(REPO_INK), t.ink(Ink::Text));
        assert!(m != r && r != c && m != c, "{name}: machine {m:?}, repo {r:?}, Coordinator {c:?}");
    }
}

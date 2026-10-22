//! More of the board's screen: a long chat message wraps inside its box and takes the rows it needs, never the text
//! box's; a very narrow window keeps the box whole; ctrl does something and alt only shows or hides; the selected row's
//! bar and its inverse video where no colour can show it; a part that fails to draw says so in its place.
use crate::common;
use common::board::{open, Board};
use common::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::json;
use sw::board::keys::key_name;
use sw::board::theme::Theme;
use sw::json::*;
use sw::shown::ink::Ink;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn coord_world() -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never" }), ..Default::default() });
    let home = w.base.join("home-coord"); std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
/// The Coordinator, as the board's loader would have read it, saying this.
fn coordinator_says(b: &mut Board, said: &str) {
    b.s.data.can_dispatch = true; b.s.chat_open = true;
    b.s.data.coord = sw::coordinator::look::Look { state: "idle".into(), readable: true, said: said.into(), ..Default::default() };
}

#[test]
fn a_long_message_wraps_inside_the_box_and_takes_the_rows_it_needs_never_the_text_boxs() {
    let w = coord_world();
    for i in 1..=40 { w.add(&format!("Task number {i}")); }
    let mut s = open(&w.root, 30); s.s.side = false;
    let said: Vec<String> = (1..=20).map(|i| format!("Line {i} of **what** it said, long enough to wrap in a narrow window.")).collect();
    coordinator_says(&mut s, &said.join("\n"));
    let me = sw::board::update::heads::label(&s.s, "");
    for width in [44, 80, 200] {
        let c = crate::board_chat::chat(&mut s, width);
        let body = &c.boxed[..c.boxed.len() - 2];
        assert!(has(&c.boxed[c.boxed.len() - 2], &format!(r"^ +({} )?more ↓$", regex::escape(&key_name("chatScrollDown")))), "{:?}", c.boxed);
        assert!(c.boxed.last().unwrap().starts_with("❯ Send message to "), "{:?}", c.boxed);
        // With its box it takes two thirds of the 30 rows: 14 lines of it (the box's empty rows take two), under the
        // box's edge, which names who it is. It starts at its beginning, under no label: the edge says whose it is.
        assert_eq!(body.len(), 14, "fourteen lines of it at {width} columns: {body:?}");
        assert!(c.lines.iter().any(|l| l.starts_with("╭─") && l.contains(&format!(" {me} "))), "{:?}", c.lines);
        assert!(body[0].starts_with("Line 1 of what it said"), "{body:?}");
        assert!(!body.join("\n").contains("**"));
        let rows = c.lines.iter().filter(|l| has(l, r"Task number \d+")).count();
        assert!(rows >= 1, "the list keeps rows (under the top row and the filter row): {:?}", c.lines);
    }
    // A window too short for all of it: the message gives up its lines first; who you talk to and the line you type on stay.
    s.rows = 10;
    let c = crate::board_chat::chat(&mut s, 80);
    // folded: the box's edge names who (its ladder), and the line you type on stays
    assert_eq!(c.lines.len(), 10); assert_eq!(c.boxed, vec!["❯ Send message to the machine coordinator".to_string()], "{:?}", c.lines);
    assert!(c.lines.iter().any(|l| l.starts_with("╭─") && l.contains(&sw::shown::words::clean(&sw::remote::check::hostname()))), "{:?}", c.lines);
    // The colours: an agent's message is in the brighter of the two greys (it has to be read), your own in the text colour.
    use sw::board::view::chat::{chat_message, Who};
    use sw::board::view::talk::Part;
    let who = |mine: bool, text: &str| Who { text: text.into(), parts: vec![Part { text: text.into(), mine, ..Default::default() }], mine, ..Default::default() };
    let msg = |s: &mut Board, c: Who, max: usize| { s.s.chat_at = None; chat_message(&mut s.s, Some(&c), 20, max).iter().map(|l| (l.text(), l.0.iter().map(|p| p.c).collect::<Vec<_>>())).collect::<Vec<_>>() };
    assert_eq!(msg(&mut s, who(false, "one\ntwo"), 5), vec![("one".to_string(), vec![Ink::Soft]), ("two".to_string(), vec![Ink::Soft])]);
    assert_eq!(msg(&mut s, who(true, "one\ntwo"), 5), vec![("one".to_string(), vec![Ink::Plain]), ("two".to_string(), vec![Ink::Plain])]);
    assert_eq!(msg(&mut s, who(true, "one\ntwo\nthree"), 2).iter().map(|x| x.0.clone()).collect::<Vec<_>>(), vec!["one", "two"]);
    assert!(msg(&mut s, who(false, "one\ntwo\nthree"), 0).is_empty());
}

#[test]
fn a_very_narrow_window_keeps_the_box_whole_with_the_message_in_it() {
    let w = coord_world();
    w.script(json!({ "coordinator": [{ "said": "Six tasks are in progress and nothing waits on you." }] }));
    let mut s = open(&w.root, 34);
    // Nothing said yet: the box's edge says who it is, and the placeholder is cut with …, inside the box.
    for width in [27, 24, 16] { let c = crate::board_chat::chat(&mut s, width); assert!(c.framed, "at {width}"); assert_eq!(c.boxed.len(), 1); assert!(has(&c.boxed[0], "^❯ A.*…$"), "at {width}: {:?}", c.boxed); }
    let me = sw::board::update::heads::label(&s.s, "");
    assert!(crate::board_chat::chat(&mut s, 80).boxed[0] == "❯ Ask it something, e.g. \"what's left to do here?\"" && crate::board_chat::chat(&mut s, 80).lines.iter().any(|l| l.starts_with("╭─") && l.contains(&format!(" {me} "))), "never ran: {:?}", crate::board_chat::chat(&mut s, 80).lines);
    s.typed("hi"); s.press(&["enter"]); drain(); s.load();
    for width in [60, 24, 23, 20, 19, 12, 8] {
        let c = crate::board_chat::chat(&mut s, width);
        let body = &c.boxed[..c.boxed.len() - 2];
        assert!(c.framed, "the box is there at {width} columns"); assert_eq!(c.lines.len(), 34);
        assert!(c.lines.iter().all(|l| sw::text::width(l) <= width), "nothing is wider than {width} columns");
        assert!(!body.is_empty(), "the message shows at {width} columns"); assert_eq!(c.boxed[c.boxed.len() - 2], "");
        if width >= 20 { assert!(body.last().unwrap().ends_with("you."), "its end is in view at {width}: {body:?}"); }
        assert!(c.boxed.last().unwrap().starts_with('❯'));
    }
    // Typed words wrap inside it too.
    s.typed("a reply that is longer than the box is wide");
    for width in [24, 12] { let c = crate::board_chat::chat(&mut s, width); assert!(c.framed && c.lines.iter().all(|l| sw::text::width(l) <= width)); assert!(has(&c.boxed.join(" "), "❯ a +reply"), "{:?}", c.boxed); }
}

/// A key as crossterm reads what a terminal sends (esc and a letter, or the kitty protocol's CSI u: the same event).
fn key(b: &mut Board, code: KeyCode, mods: KeyModifiers) { let p = sw::board::run::input::press_of(&KeyEvent::new(code, mods)).unwrap(); b.press(&[&p.chord]); }
fn tasks() -> String { json!(sw::store::tasks::list(sw::store::tasks::ListOpts { include_archived: true, ..Default::default() }).iter().map(|t| json!([t.s("id"), t.s("stage"), t.b("pinned"), t.b("archivedAt")])).collect::<Vec<_>>()).to_string() }
fn shown(b: &Board) -> String { json!([b.s.chat_open, b.s.side, b.s.full, b.s.archive_view, b.s.by_repo, b.s.all_done, b.s.scope]).to_string() }

#[test]
fn ctrl_does_something_alt_only_shows_or_hides_and_no_old_key_is_left() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let id = w.add("One"); w.add("Two");
    let mut s = open(&w.root, 34); s.s.sel = id.clone(); s.load();
    let (first, start) = (tasks(), shown(&s));
    // Alt: each key flips one thing you see, and flips it back; no task changes.
    let field = |b: &Board, l: char| match l { 'c' => b.s.chat_open, 'p' => b.s.side, 't' => b.s.full, 'a' => b.s.archive_view, 's' => b.s.by_repo, _ => b.s.all_done };
    for l in ['c', 'p', 't', 'a', 's', 'd'] {
        let was = field(&s, l);
        key(&mut s, KeyCode::Char(l), KeyModifiers::ALT); assert_eq!(field(&s, l), !was, "alt+{l}"); assert_eq!(s.s.value(), "");
        key(&mut s, KeyCode::Char(l), KeyModifiers::ALT); assert_eq!(field(&s, l), was, "alt+{l} again");
        assert_eq!(tasks(), first, "alt+{l} changes no task"); assert!(s.s.ask.is_none());
    }
    // alt+m goes round whose board you see; with no other machine there is nothing to show, and it says so.
    // (The terminal board asks herdr for its machines on a thread of its own and hands the answer in as an event: here
    // the test hands it in.)
    s.press(&["alt+m"]); let _ = sw::board::update::update(&mut s.s, sw::board::update::Event::Machines(sw::remote::machines())); assert_eq!(s.s.scope, ""); assert!(s.text().contains("No other machines."), "{}", s.text()); assert_eq!(tasks(), first);
    assert_eq!(shown(&s), start);
    // The keys that are gone do nothing at all: not their old action, not a new one, and nothing is typed.
    use KeyCode::*;
    let (a, c, sh, n) = (KeyModifiers::ALT, KeyModifiers::CONTROL, KeyModifiers::SHIFT, KeyModifiers::NONE);
    for (code, mods) in [(Char('v'), a), (Char('h'), a), (Char('r'), a), (Char('n'), a), (Delete, n), (Char('d'), c), (Char('o'), c), (Char('t'), c), (Char('g'), c), (Right, n), (Up, a), (Down, a), (Up, c), (Down, c), (Home, n), (End, n), (Char('w'), a), (Up, sh), (Down, sh), (Right, sh), (Up, c | sh), (Down, c | sh), (Up, a | sh), (Down, a | sh)] {
        let k = format!("{code:?} {mods:?}");
        key(&mut s, code, mods);
        assert_eq!(tasks(), first, "{k}"); assert_eq!(shown(&s), start, "{k}"); assert!(s.s.ask.is_none() && s.s.menu.is_none(), "{k}"); assert_eq!(s.s.value(), "", "{k}"); assert_eq!(s.s.sel, id, "{k}"); assert!(!s.closed, "{k}");
    }
    // Ctrl: each does its one thing. ctrl+p pins (it is no longer the side panel), ctrl+a archives (no longer "yes").
    key(&mut s, Char('p'), c); assert!(task(&id).b("pinned")); assert_eq!(shown(&s), start);
    assert_ne!(s.s.sel, id, "the pinned task went to the top, the selection stayed where it was"); s.s.sel = id.clone(); s.load();
    key(&mut s, Char('p'), c); assert!(!task(&id).b("pinned"));
    s.press(&["ctrl+l"]); assert!(s.text().contains("Nothing here waits for a yes."), "{}", s.text()); assert!(s.s.ask.is_none());
    s.s.sel = id.clone(); s.load();
    s.press(&["ctrl+a"]); assert!(task(&id).b("archivedAt")); assert!(s.s.ask.is_none());
    s.press(&["ctrl+z"]); assert!(!task(&id).b("archivedAt"));
    s.press(&["ctrl+f"]); assert_eq!(s.s.mode, sw::board::state::Mode::Find); s.press(&["esc"]); assert_eq!(s.s.mode, sw::board::state::Mode::New);
    // ctrl+e opens the task's settings full screen and the key again (or esc) closes them.
    for _ in 0..2 {
        key(&mut s, Char('e'), c); assert!(s.s.full && s.s.editing.as_ref().map(|e| e.id.clone()) == Some(s.s.sel.clone()));
        key(&mut s, Char('e'), c); assert!(s.s.full && s.s.editing.is_none());
        s.press(&["esc"]); assert_eq!(shown(&s), start);
    }
    assert_eq!(tasks(), first);
}

/// The screen drawn into cells, as the terminal shows it.
fn cells(b: &mut Board, plain: bool) -> ratatui::buffer::Buffer {
    let t = Theme::default_dark(plain);
    let lines = sw::board::view::screen(&mut b.s, &t, 100, b.rows).lines;
    let mut buf = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 100, b.rows as u16));
    sw::board::run::draw::draw(&t, &lines, &mut buf);
    buf
}
fn row_text(buf: &ratatui::buffer::Buffer, y: u16) -> String { (0..buf.area.width).map(|x| buf[(x, y)].symbol().to_string()).collect() }

#[test]
fn the_selected_row_has_a_bar_in_its_gutter_on_every_line_and_inverse_video_where_no_colour_can_show_it() {
    use ratatui::style::{Color, Modifier};
    let w = world();
    let live = w.add("Reviewed work");
    sw::store::tasks::update(&live, |t| { t.put("stage", "approval"); t.put("app", "claude"); t.put("agent", "claude"); t.put("review", json!({ "text": "Fine.", "meets": true, "pass": true, "gate": "major", "findings": [] })); t.put("reviewRound", 1); t.put("summary", "Done."); Ok(sw::store::tasks::Change::Write) }).unwrap();
    w.add("A to-do");
    let mut s = open(&w.root, 34); s.s.side = false; s.s.sel = live; s.load();
    let lit = |buf: &ratatui::buffer::Buffer| (0..buf.area.height).filter(|y| row_text(buf, *y).starts_with('▌')).collect::<Vec<u16>>();
    let buf = cells(&mut s, false);
    let rows = lit(&buf);
    assert_eq!(rows.len(), 2, "both lines of the selected row");
    for &y in &rows { assert!((1..100).any(|x| buf[(x, y)].bg != Color::Reset), "with the theme's tint behind it"); assert!((0..100).all(|x| !buf[(x, y)].modifier.contains(Modifier::REVERSED))); }
    assert!(!(0..buf.area.height).any(|y| row_text(&buf, y).contains("A to-do") && row_text(&buf, y).contains('▌')), "no other row has it");
    // No colour: the bar is still there, and the row is in inverse video, with no colour at all.
    let buf = cells(&mut s, true);
    let rows = lit(&buf);
    assert_eq!(rows.len(), 2);
    for &y in &rows {
        assert!((1..100).any(|x| buf[(x, y)].modifier.contains(Modifier::REVERSED)), "inverse video: {}", row_text(&buf, y));
        assert!((0..100).all(|x| buf[(x, y)].fg == Color::Reset && buf[(x, y)].bg == Color::Reset), "and no colour");
    }
    // The same in the ? menu's picked line.
    s.press(&["?", "down"]);
    let buf = cells(&mut s, true);
    assert_eq!(lit(&buf).iter().filter(|y| (0..100).any(|x| buf[(x, **y)].modifier.contains(Modifier::REVERSED))).count(), 1, "{}", (0..34).map(|y| row_text(&buf, y)).collect::<Vec<_>>().join("\n"));
}

/// The old test made a part throw by replacing its drawing method; Rust cannot replace a function in a test, so this
/// checks the two pieces a failing part goes through: the guard that catches it with what went wrong, and the one red
/// line that stands in the part's place under its own headline.
#[test]
fn a_part_that_fails_to_draw_says_so_in_its_place() {
    let why = sw::board::view::guard::part(|| -> usize { panic!("{}", "artifactLines is not defined") }).unwrap_err();
    assert_eq!(why, "artifactLines is not defined");
    assert!(!sw::board::view::guard::quiet(), "the guard is left once the part is drawn");
    use sw::board::view::panel::kit::Kit;
    let mut k = Kit::new(46);
    k.failed("Reviews", &why);
    let p = &k.parts[0];
    assert!(p.failed);
    let lines: Vec<String> = Kit::out(p.lines.clone()).iter().map(|l| l.text()).collect();
    assert!(has(&lines[0], "^ Reviews ─+$"), "{lines:?}");
    assert_eq!(lines[1..].iter().map(|l| l.trim()).collect::<Vec<_>>().join(" "), "✗ This part could not be shown: artifactLines is not defined", "{lines:?}");
    assert!(p.lines[1..].iter().all(|l| l.0.iter().all(|x| x.c == Ink::Red)), "in red");
    // Drawn whole, a task's panel says nothing of the kind.
    let w = world();
    let id = sw::actions::filing::add(&w.root, "Build a flying game", sw::actions::filing::AddOpts { description: "A small game.".into(), ..Default::default() }).unwrap().t("id");
    let mut s = open(&w.root, 60); s.s.sel = id; s.load();
    for full in [false, true] { if full { s.press(&["alt+t"]); } let text = s.screen(120).join("\n"); assert!(!has(&text, "could not be shown|view error") && text.contains("Build a flying game") && text.contains("Settings ─"), "{text}"); }
}

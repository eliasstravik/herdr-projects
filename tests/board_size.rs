//! A terminal held at another size than herdr shows (a second viewer attached to it sets its size, and herdr shows a
//! cut-out of it from the top left; seen live at 435x58 in windows of 184x78 and 412x56): the board draws inside what
//! is shown and says what holds it, takes its terminal back by itself (a viewer that comes back is sent away again, a
//! take that fails is tried again), and no key does it. Also how herdr's layout and the attach processes are read.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::{json, Value};
use sw::board::keys::{key_name, Press};
use sw::board::run::held::{held_size, holders_in};
use sw::board::update::size::{fit_shown, Fit, Shown};
use sw::board::update::{list::relist, update, Effect, Event};
use sw::json::*;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn sh(cols: u16, rows: u16, split: bool) -> Option<Shown> { Some(Shown { cols, rows, split }) }
fn fit(cols: usize, rows: usize, held: bool) -> Fit { Fit { cols, rows, held } }
/// herdr's answer for a terminal of `term`; the takes it leads to.
fn see(b: &mut Board, term: (u16, u16), shown: Option<Shown>) -> Vec<(u16, u16)> {
    update(&mut b.s, Event::Shown { term, shown, by: None }).into_iter().filter_map(|e| if let Effect::Take { cols, rows } = e { Some((cols, rows)) } else { None }).collect()
}

#[test]
fn what_herdr_shows_of_the_pane_against_the_terminals_size() {
    let layout = |panes: &[(&str, i64, i64)], zoomed: bool, focused: &str| json!({ "area": { "width": 124, "height": 59, "x": 0, "y": 0 }, "zoomed": zoomed, "focused_pane_id": focused, "panes": panes.iter().map(|(id, w, h)| json!({ "pane_id": id, "rect": { "width": w, "height": h, "x": 0, "y": 0 } })).collect::<Vec<_>>() });
    let shown_of = |l: &Value, p: &str| sw::herdr::api::shown_of(l, p).map(|(cols, rows, split)| Shown { cols, rows, split });
    assert_eq!(shown_of(&layout(&[("w1:p1", 124, 59)], false, "w1:p1"), "w1:p1"), sh(124, 59, false));
    assert_eq!(shown_of(&layout(&[("w1:p1", 124, 30), ("w1:p2", 124, 29)], false, "w1:p1"), "w1:p2"), sh(124, 29, true));
    assert_eq!(shown_of(&layout(&[("w1:p1", 124, 30), ("w1:p2", 124, 29)], true, "w1:p1"), "w1:p1"), sh(124, 59, true)); // zoomed: the whole tab
    // The pane beside it is the zoomed one: this one is hidden and keeps its place and its size, so nothing holds it.
    let hidden = shown_of(&layout(&[("w1:p1", 124, 30), ("w1:p2", 124, 29)], true, "w1:p2"), "w1:p1");
    assert_eq!(hidden, sh(124, 30, true)); assert!(!fit_shown(121, 28, hidden).held);
    assert_eq!(shown_of(&layout(&[("w1:p1", 124, 59)], false, "w1:p1"), "w9:p9"), None); assert_eq!(shown_of(&Value::Null, "w1:p1"), None);
    // A terminal that fits what is shown is left alone: alone in its tab, beside other panes (borders), zoomed, or with no answer.
    for (cols, rows, shown) in [(184, 78, sh(184, 78, false)), (123, 59, sh(124, 59, false)), (121, 28, sh(124, 30, true)), (121, 27, sh(124, 29, true)), (121, 57, sh(124, 59, true)), (120, 34, None)] {
        assert_eq!(fit_shown(cols, rows, shown), fit(cols, rows, false), "{cols}x{rows}");
    }
    // Held wider and taller than shown: cut down to it. Held smaller: nothing to draw on beyond it, but it is said.
    assert_eq!(fit_shown(435, 58, sh(412, 56, false)), fit(411, 56, true));
    assert_eq!(fit_shown(435, 58, sh(184, 78, false)), fit(183, 58, true));
    assert_eq!(fit_shown(435, 58, sh(124, 30, true)), fit(121, 28, true));
    // One row short is a gap below the hints too: nothing is let through as close enough.
    assert!(fit_shown(385, 54, sh(386, 55, false)).held); assert!(fit_shown(122, 59, sh(124, 59, false)).held); assert!(fit_shown(121, 27, sh(124, 30, true)).held);
    // Beside other panes the borders are inside herdr's rectangle: a terminal held at exactly that rectangle is too big.
    assert_eq!(fit_shown(386, 28, sh(386, 28, true)), fit(383, 26, true));
    assert_eq!(fit_shown(124, 59, sh(124, 59, true)), fit(121, 57, true));
}

#[test]
fn a_terminal_held_at_another_size_the_board_draws_inside_what_is_shown_and_takes_it_back() {
    let w = world();
    for i in 1..=150 { w.add(&format!("Task number {i}")); }
    let mut s = open(&w.root, 58); s.s.side = false;
    let first = s.s.rows.iter().find_map(|r| if let sw::board::state::Row::Rec(r) = r { Some(r.t("id")) } else { None }).unwrap();
    let now = sw::clock::now();
    for r in s.s.data.items.iter_mut() { if r.s("id") == first { let o = std::sync::Arc::make_mut(r); for (k, v) in [("title", json!("Pin the chat to the bottom")), ("state", json!("working")), ("msg", json!("Checking every window size.")), ("progress", json!(55)), ("eta", json!(12)), ("pr", json!(3)), ("at", json!(now - 6 * 60000))] { o.put(k, v); } } }
    s.s.data.can_dispatch = true;
    s.s.data.coord = sw::coordinator::look::Look { state: "idle".into(), said: "Tasks 31 and 32 are both waiting.".into(), readable: true, ..Default::default() };
    relist(&mut s.s);
    // Not held: the board is the terminal's size, as before.
    let lines = s.screen(435);
    let pin = |l: &[String]| l.iter().find(|x| x.contains("Pin the chat")).cloned().unwrap_or_default();
    assert_eq!(lines.len(), 58); assert!(has(&lines[57], r"^[ ▌] \S")); assert!(has(&pin(&lines), r"55% 12m left {2}PR#3 {2}6m$"), "{}", pin(&lines)); assert!(!lines.join("\n").contains("Another viewer"));
    // Held at 435x58 while a 412x56 window shows it: everything is inside the 412x56 that can be seen.
    s.s.size.shown = sh(412, 56, false);
    let lines = s.screen(435);
    assert_eq!(lines.len(), 58, "the terminal is still filled: the rows nobody sees are empty");
    assert_eq!(lines[56..].to_vec(), vec!["", ""]);
    for l in &lines { assert!(sw::text::width(l) <= 411, "nothing is drawn right of what is shown: {l}"); }
    assert!(has(&lines[55], r"^[ ▌] \S.* · "), "the hints: the last row that is seen: {}", lines[55]);
    assert!(has(&lines[54], r"^[ ▌] Another viewer holds this terminal at 435×58; herdr shows it at 412×56\. The board takes it back\.$"), "{}", lines[54]);
    // The one rounded box ends where what is shown ends: its edge naming who it talks to, the message, an empty line,
    // the line you type on, an empty row.
    assert_eq!(lines[53], format!("╰{}╯", "─".repeat(409))); assert!(has(&lines[52], r"^│ +│$")); assert!(has(&lines[51], r"^│ +❯ .* │$")); assert!(has(&lines[50], r"^│ +│$"));
    assert!(has(&lines[49], r"^│ +Tasks 31 and 32 are both waiting\. +│$"), "{}", lines[49]);
    let me = sw::board::update::heads::label(&s.s, "");
    assert!(has(&lines[48], r"^│ +│$") && has(&lines[47], r"^╭─ .*╮$") && lines[47].contains(&format!(" {me} ")), "{}", lines[47]);
    for n in 47..=52 { assert_eq!(sw::text::width(&lines[n]), 411); }
    assert!(has(&pin(&lines), r"Pin the chat to the bottom +█+░+ +55% 12m left {2}PR#3 {2}6m$"), "the columns are back in view: {}", pin(&lines));
    assert!(has(&lines[55], &format!(r"{} hide the latest message · \? all keys *$", regex::escape(&key_name("chatMessage")))), "the chat's key is on the hint line, whole: {}", lines[55]);
    // Held at 435x58 while a taller, narrower window shows it: the width is cut down, the rows are all there are.
    s.s.size.shown = sh(184, 78, false);
    let lines = s.screen(435);
    assert_eq!(lines.len(), 58); assert!(has(&lines[57], r"^[ ▌] \S.* · ")); assert!(lines[56].contains("Another viewer holds this terminal at 435×58; herdr shows it at 184×78"));
    for l in &lines { assert!(sw::text::width(l) <= 183); }
    assert!(has(&pin(&lines), r"55% 12m left {2}PR#3 {2}6m$"));
    // No key takes the terminal back: alt+w, the key that once did, disconnects nobody, on an empty text box or in a draft.
    let alt_w = |s: &mut Board| update(&mut s.s, Event::Key(Press::chord("alt+w"))).into_iter().any(|e| matches!(e, Effect::Take { .. }));
    assert!(!alt_w(&mut s)); assert_eq!(s.s.value(), "");
    s.typed("half a draft"); assert!(!alt_w(&mut s)); assert_eq!(s.s.value(), "half a draft");
    s.s.input.set_text("");
    assert!(s.screen(435).join("\n").contains("Another viewer holds this terminal"));
    // The other viewer went away: herdr and the terminal agree again, and the note is gone.
    s.s.msg.clear(); s.rows = 78;
    let lines = s.screen(184);
    assert_eq!(lines.len(), 78); assert!(has(&lines[77], r"^[ ▌] \S.* · ")); assert_eq!(lines[76], format!("╰{}╯", "─".repeat(182))); assert!(!lines.join("\n").contains("Another viewer"));
}

// The board takes its terminal back by itself: seen held twice in a row, the other viewer is disconnected and herdr
// gives the terminal the window's size. (The take answers off the loop; the terminal's new size then has herdr asked
// again, and that answer is where the board says it has its terminal back: `see` at the new size stands for it.)
#[test]
fn the_board_takes_its_terminal_back_by_itself_and_again_when_a_viewer_comes_back() {
    let w = world();
    let one = w.add("One task");
    let tall = sh(184, 78, false);
    let mut a = open(&w.root, 58); a.s.side = false; a.s.ctx.pane = "w1:p1".into();
    a.s.sel = one.clone(); a.load(); // the task, not the top row it opens on: its keys are on the hint line
    assert_eq!(see(&mut a, (435, 58), tall), vec![], "once is a resize under way");
    assert!(a.screen(435).join("\n").contains("Another viewer holds this terminal at 435×58; herdr shows it at 184×78. The board takes it back."));
    assert_eq!(see(&mut a, (435, 58), tall), vec![(183, 58)]);
    update(&mut a.s, Event::Took(Ok(()))); a.rows = 78; see(&mut a, (184, 78), tall);
    let l = a.screen(184);
    assert_eq!(l.len(), 78); assert!(has(&l[77], r"^[ ▌] \S.* · "), "{}", l.join("\n")); assert_eq!(l[75], "─".repeat(184));
    assert!(l[76].contains("Another viewer held this terminal at 435×58: it was disconnected, and your window has it again."), "{}", l[76]);
    // A viewer that comes back is sent away again, every time: the board has its pane's size, always.
    let mut took = 1;
    for _round in 2..=4 {
        a.rows = 58;
        assert_eq!(see(&mut a, (435, 58), tall), vec![]);
        assert!(a.screen(435)[56].contains("Another viewer holds this terminal at 435×58; herdr shows it at 184×78. The board takes it back."), "what is true now, not what was done before");
        took += see(&mut a, (435, 58), tall).len(); assert_eq!(took, _round);
        update(&mut a.s, Event::Took(Ok(()))); a.rows = 78; see(&mut a, (184, 78), tall);
        let l = a.screen(184); assert_eq!(l.len(), 78); assert!(has(&l[77], r"^[ ▌] \S.* · ")); assert!(!l.join("\n").contains("holds this terminal"));
    }
    // A try that fails is made again at every look, and says so; when herdr answers again the window is back.
    let mut c = open(&w.root, 58); c.s.side = false; c.s.ctx.pane = "w1:p1".into(); c.s.sel = one.clone(); c.load();
    let mut tries = vec![];
    for _ in 0..5 { for _ in see(&mut c, (435, 58), tall) { tries.push(true); update(&mut c.s, Event::Took(Err("herdr did not answer".into()))); } }
    assert_eq!(tries, vec![true; 4]);
    assert!(c.screen(435)[56].contains("Another viewer holds this terminal at 435×58; herdr shows it at 184×78. Taking it back failed (herdr did not answer): trying again."), "{}", c.screen(435)[56]);
    for _ in see(&mut c, (435, 58), tall) { tries.push(false); update(&mut c.s, Event::Took(Ok(()))); }
    c.rows = 78; see(&mut c, (184, 78), tall);
    assert_eq!(tries, vec![true, true, true, true, false]);
    let l = c.screen(184); assert_eq!(l.len(), 78); assert!(has(&l[77], r"^[ ▌] \S.* · ")); assert!(l[76].contains("it was disconnected, and your window has it again"));
    // Sizes that agree never take anything, and a transient disagreement that is gone at the next look does not either.
    let mut b = open(&w.root, 78); b.s.ctx.pane = "w1:p1".into();
    let mut none = vec![];
    for shown in [tall, tall, sh(150, 60, false), tall, sh(150, 60, false), tall, None, None] { none.extend(see(&mut b, (184, 78), shown)); }
    assert_eq!(none, vec![]);
}

#[test]
fn a_terminal_held_at_another_views_size_is_told_from_herdrs_own_window_a_zoomed_pane_and_a_frames_width() {
    let layout = |w: i64, h: i64, zoomed: bool| json!({ "area": { "x": 0, "y": 0, "width": 184, "height": 78 }, "zoomed": zoomed, "focused_pane_id": "w1:p1", "panes": [{ "pane_id": "w1:p1", "rect": { "x": 0, "y": 0, "width": w, "height": h } }, { "pane_id": "w1:p2", "rect": { "x": 92, "y": 0, "width": 92, "height": 78 } }] });
    let (whole, half, zoomed) = (layout(184, 78, false), layout(92, 78, false), layout(92, 78, true));
    // What herdr 0.9.3 gives a pane: its place, less a scrollbar column, less a frame when the tab is split.
    assert_eq!(held_size(183, 78, &whole, "w1:p1"), None); assert_eq!(held_size(89, 76, &half, "w1:p1"), None);
    // The incident: 435x58 in a pane herdr's window has 184x78 for. Wider or taller than its place is always another view's.
    assert_eq!(held_size(435, 58, &whole, "w1:p1"), Some((435, 58, (184, 78))));
    assert!(held_size(183, 50, &whole, "w1:p1").is_some()); assert!(held_size(100, 76, &half, "w1:p1").is_some());
    // A zoomed pane has the whole tab, whatever its place in the split; its old place is not its size while zoomed.
    assert_eq!(held_size(181, 76, &zoomed, "w1:p1"), None);
    assert_eq!(held_size(89, 76, &zoomed, "w1:p1"), Some((89, 76, (184, 78))));
    // The pane beside a zoomed one keeps its place.
    assert_eq!(held_size(89, 76, &zoomed, "w1:p2"), None); assert!(held_size(181, 76, &zoomed, "w1:p2").is_some());
    assert_eq!(held_size(300, 76, &zoomed, "w1:p1").map(|h| h.2), Some((184, 78)));
    // Nothing known, nothing said.
    assert_eq!(held_size(435, 58, &Value::Null, "w1:p1"), None); assert_eq!(held_size(435, 58, &whole, "w9:p9"), None);
    // Who holds it: the attach processes for this very terminal, the oldest first.
    let ps = ["  100   2400 /home/u/.local/bin/herdr terminal attach term_aa", "  101     30 herdr terminal attach --takeover term_aa", "  102     99 herdr terminal attach term_bb", "  103     50 bash -c echo herdr terminal attach term_aa", "  104      5 herdr server",
        "  105    600 herdr --session work terminal attach term_aa", "  106      7 /usr/bin/herdr --session work terminal session observe term_aa", "  107      8 herdr --session terminal attach term_cc", "  108    300 herdr --session terminal terminal attach term_aa",
        // Another terminal's viewer, in a session named like this terminal: never the one to stop.
        "  109   9000 herdr --session term_aa terminal attach term_zz", "  110   9001 herdr --session=term_aa terminal attach term_zz", "  111     20 herdr terminal attach term_aa --session work", "  112   9002 herdr terminal session observe term_aa"].join("\n");
    assert_eq!(holders_in("term_aa", &ps), vec![(100, 2400), (105, 600), (108, 300), (101, 30), (111, 20)]);
    assert_eq!(holders_in("term_zz", &ps), vec![(110, 9001), (109, 9000)]);
    assert_eq!(holders_in("", &ps), vec![]);
    // macOS's ps has no `etimes`: its `etime` is [[days-]hours:]minutes:seconds, read as seconds all the same.
    let ps = ["  200       00:07 /Users/u/.local/bin/herdr terminal attach term_mm", "  201    01:02:03 herdr terminal attach term_mm", "  202 2-00:00:01 herdr terminal attach term_mm", "  203      xx:07 herdr terminal attach term_mm"].join("\n");
    assert_eq!(holders_in("term_mm", &ps), vec![(202, 172801), (201, 3723), (200, 7)]);
}

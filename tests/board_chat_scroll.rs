//! The chat's message when it is long: the box grows with it, up to two thirds of the window, it scrolls with two keys
//! and the mouse wheel, and the box says where there is more. The wheel on the rest of the board: the list, the menu,
//! the whole-task page. A tall draft in a small window never takes the answer away.
use crate::common;
use common::board::{open, Board};
use common::*;
use sw::board::keys::key_name;
use sw::json::*;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn answer(n: usize) -> String { (1..=n).map(|i| format!("Line {i} of the answer.")).collect::<Vec<_>>().join("\n") }
/// What the Coordinator says, whatever the board read: `raw`, it is a screen, not a message.
struct Said { text: String, raw: bool, turn: String }
/// A board whose chat says `said.text`. (The old test put these words in place of the chat's own; here they are the
/// Coordinator's, as the loader would have read them: the box's edge names who said them, with no label inside.)
struct B { b: Board, said: Said, _w: World }
/// The chat as drawn: its box's first and last row on the screen, the message's lines, whether the box says there is
/// more above and under, the draft, and the hints.
struct Chat { lines: Vec<String>, top: usize, end: usize, message: Vec<String>, up: bool, down: bool, draft: Vec<String> }
impl B {
    fn new(rows: usize, tasks: usize) -> B {
        let w = world();
        for i in 1..=tasks { w.add(&format!("Task number {i}")); }
        let mut b = open(&w.root, rows); b.s.side = false;
        B { b, said: Said { text: answer(40), raw: false, turn: "turn-1".into() }, _w: w }
    }
    fn apply(&mut self) {
        let d = &mut self.b.s.data;
        d.can_dispatch = true;
        let c = &mut d.coord;
        c.state = "idle".into(); c.talk.clear(); c.sent.clear(); c.unseen.clear(); c.error.clear(); c.stale = false;
        c.readable = !self.said.raw; c.status = if self.said.raw { Some("idle".into()) } else { None };
        c.said = if self.said.raw { String::new() } else { self.said.text.clone() };
        c.turn = Some(sw::agents::read_turn::Turn { id: self.said.turn.clone(), ..Default::default() });
        d.coord_screen = if self.said.raw { self.said.text.clone() } else { String::new() };
    }
    fn key(&mut self, keys: &[&str]) { for k in keys { self.apply(); self.b.press(&[k]); } }
    fn screen(&mut self, width: usize) -> Vec<String> { self.apply(); self.b.screen(width) }
    fn wheel(&mut self, y: usize, by: i64) { self.apply(); self.b.wheel(y as u16, by); }
    fn chat(&mut self, width: usize) -> Chat {
        let lines = self.screen(width);
        let top = lines.iter().rposition(|l| has(l, "^╭─.*╮$")).expect("a box");
        let end = lines.iter().rposition(|l| has(l, "^╰─+╯$")).expect("a box's end");
        let mut inside: Vec<String> = lines[top + 1..end].iter().map(|l| { let c: Vec<char> = l.chars().collect(); c[2..c.len() - 2].iter().collect::<String>().trim_end().to_string() }).collect();
        // Its room to breathe (view/textbox.rs: chat_box): the second cell of padding, an empty row under its top edge
        // and one over its bottom edge.
        if width >= 30 { inside = inside.iter().map(|l| l.strip_prefix(' ').unwrap_or(l).to_string()).collect(); }
        if inside.len() > 2 && inside[0].is_empty() && inside.last().is_some_and(|l| l.is_empty()) { inside.remove(0); inside.pop(); }
        let at = inside.iter().rposition(|l| has(l, r"^( *((alt\+j )?more )?↓)?$")).unwrap();
        // Whose it is is the box's edge (view/chat_title.rs): inside, the message alone.
        let message: Vec<String> = inside[..at].to_vec();
        Chat { up: lines[top].contains("more ↑") || lines[top].contains('↑'), down: inside[at].contains('↓'), draft: inside[at + 1..].to_vec(), message, top, end, lines }
    }
    fn first(&mut self) -> String { self.chat(120).message[0].clone() }
}

#[test]
fn the_chats_box_grows_with_the_answer_up_to_two_thirds_of_the_window_and_shrinks_back_the_selected_task_stays_in_view() {
    let mut b = B::new(33, 30);
    // 40 lines in a window of 33 rows: the box takes 22 of them (its edge naming whose it is, an empty row, then 16 of
    // the message, where there is more, the line you type on, an empty row, its bottom), from the answer's first line.
    let c = b.chat(120);
    assert_eq!(c.end - c.top + 1, 22); assert_eq!(c.message.len(), 16);
    assert_eq!(c.message[0], "Line 1 of the answer."); assert_eq!(c.message.last().unwrap(), "Line 16 of the answer.");
    assert!(!c.up); assert!(c.down, "the box says there is more under it");
    // The list gave up the rows, and still shows the task that is selected, wherever it is.
    for n in [1, 15, 30] {
        b.b.s.sel = b.b.s.items.iter().find(|r| r.s("title") == format!("Task number {n}")).unwrap().t("id");
        let c = b.chat(120);
        assert!(c.lines[..c.top].iter().any(|l| has(l, &format!("Task number {n}( |$)"))), "task {n} is in view");
    }
    // An answer that fits is whole, with nothing said of more: 12 lines, and a short one takes one.
    b.said.text = answer(12); let c = b.chat(120);
    assert_eq!(c.message.len(), 12); assert!(!c.up && !c.down);
    b.said.text = "Done.".into(); let c = b.chat(120);
    assert_eq!(c.message, vec!["Done."]); assert_eq!(c.end - c.top + 1, 7, "its edge naming whose it is, an empty row, the message, an empty line, the line you type on, an empty row, its bottom");
    // A taller window shows more of a long one: two thirds of 60 rows.
    b.said.text = answer(40); b.b.rows = 60; let c = b.chat(120);
    assert_eq!(c.end - c.top + 1, 40); assert_eq!(c.message.len(), 34);
    // Under the whole task, which is there to be read itself, the chat folds to its head line: the page has the lines.
    b.b.rows = 30; b.key(&["alt+t"]); b.screen(120);
    assert!(b.b.s.full); assert!(b.b.s.chat_fit.is_none()); assert!(b.b.s.chat_rows.is_none());
}

#[test]
fn alt_k_and_alt_j_scroll_a_message_longer_than_its_box_with_a_draft_too_the_box_says_where_there_is_more_a_new_message_keeps_the_place() {
    let mut b = B::new(33, 30);
    let (up, down) = (regex::escape(&key_name("chatScrollUp")), regex::escape(&key_name("chatScrollDown")));
    // The box names the key where there is more: under the message at first, on its top edge too once scrolled.
    let c = b.chat(120);
    assert!(has(&c.lines[c.top], "^╭─[^\n]*╮$")); assert!(has(&c.lines[c.end - 3], &format!(" {down} more ↓  │$")), "{}", c.lines[c.end - 3]);
    // Down half a box at a time, to the end and no further.
    b.key(&["alt+j"]); let c = b.chat(120);
    assert_eq!(c.message[0], "Line 9 of the answer."); assert!(c.up && c.down);
    assert!(has(&c.lines[c.top], &format!("^╭─.* {up} more ↑ ─?╮$")), "{}", c.lines[c.top]);
    let edge = |b: &mut B, w: usize| b.chat(w).lines.into_iter().find(|l| l.contains('↑')).unwrap();
    assert!(has(&edge(&mut b, 40), &format!("^╭─.* {up} more ↑ ─?╮$"))); assert!(has(&edge(&mut b, 14), "^╭─.* more ↑ ─?╮$"), "{}", edge(&mut b, 14)); assert!(has(&edge(&mut b, 9), "^╭─.*↑ ─?╮$"), "{}", edge(&mut b, 9));
    b.key(&["alt+j", "alt+j", "alt+j", "alt+j"]); let c = b.chat(120);
    assert_eq!(c.message.last().unwrap(), "Line 40 of the answer."); assert_eq!(c.message.len(), 16); assert!(c.up && !c.down);
    // alt+u no longer scrolls it (alt+k pairs with alt+j), and types nothing.
    b.key(&["alt+u"]); assert_eq!(b.chat(120).message.last().unwrap(), "Line 40 of the answer."); assert_eq!(b.b.s.value(), "");
    // And up again, to the start and no further.
    b.key(&["alt+k"]); assert_eq!(b.first(), "Line 17 of the answer.");
    b.key(&["alt+k", "alt+k", "alt+k", "alt+k"]); let c = b.chat(120);
    assert_eq!(c.message[0], "Line 1 of the answer."); assert!(!c.up);
    // The selection never moved, and nothing was typed.
    let sel = b.b.s.sel.clone(); b.key(&["alt+j"]); assert_eq!(b.b.s.sel, sel); assert_eq!(b.b.s.value(), "");
    // With a draft in the text box the two keys still scroll, the box still names them, and the draft is whole.
    b.apply(); b.b.typed("my draft"); let c = b.chat(120);
    assert!(has(&c.lines[c.top], &format!(" {up} more ↑ ─?╮$")), "{}", c.lines[c.top]);
    b.key(&["alt+j"]); let c = b.chat(120);
    assert_eq!(c.message[0], "Line 17 of the answer."); assert_eq!(b.b.s.value(), "my draft"); assert!(has(&c.draft.join(" "), "❯ my draft"));
    b.key(&["alt+k", "alt+k", "alt+k"]); assert_eq!(b.b.s.value(), "my draft"); assert_eq!(b.first(), "Line 1 of the answer.");
    // A message that fits has nothing to scroll: the keys do nothing, and are not typed into the draft.
    b.said.text = "Done.".into(); b.chat(120); b.key(&["alt+j", "alt+k"]); assert_eq!(b.b.s.value(), "my draft"); assert_eq!(b.chat(120).message, vec!["Done."]);
    b.key(&["esc"]);
    // Scrolled down, then the message changes: the place stays (board_chat_open.rs has more). So it does for another
    // turn in the very same words, and for the same words after nothing was said a moment.
    // (alt+j at the end of the short one closed it to the newest: a long one then shows from its start.)
    b.said.text = answer(40); assert_eq!(b.first(), "Line 1 of the answer.");
    b.key(&["alt+k"; 4]); b.key(&["alt+j"]); assert_eq!(b.first(), "Line 9 of the answer.");
    b.said.text = answer(41); assert_eq!(b.first(), "Line 9 of the answer.");
    b.said.turn = "turn-2".into(); assert_eq!(b.first(), "Line 9 of the answer.");
    let words = b.said.text.clone(); b.said.text = String::new(); b.screen(120); b.said.text = words; assert_eq!(b.first(), "Line 9 of the answer.");
    // The turn is the chat's own to name: the Coordinator's turn, and a task's handled turn.
    let t = sw::board::theme::Theme::default_dark(true);
    b.apply(); b.b.s.data.coord.said = "Done.".into(); b.b.s.data.coord.turn = Some(sw::agents::read_turn::Turn { id: "t-7".into(), ..Default::default() });
    assert_eq!(sw::board::view::chat::chat_of(&mut b.b.s, &t, None).unwrap().turn, "t-7");
    let rec = (*b.b.s.items[0]).clone(); b.b.s.reply = true;
    let turn = |s: &mut sw::board::state::State, m: &str| { let mut r = rec.clone(); r.put("handled", serde_json::json!({ "msg": m })); sw::board::view::chat::chat_of(s, &t, Some(&r)).unwrap().turn };
    assert_ne!(turn(&mut b.b.s, "m1"), turn(&mut b.b.s, "m2")); b.b.s.reply = false;
    // Another chat starts closed, on its beginning: the selected task's agent (as just now), and back. (The old test
    // showed these words in the task's chat too; here the place a chat keeps is checked where it is kept: per chat, the
    // task's by its id.)
    assert_eq!(b.first(), "Line 1 of the answer.");
    b.key(&["alt+j"]); assert_eq!(b.first(), "Line 9 of the answer.");
    b.apply();
    let lines: Vec<String> = answer(41).lines().map(String::from).collect();
    let who = sw::board::view::chat::chat_of(&mut b.b.s, &t, None).unwrap();
    let fit = b.b.s.chat_fit.clone().unwrap();
    assert_eq!(sw::board::view::chat::chat_window(&mut b.b.s, &who, &lines, fit.rows, fit.latest), 8, "the same chat keeps its place");
    b.b.s.reply = true;
    assert_eq!(sw::board::view::chat::chat_window(&mut b.b.s, &who, &lines, fit.rows, fit.latest), 0, "another chat starts at its beginning");
    b.b.s.reply = false; b.b.s.chat_at = None;
    // A screen (a dialog that is up) starts at its end, where a terminal has what is new; it scrolls up from there.
    b.said.raw = true; let c = b.chat(120);
    assert_eq!(c.message.last().unwrap(), "Line 41 of the answer."); assert!(c.up && !c.down);
    b.key(&["alt+k"]); assert_eq!(b.chat(120).message.last().unwrap(), "Line 33 of the answer.");
}

#[test]
fn the_mouse_wheel_scrolls_the_chat_under_the_pointer_also_while_you_type_elsewhere_it_does_what_the_arrows_do() {
    let mut b = B::new(33, 30);
    let c = b.chat(120);
    // Over the chat (its box, and the line that says who): two lines a step.
    b.wheel(c.top + 3, 1); assert_eq!(b.first(), "Line 3 of the answer.");
    b.wheel(c.top - 1, 3); assert_eq!(b.first(), "Line 9 of the answer.");
    b.wheel(c.end, -1); assert_eq!(b.first(), "Line 7 of the answer.");
    b.apply(); b.b.typed("a draft"); b.wheel(c.top + 1, 2); assert_eq!(b.first(), "Line 11 of the answer."); assert_eq!(b.b.s.value(), "a draft");
    b.wheel(c.top + 1, -100); assert_eq!(b.first(), "Line 1 of the answer."); b.key(&["esc"]);
    // Over the list: the selection moves a task a step, and the message stays where it is.
    let ids: Vec<String> = b.b.s.rows.iter().filter_map(|r| if let sw::board::state::Row::Rec(x) = r { Some(x.t("id")) } else { None }).collect();
    b.b.s.sel = ids[0].clone();
    b.wheel(2, 1); assert_eq!(b.b.s.sel, ids[1]); b.wheel(2, 3); assert_eq!(b.b.s.sel, ids[2]); b.wheel(2, -1); assert_eq!(b.b.s.sel, ids[1]);
    assert_eq!(b.first(), "Line 1 of the answer.");
    // In the menu the pick moves.
    b.key(&["?"]); b.screen(120); assert_eq!(b.b.s.menu.as_ref().unwrap().at, -1);
    b.wheel(2, 1); b.wheel(2, 1); assert_eq!(b.b.s.menu.as_ref().unwrap().at, 1); b.wheel(2, -1); assert_eq!(b.b.s.menu.as_ref().unwrap().at, 0); b.key(&["esc"]);
    // A question keeps the screen as it is: neither the wheel nor a click is taken.
    b.key(&["ctrl+x"]); assert!(b.b.s.ask.is_some(), "{}", b.b.text());
    b.wheel(2, 1); assert_eq!(b.b.s.sel, ids[1]);
    b.screen(120); b.b.click(3); b.b.click(4); assert_eq!(b.b.s.sel, ids[1]);
    b.key(&["n"]); assert!(b.b.s.ask.is_none());
}

#[test]
fn the_mouse_wheel_scrolls_the_whole_task_page_with_a_draft_too_over_the_chat_under_it_the_chat() {
    let mut b = B::new(30, 1);
    let id = b.b.s.items[0].t("id");
    sw::actions::filing::describe(&id, &(1..=80).map(|i| format!("Description line {i}.")).collect::<Vec<_>>().join("\n")).unwrap();
    b.b.load(); b.b.s.sel = id.clone(); b.key(&["alt+t"]);
    let l = b.screen(120);
    assert!(b.b.s.full); assert!(has(&l[0], r"Task details .* lines 1–\d+ of \d+"), "{}", l[0]);
    b.wheel(5, 1); assert!(has(&b.screen(120)[0], "lines 3–")); assert_eq!(b.b.s.sel, id);
    b.wheel(5, 2); assert!(has(&b.screen(120)[0], "lines 7–"));
    b.apply(); b.b.typed("typing"); b.wheel(5, -1); assert!(has(&b.screen(120)[0], "lines 5–")); assert_eq!(b.b.s.value(), "typing");
    // The chat under the page is folded to its head line: there is no message there to scroll, and the page stays.
    let l = b.screen(120); assert!(b.b.s.chat_fit.is_none()); assert!(has(&l[0], "lines 5–"));
    b.wheel(5, -100); assert!(has(&b.screen(120)[0], "lines 1–"));
}

#[test]
fn a_long_message_at_every_board_size_the_window_is_filled_exactly_nothing_is_wider_than_it_and_the_text_box_is_whole() {
    let mut b = B::new(30, 12);
    b.said.text = (1..=30).map(|i| format!("Line {i} of an answer that is long enough to wrap in a window that is narrow.")).collect::<Vec<_>>().join("\n");
    for rows in [8, 12, 20, 34, 60] {
        b.b.rows = rows;
        for width in [8, 20, 80, 200] {
            for scrolled in [false, true] {
                if scrolled { b.screen(width); b.key(&["alt+j"]); }
                let note = format!("{width}×{rows}{}", if scrolled { ", scrolled" } else { "" });
                let lines = b.screen(width);
                assert_eq!(lines.len(), rows, "{note}"); assert!(lines.iter().all(|l| sw::text::width(l) <= width), "{note}: nothing is wider than the window");
                assert!(lines.iter().any(|l| l.contains('❯')), "{note}: the line you type on is there");
                let top = lines.iter().rposition(|l| has(l, "^╭─.*╮$")); let end = lines.iter().rposition(|l| has(l, "^╰─+╯$"));
                if let Some(top) = top {
                    let end = end.unwrap(); assert!(end > top, "{note}");
                    assert!(end - top + 1 <= 6.max(rows * 2 / 3), "{note}: the box keeps to two thirds");
                    for l in &lines[top..=end] { assert_eq!(sw::text::width(l), width, "{note}: the box is whole"); }
                    if b.b.s.chat_fit.as_ref().map(|f| f.above > 0).unwrap_or(false) { assert!(lines[top].contains('↑'), "{note}: it says there is more above"); }
                }
            }
            b.key(&["alt+k"; 8]);
        }
    }
}

#[test]
fn a_tall_draft_in_a_small_window_never_takes_the_answer_away_it_gives_rows_back_and_the_answer_still_scrolls() {
    let mut b = B::new(30, 4);
    let draft = (1..=6).map(|i| format!("draft line {i} of thirty characters")).collect::<Vec<_>>().join("\n");
    b.b.s.input.set_text(&draft);
    for (rows, least) in [(8, 1), (12, 3), (15, 3), (30, 3)] {
        b.b.rows = rows;
        let lines = b.screen(120);
        assert_eq!(lines.len(), rows);
        // In a window of only a few lines the chat folds to its head line by itself, so the list still has rows.
        if rows < 14 { assert!(b.b.s.chat_fit.is_none(), "folded at {rows} rows"); continue; }
        let fit = b.b.s.chat_fit.clone().unwrap_or_else(|| panic!("the answer shows at {rows} rows"));
        assert!(fit.rows >= least, "at least {least} of its lines at {rows} rows");
        assert!(lines.iter().any(|l| l.contains("Line 1 of the answer.")), "its first line at {rows} rows");
        assert!(lines.iter().any(|l| l.contains("more ↓") || l.ends_with("↓ │")), "it says there is more at {rows} rows");
        assert!(lines.iter().any(|l| has(l, r"draft line \d")), "the draft is on show at {rows} rows");
        b.key(&["alt+j"]); let lines = b.screen(120);
        assert!(b.b.s.chat_fit.as_ref().unwrap().above > 0, "alt+j scrolls at {rows} rows"); assert!(lines.iter().any(|l| l.contains('↑')));
        assert_eq!(b.b.s.value(), draft, "the draft is whole");
        b.key(&["alt+k"; 20]);
    }
}

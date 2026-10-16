//! A chat opened to the conversation (its first scroll up) stays open until you close it: whatever happens meanwhile
//! (the Coordinator at work, a new message, the list read again, tasks added), on the lines you were reading (also in
//! a reply said twice), and on its end, where the new message comes, when you were there. Up goes further up each time,
//! up to its start, and stays open there; down once more from its end, hiding the chat, or talking to another closes it.
use crate::common;
use common::board::{open, Board};
use common::*;
use sw::board::data::{load, Wants};
use sw::board::update::{update, Event};

/// The Coordinator's conversation: messages `from` to `to`, yours and its own in turn, four lines each; those in `twice`
/// in the very same words.
fn said(from: usize, to: usize, twice: &[usize]) -> Vec<sw::agents::talk::Said> {
    let line = |i: usize, j: usize| if twice.contains(&i) { format!("The same answer, line {j}.") } else { format!("Message {i}, line {j}.") };
    (from..=to).map(|i| sw::agents::talk::Said { you: i % 2 == 1, text: (1..=4).map(|j| line(i, j)).collect::<Vec<_>>().join("\n"), at: 0 }).collect()
}
/// The list read again, as the board's own reads bring it: the Coordinator in `state`, saying messages `from` to `to`.
fn read(b: &mut Board, from: usize, to: usize, state: &str) { read_twice(b, from, to, state, &[]) }
fn read_twice(b: &mut Board, from: usize, to: usize, state: &str, twice: &[usize]) {
    let mut d = load(&Wants { ids: vec![b.s.sel.clone()], cwd: b.s.ctx.cwd.clone(), own_pane: String::new(), web: false, ..Default::default() });
    d.can_dispatch = true;
    let c = &mut d.coord;
    c.state = state.into(); c.readable = true; c.status = None; c.error.clear(); c.sent.clear(); c.unseen.clear(); c.stale = false;
    c.talk = said(from, to, twice); c.said = c.talk.last().unwrap().text.clone();
    c.turn = Some(sw::agents::read_turn::Turn { id: format!("turn-{to}"), ..Default::default() });
    let fx = update(&mut b.s, Event::Loaded(Box::new(d)));
    b.run(fx);
}
fn tick(b: &mut Board) { sw::clock::advance(1_000); let now = sw::clock::now(); let fx = update(&mut b.s, Event::Tick(now)); b.run(fx); }
/// The lines in the chat's box, without the line you type on.
fn shown(b: &mut Board) -> Vec<String> { let c = b.chat(120); assert!(c.framed, "{}", c.lines.join("\n")); c.boxed.into_iter().take_while(|l| !l.starts_with('❯')).collect() }
fn has(lines: &[String], text: &str) -> bool { lines.iter().any(|l| l.contains(text)) }

#[test]
fn an_opened_chat_stays_open_on_what_you_read_while_the_coordinator_works_and_says_more_and_the_list_is_read_again() {
    let w = world();
    for i in 1..=5 { w.add(&format!("Task number {i}")); }
    let mut b = open(&w.root, 34); b.s.side = false;
    read(&mut b, 1, 12, "idle");
    // Closed, the box holds the newest alone: the answer, your words above it.
    let l = shown(&mut b);
    assert!(has(&l, "Message 12, line 4.") && !has(&l, "Message 11,"), "{l:?}");
    // Up, and up again: the conversation, from an older message.
    for _ in 0..3 { b.press(&["alt+k"]); shown(&mut b); }
    let reading = shown(&mut b);
    assert!(!has(&reading, "Message 12, line 4."), "scrolled up from the end: {reading:?}");
    let first = reading[0].clone();
    // The Coordinator at work, the board's beats, the list read again, a task added, another read: the same lines.
    read(&mut b, 1, 12, "working"); assert_eq!(shown(&mut b), reading, "the Coordinator at work");
    for _ in 0..5 { tick(&mut b); } assert_eq!(shown(&mut b), reading, "the board's beats");
    w.add("A task added meanwhile"); b.load(); read(&mut b, 1, 12, "working"); assert_eq!(shown(&mut b), reading, "a task added and the list read again");
    // A new message, the oldest dropping off as it comes (a conversation shows its last twelve): what you read stays put.
    read(&mut b, 2, 13, "working"); let l = shown(&mut b);
    assert_eq!(l[0], first, "the line you read stays at the top: {l:?}"); assert_eq!(l, reading);
    read(&mut b, 3, 14, "idle"); for _ in 0..3 { tick(&mut b); } assert_eq!(shown(&mut b)[0], first, "and the next one");
    // At its end, the new messages come into view as they come, and it stays open.
    while b.s.chat_fit.as_ref().unwrap().below > 0 { b.press(&["alt+j"]); shown(&mut b); }
    let l = shown(&mut b); assert!(has(&l, "Message 14, line 4."), "{l:?}");
    read(&mut b, 4, 15, "working"); let l = shown(&mut b);
    assert!(has(&l, "Message 15, line 4.") && has(&l, "Message 13, line 1."), "the new message under the ones before it: {l:?}");
    read(&mut b, 5, 16, "idle"); tick(&mut b); let l = shown(&mut b);
    assert!(has(&l, "Message 16, line 4.") && has(&l, "Message 14, line 1."), "{l:?}");
    // Down once more from its end closes it to the newest.
    b.press(&["alt+j"]); let l = shown(&mut b); assert!(!has(&l, "Message 14,"), "closed: {l:?}");
}

#[test]
fn hiding_the_chat_or_talking_to_another_closes_an_opened_chat_nothing_else_does() {
    let w = world();
    w.add("Task number 1");
    let mut b = open(&w.root, 34); b.s.side = false;
    read(&mut b, 1, 12, "idle");
    let closed = shown(&mut b);
    // Up, each press further, to its start, and up more there: still open, on its start. Down to its end, and once more:
    // closed to the newest.
    b.press(&["alt+k"]); let mut n = 0; let mut was = usize::MAX;
    while b.s.chat_fit.as_ref().unwrap().top > 0 { let top = b.s.chat_fit.as_ref().unwrap().top; assert!(top < was, "further up each press"); was = top; b.press(&["alt+k"]); b.chat(120); n += 1; assert!(n < 20); }
    let start = shown(&mut b); assert!(has(&start, "Message 1, line 1."), "{start:?}");
    b.press(&["alt+k", "alt+k"]); assert_eq!(shown(&mut b), start, "up at its start stays open there");
    while b.s.chat_fit.as_ref().unwrap().below > 0 { b.press(&["alt+j"]); shown(&mut b); n += 1; assert!(n < 40); }
    assert_ne!(shown(&mut b), closed, "at its end, still open");
    b.press(&["alt+j"]); assert_eq!(shown(&mut b), closed);
    // Hidden and shown again (esc hides it, as its own key does): the newest, closed.
    b.press(&["alt+k"]); assert_ne!(shown(&mut b), closed);
    b.press(&["esc"]); assert!(!b.s.chat_open); b.press(&["alt+c"]); assert_eq!(shown(&mut b), closed);
    b.press(&["alt+k"]); assert_ne!(shown(&mut b), closed);
    b.press(&["alt+c", "alt+c"]); assert_eq!(shown(&mut b), closed);
    // The task's agent, and back to the Coordinator: it starts closed again.
    b.press(&["alt+k"]); assert_ne!(shown(&mut b), closed);
    b.press(&["space"]); b.chat(120); b.press(&["space"]); read(&mut b, 1, 12, "idle");
    assert!(!b.s.reply); assert_eq!(shown(&mut b), closed);
    // A window too short for the box folds it to its head line for a while: open, it is still open after.
    b.press(&["alt+k"]); let open_at = shown(&mut b);
    b.rows = 12; b.chat(120); assert!(b.s.chat_fit.is_none());
    b.rows = 34; read(&mut b, 1, 12, "working"); assert_eq!(shown(&mut b), open_at);
}

#[test]
fn a_reply_said_twice_keeps_you_on_the_one_you_read_when_a_new_message_comes() {
    let w = world();
    let mut b = open(&w.root, 34); b.s.side = false;
    // The Coordinator gave the same answer as message 2 and as message 6; you read the second of them.
    read_twice(&mut b, 1, 12, "idle", &[2, 6]); shown(&mut b);
    b.press(&["alt+k"]); shown(&mut b);
    let lines = b.s.chat_at.as_ref().unwrap().lines.clone();
    let at = lines.iter().enumerate().filter(|(_, l)| l.as_str() == "The same answer, line 1.").map(|x| x.0).nth(1).unwrap();
    let place = b.s.chat_at.as_mut().unwrap(); place.top = Some(at); place.end = false; // scrolled there
    let reading = shown(&mut b);
    assert_eq!(reading[0], "The same answer, line 1."); assert!(has(&reading, "Message 7, line 1."), "{reading:?}");
    // Message 1 drops off as message 13 comes, then 2 (the first of the two) as 14 comes: still the second one.
    read_twice(&mut b, 2, 13, "idle", &[2, 6]); assert_eq!(shown(&mut b), reading);
    read_twice(&mut b, 3, 14, "idle", &[2, 6]); assert_eq!(shown(&mut b), reading);
}

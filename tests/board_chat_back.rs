//! Back to the newest message: the chat's box holds one message, the newest, whoever said it (your words only until
//! the answer comes); alt+k opens it to the conversation, alt+j scrolls down, and alt+j once more from the end always
//! closes it to that one message again, from wherever it was opened, for the Coordinator and for a task's agent, also
//! after new messages came while it was open and with a draft.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::{json, Value};
use sw::json::*;
use sw::agents::talk::Said;

fn v(a: &[&str]) -> Vec<String> { a.iter().map(|s| s.to_string()).collect() }
fn lines(name: &str, n: usize) -> String { (1..=n).map(|i| format!("{name} line {i}.")).collect::<Vec<_>>().join("\n") }
/// A conversation with the Coordinator, oldest first: (yours, the words).
type Talk = Vec<(bool, String)>;
fn exchange(n: usize) -> Talk { (1..=n).flat_map(|i| [(true, format!("Question {i}?")), (false, lines(&format!("Answer {i}"), 3))]).collect() }
/// A board whose Coordinator's conversation is `talk`, as the loader would have read it.
struct C { b: Board, talk: Talk, turn: String, _w: World }
impl C {
    fn new(talk: Talk) -> C {
        let w = world();
        for i in 1..=20 { w.add(&format!("Task number {i}")); }
        let mut b = open(&w.root, 30); b.s.side = false;
        C { b, talk, turn: "turn-1".into(), _w: w }
    }
    fn apply(&mut self) { fill(&mut self.b, &self.talk, &self.turn); }
    fn key(&mut self, k: &str) { self.apply(); self.b.press(&[k]); }
    fn boxed(&mut self) -> Vec<String> { self.apply(); crate::board_chat::chat(&mut self.b, 120).boxed }
    /// `back` on this board, with the Coordinator's conversation as it is now.
    fn back(&mut self) -> usize { let (talk, turn) = (self.talk.clone(), self.turn.clone()); back(&mut self.b, &mut |b| fill(b, &talk, &turn), 120) }
}
fn fill(b: &mut Board, talk: &Talk, turn: &str) {
    let d = &mut b.s.data;
    d.can_dispatch = true;
    let c = &mut d.coord;
    c.state = "idle".into(); c.sent.clear(); c.unseen.clear(); c.error.clear(); c.stale = false; c.readable = true; c.status = None;
    c.talk = talk.iter().map(|(you, text)| Said { you: *you, text: text.clone(), at: 0 }).collect();
    c.said = talk.iter().rev().find(|x| !x.0).map(|x| x.1.clone()).unwrap_or_default();
    c.turn = Some(sw::agents::read_turn::Turn { id: turn.into(), ..Default::default() });
}
/// The box's message, without the line it says more on, the empty line and the line you type on.
fn message(boxed: &[String]) -> Vec<String> {
    let at = boxed.iter().rposition(|l| l.starts_with('❯')).unwrap();
    boxed[..at.saturating_sub(1)].to_vec()
}
/// alt+j until the box is closed to the newest message again: every press goes down, and the one at the end closes it.
/// Returns how many presses that took.
fn back(b: &mut Board, before: &mut dyn FnMut(&mut Board), cols: usize) -> usize {
    for n in 1..=40 {
        before(b); b.screen(cols);
        let f = b.s.chat_fit.clone();
        let open = b.s.chat_at.as_ref().map(|a| a.top.is_some()).unwrap_or(false);
        if !open { return n - 1; }
        let at_end = f.as_ref().map(|f| f.below == 0).unwrap_or(true);
        before(b); b.press(&["alt+j"]); before(b); b.screen(cols);
        let now_open = b.s.chat_at.as_ref().map(|a| a.top.is_some()).unwrap_or(false);
        if at_end { assert!(!now_open, "alt+j at the end closes it to the newest message"); return n; }
        let below = b.s.chat_fit.as_ref().map(|f| f.below).unwrap_or(0);
        assert!(!now_open || below < f.unwrap().below, "alt+j goes down");
    }
    panic!("alt+j never closed the box");
}

#[test]
fn the_box_holds_only_the_newest_message_whoever_said_it_your_words_only_until_the_answer() {
    // The answer alone: your question is one scroll up.
    let mut c = C::new(exchange(1));
    assert_eq!(message(&c.boxed()), v(&["Answer 1 line 1.", "Answer 1 line 2.", "Answer 1 line 3."]));
    c.key("alt+k"); assert!(c.boxed().contains(&"Question 1?".to_string()), "{:?}", c.boxed());
    c.key("alt+j"); assert_eq!(message(&c.boxed()), v(&["Answer 1 line 1.", "Answer 1 line 2.", "Answer 1 line 3."]), "alt+j at the end: back to the answer alone");
    // Your words, until the answer comes: then alone too.
    c.talk.push((true, "And now?".into()));
    assert_eq!(message(&c.boxed()), v(&["And now?"]));
    c.talk.push((false, "Now this.".into())); c.turn = "turn-2".into();
    assert_eq!(message(&c.boxed()), v(&["Now this."]));
    // A task's agent the same, on the web board too: one part, the newest.
    use sw::board::view::talk::{opens_at, talk, Part};
    let part = |by: &str, sent: bool| Part { by: by.into(), text: "x".into(), sent, ..Default::default() };
    assert_eq!(opens_at(&[part("Board", true), part("Agent", false), part("You", true), part("Agent", false), part("Agent", false)]), 4);
    assert_eq!(opens_at(&[part("Agent", false), part("You", true)]), 1, "your words, while it answers nothing yet");
    assert_eq!(opens_at(&[]), 0);
    // An agent that has said nothing yet: your words hold the box, with no note under them.
    let rec = |list: Value| obj(json!({ "id": "p/1", "stage": "working", "state": "working", "agent": "claude", "snap": { "talk": { "readable": true, "who": "agent", "list": list } } }));
    let k = talk(&rec(json!([{ "who": "board", "text": "The brief." }, { "who": "you", "text": "Hurry up." }])));
    assert_eq!(k.parts.iter().map(|p| p.by.as_str()).collect::<Vec<_>>(), vec!["Board", "You"]);
    let k = talk(&rec(json!([{ "who": "board", "text": "The brief." }])));
    assert_eq!(k.parts.last().unwrap().note, "empty", "the board's brief alone still says the agent has said nothing yet");
}

#[test]
fn alt_j_scrolls_down_and_once_more_at_the_end_closes_the_coordinators_chat_to_the_newest_message_from_anywhere() {
    let mut c = C::new(exchange(4));
    let newest = message(&c.boxed());
    assert_eq!(newest, v(&["Answer 4 line 1.", "Answer 4 line 2.", "Answer 4 line 3."]));
    // From every place in the conversation: alt+k that many times, then alt+j down to the end and once more.
    for ups in 1..=8 {
        for _ in 0..ups { c.key("alt+k"); }
        assert!(c.b.s.chat_at.as_ref().unwrap().top.is_some(), "alt+k opened it");
        assert_ne!(message(&c.boxed()), newest);
        c.back();
        assert_eq!(message(&c.boxed()), newest, "after {ups} alt+k");
    }
    // At the end of the open conversation one alt+j closes it.
    c.key("alt+k"); c.key("alt+j"); c.boxed();
    assert_eq!(c.b.s.chat_fit.as_ref().unwrap().below, 0); assert!(c.b.s.chat_at.as_ref().unwrap().top.is_some());
    c.key("alt+j"); assert_eq!(message(&c.boxed()), newest);
    // With a draft: the same, and the draft is whole.
    c.apply(); c.b.typed("half typed");
    c.key("alt+k"); c.key("alt+k"); c.back();
    assert_eq!(message(&c.boxed()), newest); assert_eq!(c.b.s.value(), "half typed");
    c.key("esc");
    // Hidden while open (alt+c, the only key that hides and shows it): shown again, the newest alone (board_chat_open.rs).
    c.key("alt+k"); c.key("alt+c"); assert!(!c.b.s.chat_open); c.boxed(); c.key("alt+j"); c.key("alt+c");
    assert_eq!(message(&c.boxed()), newest);
}

#[test]
fn a_short_conversation_that_starts_with_your_words_closes_too() {
    // Your words first, the answer under them: the whole conversation fits the box, open or not.
    let mut c = C::new(vec![(true, "Where do things stand?".into()), (false, "Six tasks are in progress.".into())]);
    assert_eq!(message(&c.boxed()), v(&["Six tasks are in progress."]));
    c.key("alt+k"); assert_eq!(message(&c.boxed()), v(&["Where do things stand?", "", "Six tasks are in progress."]));
    c.key("alt+j"); assert_eq!(message(&c.boxed()), v(&["Six tasks are in progress."]));
}

#[test]
fn a_newest_message_longer_than_the_box_alt_j_from_its_end_shows_it_from_its_start() {
    let mut c = C::new(vec![(true, "Tell me everything.".into()), (false, lines("Long", 40))]);
    let first = |c: &mut C| message(&c.boxed())[..1].to_vec();
    assert_eq!(first(&mut c), v(&["Long line 1."]));
    // Read down through it, then once more: back at its start.
    let n = back_from_start(&mut c);
    assert!(n > 2, "it scrolled down first: {n} presses");
    assert_eq!(first(&mut c), v(&["Long line 1."]));
    // Opened above it to your words, the same.
    for _ in 0..3 { c.key("alt+k"); }
    assert!(c.boxed().contains(&"Tell me everything.".to_string()));
    c.back();
    assert_eq!(first(&mut c), v(&["Long line 1."]));
}
/// alt+j from the start of a long newest message, down to its end and once more.
fn back_from_start(c: &mut C) -> usize {
    for n in 1..=40 {
        c.boxed(); let below = c.b.s.chat_fit.as_ref().unwrap().below;
        c.key("alt+j");
        if below == 0 { assert!(c.b.s.chat_at.as_ref().unwrap().top.is_none()); return n; }
    }
    panic!("never reached its end");
}

// Whether new messages leave an opened box open is HSW-226's to settle (it does on its branch); this holds either way:
// alt+j from wherever the box is ends on the newest message alone.
#[test]
fn new_messages_while_it_is_open_alt_j_still_goes_down_to_the_end_and_closes_it_to_the_newest() {
    for at_end in [false, true] {
        let mut c = C::new(exchange(3));
        for _ in 0..4 { c.key("alt+k"); }
        if at_end { c.key("alt+j"); c.key("alt+j"); c.key("alt+j"); c.key("alt+j"); }
        // The Coordinator answers again, and again, while the human reads.
        c.talk.push((true, "Question 4?".into())); c.boxed();
        c.talk.push((false, lines("Answer 4", 3))); c.turn = "turn-2".into(); c.boxed();
        c.talk.push((false, "And one more thing.".into())); c.turn = "turn-3".into(); c.boxed();
        c.back();
        assert_eq!(message(&c.boxed()), v(&["And one more thing."]), "at the end: {at_end}");
    }
}

/// The agent's conversation as Claude Code saves it: the brief, then `said` (yours: true for your words).
fn save_talk(w: &World, id: &str, said: &[(bool, String)]) {
    let t = task(id);
    let at = || sw::clock::iso(sw::clock::now());
    let mut rows: Vec<Value> = vec![json!({ "type": "user", "timestamp": at(), "message": { "role": "user", "content": prompts(w, t.at("link").s("pane"))[0] } })];
    for (n, (yours, text)) in said.iter().enumerate() {
        rows.push(if *yours { json!({ "type": "attachment", "timestamp": at(), "attachment": { "type": "queued_command", "commandMode": "prompt", "origin": { "kind": "human" }, "prompt": text } }) }
            else { json!({ "type": "assistant", "timestamp": at(), "message": { "id": format!("m{n}"), "role": "assistant", "content": [{ "type": "text", "text": text }] } }) });
    }
    let file = std::path::PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap()).join("projects").join("x").join(format!("{}.jsonl", t.s("session")));
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, rows.iter().map(|r| r.to_string()).collect::<Vec<_>>().join("\n") + "\n").unwrap();
}

#[test]
fn a_task_agents_chat_alt_j_at_the_end_closes_it_to_the_newest_message_also_after_new_ones_came() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let mut talk: Talk = (1..=4).flat_map(|i| [(false, lines(&format!("Answer {i}"), 3)), (true, format!("Question {i}?"))]).collect();
    talk.push((false, "Made g.".into()));
    save_talk(&w, &id, &talk);
    let mut s = open(&w.root, 30); s.s.side = false;
    s.s.sel = id.clone(); s.load(); s.press(&["space"]); assert!(s.s.reply);
    assert_eq!(message(&crate::board_chat::chat(&mut s, 120).boxed), v(&["Made g."]));
    for ups in [1, 3, 8] {
        for _ in 0..ups { s.press(&["alt+k"]); }
        assert_ne!(message(&crate::board_chat::chat(&mut s, 120).boxed), v(&["Made g."]));
        back(&mut s, &mut |_| {}, 120);
        assert_eq!(message(&crate::board_chat::chat(&mut s, 120).boxed), v(&["Made g."]), "after {ups} alt+k");
    }
    // Open, then the agent says more: alt+j down to the end and once more, the newest alone.
    for _ in 0..3 { s.press(&["alt+k"]); }
    talk.push((true, "And h?".into())); talk.push((false, "Made h.".into()));
    save_talk(&w, &id, &talk); s.load();
    back(&mut s, &mut |_| {}, 120);
    assert_eq!(message(&crate::board_chat::chat(&mut s, 120).boxed), v(&["Made h."]));
}

//! The chat's box: the latest message, an empty line, the line you type on. Whose message it holds (a task's agent, the
//! Coordinator), your just-sent words in its place until the answer, and the answer taking that place even in the very
//! same words: the box holds one message, never two; hidden, only who and the line you type on.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::keys::key_name;

/// Skills in the text box: typing `/` lists those of whoever the chat talks to.
mod skills;

fn coord_world() -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never" }), ..Default::default() });
    let home = w.base.join("home-coord"); std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn v(a: &[&str]) -> Vec<String> { a.iter().map(|s| s.to_string()).collect() }
/// The chat as the screen shows it, now that the box's own title says who it talks to (view/chat_title.rs): `who` is
/// the box's first line (whose it is and what it looks after), `boxed` what is under it, `framed` whether it shows a
/// message. A box with no title (find, or no coordinator and no task talked to) reads as before.
pub fn chat(s: &mut common::board::Board, cols: usize) -> common::board::Chat {
    let mut c = s.chat(cols);
    let titled = c.lines.iter().rposition(|l| l.starts_with("╭─") && l.ends_with('╮')).is_some_and(|t| c.lines[t].starts_with("╭─ "));
    if !c.framed || !titled || c.boxed.is_empty() { return c; }
    c.who = c.boxed.remove(0);
    if c.boxed.first().is_some_and(|l| l.starts_with('❯')) { c.framed = false; }
    c
}
/// The top row's coordinator's name on a board of this machine alone: `vm-str's coordinator`.
pub fn top_name() -> String { sw::board::update::top::coordinator_of(&sw::shown::words::clean(&sw::remote::check::hostname())) }

#[test]
fn a_task_agents_chat_its_message_then_your_words_in_its_place_then_its_next_message() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f, with one line in it." }, { "files": { "g.txt": "1\n" }, "commit": "2", "note": "Made g." }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 34); s.s.side = false;
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    for width in [120, 48] {
        let c = chat(&mut s, width);
        assert!(c.framed); assert!(c.who.starts_with("PRJ-1's agent."), "{}", c.who);
        assert_eq!(c.boxed, v(&["Agent:", "Made f, with one line in it.", "", "❯ Send message to Agent"]));
        assert!(c.hints.ends_with("space back"), "{}", c.hints);
    }
    assert!(chat(&mut s, 120).hints.ends_with(&format!("{} hide the latest message · space back", key_name("chatMessage"))));
    s.typed("yes, g too");
    assert_eq!(chat(&mut s, 120).boxed, v(&["Agent:", "Made f, with one line in it.", "", "❯ yes, g too"]));
    assert_eq!(chat(&mut s, 120).hints, "enter send to PRJ-1's agent · shift+enter new line · esc put away");
    s.press(&["enter"]);
    assert_eq!(chat(&mut s, 120).boxed, v(&["You:", "yes, g too", "", "❯ Send message to Agent"]));
    assert!(!s.text().contains("Sent to the agent"));
    settle(&id); s.load();
    assert_eq!(chat(&mut s, 120).boxed, v(&["Agent:", "Made g.", "", "❯ Send message to Agent"]));
    assert!(s.s.just_sent.is_empty());
    s.press(&["alt+c"]);
    let c = chat(&mut s, 120);
    assert!(!c.framed); assert_eq!(c.boxed, v(&["❯ Send message to Agent"]));
    assert!(c.hints.ends_with(&format!("{} show the latest message · space back", key_name("chatMessage"))), "{}", c.hints);
    assert!(!s.text().contains("│ Made g."));
    s.typed("thanks"); s.press(&["enter"]); assert!(s.text().contains("Sent to the agent"), "with the chat hidden one line says they went");
    s.press(&["alt+c"]); assert_eq!(chat(&mut s, 120).boxed, v(&["You:", "thanks", "", "❯ Send message to Agent"]));
    s.press(&["space"]);
    let c = chat(&mut s, 120);
    // no coordinator here: the box adds tasks, and says so
    assert!(!c.framed); assert_eq!(c.boxed, v(&["❯ Add a task"]), "{:?}", c.lines);
    assert!(!c.hints.contains(&key_name("chatMessage")));
}

#[test]
fn the_coordinators_chat_keeps_what_you_sent_until_it_says_something_new() {
    let w = coord_world();
    let socket = sw::herdr::cli::current_socket();
    w.script(json!({ "coordinator": [{ "said": "Six tasks are in progress." }, { "status": "working" }] }));
    let mut s = open(&w.root, 34);
    s.typed("where do things stand?"); s.press(&["enter"]); s.load();
    assert_eq!(chat(&mut s, 120).boxed, v(&["You:", "where do things stand?", "", &format!("❯ Send message to {}", top_name())]));
    assert!(chat(&mut s, 120).who.starts_with(&format!("{}.", top_name())), "{}", chat(&mut s, 120).who);
    drain(); s.load();
    assert_eq!(chat(&mut s, 120).boxed, v(&["Coordinator:", "Six tasks are in progress.", "", &format!("❯ Send message to {}", top_name())]), "the answer alone, your words above it");
    assert!(!chat(&mut s, 120).who.contains("Not started yet"), "it ran: {}", chat(&mut s, 120).who);
    s.typed("archive the last one"); s.press(&["enter"]); drain(); s.load();
    let look = sw::coordinator::look::look(&socket);
    assert_eq!((look.said.as_str(), look.sent.as_str()), ("Six tasks are in progress.", "archive the last one"));
    assert_eq!(chat(&mut s, 120).boxed, v(&["You:", "archive the last one", "", &format!("❯ Send message to {}", top_name())]));
    assert!(!s.text().contains("Six tasks"));
    assert_eq!(chat(&mut open(&w.root, 34), 120).boxed, v(&["You:", "archive the last one", "", &format!("❯ Send message to {}", top_name())]), "a board opened now shows the same");
    sw::coordinator::requests::typed_there(&socket); s.load();
    assert_eq!(chat(&mut s, 120).boxed, v(&["Coordinator:", "Six tasks are in progress.", "", &format!("❯ Send message to {}", top_name())]));
}

#[test]
fn an_answer_in_the_very_same_words_still_takes_your_words_place() {
    let w = coord_world();
    let socket = sw::herdr::cli::current_socket();
    w.script(json!({ "coordinator": [{ "said": "Done." }, { "said": "Done." }], "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Done." }, { "files": { "g.txt": "1\n" }, "commit": "2", "note": "Done." }] }));
    let mut s = open(&w.root, 34);
    s.typed("first request"); s.press(&["enter"]); drain(); s.load();
    assert_eq!(chat(&mut s, 120).boxed, v(&["Coordinator:", "Done.", "", &format!("❯ Send message to {}", top_name())]));
    s.typed("second request"); s.press(&["enter"]); s.load();
    assert_eq!(chat(&mut s, 120).boxed, v(&["You:", "second request", "", &format!("❯ Send message to {}", top_name())]));
    drain(); s.load();
    let look = sw::coordinator::look::look(&socket);
    assert_eq!((look.state.as_str(), look.sent.as_str()), ("idle", ""));
    assert_eq!(chat(&mut s, 120).boxed, v(&["Coordinator:", "Done.", "", &format!("❯ Send message to {}", top_name())]));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    assert_eq!(chat(&mut s, 120).boxed, v(&["Agent:", "Done.", "", "❯ Send message to Agent"]));
    s.typed("make g too"); s.press(&["enter"]); s.load();
    assert_eq!(chat(&mut s, 120).boxed, v(&["You:", "make g too", "", "❯ Send message to Agent"]));
    settle(&id); s.load();
    assert_eq!(chat(&mut s, 120).boxed, v(&["Agent:", "Done.", "", "❯ Send message to Agent"]));
    w.script(json!({ "worker": [{ "files": { "h.txt": "1\n" }, "commit": "3", "note": "Done." }] }));
    s.typed("and h"); s.press(&["enter", "space"]); assert!(!s.s.reply); s.screen(120);
    settle(&id); s.load(); s.screen(120);
    s.press(&["space"]); assert!(s.s.reply);
    assert_eq!(chat(&mut s, 120).boxed, v(&["Agent:", "Done.", "", "❯ Send message to Agent"]));
}

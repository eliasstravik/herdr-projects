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

#[test]
fn a_task_agents_chat_its_message_then_your_words_in_its_place_then_its_next_message() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f, with one line in it." }, { "files": { "g.txt": "1\n" }, "commit": "2", "note": "Made g." }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 34); s.s.side = false;
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    for width in [120, 48] {
        let c = s.chat(width);
        assert!(c.framed); assert!(regex::Regex::new(r"^ \S PRJ-\d+ +Make f  \S").unwrap().is_match(&c.who), "{}", c.who);
        assert!(!regex::Regex::new("agent|claude|alt\\+|last message").unwrap().is_match(&c.who));
        assert_eq!(c.boxed, v(&["Agent:", "Made f, with one line in it.", "", "❯ Send message to Agent"]));
        assert!(c.hints.ends_with("space back to the Coordinator"), "{}", c.hints);
    }
    assert!(s.chat(120).hints.ends_with(&format!("{} hide the latest message · space back to the Coordinator", key_name("chatMessage"))));
    s.typed("yes, g too");
    assert_eq!(s.chat(120).boxed, v(&["Agent:", "Made f, with one line in it.", "", "❯ yes, g too"]));
    assert_eq!(s.chat(120).hints, "enter send to PRJ-1's agent · shift+enter new line · esc put away");
    s.press(&["enter"]);
    assert_eq!(s.chat(120).boxed, v(&["You:", "yes, g too", "", "❯ Send message to Agent"]));
    assert!(!s.text().contains("Sent to the agent"));
    settle(&id); s.load();
    assert_eq!(s.chat(120).boxed, v(&["Agent:", "Made g.", "", "❯ Send message to Agent"]));
    assert!(s.s.just_sent.is_empty());
    s.press(&["alt+c"]);
    let c = s.chat(120);
    assert!(!c.framed); assert_eq!(c.boxed, v(&["❯ Send message to Agent"]));
    assert!(c.hints.ends_with(&format!("{} show the latest message · space back to the Coordinator", key_name("chatMessage"))), "{}", c.hints);
    assert!(!s.text().contains("│ Made g."));
    s.typed("thanks"); s.press(&["enter"]); assert!(s.text().contains("Sent to the agent"), "with the chat hidden one line says they went");
    s.press(&["alt+c"]); assert_eq!(s.chat(120).boxed, v(&["You:", "thanks", "", "❯ Send message to Agent"]));
    s.press(&["space"]);
    let c = s.chat(120);
    assert!(!c.framed); assert!(c.who.starts_with(" + New task  added word for word"), "{}", c.who);
    assert!(!c.hints.contains(&key_name("chatMessage")));
}

#[test]
fn the_coordinators_chat_keeps_what_you_sent_until_it_says_something_new() {
    let w = coord_world();
    let socket = sw::herdr::cli::current_socket();
    w.script(json!({ "coordinator": [{ "said": "Six tasks are in progress." }, { "status": "working" }] }));
    let mut s = open(&w.root, 34);
    s.typed("where do things stand?"); s.press(&["enter"]); s.load();
    assert_eq!(s.chat(120).boxed, v(&["You:", "where do things stand?", "", "❯ Send message to Coordinator"]));
    assert!(regex::Regex::new(r"^[ ▌]. Coordinator  starting · 1 message waits to be sent$").unwrap().is_match(&s.chat(120).who));
    drain(); s.load();
    assert_eq!(s.chat(120).boxed, v(&["Coordinator:", "Six tasks are in progress.", "", "❯ Send message to Coordinator"]), "the answer alone, your words above it");
    assert_eq!(s.chat(120).who, " ○ Coordinator  idle");
    s.typed("archive the last one"); s.press(&["enter"]); drain(); s.load();
    let look = sw::coordinator::look::look(&socket);
    assert_eq!((look.said.as_str(), look.sent.as_str()), ("Six tasks are in progress.", "archive the last one"));
    assert_eq!(s.chat(120).boxed, v(&["You:", "archive the last one", "", "❯ Send message to Coordinator"]));
    assert!(!s.text().contains("Six tasks"));
    assert_eq!(open(&w.root, 34).chat(120).boxed, v(&["You:", "archive the last one", "", "❯ Send message to Coordinator"]), "a board opened now shows the same");
    sw::coordinator::requests::typed_there(&socket); s.load();
    assert_eq!(s.chat(120).boxed, v(&["Coordinator:", "Six tasks are in progress.", "", "❯ Send message to Coordinator"]));
}

#[test]
fn an_answer_in_the_very_same_words_still_takes_your_words_place() {
    let w = coord_world();
    let socket = sw::herdr::cli::current_socket();
    w.script(json!({ "coordinator": [{ "said": "Done." }, { "said": "Done." }], "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Done." }, { "files": { "g.txt": "1\n" }, "commit": "2", "note": "Done." }] }));
    let mut s = open(&w.root, 34);
    s.typed("first request"); s.press(&["enter"]); drain(); s.load();
    assert_eq!(s.chat(120).boxed, v(&["Coordinator:", "Done.", "", "❯ Send message to Coordinator"]));
    s.typed("second request"); s.press(&["enter"]); s.load();
    assert_eq!(s.chat(120).boxed, v(&["You:", "second request", "", "❯ Send message to Coordinator"]));
    drain(); s.load();
    let look = sw::coordinator::look::look(&socket);
    assert_eq!((look.state.as_str(), look.sent.as_str()), ("idle", ""));
    assert_eq!(s.chat(120).boxed, v(&["Coordinator:", "Done.", "", "❯ Send message to Coordinator"]));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    assert_eq!(s.chat(120).boxed, v(&["Agent:", "Done.", "", "❯ Send message to Agent"]));
    s.typed("make g too"); s.press(&["enter"]); s.load();
    assert_eq!(s.chat(120).boxed, v(&["You:", "make g too", "", "❯ Send message to Agent"]));
    settle(&id); s.load();
    assert_eq!(s.chat(120).boxed, v(&["Agent:", "Done.", "", "❯ Send message to Agent"]));
    w.script(json!({ "worker": [{ "files": { "h.txt": "1\n" }, "commit": "3", "note": "Done." }] }));
    s.typed("and h"); s.press(&["enter", "space"]); assert!(!s.s.reply); s.screen(120);
    settle(&id); s.load(); s.screen(120);
    s.press(&["space"]); assert!(s.s.reply);
    assert_eq!(s.chat(120).boxed, v(&["Agent:", "Done.", "", "❯ Send message to Agent"]));
}

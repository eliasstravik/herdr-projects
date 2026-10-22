//! Your words in a task agent's chat, as in the Coordinator's: the box shows what you just sent (bright, no label) until the
//! answer comes, then the answer alone, also where you sent it while the agent worked (Claude Code queues it); the
//! conversation keeps your words above it (alt+k), and the web board opens on the same part.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use sw::json::*;

fn v(a: &[&str]) -> Vec<String> { a.iter().map(|s| s.to_string()).collect() }
/// The agent's conversation as Claude Code saves it: the brief, its first answer, your words (typed while it worked:
/// queued, so written down as they went in), then `answer`.
fn save_talk(w: &World, id: &str, yours: &str, answer: &str) {
    let t = task(id);
    let at = || sw::clock::iso(sw::clock::now());
    let said = |n: u32, text: &str| json!({ "type": "assistant", "timestamp": at(), "message": { "id": format!("m{n}"), "role": "assistant", "content": [{ "type": "text", "text": text }] } });
    let rows: Vec<Value> = vec![
        json!({ "type": "user", "timestamp": at(), "message": { "role": "user", "content": prompts(w, t.at("link").s("pane"))[0] } }),
        said(0, "Made f, with one line in it."),
        json!({ "type": "attachment", "timestamp": at(), "attachment": { "type": "queued_command", "commandMode": "prompt", "origin": { "kind": "human" }, "prompt": yours } }),
        said(1, answer),
    ];
    let file = std::path::PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap()).join("projects").join("x").join(format!("{}.jsonl", t.s("session")));
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, rows.iter().map(|r| r.to_string()).collect::<Vec<_>>().join("\n") + "\n").unwrap();
}

#[test]
fn a_task_agents_chat_shows_your_words_until_its_answer_takes_their_place_and_keeps_them_in_the_conversation() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f, with one line in it." }, { "files": { "g.txt": "1\n" }, "commit": "2", "note": "Made g." }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 34); s.s.side = false;
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    s.typed("yes, g too"); s.press(&["enter"]);
    assert_eq!(crate::board_chat::chat(&mut s, 120).boxed, v(&["yes, g too", "", "❯ Send message to the agent"]));
    // the answer takes their place: the box holds one message
    settle(&id); save_talk(&w, &id, "yes, g too", "Made g."); s.load();
    assert_eq!(crate::board_chat::chat(&mut s, 120).boxed, v(&["Made g.", "", "❯ Send message to the agent"]));
    assert!(s.s.just_sent.is_empty(), "your words are the conversation's own now");
    // above them, the rest of the conversation
    let mut all = vec![];
    for _ in 0..6 { s.press(&["alt+k"]); all.extend(crate::board_chat::chat(&mut s, 120).boxed); }
    for l in ["Board:", "Made f, with one line in it.", "yes, g too"] { assert!(all.iter().any(|x| x == l), "{l} in {all:?}"); }
    // the web board opens on the same part: the answer alone
    s.s.chat_at = None;
    let side = sw::webui::model::side::side_of(&mut s.s, &s.theme.clone());
    let (parts, from) = (side.at("chat").arr("parts").to_vec(), side.at("chat").at("from").as_u64().unwrap() as usize);
    assert_eq!(parts[from..].iter().map(|p| format!("{}: {}", p.s("by"), p.s("text"))).collect::<Vec<_>>(), vec!["Agent: Made g."]);
    assert!(from > 0, "the rest is above it");
}

#[test]
fn an_answer_too_long_for_the_box_with_your_words_starts_the_box_your_words_just_above_it() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let long: String = (1..=40).map(|n| format!("Line {n} of the answer.")).collect::<Vec<_>>().join("\n");
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f, with one line in it." }, { "files": { "g.txt": "1\n" }, "commit": "2", "note": long }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let mut s = open(&w.root, 34); s.s.side = false;
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    s.typed("and g, with forty lines"); s.press(&["enter"]);
    settle(&id); save_talk(&w, &id, "and g, with forty lines", &long); s.load();
    let b = crate::board_chat::chat(&mut s, 120).boxed;
    assert_eq!(b[..1].to_vec(), v(&["Line 1 of the answer."]), "{b:?}");
    s.press(&["alt+k"]);
    assert!(crate::board_chat::chat(&mut s, 120).boxed.iter().any(|l| l == "and g, with forty lines"), "{:?}", crate::board_chat::chat(&mut s, 120).boxed);
}

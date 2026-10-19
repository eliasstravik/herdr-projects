//! What the old board-talk test checked that tests/talk.rs does not: under review the web board's chat has what the
//! reviewer says and its text box says the review goes on through a message; another machine's task shows what its machine sent,
//! or says that machine is too old to send it; the reviewer's screen is read fresh; a dialog is said in words; and a
//! working agent's chat is its words (markdown read as text), never its tool calls, or its screen where it cannot be read.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use sw::board::keys::key_name;
use sw::board::view::talk::talk;
use sw::json::*;

fn jsonl(file: &Path, rows: &[Value]) -> PathBuf { std::fs::create_dir_all(file.parent().unwrap()).unwrap(); std::fs::write(file, rows.iter().map(|r| r.to_string()).collect::<Vec<_>>().join("\n") + "\n").unwrap(); file.to_path_buf() }
fn ts(n: u32) -> String { format!("2026-10-13T12:00:{n:02}.000Z") }
/// A Claude Code conversation as the program saves it: a prompt, then each thing said with a tool call and its result.
fn claude_rows(prompt: &str, said: &[&str]) -> Vec<Value> {
    let mut v = vec![json!({ "type": "user", "timestamp": ts(1), "message": { "role": "user", "content": prompt } })];
    for (n, text) in said.iter().enumerate() {
        let n = n as u32;
        v.push(json!({ "type": "assistant", "timestamp": ts(2 + n * 3), "message": { "id": format!("m{n}"), "role": "assistant", "content": [{ "type": "text", "text": text }] } }));
        v.push(json!({ "type": "assistant", "timestamp": ts(3 + n * 3), "message": { "id": format!("m{n}"), "role": "assistant", "content": [{ "type": "tool_use", "id": format!("t{n}"), "name": "Bash", "input": { "command": "npm run check -- --all" } }] } }));
        v.push(json!({ "type": "user", "timestamp": ts(4 + n * 3), "message": { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": format!("t{n}"), "content": "76 test files passed" }] } }));
    }
    v
}
fn boxed(s: &mut common::board::Board) -> Vec<String> { s.chat(120).boxed }
fn with(r: &Obj, more: Value) -> Obj { let mut o = r.clone(); for (k, v) in more.as_object().unwrap() { o.insert(k.clone(), v.clone()); } o }

#[test]
fn under_review_the_web_board_another_machine_and_a_dialog_say_what_the_reviewer_says() {
    let w = world_with(Opts { repo: true, settings: json!({ "reviewer.agent": "claude" }), ..Default::default() });
    let claude_dir = PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap());
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }] }));
    let id = to_approval(&w, "Make f");
    // A review under way, as the review runner leaves it while its reviewer works (as in tests/talk.rs).
    let api = sw::herdr::Herdr::new(None);
    let pane = api.tab_create(&task(&id).at("link").t("workspace"), "", "Reviewer #1", &[]).unwrap().at("root_pane").t("pane_id");
    api.agent_start("sw-proj-1-r", "claude", &pane, &[], 1000).unwrap();
    let screen = |w: &World, text: &str| w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["status"] = json!("working"); p["agent"]["screen"] = json!(text); } } });
    screen(&w, ">_ OpenAI Codex (v0.162.1)\n\n› Ask Codex to do anything\n");
    sw::store::tasks::update(&id, |t| { t.put("stage", "reviewing"); t.put("msg", ""); t.put("reviewSession", "rev-1"); t.put("reviewRound", 1); for k in ["review", "reviews", "stat", "pr"] { t.shift_remove(k); } t.obj_mut("link").put("reviewer", json!({ "pane": pane, "kind": "claude" })); Ok(sw::store::tasks::Change::Write) }).unwrap();
    let mut rows = claude_rows("Work board task proj/1, role: reviewer, round 1.", &["The full check passes. Now the browser."]);
    rows.push(json!({ "type": "assistant", "timestamp": ts(40), "message": { "id": "m7", "role": "assistant", "content": [{ "type": "text", "text": "One fault left: the header is cut." }] } }));
    jsonl(&claude_dir.join("projects").join("x").join("rev-1.jsonl"), &rows);
    let mut s = common::board::open(&w.root, 34); s.s.side = false;
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    assert_eq!(boxed(&mut s)[..2].to_vec(), vec!["Reviewer:", "One fault left: the header is cut."]);
    // the web board gets the same parts, and its text box says what a message does now
    let side = sw::webui::model::side::side_of(&mut s.s, &s.theme.clone());
    assert_eq!(side.at("chat").arr("parts").last().unwrap(), &json!({ "by": "Reviewer", "text": "One fault left: the header is cut.", "mine": false, "tone": "", "raw": false }));
    assert_eq!(side.at("chat").at("raw"), &json!(false));
    assert_eq!(side.at("send").s("place"), "Send message to Agent: the review goes on unless the code changes");
    // another machine's task: what that machine sent; one on an older version sends none, and that is said
    let r = s.s.current().unwrap().clone();
    let sent = sw::remote::remote_list().at("tasks").as_array().unwrap().iter().find(|x| x.s("id") == id).unwrap().at("snap").at("talk").clone();
    let far = talk(&with(&r, json!({ "host": "vm-a", "link": { "pane": "w1:p1" }, "snap": { "talk": sent } })));
    assert_eq!(far.who, "reviewer"); assert_eq!(far.parts.last().map(|p| (p.by.clone(), p.text.clone())), Some(("Reviewer".into(), "One fault left: the header is cut.".into())));
    let old = talk(&with(&r, json!({ "host": "vm-a", "snap": {} })));
    assert!(old.parts.last().unwrap().text.contains("vm-a runs an older herdr-projects"), "{:?}", old.parts);
    // the reviewer's screen (read where its program cannot be) is the one it shows now, not tied to the agent's turn,
    // which stands still during a review: a new frame is there at the next read
    assert!(sw::board::screens::screen_of(&r).contains("OpenAI Codex"), "{}", sw::board::screens::screen_of(&r));
    screen(&w, "Reading the diff\n"); sw::clock::advance(2500);
    assert_eq!(sw::board::screens::screen_of(&r), "Reading the diff");
    // a dialog on the reviewer's screen is said in words, with the way to it
    let asks = talk(&with(&r, json!({ "stage": "reviewneeds", "state": "blocked" })));
    assert!(asks.dialog);
    assert_eq!(asks.parts.last().unwrap().text, format!("The reviewer shows a dialog on its screen that waits for your answer. {} opens the task's space: the reviewer is in its tab \"Reviewer #1\".", key_name("enter")));
}

#[test]
fn a_working_agents_chat_is_its_words_never_its_tool_calls_or_its_screen_where_it_cannot_be_read() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let claude_dir = PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap());
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Bash(npm test)\n  ⎿ 3 passed\n✻ Transmuting… (7s · ↓ 625 tokens)\n❯ \n  ⏵⏵ bypass permissions on\n" }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    let t = task(&id);
    assert_eq!(t.s("stage"), "working");
    let mut s = common::board::open(&w.root, 34); s.s.side = false;
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    let b = boxed(&mut s); assert_eq!(b[..b.len() - 2].to_vec(), vec!["The agent has just started: nothing to show yet."]);
    assert!(t.b("session"), "{t:?}");
    jsonl(&claude_dir.join("projects").join("x").join(format!("{}.jsonl", t.s("session"))), &claude_rows("Work board task proj/1, role: agent.\n\nMake f", &["I am writing **f** now:\n- the file\n- its test"]));
    s.load();
    let b = boxed(&mut s); assert_eq!(b[..b.len() - 2].to_vec(), vec!["Agent:", "I am writing f now:", "- the file", "- its test"]);
    assert!(!regex::Regex::new(r"Bash\(|3 passed|Transmuting|bypass permissions|tokens\)").unwrap().is_match(&s.text()), "{}", s.text());
    s.press(&["alt+k"]); assert_eq!(boxed(&mut s)[..2].to_vec(), vec!["Board:", "Work board task proj/1, role: agent."]);
    // a dialog only its screen has: said plainly, with the key that goes there; not the screen
    let r = s.s.current().unwrap().clone();
    let mut live = r.at("live").clone(); if !live.is_object() { live = json!({}); } live["status"] = json!("blocked");
    let asks = talk(&with(&r, json!({ "live": live })));
    assert!(asks.dialog);
    assert_eq!(asks.parts.last().unwrap().text, format!("The agent shows a dialog on its screen that waits for your answer: {} opens it.", key_name("enter")));
    assert_eq!(asks.parts[asks.parts.len() - 2].by, "Agent");
    // a program whose conversation cannot be read: the words on its screen, named as that
    let other = talk(&with(&r, json!({ "host": "vm-a", "snap": { "talk": { "who": "agent", "kind": "other", "readable": false, "list": [] }, "screen": "────────\nReading the code\n⣿⣿⣿⣿" } })));
    assert_eq!(other.parts.iter().map(|p| (p.by.clone(), p.text.clone(), p.raw)).collect::<Vec<_>>(), vec![("Agent, on its screen now".to_string(), "Reading the code".to_string(), true)]);
    assert!(other.raw);
}

#[test]
fn automatic_screen_checks_never_scroll_cline_even_when_it_is_idle_or_done() {
    let w = world();
    let api = sw::herdr::Herdr::new(Some(w.socket.clone()));
    let ws = api.workspace_create(&w.root_str(), "Terminal", &[]).unwrap();
    let pane = ws.at("root_pane").s("pane_id");
    api.call(sw::herdr::cli::args(&["agent", "start", "screen-probe", "--kind", "cline", "--pane", pane])).unwrap();
    w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["screen"] = json!("Sichtbarer Cline-Text"); } } });
    let start = calls(&w).len();
    for state in ["working", "idle", "done", "blocked"] {
        set_status(&w, pane, state);
        assert_eq!(sw::herdr::api::read_screen(&api, pane), "Sichtbarer Cline-Text");
    }
    let log = calls(&w);
    let reads = &log[start..];
    assert_eq!(reads.iter().filter(|c| c.starts_with("pane read ") && c.contains("--source visible")).count(), 4);
    assert!(!reads.iter().any(|c| c.starts_with("agent read ") || c.contains("send-keys") || c.contains("recent-unwrapped")));
}

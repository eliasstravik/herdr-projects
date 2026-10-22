//! The Coordinator: one long-lived agent you talk to in the chat, started once in a tab of its own in the home space,
//! your words going in exactly as typed (enter alone: it decides what to start), its reply in the chat's box in place of
//! what you said, alt+c only shows or hides that message, and where each message was sent from said to it.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::json::*;

pub fn coord_world() -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn agents(w: &World) -> Vec<serde_json::Value> { w.state().arr("panes").iter().filter(|p| p.at("agent").is_object()).cloned().collect() }

#[test]
fn the_box_talks_to_one_long_lived_coordinator_started_once_with_nothing_held_back() {
    let w = coord_world();
    let socket = sw::herdr::cli::current_socket();
    assert!(sw::coordinator::who::ready(&socket));
    w.script(json!({ "coordinator": [{ "said": "Added proj/1." }, { "said": "Delegated proj/1." }] }));
    let mut s = open(&w.root, 34); s.s.side = false;
    let t = s.text();
    // It opens on the top row, this machine's, whose coordinator the chat talks to: its box's edge names it, and the box
    // invites a first message.
    let (host, top) = (regex::escape(&sw::shown::words::clean(&sw::remote::check::hostname())), regex::escape(&crate::board_chat::top_name()));
    assert!(has(&t, &format!(r#"\n╭─  {host}  · Machine coordinator ─*╮\n(?:│ +│\n)?│ +❯ Ask it something, e\.g\. "what's left to do here\?" +│\n(?:│ +│\n)?╰─+╯\n[^\n]*\? all keys *$"#)), "{t}");
    assert!(!has(&t, "claude|default model|default effort|last message|new task|reply to"), "{t}");
    s.typed("build a login page");
    assert!(s.text().contains("❯ build a login page") && s.text().contains(&format!("enter send to {} · shift+enter new line · esc put away", crate::board_chat::top_name())), "{}", s.text());
    // tab is no second way to send: the Coordinator decides itself what to start, so the words stay where they are
    s.press(&["ctrl+s"]);
    assert!(s.s.value() == "build a login page" && sw::coordinator::state::get(&socket).unwrap_or_default().arr("queue").is_empty());
    s.press(&["enter"]);
    assert_eq!(s.s.value(), "", "{}", s.s.msg);
    assert_eq!(sw::coordinator::state::get(&socket).unwrap_or_default().arr("queue").len(), 1);
    s.load();
    assert!(has(&s.text(), &format!(r"\n╭─  {host}  · Machine coordinator ─*╮\n(?:│ +│\n)?│ +build a login page +│\n│ +│\n│ +❯ Send message to the machine coordinator +│\n(?:│ +│\n)?╰─+╯\n")), "{}", s.text());
    drain();
    let st = w.state();
    assert_eq!(st.arr("workspaces").iter().map(|x| x.t("label")).collect::<Vec<_>>(), vec!["~"]);
    assert_eq!(st.arr("tabs").iter().map(|x| x.t("label")).collect::<Vec<_>>(), vec!["1", "Coordinator"]);
    let a = agents(&w);
    assert_eq!(a.len(), 1);
    let ag = a[0].at("agent");
    assert_eq!((ag.s("name"), ag.s("kind")), ("sw-coordinator", "claude"));
    let args: Vec<String> = ag.arr("args").iter().map(text).collect();
    assert!(args.contains(&"--dangerously-skip-permissions".into()) && args.contains(&"--session-id".into()) && !args.contains(&"--append-system-prompt-file".into()));
    assert!(!args.iter().any(|x| x.contains("allowedTools") || x == "--tools"), "nothing is held back from it");
    assert!(read(&sw::coordinator::state::home(&socket).join("AGENTS.md")).contains("You are the Coordinator of the human's work board"));
    assert_eq!(ag.arr("prompts")[0].s(""), "", "{}", ag.at("prompts"));
    assert_eq!(text(&ag.arr("prompts")[0]), "build a login page", "your words go in exactly as typed");
    assert!(has(&sw::coordinator::requests::where_text(&socket), r#"^The human's last message from the board:\n- "build a login page" was sent from the repo "proj" \(.*\)\. No task was open\. It was sent with enter: delegate only what the human asked you to\.$"#), "{}", sw::coordinator::requests::where_text(&socket));
    // Its reply takes the place of what you said: the box holds one message, never both.
    s.load();
    for width in [160, 120, 60] {
        let txt = s.screen(width).join("\n");
        assert!(has(&txt, &format!(r"\n╭─  {host}  [^\n]*╮\n(?:│ +│\n)?│ +Added proj/1\. +│\n│ +│\n│ +❯ Send message to the machine coordinator +│\n(?:│ +│\n)?╰─+╯\n[ ▌] [^\n]*{} \? all keys *$", if width >= 120 { format!("{} hide the latest message ·", regex::escape(&key_name("chatMessage"))) } else { String::new() })), "at {width}: {txt}");
        assert!(s.screen(width).iter().all(|l| sw::text::width(l) <= width));
    }
    assert_eq!(sw::coordinator::look::look(&socket).unseen, "", "a reply you can see counts as seen");
    s.press(&["alt+c"]); assert!(!s.s.chat_open);
    assert!(!s.text().contains("Added proj/1."));
    assert_eq!(sw::store::files::read_obj(&sw::store::files::data_dir().join("view.json")).unwrap().s("chat"), "closed");
    s.press(&["esc"]); assert!(!s.s.chat_open && s.s.ask.is_none() && !s.closed);
    s.press(&["alt+c"]); assert!(s.s.chat_open && s.text().contains("Added proj/1."));
    // The next thing you type goes to the same agent in the same conversation, as typed: it decides what to start.
    s.press(&["alt+c"]);
    s.typed("and start it"); s.press(&["enter"]);
    drain();
    assert_eq!(agents(&w).len(), 1);
    assert_eq!(calls(&w).iter().filter(|c| c.starts_with("agent start")).count(), 1);
    let p = agents(&w)[0].at("agent").arr("prompts").to_vec();
    assert_eq!(p.len(), 2);
    assert_eq!(text(&p[1]), "and start it");
    assert!(has(&sw::coordinator::requests::where_text(&socket), r#"2 messages[^\n]*\n- "build a login page"[^\n]*It was sent with enter[^\n]*\n- "and start it"[^\n]*It was sent with enter[^\n]*$"#));
    // Hidden, the new reply waits; shown again, it is there.
    s.load(); assert!(!s.s.chat_open);
    assert!(!s.text().contains("Delegated proj/1."), "{}", s.text());
    s.press(&["alt+c"]); assert!(s.text().contains("Delegated proj/1."));
}

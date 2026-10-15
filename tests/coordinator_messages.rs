//! What you say to the Coordinator is never lost: a dialog on its screen holds it, a failed start says why, each
//! message keeps its own repo and tab choice, none it has not dealt with is dropped, and it can be turned on and off
//! while the board is open.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use sw::board::keys::key_name;
use sw::coordinator::{look::look, pump::tick, requests::{send, tab_note, where_text}, state, who::pick_of};
use sw::json::*;

pub fn coord_world_with(settings: Value) -> World {
    let mut s = json!({ "coordinator.agent": "claude", "review.when": "never" });
    for (k, v) in settings.as_object().cloned().unwrap_or_default() { s[k] = v; }
    let w = world_with(Opts { repo: true, settings: s, ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    unsafe { std::env::remove_var("HERDR_PROJECTS_HANDLED_MS") };
    w
}
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn agents(w: &World) -> Vec<Value> { w.state().arr("panes").iter().filter(|p| p.at("agent").is_object()).cloned().collect() }
fn get(w: &World) -> Obj { state::get(&w.socket).unwrap_or_default() }
fn said(w: &World) -> Vec<String> { get(w).arr("where").iter().map(|x| x.t("said")).collect() }
fn wheres(w: &World) -> Vec<(String, String, bool)> { get(w).arr("where").iter().map(|x| (x.t("said"), x.t("repo"), x.at("delegate") == &json!(true))).collect() }
fn status(w: &World, st: &str) { w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.at("agent").is_object() { p["agent"]["status"] = json!(st); } } }); }
fn unstamp(name: &str) { let _ = std::fs::remove_file(sw::store::marks::stamp_file(name)); }
fn tell(w: &World, cwd: &std::path::Path, text: &str, delegate: bool) { send(cwd, text, delegate, "", &w.socket).unwrap(); }
fn last_lines(w: &World) -> Vec<String> { agents(w)[0].at("agent").arr("prompts").iter().map(|p| text(p).rsplit('\n').next().unwrap_or("").to_string()).collect() }
fn s3(a: &str, b: &str, c: bool) -> (String, String, bool) { (a.into(), b.into(), c) }

#[test]
fn what_you_say_is_never_lost_a_dialog_on_its_screen_holds_it_a_failed_start_says_why() {
    let w = coord_world_with(json!({}));
    let so = w.socket.clone();
    w.patch(|s| s["dialogs"]["claude"] = json!(1)); // a first-run dialog blocks the start
    tell(&w, &w.root, "first", false); drain();
    assert_eq!(get(&w).arr("queue").len(), 1); assert_eq!(look(&so).state, "blocked");
    let mut s = open(&w.root, 34);
    assert!(has(&s.text(), r"\n[ ▌]. Coordinator {2}asks you · 1 message waits to be sent *\n╭─[^\n]*╮\n"), "{}", s.text());
    s.press(&["alt+c"]); assert!(has(&s.text(), r"\n[ ▌]. Coordinator {2}asks you · 1 message waits to be sent *\n─[^\n]*\n❯ "), "{}", s.text());
    // herdr's sidebar names it and says where it stands in the same words
    sw::coordinator::sidebar::tell_sidebar(&so);
    let meta = |w: &World| w.state().arr("metadata").last().map(|m| m.arr("args").iter().map(text).collect::<Vec<_>>().join(" ")).unwrap_or_default();
    assert!(has(&meta(&w), "--display-agent Coordinator · asks you · 1 message waits to be sent --title Coordinator "), "{}", meta(&w));
    let told = w.state().arr("metadata").len(); unstamp("coordinator-sidebar-default"); sw::coordinator::sidebar::tell_sidebar(&so); assert_eq!(w.state().arr("metadata").len(), told, "the same words: not written again");
    // You answer the dialog in its chat; the tick then types the waiting words in.
    status(&w, "idle"); unstamp("coordinator-default");
    tick(&so); drain();
    assert_eq!(get(&w).arr("queue").len(), 0); assert_eq!(last_lines(&w), vec!["first"]);
    // A second message refused by a dialog did not go in: the first keeps its repo and its delegate choice.
    let other = w.base.join("elsewhere"); std::fs::create_dir_all(&other).unwrap();
    status(&w, "blocked");
    tell(&w, &other, "later", true); drain();
    assert_eq!(get(&w).arr("queue").len(), 1); assert_eq!(wheres(&w), vec![s3("first", "proj", false)]);
    assert!(!has(&where_text(&so), "later|elsewhere|was sent with tab"));
    status(&w, "idle"); unstamp("coordinator-default"); tick(&so); drain();
    assert_eq!(wheres(&w), vec![s3("first", "proj", false), s3("later", "elsewhere", true)]);
    // Two messages in the same millisecond, the same words, from different repos with opposite tab choices.
    sw::clock::set(Some(1700000000000)); tell(&w, &w.root, "same words", false); tell(&w, &other, "same words", true); sw::clock::set(None);
    drain();
    assert_eq!(get(&w).arr("queue").len(), 0);
    let same: Vec<String> = agents(&w)[0].at("agent").arr("prompts").iter().map(text).filter(|p| p.starts_with("same words")).collect();
    assert_eq!(same, vec!["same words".to_string(), format!("same words\n\n{}", tab_note())]);
    assert_eq!(wheres(&w)[wheres(&w).len() - 2..].to_vec(), vec![s3("same words", "proj", false), s3("same words", "elsewhere", true)]);
    // The list is full (five that went in). One more that a dialog refuses takes nothing away from the oldest.
    unsafe { std::env::set_var("HERDR_PROJECTS_HANDLED_MS", "0") };
    tell(&w, &w.root, "fifth", false); drain();
    let full = said(&w); assert_eq!(full, vec!["first", "later", "same words", "same words", "fifth"]);
    status(&w, "blocked");
    tell(&w, &other, "sixth", true); drain();
    assert_eq!(said(&w), full, "the refused one is not said, and the first is still there");
    status(&w, "idle"); unstamp("coordinator-default"); tick(&so); drain();
    assert_eq!(said(&w), vec!["later", "same words", "same words", "fifth", "sixth"]);
    // Still working through what it was sent: nothing it has not dealt with yet is dropped.
    w.script(json!({ "coordinator": (0..7).map(|_| json!({ "status": "working" })).collect::<Vec<_>>() }));
    for n in 1..=7 { tell(&w, if n == 1 { &other } else { &w.root }, &format!("busy {n}"), n == 1); }
    drain();
    assert_eq!(look(&so).state, "working");
    assert_eq!(said(&w), (1..=7).map(|n| format!("busy {n}")).collect::<Vec<_>>(), "all seven it still has ahead of it");
    assert!(has(&where_text(&so), r#"- "busy 1" was sent from the repo "elsewhere"[^\n]*It was sent with tab"#));
    // There is no number at which one it has not dealt with is let go.
    w.script(json!({ "coordinator": (0..61).map(|_| json!({ "status": "working" })).collect::<Vec<_>>() }));
    for n in 8..=68 { tell(&w, &w.root, &format!("busy {n}"), false); }
    drain();
    assert_eq!(get(&w).arr("queue").len(), 0); assert_eq!(get(&w).arr("where").len(), 68);
    assert_eq!(wheres(&w)[0], s3("busy 1", "elsewhere", true));
    // Restarted in the middle of that work: idle alone is not taken for dealt with.
    sw::coordinator::requests::restart(&so).unwrap(); drain();
    assert_eq!(look(&so).state, "idle"); assert_eq!(get(&w).arr("where").len(), 68);
    w.script(json!({ "coordinator": [{ "said": "ok" }] }));
    tell(&w, &w.root, "after the restart", false); drain();
    assert_eq!(get(&w).arr("where").len(), 69); assert_eq!(wheres(&w)[0], s3("busy 1", "elsewhere", true));
    // It has finished a turn since and is at rest: the next message finds them dealt with.
    w.script(json!({ "coordinator": [{ "said": "ok" }] }));
    tell(&w, &w.root, "after", false); drain();
    assert_eq!(said(&w), vec!["busy 66", "busy 67", "busy 68", "after the restart", "after"]);
    // A message typed in a moment ago may not have set it working yet: it is not taken for dealt with.
    unsafe { std::env::set_var("HERDR_PROJECTS_HANDLED_MS", "15000") };
    w.script(json!({ "coordinator": (0..6).map(|_| json!({ "said": "ok" })).collect::<Vec<_>>() }));
    for n in 1..=6 { tell(&w, &w.root, &format!("quick {n}"), false); }
    drain();
    assert_eq!(look(&so).state, "idle");
    assert_eq!(said(&w), std::iter::once("after".to_string()).chain((1..=6).map(|n| format!("quick {n}"))).collect::<Vec<_>>());
    unsafe { std::env::remove_var("HERDR_PROJECTS_HANDLED_MS") };
    // A program that is not installed cannot be started: the reason shows, and the words wait.
    // Only pi is missing: claude stays installed as the world has it, never asked of this machine.
    let versions = sw::store::files::data_dir().join("versions.json");
    let mut known = sw::store::files::read_json(&versions).unwrap(); known["pi"] = json!({ "v": "", "at": sw::clock::now(), "found": false });
    sw::store::files::write_json(&versions, &known).unwrap();
    sw::coordinator::requests::fresh(pick_of("pi", "").unwrap(), &so).unwrap(); tell(&w, &w.root, "second", false); drain();
    assert!(has(&look(&so).error, r#""pi" is not installed here"#), "{}", look(&so).error); assert_eq!(get(&w).arr("queue").len(), 1);
    s.load();
    assert!(has(&s.text(), r#"Coordinator {2}could not be started[^\n]*\n╭─[^\n]*╮\n│ It could not be started: "pi" is not installed here"#), "also with its message hidden: {}", s.text());
    // alt+c hides that too, as the chat's line says, and so does esc with nothing else to step back from.
    s.s.chat_open = false; s.s.input.set_text(""); s.s.reply = false; s.s.full = false; s.s.filter = String::new(); s.s.msg = String::new();
    let chat_key = regex::escape(&key_name("chatMessage"));
    assert!(has(&s.text(), &format!("{chat_key} hide the latest message")), "{}", s.text());
    s.press(&["esc"]); assert!(s.s.ask.is_none() && !s.closed && s.s.closing.is_none());
    assert!(!s.text().contains("It could not be started:"));
    assert!(has(&s.text(), &format!(r"Coordinator {{2}}could not be started[^\n]*\n─[^\n]*\n❯ [^\n]*\n─+\n[^\n]*{chat_key} show the latest message")), "{}", s.text());
    s.load(); assert!(!s.text().contains("It could not be started:"), "the same error stays hidden");
    s.press(&["esc"]); s.s.closing = None;
    // alt+c shows it again and hides it again, shown by the setting or not.
    s.press(&["alt+c"]); assert!(s.text().contains("It could not be started:")); s.press(&["alt+c"]); assert!(!s.text().contains("It could not be started:")); assert!(s.s.ask.is_none());
    s.s.err_hid = String::new(); s.s.chat_open = true; assert!(s.text().contains("It could not be started:")); s.press(&["alt+c"]); assert!(!s.text().contains("It could not be started:"));
    let e = pick_of("not a kind", "").unwrap_err();
    assert!(has(&e.msg, r#"coordinator\.agent: no agent program "not a kind"\. It must be claude, codex, pi, agy, .*, qwen or none\."#), "{}", e.msg);
}

#[test]
fn turning_the_coordinator_on_while_the_board_is_open_takes_effect_at_once_and_off_again_too() {
    use sw::store::settings::{change::{set_value, unset_value}, lookup::Scope};
    let w = coord_world_with(json!({ "coordinator.agent": "none" }));
    let mut s = open(&w.root, 34);
    assert!(has(&s.text(), r"\n *\n \+ New task {2}added word for word *\n─[^\n]*\n❯ Add a task *\n"), "{}", s.text()); assert!(!s.text().contains("Coordinator"));
    s.typed("first thing"); s.press(&["enter"]);
    assert_eq!(task("proj/1").s("title"), "first thing"); assert!(state::get(&w.socket).is_none());
    unset_value("coordinator.agent", &Scope::you()).unwrap(); // the board stays open
    // (Rust: a key press reads no file; the setting is seen on the board's next load, which a running board does every beat.)
    s.load();
    s.typed("second thing"); s.press(&["enter"]);
    assert!(sw::store::tasks::get("proj/2").is_none(), "not added word for word any more"); assert_eq!(get(&w).arr("queue").len(), 1);
    drain();
    assert_eq!(text(&agents(&w)[0].at("agent").arr("prompts")[0]), "second thing"); assert!(!s.text().contains("no Coordinator"));
    // Off again: the next look sees it, even with one still running.
    set_value("coordinator.agent", &json!("none"), &Scope::you()).unwrap();
    s.load();
    s.typed("third thing"); s.press(&["enter"]);
    assert_eq!(task("proj/2").s("title"), "third thing"); assert_eq!(agents(&w)[0].at("agent").arr("prompts").len(), 1);
    // And your next message turns up a Coordinator that was switched on after the board opened.
    let mut off = open(&w.root, 34); assert!(!off.s.data.can_dispatch); assert!(off.text().contains("+ New task"));
    unset_value("coordinator.agent", &Scope::you()).unwrap();
    off.load(); off.typed("fourth thing"); off.press(&["enter"]); drain(); off.load();
    assert!(off.s.data.can_dispatch); assert!(has(&off.text(), r"\n[ ▌]. Coordinator {2}"), "{}", off.text());
    assert_eq!(get(&w).arr("where").last().map(|x| x.t("said")).unwrap_or_default(), "fourth thing");
}

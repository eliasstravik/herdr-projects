//! Words typed into an agent's box reach it: an enter a starting program swallowed is pressed again, never the words
//! twice; a message typed into a running turn is not taken for sent because the agent works; and an agent that never
//! takes its instructions says so ("Not started: its instructions sit unsent in its box") until retry hands them over,
//! with the last lines of its pane. One whose brief never reached it, while herdr's counters for it moved, says so too
//! once the bound for a proof has passed; one whose saved conversation holds the brief is not failed.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::submit::{NOT_STARTED, TRIES};

fn waits(ready: i64, submit: i64) { unsafe { std::env::set_var("HERDR_PROJECTS_READY_MS", ready.to_string()); std::env::set_var("HERDR_PROJECTS_SUBMIT_MS", submit.to_string()); } }
fn pass(w: &World) { sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes); }
fn count(w: &World, re: &str) -> usize { let r = regex::Regex::new(re).unwrap(); calls(w).iter().filter(|c| r.is_match(c)).count() }
fn the_agent(w: &World, id: &str) -> serde_json::Value { agent_of(w, task(id).at("link").s("pane")) }
/// A program at its prompt, with nothing in its box.
const EMPTY_BOX: &str = "Claude Code v2.1.296\n\n╭──────────────────────────────╮\n│ >                            │\n╰──────────────────────────────╯\n  ? for shortcuts";
/// The task's agent shows this on its screen; `moved`: it was drawn again with its box empty, which moves herdr's change
/// counter for it while it stays idle.
fn shows(w: &World, id: &str, screen: &str, moved: bool) {
    let pane = task(id).at("link").t("pane");
    w.patch(|s| {
        let seq = s.i("seq") + 1; if moved { s["seq"] = json!(seq); }
        for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["screen"] = json!(screen); if moved { p["agent"]["box"] = serde_json::Value::Null; p["agent"]["seq"] = json!(seq); } } }
    });
}
/// A task whose agent's program lost its brief: typed, never in its box, and the screen drawn again.
fn lost_brief(title: &str) -> (World, String, String) {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    waits(0, 0);
    w.patch(|s| s["swallow"] = json!(1));
    let id = w.add(title);
    delegate(&id);
    sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); });
    let typed = task(&id).at("sent").t("text");
    assert!(!typed.is_empty());
    shows(&w, &id, EMPTY_BOX, true);
    pass(&w);
    let c = task(&id);
    assert!(c.at("sent").is_null() && c.at("unconfirmed").is_object() && c.s("stage") == "working", "{}", c.at("sent"));
    (w, id, typed)
}

#[test]
fn a_message_typed_into_a_running_turn_gets_its_enter_once_the_agent_rests_with_it_unsent() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    waits(60000, 0);
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }, { "status": "working" }] }));
    let id = w.add("Busy");
    delegate(&id);
    settle(&id);
    assert_eq!(the_agent(&w, &id).s("status"), "working");
    w.patch(|s| s["swallow"] = json!(1));
    sw::actions::talk::tell(&id, "while you work", false).unwrap();
    for _ in 0..3 { pass(&w); }
    assert!(task(&id).at("sent").b("busy"), "its working says nothing about the new text");
    assert_eq!(count(&w, r"^agent send-keys .* enter$"), 0);
    let rests = |w: &World| w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); for p in s["panes"].as_array_mut().unwrap() { if p.at("agent").is_object() { p["agent"]["status"] = json!("idle"); p["agent"]["seq"] = json!(seq); p["agent"]["completion"] = json!(seq); } } });
    rests(&w);
    for _ in 0..4 { pass(&w); settle(&id); }
    let a = the_agent(&w, &id);
    assert_eq!(count(&w, r"^agent send-keys .* enter$"), 1);
    assert!(a.at("box").is_null());
    assert_eq!(a.arr("prompts").iter().filter(|x| text(x).contains("while you work")).count(), 1);
    assert_eq!((task(&id).t("stage"), task(&id).at("sent").is_null(), a.t("status")), ("working".into(), true, "working".into()));
    // Every enter refused: that turn is not finished in the message's place; the task says not started until retry.
    w.patch(|s| { s["swallow"] = json!(1); s["deaf"] = json!(99); s["pids"] = json!(true); });
    sw::actions::talk::tell(&id, "REFUSED WHILE BUSY", false).unwrap();
    rests(&w);
    for _ in 0..(TRIES + 3) { pass(&w); settle(&id); }
    assert_eq!((task(&id).t("stage"), row(&id).t("note"), task(&id).at("sent").is_null()), ("failed".into(), NOT_STARTED.to_string(), true));
    assert!(task(&id).at("unsent").s("text").contains("REFUSED WHILE BUSY"));
    assert!(the_agent(&w, &id).s("box").contains("REFUSED WHILE BUSY"));
    w.patch(|s| s["deaf"] = json!(0));
    sw::actions::finish_line::retry(&id).unwrap();
    settle(&id);
    assert_eq!(count(&w, r"^agent start"), 2);
    assert_eq!(the_agent(&w, &id).arr("prompts").iter().filter(|x| text(x).contains("REFUSED WHILE BUSY")).count(), 1, "handed to the new program, once");
    assert!(!task(&id).b("unsent") && task(&id).at("failure").s("stage") != "send");
}

#[test]
fn an_agent_that_never_starts_is_shown_as_not_started_and_retry_starts_it_again_with_the_same_text() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    waits(0, 0);
    w.patch(|s| { s["slowStart"] = json!(99); s["deaf"] = json!(99); s["pids"] = json!(true); });
    let id = w.add("Never starts");
    delegate(&id);
    sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); });
    let typed = the_agent(&w, &id).t("box");
    assert_eq!((task(&id).t("stage"), row(&id).t("note")), ("working".into(), "Has not started yet: pressing enter again".into()));
    shows(&w, &id, &format!("╭──────────╮\n│ > {} │\n╰──────────╯\n  ? for shortcuts", typed.lines().last().unwrap_or("")), false);
    for _ in 0..(TRIES + 2) { pass(&w); }
    let c = task(&id); let r = row(&id);
    assert_eq!(count(&w, r"^agent send-keys .* enter$") as i64, TRIES, "a few tries, then no more");
    assert_eq!(count(&w, r"^agent prompt"), 1);
    assert_eq!((c.t("stage"), r.t("state"), r.t("note")), ("failed".into(), "error".into(), NOT_STARTED.to_string()));
    assert_eq!(c.at("failure").s("title"), "The agent has not started on its instructions");
    assert!(c.at("failure").s("next").starts_with(&format!("Retry ({}) starts the agent's program again", sw::board::keys::key_name("yes"))));
    assert_eq!((c.at("failure").at("agentCanFix"), c.at("unsent").t("text")), (&json!(false), typed.clone()));
    assert!(c.at("failure").s("detail").ends_with(&format!("The last lines of its terminal:\n╭──────────╮\n│ > {} │\n╰──────────╯\n  ? for shortcuts", typed.lines().last().unwrap_or(""))), "{}", c.at("failure").s("detail"));
    w.patch(|s| { s["slowStart"] = json!(0); s["deaf"] = json!(0); });
    assert!(sw::actions::finish_line::send_error(&id).unwrap().msg.contains("starting the agent again with the same text"));
    settle(&id);
    let c = task(&id);
    let ps: Vec<String> = calls(&w).into_iter().filter(|x| x.starts_with("agent prompt")).collect();
    assert_eq!(count(&w, r"^agent start"), 2);
    assert_eq!(ps.len(), 2); assert_eq!(ps[1], ps[0]);
    assert!(!c.b("unsent") && !c.b("sent") && c.at("failure").s("stage") != "send" && c.s("stage") != "failed");
}

#[test]
fn a_brief_that_never_reached_the_agent_is_not_started_once_the_bound_passes_though_herdr_s_counters_moved() {
    let (w, id, typed) = lost_brief("Never reached");
    let enters = count(&w, r"^agent send-keys .* enter$");
    sw::clock::set(Some(task(&id).at("unconfirmed").i("seen")));
    for _ in 0..3 { pass(&w); }
    sw::clock::advance(180_000 - 1);
    pass(&w);
    assert_eq!((task(&id).t("stage"), task(&id).at("unconfirmed").is_object()), ("working".into(), true), "not before the bound");
    sw::clock::advance(1);
    pass(&w);
    let c = task(&id);
    assert_eq!((c.t("stage"), row(&id).t("note"), c.at("failure").t("stage")), ("failed".into(), NOT_STARTED.to_string(), "send".into()));
    assert_eq!((c.at("unsent").t("text"), c.at("unsent").b("typed"), c.at("unconfirmed").is_null()), (typed, true, true));
    let d = c.at("failure").t("detail");
    assert!(d.contains("never reached the agent") && d.ends_with("The last lines of its terminal:\nClaude Code v2.1.296\n╭──────────────────────────────╮\n│ >                            │\n╰──────────────────────────────╯\n  ? for shortcuts"), "{d}");
    assert!(!c.at("failure").s("next").contains("herdr agent prompt"));
    assert_eq!(count(&w, r"^agent send-keys .* enter$"), enters, "no more enters into an empty box");
    sw::clock::set(None);
    sw::actions::finish_line::retry(&id).unwrap();
    settle(&id);
    let ps: Vec<String> = calls(&w).into_iter().filter(|x| x.starts_with("agent prompt")).map(|x| x.splitn(4, ' ').nth(3).unwrap_or("").to_string()).collect();
    assert_eq!((count(&w, r"^agent start"), ps.len()), (2, 2));
    assert_eq!(ps[1], ps[0], "the same text, handed to the new program");
    assert!(!task(&id).b("unsent") && task(&id).at("failure").s("stage") != "send");
}

#[test]
fn a_start_whose_saved_conversation_holds_the_brief_is_not_failed_though_only_herdr_s_counters_moved() {
    let (w, id, typed) = lost_brief("Took it quietly");
    let file = w.base.join("conversation.jsonl");
    std::fs::write(&file, format!("{}\n", json!({ "type": "user", "message": { "role": "user", "content": typed }, "timestamp": sw::clock::iso(sw::clock::now()) }))).unwrap();
    sw::store::tasks::patch(&id, |t| { t.put("transcript", file.display().to_string()); }).unwrap();
    pass(&w);
    assert!(task(&id).at("unconfirmed").is_null(), "its saved conversation proves it took the text");
    sw::clock::set(Some(sw::clock::now() + 600_000));
    for _ in 0..2 { pass(&w); }
    let c = task(&id);
    assert_eq!((c.t("stage"), c.b("unsent"), c.at("failure").s("stage") == "send"), ("working".into(), false, false));
}

#[test]
fn the_pane_shows_the_words_even_when_its_program_wrapped_them_in_its_box() {
    use sw::submit::give_up::on_screen;
    let text = "Work board task proj/1, role: agent.\n\nMake the login page load again";
    assert!(on_screen("╭──────────────────────────╮\n│ > Work board task proj/1,│\n│   role: agent.           │\n╰──────────────────────────╯", text));
    assert!(on_screen("> Make the login page\nload again\n\n  ? for shortcuts", text));
    assert!(!on_screen(EMPTY_BOX, text));
}

//! Words typed into an agent's box reach it: an enter a starting program swallowed is pressed again, never the words
//! twice; a message typed into a running turn is not taken for sent because the agent works; and an agent that never
//! takes its instructions says so ("Not started: its instructions sit unsent in its box") until retry hands them over.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::submit::{NOT_STARTED, TRIES};

fn waits(ready: i64, submit: i64) { unsafe { std::env::set_var("HERDR_PROJECTS_READY_MS", ready.to_string()); std::env::set_var("HERDR_PROJECTS_SUBMIT_MS", submit.to_string()); } }
fn pass(w: &World) { sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes); }
fn count(w: &World, re: &str) -> usize { let r = regex::Regex::new(re).unwrap(); calls(w).iter().filter(|c| r.is_match(c)).count() }
fn the_agent(w: &World, id: &str) -> serde_json::Value { agent_of(w, task(id).at("link").s("pane")) }

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
fn a_brief_the_agent_never_took_fails_and_says_how_to_hand_it_over() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    waits(0, 0);
    unsafe { std::env::set_var("HERDR_PROJECTS_LOST_MS", "60000") };
    // The prompt comes back as sent and leaves nothing at all in the pane: the box is empty and no work is going on.
    w.patch(|s| { s["lost"] = json!(99); s["deaf"] = json!(99); });
    let id = w.add("Brief never arrives");
    delegate(&id);
    sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); });
    assert_eq!(the_agent(&w, &id).t("box"), "");
    // herdr's own change counter moves on, as a program still drawing its screen does: that alone would read as "it
    // started". A minute later the agent has still not started on the text, and its conversation does not hold it.
    w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); for p in s["panes"].as_array_mut().unwrap() { if p.at("agent").is_object() { p["agent"]["seq"] = json!(seq); } } });
    sw::clock::advance(61000);
    for _ in 0..2 { pass(&w); }
    let c = task(&id); let r = row(&id);
    assert_eq!((c.s("stage"), r.t("state")), ("failed".into(), "error".into()), "{}", c.at("failure"));
    assert_eq!(c.at("failure").s("stage"), "send");
    assert!(c.at("failure").s("detail").contains("the agent has not started on it"), "{}", c.at("failure"));
    assert!(c.at("failure").s("detail").contains("pane shows nothing"), "the pane was read: {}", c.at("failure"));
    assert!(c.at("failure").s("next").contains("herdr agent prompt "), "{}", c.at("failure"));
    assert!(c.at("unsent").s("text").contains("Brief never arrives"), "the brief is kept for retry: {}", c.at("unsent"));
    assert!(count(&w, r"^agent send-keys .* enter$") <= TRIES as usize, "a few tries, then no more");
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
    for _ in 0..(TRIES + 2) { pass(&w); }
    let c = task(&id); let r = row(&id);
    assert_eq!(count(&w, r"^agent send-keys .* enter$") as i64, TRIES, "a few tries, then no more");
    assert_eq!(count(&w, r"^agent prompt"), 1);
    assert_eq!((c.t("stage"), r.t("state"), r.t("note")), ("failed".into(), "error".into(), NOT_STARTED.to_string()));
    assert_eq!(c.at("failure").s("title"), "The agent has not started on its instructions");
    assert!(c.at("failure").s("next").starts_with(&format!("Retry ({}) starts the agent's program again", sw::board::keys::key_name("yes"))));
    assert_eq!((c.at("failure").at("agentCanFix"), c.at("unsent").t("text")), (&json!(false), typed));
    w.patch(|s| { s["slowStart"] = json!(0); s["deaf"] = json!(0); });
    assert!(sw::actions::finish_line::send_error(&id).unwrap().msg.contains("starting the agent again with the same text"));
    settle(&id);
    let c = task(&id);
    let ps: Vec<String> = calls(&w).into_iter().filter(|x| x.starts_with("agent prompt")).collect();
    assert_eq!(count(&w, r"^agent start"), 2);
    assert_eq!(ps.len(), 2); assert_eq!(ps[1], ps[0]);
    assert!(!c.b("unsent") && !c.b("sent") && c.at("failure").s("stage") != "send" && c.s("stage") != "failed");
}

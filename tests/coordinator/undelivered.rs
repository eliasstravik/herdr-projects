//! `coordinator tell` where the herdr program cannot be found (an ssh forced command, cron): the message is queued all
//! the same, and the answer says it is not delivered, and why, as an error; `doctor` names messages that wait so.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::coordinator::state;
use sw::doctor::queue;
use sw::json::*;
use super::coord_world;

fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn tell(text: &str) -> sw::err::Res<String> {
    let a = sw::cli::flags::parse(&["coordinator", "tell", text].iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let r = sw::cli::tasks::command(&a.pos[0], &a).map(|(_, said)| said);
    drain();
    r
}
fn queued(w: &World) -> Vec<String> { state::get(&w.socket).map(|c| c.arr("queue").iter().map(|m| m.t("said")).collect()).unwrap_or_default() }
fn agents(w: &World) -> Vec<Value> { w.state().arr("panes").iter().filter(|p| p.at("agent").is_object()).cloned().collect() }

#[test]
fn with_no_herdr_to_be_found_tell_fails_saying_the_message_is_queued_not_delivered_and_doctor_names_it() {
    let w = coord_world();
    w.patch(|st| st["noProgram"] = json!(true));
    let e = tell("hello").expect_err("tell must fail where herdr cannot be found");
    assert!(e.msg.starts_with("Queued for the Coordinator, but not delivered: the herdr program was not found here"), "{}", e.msg);
    assert!(e.msg.contains("~/.local/bin") && e.msg.contains("HERDR_BIN_PATH"), "{}", e.msg);
    assert_eq!(queued(&w), vec!["hello"], "the message is kept in the queue");
    assert!(agents(&w).is_empty(), "nothing was started");
    // Just sent, it is still on its way; a while later doctor names it as a problem.
    assert!(queue::ask(&w.socket, sw::clock::now()).is_empty());
    sw::clock::advance(3 * 60_000);
    let now = sw::clock::now();
    let found = queue::check(&queue::ask(&w.socket, now), now);
    assert_eq!(found.len(), 1);
    assert!(found[0].problem);
    assert_eq!(found[0].what, "1 message waits in the Coordinator's queue, not delivered (the oldest sent 3 minutes ago)");
    // Once herdr is found, the next tell delivers both, in order, and nothing waits.
    w.patch(|st| st["noProgram"] = json!(false));
    w.script(json!({ "coordinator": [{ "said": "Hi." }, { "said": "Hi again." }] }));
    assert!(tell("again").unwrap().contains("sent to the Coordinator"));
    assert!(queued(&w).is_empty());
    let prompts: Vec<String> = agents(&w)[0].at("agent").arr("prompts").iter().map(|p| text(p).rsplit('\n').next().unwrap_or("").to_string()).collect();
    assert_eq!(prompts.last().map(String::as_str), Some("again"));
    assert!(prompts.iter().any(|p| p == "hello"), "{prompts:?}");
    sw::clock::advance(3 * 60_000);
    let now = sw::clock::now();
    let found = queue::check(&queue::ask(&w.socket, now), now);
    assert_eq!(found.iter().map(|f| (f.problem, f.what.as_str())).collect::<Vec<_>>(), vec![(false, "No message waits undelivered for a coordinator")]);
}

#[test]
fn with_herdr_found_tell_says_sent_as_before() {
    let w = coord_world();
    w.script(json!({ "coordinator": [{ "said": "Hello." }] }));
    assert!(tell("hello").unwrap().contains("sent to the Coordinator"));
    assert!(queued(&w).is_empty());
    assert_eq!(agents(&w).len(), 1);
}

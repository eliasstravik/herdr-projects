//! The Coordinator's start that fails, and stop: a failed start loses nothing (a restart asked for meanwhile still
//! happens, a long launch line never reaches the shell), stop leaves it stopped with words still waiting, and a stop asked
//! for while a message goes in holds back the ones still waiting.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;
use sw::coordinator::{look::look, pump::{pump, tick}, requests::{fresh, restart, send, stop}, state, who::pick_of};
use sw::herdr::cli::{Fake, Ran};
use sw::json::*;

fn coord_world_with(settings: Value) -> World {
    let mut s = json!({ "coordinator.agent": "claude", "review.when": "never" });
    for (k, v) in settings.as_object().cloned().unwrap_or_default() { s[k] = v; }
    let w = world_with(Opts { repo: true, settings: s, ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    unsafe { std::env::remove_var("HERDR_PROJECTS_HANDLED_MS") };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn no_jobs() { sw::runner::JOBS.with(|j| j.borrow_mut().clear()); }
fn agents(w: &World) -> Vec<Value> { w.state().arr("panes").iter().filter(|p| p.at("agent").is_object()).cloned().collect() }
fn get(w: &World) -> Obj { state::get(&w.socket).unwrap_or_default() }
fn tell(w: &World, text: &str) { send(&w.root, text, false, "", &w.socket).unwrap(); }
fn unstamp(name: &str) { let _ = std::fs::remove_file(sw::store::marks::stamp_file(name)); }
fn last_lines(w: &World) -> Vec<String> { agents(w)[0].at("agent").arr("prompts").iter().map(|p| text(p).rsplit('\n').next().unwrap_or("").to_string()).collect() }
/// The world's herdr, with one call answered (or something done first) by the test: `on` returns Some to answer itself.
struct Hooked { inner: Rc<common::herdr::FakeHerdr>, on: RefCell<Box<dyn FnMut(&[String]) -> Option<Ran>>> }
impl Fake for Hooked {
    fn run(&self, args: &[String], input: &str) -> Ran { let r = (self.on.borrow_mut())(args); r.unwrap_or_else(|| self.inner.run(args, input)) }
    fn kids(&self, pid: i64) -> Vec<i64> { self.inner.kids(pid) }
    fn wired_to(&self, pid: i64, group: &[i64]) -> Option<bool> { self.inner.wired_to(pid, group) }
    fn kill(&self, pid: i64) { self.inner.kill(pid) }
}
fn hook(w: &World, on: impl FnMut(&[String]) -> Option<Ran> + 'static) { sw::herdr::cli::set_fake(Some(Rc::new(Hooked { inner: w.fake.clone(), on: RefCell::new(Box::new(on)) }))); }
fn unhook(w: &World) { sw::herdr::cli::set_fake(Some(w.fake.clone())); }

#[test]
fn a_start_that_fails_loses_nothing_a_restart_asked_for_meanwhile_still_happens_and_a_long_launch_line_never_reaches_the_shell() {
    let w = coord_world_with(json!({ "coordinator.agent": "codex" }));
    let so = w.socket.clone();
    // The whole launch line stays well under what a terminal takes on one line: the rules ride on no flag.
    tell(&w, "hello"); drain();
    let agent = agents(&w)[0].at("agent").clone();
    let args: Vec<String> = agent.arr("args").iter().map(text).collect();
    assert!(format!("codex {}", args.join(" ")).len() < 2000, "the launch line is short");
    assert!(!args.iter().any(|a| a.contains("developer_instructions")));
    assert!(read(&state::home(&so).join("AGENTS.md")).contains("board add \"<short title"));
    assert_eq!(text(&agent.arr("prompts")[0]), "hello", "the rules are never put on what you say");
    tell(&w, "again"); drain();
    assert_eq!(text(&agents(&w)[0].at("agent").arr("prompts")[1]), "again");
    // A start that fails while you ask for a new one: the failure is said, and your request is still carried out.
    let so2 = so.clone();
    let mut failed = false;
    hook(&w, move |a| {
        if failed || a.first().map(|s| s.as_str()) != Some("agent") || a.get(1).map(|s| s.as_str()) != Some("start") { return None; }
        failed = true; fresh(pick_of("claude", "").unwrap(), &so2).unwrap();
        Some(Ran { status: 1, stdout: String::new(), stderr: "{\"error\":{\"code\":\"timeout\",\"message\":\"the agent did not become ready\"}}\n".into() })
    });
    restart(&so).unwrap();
    no_jobs(); pump(&so); // the restart's start fails; a "new" was asked for while it ran
    assert!(look(&so).error.contains("did not become ready"), "{}", look(&so).error);
    assert_eq!(get(&w).at("want").s("what"), "new", "the newer request survives the failure");
    assert!(get(&w).at("pane").is_null(), "the half-started pane is not reused");
    drain();
    unhook(&w);
    assert_eq!(agents(&w).len(), 1); assert_eq!(agents(&w)[0].at("agent").s("kind"), "claude"); assert!(!get(&w).b("want")); assert_eq!(look(&so).state, "idle");
    assert_eq!(look(&so).error, "", "the failure of the start it replaced is not shown over a Coordinator that is there");
    let mut s = open(&w.root, 34); assert!(!s.text().contains("could not be started")); assert!(regex::Regex::new(r"\n[ ▌]○ [▾▸] ").unwrap().is_match(&s.text()), "idle: its row's plain mark, {}", s.text());
    // A start that failed is not tried again every few seconds by itself; your next message tries.
    // Only pi is missing: claude stays installed as the world has it, never asked of this machine.
    let versions = sw::store::files::data_dir().join("versions.json");
    let mut known = sw::store::files::read_json(&versions).unwrap(); known["pi"] = json!({ "v": "", "at": sw::clock::now(), "found": false });
    sw::store::files::write_json(&versions, &known).unwrap();
    fresh(pick_of("pi", "").unwrap(), &so).unwrap(); tell(&w, "x"); drain();
    assert!(get(&w).s("error").contains("not installed"), "{}", get(&w).s("error"));
    unstamp("coordinator-default");
    tick(&so); assert!(sw::runner::JOBS.with(|j| j.borrow().is_empty()));
}

#[test]
fn stop_leaves_it_stopped_also_with_words_still_waiting_they_go_in_with_your_next_message() {
    let w = coord_world_with(json!({}));
    let so = w.socket.clone();
    sw::clock::set(Some(sw::clock::now()));
    tell(&w, "first"); drain();
    assert_eq!(agents(&w).len(), 1);
    // Words on their way when the stop is asked for: the stop does not start it again to deliver them.
    tell(&w, "waiting"); sw::clock::advance(5); stop(&so).unwrap(); drain();
    assert_eq!(agents(&w).len(), 0); assert_eq!(get(&w).arr("queue").len(), 1); assert_eq!(get(&w).at("stopped"), &json!(true));
    assert_eq!(look(&so).state, "exited"); assert_eq!(look(&so).waiting, vec!["waiting"]); // the drawer says stopped, not starting
    unstamp("coordinator-default"); tick(&so); drain(); pump(&so); // neither the tick nor a stray pump
    assert_eq!(agents(&w).len(), 0); assert_eq!(get(&w).arr("queue").len(), 1);
    // The next message starts it, where it left off, and nothing was lost.
    sw::clock::advance(5); tell(&w, "next"); drain();
    assert_eq!(get(&w).arr("queue").len(), 0); assert_eq!(get(&w).at("stopped"), &json!(false));
    assert_eq!(last_lines(&w), vec!["waiting", "next"]);
    // Said after the stop was asked for, before it was carried out: that does start it again.
    stop(&so).unwrap(); sw::clock::advance(5); tell(&w, "after"); drain();
    assert_eq!(agents(&w).len(), 1); assert_eq!(get(&w).arr("queue").len(), 0); assert_eq!(get(&w).at("stopped"), &json!(false));
}

#[test]
fn a_stop_asked_for_while_a_message_is_going_in_holds_back_the_ones_still_waiting() {
    let w = coord_world_with(json!({}));
    let so = w.socket.clone();
    sw::clock::set(Some(sw::clock::now()));
    tell(&w, "hello"); drain();
    // herdr as it is, except that the stop is asked for in the middle of the first of two deliveries.
    let asked = Rc::new(std::cell::Cell::new(false));
    let (a2, so2) = (asked.clone(), so.clone());
    hook(&w, move |a| { if !a2.get() && a.first().map(|s| s.as_str()) == Some("agent") && a.get(1).map(|s| s.as_str()) == Some("prompt") { a2.set(true); stop(&so2).unwrap(); } None });
    tell(&w, "one"); sw::clock::advance(5); tell(&w, "two");
    no_jobs(); pump(&so);
    assert!(asked.get());
    assert_eq!(agents(&w).len(), 0, "it is stopped");
    assert_eq!(get(&w).arr("queue").iter().map(|m| m.t("said")).collect::<Vec<_>>(), vec!["two"], "the second message was not sent ahead of the stop");
    assert_eq!(get(&w).at("stopped"), &json!(true)); assert_eq!(look(&so).state, "exited");
    no_jobs(); unstamp("coordinator-default"); tick(&so); drain(); assert_eq!(agents(&w).len(), 0);
    unhook(&w);
    sw::clock::advance(5); tell(&w, "three"); drain();
    assert_eq!(last_lines(&w), vec!["two", "three"]);
}

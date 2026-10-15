//! The Coordinator's restart and new: restart keeps the conversation, new starts a fresh one, one Coordinator can be
//! picked for a single use, the board itself neither restarts it nor starts a new one, and out of the box it is Claude
//! Code on its defaults whatever the working agent is.
use crate::common;
use common::board::open;
use common::*;
use serde_json::{json, Value};
use sw::coordinator::{look::look, requests::send, state, who::choice};
use sw::json::*;
use sw::store::settings::{change::{set_value, unset_value}, lookup::{setting, Scope}};

fn coord_world_with(settings: Value) -> World {
    let mut s = json!({ "coordinator.agent": "claude", "review.when": "never" });
    for (k, v) in settings.as_object().cloned().unwrap_or_default() { s[k] = v; }
    let w = world_with(Opts { repo: true, settings: s, ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn agents(w: &World) -> Vec<Value> { w.state().arr("panes").iter().filter(|p| p.at("agent").is_object()).cloned().collect() }
fn get(w: &World) -> Obj { state::get(&w.socket).unwrap_or_default() }
/// The command line, in this process (where the test's herdr answers): what a person reads. Then what it started runs.
fn cli(args: &[&str]) -> String {
    let a = sw::cli::flags::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let (_, said) = sw::cli::tasks::command(&a.pos[0], &a).unwrap_or_else(|e| panic!("{args:?}: {}", e.msg));
    drain();
    said
}
fn arg(w: &World, flag: &str) -> String { let a: Vec<String> = agents(w)[0].at("agent").arr("args").iter().map(text).collect(); a.iter().position(|x| x == flag).map(|i| a[i + 1].clone()).unwrap_or_default() }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }

#[test]
fn restart_keeps_the_conversation_new_starts_a_fresh_one_and_one_coordinator_can_be_picked_for_a_single_use() {
    let w = coord_world_with(json!({ "coordinator.agent": "claude/claude-haiku-5-5" }));
    w.script(json!({ "coordinator": [{ "said": "Hello." }] })); // it has said something: there is a conversation to come back to
    assert!(cli(&["coordinator", "tell", "hello"]).contains("sent to the Coordinator"));
    let first = get(&w);
    assert_eq!(arg(&w, "--session-id"), first.s("session")); assert_eq!(arg(&w, "--model"), "claude-haiku-5-5");
    let st = cli(&["coordinator"]);
    assert!(st.contains("Coordinator: idle · claude · claude-haiku-5-5 · default effort · default: claude · claude-haiku-5-5 · default effort"), "{st}");
    // Restart: a new terminal, the same conversation.
    assert!(cli(&["coordinator", "restart"]).contains("restarting, in the same conversation"));
    assert_ne!(get(&w).s("tab"), first.s("tab")); assert_eq!(w.state().arr("workspaces").len(), 1);
    assert_eq!(arg(&w, "--resume"), first.s("session")); assert_eq!(get(&w).s("session"), first.s("session"));
    // The program exiting by itself is the same: the next message starts it again where it left off.
    w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { if p.at("agent").is_object() { p["agent"] = Value::Null; } } });
    assert_eq!(look(&w.socket).state, "exited");
    send(&w.root, "still there?", false, "", &w.socket).unwrap(); drain();
    assert_eq!(arg(&w, "--resume"), first.s("session"));
    assert_eq!(agents(&w)[0].at("agent").arr("prompts").iter().map(|p| text(p).rsplit('\n').next().unwrap().to_string()).collect::<Vec<_>>(), vec!["still there?"]);
    // New, for one use: codex, its own model and effort. The default is untouched.
    assert!(cli(&["coordinator", "new", "--agent", "codex/gpt-x", "--effort", "high"]).contains("a new Coordinator is starting"));
    let agent = agents(&w)[0].at("agent").clone();
    let args: Vec<String> = agent.arr("args").iter().map(text).collect();
    assert_eq!(agent.s("kind"), "codex"); assert_eq!(args[args.len() - 4..].to_vec(), vec!["-m", "gpt-x", "-c", "model_reasoning_effort=\"high\""]);
    assert!(read(&state::home(&w.socket).join("AGENTS.md")).contains("You are the Coordinator")); // the other program finds the rules in the same folder
    assert_eq!(get(&w).at("run"), &json!({ "kind": "codex", "model": "gpt-x", "effort": "high", "oneOff": true })); assert!(get(&w).at("session").is_null());
    assert_eq!(setting("coordinator.agent", None), json!("claude/claude-haiku-5-5")); assert!(!look(&w.socket).stale);
    let st = cli(&["coordinator"]);
    assert!(st.contains("codex · gpt-x · high (for this one only) · default: claude · claude-haiku-5-5"), "{st}");
    // Restarting a one-off keeps it; a plain new one is the default again.
    cli(&["coordinator", "restart"]); assert_eq!(agents(&w)[0].at("agent").s("kind"), "codex");
    cli(&["coordinator", "new"]);
    assert_eq!(agents(&w)[0].at("agent").s("kind"), "claude"); assert_ne!(get(&w).s("session"), first.s("session")); assert!(get(&w).at("run").at("oneOff").is_null());
    // The default is a setting like the agent's; a running one says when it changed.
    cli(&["set", "coordinator.agent", "codex"]); // the model travels with the program: Claude's is not handed to Codex
    assert_eq!(choice().map(Value::Object), Some(json!({ "kind": "codex", "model": "", "effort": "" })));
    let mut s = open(&w.root, 34);
    assert!(look(&w.socket).stale);
    assert!(s.text().contains("your default is now codex · default model · default effort: herdr-projects coordinator new starts a new one on it"), "{}", s.text());
    // The board itself neither restarts it nor starts a new one: the old keys do nothing, and the menu has no line for either.
    s.press(&["alt+r", "alt+n"]); assert!(s.s.ask.is_none()); assert!(!get(&w).b("want"));
    s.press(&["?"]); assert!(!has(&s.text(), "(?i)restart the Coordinator|new Coordinator")); s.press(&["esc"]);
    s.typed("pi some-model"); s.press(&["alt+n", "alt+r", "alt+c"]);
    assert!(s.s.ask.is_none()); assert!(s.s.chat_open); assert!(!s.closed); assert!(s.s.value().contains("pi some-model")); assert!(!get(&w).b("want"));
    s.press(&["esc"]);
    cli(&["coordinator", "restart"]); assert_eq!(agents(&w)[0].at("agent").s("kind"), "claude");
    cli(&["coordinator", "new"]);
    assert_eq!(agents(&w)[0].at("agent").s("kind"), "codex"); assert!(!look(&w.socket).stale);
    // Stop closes it (its tab is gone, the home space stays); off turns the box back into adding tasks word for word.
    cli(&["coordinator", "stop"]); assert_eq!(agents(&w).len(), 0);
    assert_eq!(w.state().arr("tabs").iter().map(|x| x.t("label")).collect::<Vec<_>>(), vec!["1"]); assert_eq!(look(&w.socket).state, "exited");
    cli(&["set", "coordinator.agent", "none"]);
    assert!(choice().is_none());
}

#[test]
fn out_of_the_box_the_coordinator_is_claude_code_on_its_defaults_whatever_the_working_agent_is_set_to() {
    let w = coord_world_with(json!({ "worker.agent": "codex/gpt-5.5", "worker.effort": "high" }));
    assert_eq!(sw::store::settings_list::entry_of("coordinator.agent").unwrap().builtin, json!("claude"));
    assert_eq!(choice().map(Value::Object), Some(json!({ "kind": "claude", "model": "", "effort": "" })));
    unset_value("coordinator.agent", &Scope::you()).unwrap(); // nothing set is the default too, not the working agent
    assert_eq!(choice().map(Value::Object), Some(json!({ "kind": "claude", "model": "", "effort": "" })));
    send(&w.root, "hello", false, "", &w.socket).unwrap(); drain();
    assert_eq!(get(&w).at("run"), &json!({ "kind": "claude", "model": "", "effort": "" }));
    let a = agents(&w)[0].at("agent").clone();
    assert_eq!(a.s("kind"), "claude");
    assert!(!has(&a.at("args").to_string(), r"--model|--effort|gpt-5\.5"), "the agent's model and effort are not passed on");
    set_value("coordinator.agent", &json!("claude/claude-haiku-5-5"), &Scope::you()).unwrap();
    assert_eq!(choice().map(Value::Object), Some(json!({ "kind": "claude", "model": "claude-haiku-5-5", "effort": "" })));
}

/// A program whose conversation the board does not read (Kimi Code) is named its conversation by herdr's integration,
/// once its first message went in. A restart right then reopens it: the id is taken before its terminal goes.
#[test]
fn a_coordinator_herdr_names_the_conversation_of_restarts_in_it_also_right_after_its_first_message() {
    let w = coord_world_with(json!({ "coordinator.agent": "kimi" }));
    w.script(json!({ "coordinator": [{ "session": "kimi-real-session" }] }));
    cli(&["coordinator", "tell", "hello"]);
    assert!(get(&w).at("session").is_null(), "not known yet: herdr named it once the message went in");
    assert!(cli(&["coordinator", "restart"]).contains("restarting"));
    assert_eq!(get(&w).s("session"), "kimi-real-session");
    assert_eq!(arg(&w, "--session"), "kimi-real-session", "{:?}", agents(&w)[0].at("agent").at("args"));
    // A look at it learns it too, with no restart.
    drop(w);
    let w = coord_world_with(json!({ "coordinator.agent": "kimi" }));
    w.script(json!({ "coordinator": [{ "session": "kimi-other" }] }));
    cli(&["coordinator", "tell", "hello"]);
    sw::clock::advance(5000); // a look asks again at most every 3 seconds
    look(&w.socket);
    assert_eq!(get(&w).s("session"), "kimi-other");
}

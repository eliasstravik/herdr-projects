//! Command Code tasks through work, review, resumption, planning, coordinator chat and provider failures.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::json::*;

fn settings() -> Value { json!({ "worker.agent": "commandcode", "reviewer.agent": "commandcode", "review.when": "always" }) }
fn work() -> Value { json!({ "files": { "f.txt": "v1\n" }, "commit": "v1", "ends": "commandcode-finished-made", "hook": true }) }
fn hook_on() { unsafe { std::env::remove_var("HERDR_PROJECTS_NO_HOOK") }; }

#[test]
fn commandcode_works_and_reviews_with_transcript_tokens_and_reopens_its_own_session() {
    let w = world_with(Opts { repo: true, settings: settings(), ..Default::default() }); hook_on();
    w.script(json!({ "worker": [work(), { "status": "working" }], "reviewer": [{ "verdict": { "meets": true, "findings": [], "text": "Good." }, "ends": "commandcode-finished-made", "hook": true }] }));
    let id = w.add("Command Code task"); delegate(&id); let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(t.s("said"), "Implemented the change.");
    assert!(t.b("session") && t.b("transcript") && t.b("hooked"), "{t:?}");
    let file = std::path::Path::new(t.s("transcript"));
    assert_eq!(sw::agents::read_tokens::tokens("commandcode", file, &Value::Null).unwrap().f("n"), 45.0);
    assert!(sw::agents::talk::talk_of("commandcode", Some(file), t.s("session"), std::path::Path::new(t.s("worktree")), 10).unwrap().iter().any(|s| s.text == "Implemented the change."));
    let pane = t.at("link").t("pane");
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"] = Value::Null; } } });
    sw::actions::talk::tell(&id, "Continue in this conversation.", false).unwrap(); settle(&id);
    let a = agent_of(&w, &pane);
    let args: Vec<String> = a.arr("args").iter().map(text).collect();
    assert_eq!(args.iter().position(|a| a == "--resume").map(|i| args[i + 1].as_str()), Some(t.s("session")));
}

#[test]
fn commandcode_without_its_callback_still_finishes_from_herdr_and_its_native_transcript() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "commandcode", "review.when": "never" }), ..Default::default() });
    let mut step = work(); step["hook"] = json!(false); w.script(json!({ "worker": [step] }));
    let id = w.add("No callback"); delegate(&id); let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(t.s("said"), "Implemented the change.");
}

#[test]
fn commandcode_can_plan_in_the_read_only_mount_and_build_after_the_humans_approval() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "commandcode", "plan.approve": "you", "review.when": "never" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap"); }
    w.patch(|s| s["pids"] = json!(true));
    w.script(json!({ "worker": [{ "plan": "# Plan\n\n1. Add f.txt.\n", "note": "Plan written." }, { "status": "working" }] }));
    let id = w.add("Plan then build"); delegate(&id); let t = settle(&id);
    assert_eq!(t.s("stage"), "plan", "{} {}", t.s("msg"), t.at("failure"));
    assert!(agent_of(&w, t.at("link").s("pane")).b("boxed"));
    sw::actions::plan::approve_plan(&id).unwrap(); let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{} {}", t.s("msg"), t.at("failure"));
    assert!(!agent_of(&w, t.at("link").s("pane")).b("boxed"));
}

#[test]
fn commandcode_coordinator_reads_and_resumes_its_conversation() {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "commandcode" }), ..Default::default() });
    w.patch(|s| s["pids"] = json!(true));
    let home = w.base.join("coord-home"); std::fs::create_dir(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home); }
    w.script(json!({ "coordinator": [{ "ends": "commandcode-finished-made" }] }));
    sw::coordinator::requests::send(&w.root, "Hello", false, "", &w.socket).unwrap();
    sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); });
    let _ = std::fs::remove_file(sw::store::marks::stamp_file("coordinator-session-default"));
    let look = sw::coordinator::look::look(&w.socket); assert_eq!(look.said, "Implemented the change.");
    let c = sw::coordinator::state::get(&w.socket).unwrap(); assert!(c.b("session"));
    sw::coordinator::requests::restart(&w.socket).unwrap();
    sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); });
    assert!(calls(&w).iter().any(|c2| c2.starts_with("pane run") && c2.contains("--resume") && c2.contains(c.s("session"))));
}

#[test]
fn commandcode_an_error_before_a_transcript_uses_the_worker_fallback() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "commandcode", "worker.fallback": "codex", "review.when": "never" }), ..Default::default() }); hook_on();
    w.patch(|s| s["pids"] = json!(true));
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }] }));
    let id = w.add("Provider failed"); delegate(&id); let t = settle(&id);
    let tokens: Value = serde_json::from_str(&read(&sw::store::files::dir_of(&id).join("tokens.json"))).unwrap();
    sw::turn::turn_ended(&id, "agent", tokens.s("agent"), 0, &[json!({ "session_id": "failed-session", "error": "Login required", "code": "unauthorized", "status": 401 }).to_string()]);
    // The callback can arrive before herdr's idle report, as on every native agent.
    set_status(&w, t.at("link").s("pane"), "idle");
    let t = settle(&id); assert_eq!(t.s("agent"), "codex", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(sw::store::handovers::handovers(&t).len(), 1);
}

#[test]
fn commandcode_a_question_to_the_human_does_not_start_checks_or_review() {
    let w = world_with(Opts { repo: true, settings: settings(), ..Default::default() }); hook_on();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Needs an answer"); delegate(&id); let t = settle(&id);
    let began = t.i("armedAt") + 1; sw::clock::set(Some(began + 10));
    sw::actions::filing::report(&id, sw::actions::filing::Report { state: "blocked".into(), kind: "question".into(), msg: "Which format should I use?".into(), as_agent: true, ..Default::default() }).unwrap();
    let file = w.base.join("question.jsonl");
    let lines = [json!({ "type": "message", "id": "u", "parentId": null, "timestamp": sw::clock::iso(began), "message": { "role": "user", "content": [{ "type": "text", "text": "Do the work" }] } }), json!({ "type": "message", "id": "a", "parentId": "u", "timestamp": sw::clock::iso(began + 20), "message": { "role": "assistant", "content": [{ "type": "text", "text": "Which format should I use?" }] } })];
    std::fs::write(&file, lines.iter().map(|e| format!("{e}\n")).collect::<String>()).unwrap();
    let token: Value = serde_json::from_str(&read(&sw::store::files::dir_of(&id).join("tokens.json"))).unwrap();
    sw::clock::set(Some(began + 30));
    sw::turn::turn_ended(&id, "agent", token.s("agent"), 0, &[json!({ "session_id": "question", "transcript_path": file, "stop": "end_turn" }).to_string()]);
    set_status(&w, t.at("link").s("pane"), "idle");
    let t = settle(&id); assert_eq!(t.s("stage"), "asks", "{} {}", t.s("msg"), t.at("failure"));
    assert!(!t.b("reviewRound") && !t.b("check"));
}

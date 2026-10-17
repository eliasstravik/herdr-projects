//! Account profiles: repo defaults, per-task delegation, pinned readers, trust, hooks, unsets and independent threads.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::agents::{account, adapter, homes, profiles};
use sw::json::*;
use sw::store::settings::{change::set_value, lookup::{setting, Scope}};

fn definitions() -> Value { json!({
    "main": {"env": {"CLAUDE_CONFIG_DIR": null}},
    "channels": {"env": {"CLAUDE_CONFIG_DIR": "${HOME}/.claude-channels", "PROFILE_TEST": "it's $(touch forbidden) `nope`"}},
    "code": {"env": {"CODEX_HOME": "~/.codex-extra"}}
}) }
fn save(v: Value) { std::fs::write(sw::store::files::config_dir().join("profiles.json"), v.to_string()).unwrap(); }
fn set(n: &str, v: &str, s: &Scope) { set_value(n, &json!(v), s).unwrap(); }
fn transcript(home: &std::path::Path, session: &str, text: &str) -> std::path::PathBuf {
    let f = home.join("projects/fake").join(format!("{session}.jsonl"));
    std::fs::create_dir_all(f.parent().unwrap()).unwrap();
    std::fs::write(&f, format!("{}\n", json!({"type":"assistant", "timestamp":"2026-10-17T12:00:00Z", "message":{"id":"message", "role":"assistant", "content":[{"type":"text","text":text}], "stop_reason":"end_turn"}}))).unwrap(); f
}

#[test]
fn account_profiles_repo_default_task_override_and_none_unsets_every_managed_variable() {
    let w = world(); save(definitions());
    let id = w.add("Account choice");
    set("worker.profile", "main", &Scope::you());
    set("worker.profile", "channels", &Scope::repo("proj"));
    assert_eq!(setting("worker.profile", Some(&task(&id))), json!("channels"));
    set("worker.profile", "main", &Scope::task(&task(&id)));
    let main = account::configured(Some(&task(&id)), "worker");
    assert!(main.at("CLAUDE_CONFIG_DIR").is_null());
    assert!(main.at("PROFILE_TEST").is_null());
    assert!(main.at("CODEX_HOME").is_null());
    let alt = profiles::resolve("channels").unwrap();
    assert_eq!(alt.s("CLAUDE_CONFIG_DIR"), format!("{}/.claude-channels", sw::store::files::user_home().display()));
    assert_eq!(profiles::resolve("none").unwrap().at("PROFILE_TEST"), &Value::Null);
    assert!(set_value("worker.profile", &json!("missing"), &Scope::task(&task(&id))).is_err());
    assert!(set_value("watcher.profile", &json!("main"), &Scope::repo("proj")).is_err());
}

#[test]
fn account_profiles_read_the_selected_home_and_cache_without_cross_account_mixups() {
    let w = world(); save(definitions()); let id = w.add("Same UUID");
    let alt = sw::store::files::user_home().join(".claude-channels");
    let a = transcript(&alt, "same-session", "Channels answer");
    let normal = sw::store::files::user_home().join(".claude");
    let b = transcript(&normal, "same-session", "Main answer");
    let mut t = task(&id); t.put("agent", "claude");t.put("session", "same-session");
    for (name, wanted, path) in [("channels", "Channels answer", a), ("main", "Main answer", b)] {
        t.obj_mut("agentEnvs").put("agent", profiles::resolve(name).unwrap());
        let _scope = account::task(&t, "agent");
        assert_eq!(adapter::transcript("claude", "same-session", &w.root), Some(path));
        let list = sw::flow::talk::task_talk(&t, 10, 1000);
        assert_eq!(list.arr("list").last().unwrap().s("text"), wanted);
        assert!(sw::turn::why_stopped(&t, "agent", None).turn.unwrap().said.contains(wanted));
    }
}

#[test]
fn account_profiles_scope_is_thread_local_nested_and_unwind_safe() {
    let _w = world();save(definitions());
    let process_env = std::env::var("CLAUDE_CONFIG_DIR").unwrap();
    let alt = profiles::resolve("channels").unwrap();
    let _scope = account::enter(&alt);
    let expected = homes::claude_home();
    assert_eq!(std::env::var("CLAUDE_CONFIG_DIR").unwrap(), process_env);
    assert_ne!(std::thread::spawn(homes::claude_home).join().unwrap(), expected);
    let _ = std::panic::catch_unwind(|| { let _inner = account::enter(&json!({"CLAUDE_CONFIG_DIR":null})); assert_ne!(homes::claude_home(), expected); panic!("unwind the read"); });
    assert_eq!(homes::claude_home(), expected);
}

#[test]
fn account_profiles_trust_and_launch_hook_stay_in_the_selected_account() {
    let w = world();save(definitions());
    let e = profiles::resolve("channels").unwrap(); let _scope = account::enter(&e);
    std::fs::create_dir_all(homes::claude_home()).unwrap();
    assert!(adapter::trust("claude", &w.root));
    assert_eq!(homes::claude_file(), homes::claude_home().join(".claude.json"));
    assert!(!sw::store::files::user_home().join(".claude.json").exists());
    let id=w.add("Hook");let dir=sw::store::files::dir_of(&id);
    let h=adapter::Hook{shim:"/fake/herdr-projects".into(),id,role:"agent".into(),token:"token".into(),round:0,dir:dir.clone(),plan:false};
    assert_eq!(adapter::hook_args("claude",&h).unwrap().unwrap()[0], "--settings");
    assert!(read(&dir.join("hook-agent.json")).contains("turn-ended"));
}

#[test]
fn account_profiles_task_delegation_pins_env_and_exports_before_canonical_start() {
    let w = world();save(definitions());let id=w.add("Reinforcement");
    delegate_with(&id,json!({"worker.profile":"channels"}));
    let t=settle(&id);
    let env=t.at("agentEnvs").at("agent").clone();
    assert!(env.s("CLAUDE_CONFIG_DIR").ends_with("/.claude-channels"), "{t:?}");
    let calls=calls(&w);
    let export=calls.iter().position(|s|s.starts_with("pane run") && s.contains("export CLAUDE_CONFIG_DIR=")).unwrap();
    let start=calls.iter().position(|s|s.starts_with("agent start")).unwrap();
    assert!(export<start);assert!(calls[start].contains("--kind claude"));
    assert!(calls[export..start].iter().any(|s|s.starts_with("pane wait-output") && s.contains("--regex ^hp-profile-")));
    set("worker.profile","main",&Scope::repo("proj"));
    assert_eq!(account::selected(&task(&id),"agent"),env);
    assert!(!w.root.join("forbidden").exists());
}

#[test]
fn account_profiles_bad_definitions_are_refused_and_shell_values_are_literal() {
    let _w=world();save(definitions());
    let env=profiles::resolve("channels").unwrap();
    let line=profiles::shell_line(env.as_object().unwrap()).unwrap();
    assert!(line.contains("unset CODEX_HOME"));assert!(line.contains("$(touch forbidden)"));assert!(line.contains("'\\''"));
    for name in ["PATH","HOME","SHELL","HERDR_SOCKET_PATH","LD_PRELOAD","bad","9BAD"] { assert!(profiles::check_env(name,&json!("x")).is_err()); }
    for value in [json!(4),json!("bad\nline"),json!("back\\slash")] { assert!(profiles::check_env("SAFE",&value).is_err()); }
    save(json!({"bad":{"env":{"CLAUDE_CONFIG_DIR":"relative"}}}));assert!(profiles::resolve("bad").is_err());
    save(json!({"bad":{"env":{"PATH":"/unsafe"}}}));assert!(profiles::load().is_err());
}

#[test]
fn account_profiles_watcher_child_gets_unsets_without_modifying_the_board_environment() {
    let _w=world();save(definitions());
    let _scope=account::enter(&profiles::resolve("main").unwrap());
    let mut child=std::process::Command::new("not-executed");account::command(&mut child);
    let env:Vec<_>=child.get_envs().collect();
    assert!(env.iter().any(|(k,v)|*k=="CLAUDE_CONFIG_DIR" && v.is_none()));
    assert!(std::env::var("CLAUDE_CONFIG_DIR").is_ok());
}

#[test]
fn account_profiles_reviewer_keeps_its_own_account_instead_of_the_worker_override() {
    let w=world_with(Opts{settings:json!({"review.when":"always","reviewer.agent":"claude"}),..Default::default()});
    save(definitions());set("worker.profile","channels",&Scope::you());set("reviewer.profile","main",&Scope::you());
    w.script(json!({"worker":[{"note":"Work completed."}],"reviewer":[{"verdict":{"meets":true,"findings":[],"text":"Passed."}}]}));
    let id=w.add("Independent reviewer");delegate(&id);let t=settle(&id);
    assert!(t.at("agentEnvs").at("agent").s("CLAUDE_CONFIG_DIR").ends_with("channels"));
    assert!(t.at("agentEnvs").at("reviewer").at("CLAUDE_CONFIG_DIR").is_null());
    assert!(calls(&w).iter().any(|s|s.starts_with("pane run") && s.contains("unset CLAUDE_CONFIG_DIR")));
}

#[test]
fn account_profiles_coordinator_choice_carries_the_same_resolved_environment_as_its_readers() {
    let _w=world_with(Opts{settings:json!({"coordinator.agent":"claude"}),..Default::default()});save(definitions());
    set("coordinator.profile","channels",&Scope::you());
    let choice=sw::coordinator::who::choice().unwrap();
    assert!(choice.at("profile").s("CLAUDE_CONFIG_DIR").ends_with("channels"));
    let pick=sw::coordinator::who::pick_of("claude/sonnet","").unwrap().unwrap();
    assert_eq!(pick.at("profile"),choice.at("profile"));
    set("coordinator.profile","main",&Scope::you());
    assert!(!sw::coordinator::who::same_choice(&choice,&sw::coordinator::who::choice().unwrap()));
}

#[test]
fn account_profiles_past_token_debt_uses_the_conversations_original_environment() {
    let w=world();save(definitions());let id=w.add("Tokens");
    let home=sw::store::files::user_home().join(".claude-channels");
    let file=transcript(&home,"past-session","Old answer");
    let mut row:Value=serde_json::from_str(read(&file).trim()).unwrap();
    row["message"]["usage"]=json!({"input_tokens":5,"output_tokens":3});std::fs::write(&file,format!("{row}\n")).unwrap();
    let mut t=task(&id);t.put("agent","claude");t.put("session","past-session");t.obj_mut("agentEnvs").put("agent",profiles::resolve("channels").unwrap());
    let counted=sw::limits::spent::counted(&t,"agent").unwrap();assert_eq!(counted.f("total"),8.0);
    let debt=json!({"account":profiles::resolve("channels").unwrap(),"kind":"claude","session":"past-session","file":"","cwd":w.root_str()});
    t.put("spent",json!({"owed":[debt]}));t.put("session",Value::Null);t.obj_mut("agentEnvs").put("agent",profiles::resolve("main").unwrap());
    assert_eq!(sw::limits::spent::counted(&t,"agent").unwrap().f("total"),8.0);
}

#[test]
fn account_profiles_missing_after_selection_refuses_a_new_launch_instead_of_using_the_primary_account() {
    let w=world();save(definitions());let id=w.add("Removed account");
    set("worker.profile","channels",&Scope::task(&task(&id)));
    save(json!({"main":{"env":{"CLAUDE_CONFIG_DIR":null}}}));
    assert!(account::for_start(Some(&task(&id)),"worker").is_err());
}

#[test]
fn account_profiles_definition_edits_invalidate_cached_setting_validation() {
    let _w=world();save(definitions());set("worker.profile","channels",&Scope::you());
    assert!(account::for_start(None,"worker").is_ok());
    save(json!({"main":{"env":{"CLAUDE_CONFIG_DIR":null}}}));
    assert!(account::for_start(None,"worker").is_err());
    save(definitions());assert!(account::for_start(None,"worker").is_ok());
}

#[test]
fn account_profiles_coordinator_launch_exports_its_pinned_environment() {
    let w=world_with(Opts{settings:json!({"coordinator.agent":"claude"}),..Default::default()});save(definitions());set("coordinator.profile","channels",&Scope::you());
    let socket=sw::herdr::cli::current_socket();
    w.script(json!({"coordinator":[{"said":"Ready."}]}));
    sw::coordinator::requests::send(&w.root,"Prepare a task",false,"",socket.as_str()).unwrap();
    sw::runner::drain_jobs(&|job,id,extra|{sw::jobs::run_job(job,id,extra);flush();});
    let state=sw::coordinator::state::get(socket.as_str()).unwrap();
    assert!(state.at("run").at("profile").s("CLAUDE_CONFIG_DIR").ends_with("channels"));
    assert!(calls(&w).iter().any(|s|s.starts_with("pane run") && s.contains("export CLAUDE_CONFIG_DIR=")));
}

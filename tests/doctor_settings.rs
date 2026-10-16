//! `herdr-projects doctor` on your settings (HP-32): the programs they name, each once with the settings that name it;
//! a model its program does not take, written in the file by hand (a value read from a file is not checked when it is
//! read), and an effort the program does not have, each a problem with what to do.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::doctor::{agents, settings, Found};
use sw::json::JsMut;
use sw::store::tasks::{update, Change};

/// Your settings file, as somebody wrote it by hand: never checked on the way in.
fn hand_written(extra: Value) {
    let f = sw::store::files::config_dir().join("config.json");
    let mut s: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
    for (k, v) in extra.as_object().unwrap() { s[k] = v.clone(); }
    std::fs::write(f, s.to_string()).unwrap();
}
fn codex_list() {
    let now = sw::clock::now();
    std::fs::write(sw::store::files::data_dir().join("models.json"), json!({ "codex": { "at": now, "taken": ["gpt-6.1-sol", "gpt-6-luna"], "shown": ["gpt-6.1-sol", "gpt-6-luna"] } }).to_string()).unwrap();
}

#[test]
fn each_program_the_settings_name_is_asked_about_once() {
    let _w = world();
    hand_written(json!({ "worker.agent": "codex/gpt-6.1-sol", "reviewer.agent": "claude", "worker.fallback": ["pi", "claude/opus"], "coordinator.agent": "none" }));
    let named = agents::named();
    let by = |k: &str| named.iter().find(|(x, _)| x == k).map(|(_, b)| b.clone()).unwrap_or_default();
    assert_eq!(by("codex"), ["worker.agent"]);
    assert_eq!(by("pi"), ["worker.fallback"]);
    assert!(by("claude").contains(&"reviewer.agent".to_string()) && by("claude").contains(&"worker.fallback".to_string()), "{named:?}");
    assert_eq!(named.iter().filter(|(k, _)| k == "claude").count(), 1);
    assert!(!named.iter().any(|(k, _)| k == "none" || k.is_empty()));
}

#[test]
fn valid_settings_are_said_in_one_line() {
    let _w = world();
    codex_list();
    hand_written(json!({ "worker.agent": "codex/gpt-6.1-sol", "worker.effort": "high" }));
    assert_eq!(settings::check(), [Found::ok("The agent, model and effort settings are valid")]);
}

#[test]
fn a_model_or_an_effort_the_program_does_not_take_is_a_problem() {
    let _w = world();
    codex_list();
    hand_written(json!({ "worker.agent": "codex/gpt-5.5" }));
    let f = settings::check();
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].problem && f[0].what.starts_with("worker.agent: codex has no model \"gpt-5.5\""), "{:?}", f[0]);
    assert_eq!(f[0].fix, "Set it again: herdr-projects set worker.agent <program/model>  (or herdr-projects unset worker.agent)");
    // An effort the program has not: it does not count, and the value below it is used.
    hand_written(json!({ "worker.agent": "claude", "worker.effort": "minimal" }));
    let f = settings::check();
    assert!(f.iter().any(|f| f.problem && f.what.contains("minimal")), "{f:?}");
    // A program that cannot do the role.
    hand_written(json!({ "worker.effort": "default", "reviewer.agent": "mastracode" }));
    let f = settings::check();
    assert!(f.iter().any(|f| f.problem && f.what.contains("reviewer.agent: mastracode cannot review")), "{f:?}");
}

#[test]
fn a_repo_s_entry_and_a_task_s_own_settings_are_checked_too() {
    let w = world();
    codex_list();
    let id = w.add("one");
    // The reviewer's case: your entry for the repo names a model codex does not take, and an effort it has not.
    hand_written(json!({ "repos": { "proj": { "worker.agent": "codex/this-model-does-not-exist", "worker.effort": "banana" } } }));
    let f = settings::check();
    let what: Vec<&str> = f.iter().filter(|f| f.problem).map(|f| f.what.as_str()).collect();
    assert!(what.iter().any(|w| w.starts_with("worker.agent for proj: codex has no model \"this-model-does-not-exist\"")), "{f:?}");
    assert!(what.iter().any(|w| w.starts_with("your entry for proj: worker.effort:") && w.contains("banana")), "{f:?}");
    assert!(f.iter().any(|f| f.fix.ends_with("--repo proj  (or herdr-projects unset worker.agent --repo proj)")), "{f:?}");
    // Said once, not again for the task in that repo that only inherits it.
    assert_eq!(what.iter().filter(|w| w.contains("this-model-does-not-exist")).count(), 1, "{f:?}");
    let named = agents::named();
    assert!(named.iter().any(|(k, by)| k == "codex" && by.contains(&"worker.agent for proj".to_string())), "{named:?}");
    // A task's own setting.
    hand_written(json!({ "repos": {} }));
    update(&id, |t| { t.put("settings", json!({ "worker.agent": "codex/gpt-0" })); Ok(Change::Write) }).unwrap();
    let f = settings::check();
    let k = short(&id);
    assert!(f.iter().any(|f| f.problem && f.what.starts_with(&format!("worker.agent of {k}: codex has no model \"gpt-0\"")) && f.fix.contains(&format!("--task {k}"))), "{f:?}");
}

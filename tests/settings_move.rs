//! The one-time move from the old settings: your config.json, each task's own settings and what it took from its repo's
//! file become the new names, once; the old file is kept beside the new one; tasks under way keep their choices.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::json::*;
use sw::store::settings::lookup::{lookup, setting, Scope};
use sw::store::settings::migrate::{move_settings, moved_config, moved_task};
use sw::store::settings::places::yours;

fn file() -> std::path::PathBuf { sw::store::files::config_dir().join("config.json") }
fn kept() -> std::path::PathBuf { sw::store::files::config_dir().join("config.old.json") }
fn json_of(f: &std::path::Path) -> Value { serde_json::from_str(&read(f)).unwrap() }
fn task_file(id: &str) -> std::path::PathBuf { sw::store::files::dir_of(id).join("task.json") }
fn o(v: Value) -> Obj { v.as_object().cloned().unwrap() }
/// This computer's settings on the day of the move, and its repo's own file.
fn old() -> Value { json!({ "max": 10, "rounds": 5, "gate": "critical", "reviewerFast": true, "autoMerge": true, "coordinator": "claude" }) }
fn old_repo() -> Value { json!({ "setup": "npm ci --no-audit --no-fund", "check": "npm run check -- --all", "land": "ask", "teardown": "", "include": [] }) }
/// A task as the old version saved it.
fn as_old(id: &str, patch: Value) { let mut t = o(json_of(&task_file(id))); t.del("settings"); t.put("own", json!({})); for (k, v) in o(patch) { t.insert(k, v); } std::fs::write(task_file(id), Value::Object(t).to_string()).unwrap(); }

#[test]
fn the_move_with_exactly_this_computers_file_every_old_name_becomes_its_new_one_once_and_the_old_file_is_kept() {
    let w = world();
    // tasks as the old version saved them: one under way with choices of its own, one untouched
    let (a, b, c) = (w.add("Under way"), w.add("Untouched"), w.add("Research"));
    std::fs::write(file(), serde_json::to_string_pretty(&old()).unwrap()).unwrap();
    as_old(&a, json!({ "stage": "approval", "startedOnce": true, "agent": "claude", "repoCfg": old_repo(), "own": { "reviewer": "claude", "reviewerModel": "opus", "model": "sonnet", "effort": "high", "gate": "minor", "autoMerge": false, "conflictAuto": true, "watcher": "off" }, "base": "head" }));
    as_old(&b, json!({})); as_old(&c, json!({ "own": { "research": true, "reviewer": "off" }, "worktreeMode": "none" }));
    assert!(move_settings());
    assert_eq!(json_of(&file()), json!({ "review": { "mustFix": "critical", "rounds": 5 }, "reviewer": { "fast": true }, "coordinator": { "agent": "claude" }, "merge": { "by": "itself" }, "board": { "agents": 10 } }));
    assert_eq!(json_of(&kept()), old()); // a copy of the old file, beside the new one
    assert!(sw::store::settings::show::problems(None).is_empty());
    for (name, value) in [("board.agents", json!(10)), ("review.rounds", json!(5)), ("review.mustFix", json!("critical")), ("reviewer.fast", json!(true)), ("merge.by", json!("itself")), ("coordinator.agent", json!("claude"))] {
        let f = lookup(name, &Scope::you(), &yours()).unwrap(); assert_eq!((f.value, f.from.as_str()), (value, "you"), "{name}");
    }
    // the task under way keeps the choices it had, in the new names; the old ones are gone from it
    let t = task(&a);
    assert_eq!(t.at("settings"), &json!({ "worker.agent": "claude/sonnet", "worker.effort": "high", "reviewer.agent": "claude/opus", "review.mustFix": "minor", "merge.by": "you", "workspace.from": "here" }));
    assert_eq!(t.at("repoFile"), &json!({ "workspace.setup": "npm ci --no-audit --no-fund", "check.command": "npm run check -- --all" }));
    for k in ["own", "repoCfg", "base", "worktreeMode"] { assert!(!t.contains_key(k), "{k}"); }
    assert_eq!(setting("check.command", Some(&t)), json!("npm run check -- --all")); assert_eq!(setting("merge.by", Some(&t)), json!("you")); assert_eq!(setting("review.rounds", Some(&t)), json!(5));
    assert_eq!(task(&b).at("settings"), &json!({})); assert!(!task(&b).contains_key("own"));
    assert!(task(&c).at("research").is_null()); assert_eq!(task(&c).at("settings"), &json!({ "review.when": "never", "research.only": true })); // the old switch is the setting now (store/old_research.rs)
    // once: a second run changes nothing, also after you changed something since
    sw::store::settings::change::set_value("board.agents", &json!(3), &Scope::you()).unwrap();
    let (now, held) = (read(&file()), read(&task_file(&a)));
    assert!(!move_settings()); assert_eq!(read(&file()), now); assert_eq!(read(&task_file(&a)), held); assert_eq!(json_of(&kept()), old());
}

#[test]
fn the_move_the_other_old_names_a_repo_that_merged_by_itself_and_a_new_install_with_nothing_to_move() {
    let w = world();
    let moved = |v: Value| moved_config(&v).map(Value::Object);
    assert_eq!(moved(json!({ "agent": "codex", "model": "gpt-5.5", "effort": "high", "fast": "on", "reviewer": "off", "coordinator": "off", "coordinatorModel": "x", "helper": "codex", "helperModel": "mini", "watcherEvery": 120, "review": "all", "gate": "off", "rounds": 2,
        "artifacts": false, "worktree": "none", "base": "head", "pr": false, "merge": "rebase", "cleanup": false, "machines": "vm-a, me@vm-b", "remoteUser": "exedev", "theme": "dark", "ownTerminal": false, "serve": "local", "remerge": "review", "skill": false, "trust": false, "agentArgs": {}, "bell": true })),
        Some(json!({ "worker": { "agent": "codex/gpt-5.5", "effort": "high", "fast": true }, "review": { "when": "never", "rounds": 0, "evidence": false }, "coordinator": { "agent": "none" }, "watcher": { "every": "120s" },
        "workspace": { "from": "here", "cleanup": false }, "merge": { "pr": false, "method": "rebase" }, "board": { "machines": "exedev@vm-a, me@vm-b", "theme": "dark" } })));
    assert_eq!(moved(json!({ "reviewAuto": false, "watcher": "off", "autoMerge": false, "machines": "off" })), Some(json!({ "watcher": { "agent": "none" }, "review": { "rounds": 0 }, "merge": { "by": "you" }, "board": { "machines": "none" } })));
    assert_eq!(moved(json!({ "review": { "rounds": 5 }, "board": { "agents": 2 } })), None); assert_eq!(moved(json!({})), None); assert_eq!(moved(Value::Null), None); // already new, or nothing
    // a repo whose old file said "land": "auto" merged by itself where you had chosen nothing: a task that ran on that keeps it
    let ran = moved_task(&o(json!({ "id": "p/1", "startedOnce": true, "own": {}, "repoCfg": { "check": "t", "land": "auto" } })), &Obj::new()).unwrap();
    assert_eq!((ran.at("settings"), ran.at("repoFile")), (&json!({ "merge.by": "itself" }), &json!({ "check.command": "t" })));
    assert_eq!(moved_task(&o(json!({ "id": "p/2", "startedOnce": true, "own": {}, "repoCfg": { "land": "auto" } })), &o(json!({ "autoMerge": false }))).unwrap().at("settings"), &json!({})); // you had said no
    assert_eq!(moved_task(&o(json!({ "id": "p/3", "own": {}, "repoCfg": { "land": "auto" } })), &Obj::new()).unwrap().at("settings"), &json!({})); // never started: nothing ran on it
    assert!(moved_task(&o(json!({ "id": "p/4", "settings": {} })), &Obj::new()).is_none());
    // a new install: nothing to move, no copy made, nothing written anywhere
    let before = read(&file());
    assert!(!move_settings()); assert!(!kept().exists()); assert_eq!(read(&file()), before); assert!(!sw::store::files::data_dir().join("settings-moved").exists());
    // a task an older version saves after the move reads in the new names, and is saved so at its next change
    let id = w.add("Late");
    let mut raw = o(json_of(&task_file(&id))); raw.del("settings"); raw.put("own", json!({ "rounds": 2 })); raw.put("repoCfg", json!({ "check": "npm test" }));
    std::fs::write(task_file(&id), Value::Object(raw).to_string()).unwrap();
    assert_eq!(task(&id).at("settings"), &json!({ "review.rounds": 2 })); assert_eq!(setting("check.command", Some(&task(&id))), json!("npm test")); assert!(json_of(&task_file(&id)).as_object().unwrap().contains_key("own"));
    sw::store::tasks::set(&id, json!({ "msg": "x" })).unwrap();
    assert!(!json_of(&task_file(&id)).as_object().unwrap().contains_key("own")); assert_eq!(json_of(&task_file(&id)).at("repoFile"), &json!({ "check.command": "npm test" }));
    // asking for help converts nothing
    std::fs::write(file(), old().to_string()).unwrap();
    sw::cli::main(vec!["help".into()]); sw::cli::main(vec!["--skill".into()]);
    assert_eq!(json_of(&file()), old()); assert!(!kept().exists());
}

#[test]
fn a_task_under_way_when_the_move_happens_keeps_working_its_check_its_review_bar_and_its_merge_are_what_they_were() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "check.command": "test -f f.txt" })), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "v1", "note": "Did it." }], "reviewer": [{ "verdict": { "meets": true, "text": "Fine.", "findings": [{ "severity": "minor", "subject": "A name", "where": "f.txt", "proof": "read it" }] } }] }));
    let id = w.add("Under way");
    delegate(&id);
    // the old version's shape, while its agent works: your old file, and the task as it was saved
    let mut cfg = o(old()); cfg.put("autoMerge", false); cfg.put("watcher", "off"); cfg.put("coordinator", "off");
    std::fs::write(file(), Value::Object(cfg).to_string()).unwrap();
    let mut t = o(json_of(&task_file(&id))); for k in ["settings", "repoFile", "repoRefused"] { t.del(k); }
    t.put("own", json!({ "gate": "trivial", "rounds": 0 })); t.put("repoCfg", json!({ "setup": "", "check": "test -f f.txt", "land": "ask", "teardown": "", "include": [] }));
    std::fs::write(task_file(&id), Value::Object(t).to_string()).unwrap();
    assert!(move_settings());
    let done = settle(&id);
    assert_eq!(done.s("stage"), "approval", "{}", done.at("failure")); assert_eq!(done.at("review").s("gate"), "trivial"); assert_eq!(done.at("review").at("pass"), &json!(false)); // its own bar, and nothing sent back by itself
    assert_eq!(done.i("reviewRounds"), 0); assert_eq!(done.at("settings"), &json!({ "review.rounds": 0, "review.mustFix": "trivial" }));
}

#[test]
fn a_test_run_has_a_home_folder_of_its_own_nothing_here_reads_or_converts_your_real_one() {
    let _w = world_with(Opts::default());
    // The account's own home, as the system has it (the world gave this process another HOME).
    let real = unsafe { let pw = libc::getpwuid(libc::getuid()); std::path::PathBuf::from(std::ffi::CStr::from_ptr((*pw).pw_dir).to_string_lossy().into_owned()) }.join(".herdr-projects");
    assert!(std::env::var("HERDR_PROJECTS_HOME").is_ok(), "the run names its own home folder");
    use sw::store::files::{config_dir, data_dir, home_dir, reports_dir};
    for dir in [home_dir(), data_dir(), config_dir(), reports_dir()] { assert!(dir != real && !dir.starts_with(&real), "{}", dir.display()); }
}

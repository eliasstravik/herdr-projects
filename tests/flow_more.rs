//! More of a task's flow: a start-up dialog's answer that herdr counts as a finished turn is not the end of the agent's
//! work, and a board kept in its own folder is named by the agent's command and its herdr space.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::json::*;

fn agent(w: &World, pane: &str) -> Value { agent_of(w, pane) }
fn patch_agent(w: &World, pane: &str, f: impl FnOnce(&mut Value, i64)) {
    w.patch(|s| { let seq = s.i("seq"); let p = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.s("pane_id") == pane).unwrap(); f(&mut p["agent"], seq); });
}
fn starting(id: &str) -> (String, String, String, String) { let t = task(id); let r = row(id); (t.t("stage"), t.t("msg"), r.t("state"), r.t("note")) }
fn still_starting() -> (String, String, String, String) { ("working".into(), "".into(), "working".into(), "Starting".into()) }

#[test]
fn a_startup_dialog_answered_and_the_first_prompt_not_picked_up_yet_is_still_starting_however_often_herdr_is_read() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.patch(|s| s["dialogs"]["claude"] = json!(1));
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Behind a dialog");
    delegate(&id);
    let t = settle(&id);
    let pane = t.at("link").t("pane");
    sw::herdr::Herdr::new(None).agent_keys(&pane, &["down", "enter"]).unwrap(); // herdr counts the answer as a finished turn
    let before = (agent(&w, &pane).at("status").clone(), agent(&w, &pane).at("seq").clone());
    on_pane(&w, &pane); // the reading that sends the waiting prompt
    assert_eq!(sw::store::inbox::pending_text(&id).len(), 0);
    assert_eq!(prompts(&w, &pane).len(), 1);
    // the agent program has taken the prompt and its model has not begun: herdr reads exactly as before
    patch_agent(&w, &pane, |a, _| { a["status"] = before.0.clone(); a["seq"] = before.1.clone(); });
    sw::store::tasks::update(&id, |t| { t.del("workedAt"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    for _ in 0..3 { on_pane(&w, &pane); assert_eq!((row(&id).t("state"), row(&id).t("note"), task(&id).t("stage")), ("working".into(), "Starting".into(), "working".into())); }
    assert!(!task(&id).at("last").at("comp").is_null() && !task(&id).has("workedAt"));
    // and the finish line does not take the answered dialog for the end of the agent's work
    settle(&id);
    let (stage, msg, _, note) = starting(&id);
    assert_eq!((stage.as_str(), msg.as_str(), note.as_str()), ("working", "", "Starting"));
    // herdr's state moves on with no work done: the prompt counts as taken, and the dialog's count is still not a turn's end
    w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["seq"] = json!(seq); } } });
    for _ in 0..3 { on_pane(&w, &pane); }
    assert!(task(&id).at("sent").is_null());
    assert!(sw::runner::JOBS.with(|j| j.borrow().is_empty()), "no finish line was started");
    settle(&id);
    assert_eq!(starting(&id), still_starting());
}

#[test]
fn a_startup_dialogs_turn_that_herdr_counts_only_after_the_first_prompt_went_out_leaves_the_untouched_task_alone() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.patch(|s| s["dialogs"]["claude"] = json!(1));
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Behind a dialog");
    delegate(&id);
    let t = settle(&id);
    let pane = t.at("link").t("pane");
    let reading = |complete: Option<bool>| {
        w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { let a = &mut p["agent"]; a["status"] = json!("idle"); a["seq"] = json!(seq);
            match complete { Some(true) => a["completion"] = json!(seq), Some(false) => if a.at("completion").is_null() { a["completion"] = json!(seq) }, None => {} } } } });
        on_pane(&w, &pane);
    };
    reading(None); // the dialog is answered, and herdr has not counted that yet: the waiting prompt goes out
    assert_eq!(sw::store::inbox::pending_text(&id).len(), 0);
    assert_eq!(prompts(&w, &pane).len(), 1);
    // now herdr counts it, the model has not begun: every reading starts the finish line, which finds nothing to finish
    for _ in 0..3 { reading(Some(false)); settle(&id); assert_eq!(starting(&id), still_starting()); }
    assert_eq!(prompts(&w, &pane).len(), 1);
    // the agent then works and ends its turn as any other: that end is still seen
    w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["status"] = json!("working"); p["agent"]["seq"] = json!(seq); } } });
    on_pane(&w, &pane);
    assert_eq!(row(&id).s("note"), "Working");
    reading(Some(true));
    assert_eq!(settle(&id).s("stage"), "nosummary", "it worked and said nothing: that is for the human");
}

/// The shim as the real program writes it: herdr is faked here, and the shim is not written while it is.
fn real_shim(w: &World) -> std::path::PathBuf { sw::herdr::cli::set_fake(None); let f = sw::flow::send::shim(); sw::herdr::cli::set_fake(Some(w.fake.clone())); f }
fn env_obj(v: Vec<(String, String)>) -> Value { Value::Object(v.into_iter().map(|(k, v)| (k, json!(v))).collect()) }

#[test]
fn a_board_kept_in_its_own_folder_the_agents_command_and_its_herdr_space_both_name_that_board() {
    let w = world_with(Opts { repo: false, ..Default::default() }); // a plain folder: its space is herdr's to make, not a worktree
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }] }));
    let id = w.add("Plain folder");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(w.pane_of(&t).at("env"), &env_obj(sw::store::files::board_env()), "a space herdr can hand an environment");
    let cmd = real_shim(&w);
    assert!(sw::delegate::rules::rules(&task(&id)).contains(&format!("{} report ", cmd.display())));
    // folders that come from the home folder rather than their own variables are carried just the same
    let other = w.base.join("other-home");
    unsafe { std::env::set_var("HERDR_PROJECTS_HOME", &other) }
    std::fs::create_dir_all(other.join("settings")).unwrap();
    std::fs::write(other.join("settings/config.json"), json!({ "max": 23 }).to_string()).unwrap();
    let want = vec![("HERDR_PROJECTS_HOME".to_string(), other.display().to_string())];
    let got = sw::store::files::board_env();
    let text = read(&real_shim(&w)); // the command names it, so the agent's own environment does not count
    unsafe { std::env::set_var("HERDR_PROJECTS_HOME", w.base.join("sw")) }
    assert_eq!(got, want);
    for (k, v) in &want { assert!(text.contains(&format!("export {k}='{v}'")), "{text}"); }
    // a shim an older version left as a link is replaced
    std::fs::remove_file(&cmd).unwrap(); std::os::unix::fs::symlink(sw::store::files::bin(), &cmd).unwrap();
    assert!(!std::fs::symlink_metadata(real_shim(&w)).unwrap().file_type().is_symlink());
}

#[test]
fn real_programs_the_agents_command_names_its_board_with_nothing_in_the_environment_pointing_at_it() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    let cmd = real_shim(&w);
    // Run as the agent would: by the path in its rules, from its folder, with nothing in the environment pointing at
    // this board. The shim here runs this test's own program, so it is pointed at the real one first.
    let text = read(&cmd).replace(&sw::store::files::bin().display().to_string(), env!("CARGO_BIN_EXE_herdr-projects"));
    std::fs::write(&cmd, text).unwrap();
    let o = std::process::Command::new(&cmd).args(["add", "Found on the way", "--desc", "A follow-up."]).current_dir(&w.root)
        .env("HERDR_PROJECTS_HOME", w.base.join("default-home")).env_remove("HERDR_PROJECTS_DATA_DIR").env_remove("HERDR_PROJECTS_CONFIG_DIR")
        .env("HERDR_SOCKET_PATH", "/nonexistent").env("HERDR_BIN_PATH", "/bin/false").output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(sw::flow::live::board(false).iter().any(|x| x.s("title") == "Found on the way"));
    assert!(!w.base.join("default-home").exists(), "nothing went to the default board");
}

//! More of a task's life: reconciled with herdr after a gone pane, an exited agent or a missed turn end; a task of
//! another herdr session; a task outside git; a failed setup; a landed task delegated again in a fresh worktree.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::json::*;

fn mark_done(w: &World, pane: &str) {
    w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["status"] = json!("done"); p["agent"]["seq"] = json!(seq); p["agent"]["completion"] = json!(seq); } } });
}
fn later(file: &Path) { let at = std::time::SystemTime::now() + std::time::Duration::from_millis(2000); std::fs::File::options().write(true).open(file).unwrap().set_modified(at).unwrap(); }

#[test]
fn reconcile_a_pane_that_is_gone_an_agent_that_exited_and_a_finished_turn_missed_while_herdr_restarted() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Reconcile");
    delegate(&id);
    let t = settle(&id);
    let pane = t.at("link").t("pane");
    let api = sw::herdr::Herdr::new(None);
    // the agent exits to its shell
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"] = serde_json::Value::Null; } } });
    sw::hook::reconcile(&task(&id), &api);
    assert_eq!((row(&id).t("state"), row(&id).t("note")), ("error".into(), "The agent has exited".into()));
    // the pane is closed
    w.patch(|s| s["panes"].as_array_mut().unwrap().retain(|p| p.s("pane_id") != pane));
    sw::hook::reconcile(&task(&id), &api);
    assert_eq!((row(&id).t("state"), row(&id).t("note")), ("error".into(), "The agent's terminal is gone".into()));
    // a pane move gave it a new id: relinked by agent name in the task's folder
    w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); s["panes"].as_array_mut().unwrap().push(json!({ "pane_id": "w9:p1", "workspace_id": "w9", "tab_id": "w9:t1", "terminal_id": "term_new", "cwd": t.s("worktree"), "agent": { "kind": "claude", "name": "sw-proj-1", "status": "working", "seq": seq } })); });
    sw::hook::reconcile(&task(&id), &api);
    assert_eq!(task(&id).at("link").s("pane"), "w9:p1");
    // it finished while no hook fired
    let note = sw::flow::task::note_file(&t);
    std::fs::create_dir_all(note.parent().unwrap()).unwrap();
    std::fs::write(&note, "Finished unseen.").unwrap(); later(&note);
    mark_done(&w, "w9:p1");
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "nothing to land: simply done");
    assert_eq!(t.s("msg"), "Finished unseen. No change needed.");
}

#[test]
fn tasks_of_another_herdr_session_show_their_last_known_state_and_refuse_live_actions_with_the_reason() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Elsewhere");
    delegate(&id);
    settle(&id);
    sw::store::tasks::update(&id, |t| { t.obj_mut("link").put("socket", "/x/sessions/other/herdr.sock"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    assert_eq!((row(&id).t("state"), row(&id).t("note")), ("working".into(), "In session other".into()));
    let e = sw::actions::talk::tell(&id, "hi", false).unwrap_err().msg;
    assert!(e.contains("lives in herdr session other"), "{e}");
}

#[test]
fn a_task_outside_git_works_in_place_in_a_plain_herdr_workspace_and_is_simply_done() {
    let w = world_with(Opts { repo: false, ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "notes.md": "x\n" }, "note": "Wrote the notes." }] }));
    let id = w.add("Plain folder");
    delegate(&id);
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("msg")), ("done", "Wrote the notes."));
    assert!(!t.b("worktree"));
    assert_eq!(w.state().arr("workspaces").len(), 0, "done: its space left herdr's sidebar by itself");
    assert!(t.b("cleaned") && !t.at("link").b("workspace"));
    assert_eq!(read(&w.root.join("notes.md")), "x\n");
    sw::actions::archive::clear(&id).unwrap();
    assert!(sw::store::tasks::get(&id).is_none());
}

#[test]
fn setup_runs_before_the_agent_starts_and_a_failed_setup_is_a_saved_failure() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "workspace.setup": "echo boom >&2; exit 3", "workspace.include": [".env"] })), ..Default::default() });
    std::fs::write(w.root.join(".gitignore"), ".env\n").unwrap(); std::fs::write(w.root.join(".env"), "SECRET=1\n").unwrap();
    git(&w.root, &["add", ".gitignore"]); git(&w.root, &["commit", "-qm", "ignore"]);
    let id = w.add("Setup fails");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed");
    assert_eq!(t.at("failure").s("title"), "Setup failed");
    assert!(t.at("failure").s("detail").contains("boom"), "{}", t.at("failure"));
    assert_eq!(read(&Path::new(t.s("worktree")).join(".env")), "SECRET=1\n", "ignored files named by include were copied");
    assert!(w.pane_of(&t).at("agent").is_null(), "the agent never started");
}

#[test]
fn delegating_a_landed_task_again_starts_a_new_round_in_a_fresh_worktree_never_in_your_checkout() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let id = to_approval(&w, "Work");
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done");
    assert!(t.b("cleaned"));
    w.script(json!({ "worker": [{ "status": "working" }] }));
    delegate(&id);
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("branch")), ("working", "work/1-work"));
    assert!(t.b("worktree") && t.s("worktree") != w.root_str() && Path::new(t.s("worktree")).exists());
    let p = w.pane_of(&t);
    assert_ne!(p.s("cwd"), w.root_str());
    let first = prompts(&w, p.s("pane_id"))[0].clone();
    assert!(first.contains("\n\nWork"), "{first}");
}

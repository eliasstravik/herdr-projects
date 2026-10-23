//! A pane id herdr handed out again: a done task's old pane given to a coordinator is the coordinator's, and its
//! commands are not refused as that task's agent's; a live task's agent (and a done task's agent still in its own
//! terminal) is still its task's. The hook leaves a done task alone when another pane took its id. A coordinator's own
//! pane, in the terminal saved with it, is never a task's, whatever a task has on record, and a task cleaned up is never
//! linked to it again. A refusal says which task's agent the caller was taken for.
use crate::common;
use common::pilot::*;
use common::*;
use serde_json::json;
use sw::coordinator::co::Co;
use sw::json::*;
use sw::store::tasks::{update, Change};

/// The coordinator on, its wake typed in and its turn still going: what follows runs in its pane.
fn in_wake(w: &World) -> Co {
    let co = proj(w);
    w.script(json!({ "coordinator": [{ "status": "working" }, { "status": "working" }, { "status": "working" }] }));
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.every", "none"]);
    tick(w);
    in_pane(w, &pane_of(&co));
    assert!(sw::autopilot::turn::autopilot_turn(&co));
    co
}
fn set(id: &str, f: impl Fn(&mut Obj)) { update(id, |t| { f(t); Ok(Change::Write) }).unwrap(); }
fn terminal_of(w: &World, pane: &str) -> String { sw::flow::pane_owner::terminal_of(&w.socket, pane) }

#[test]
fn a_done_tasks_old_pane_id_given_to_a_coordinator_is_the_coordinators() {
    let w = pilot_world();
    let co = in_wake(&w);
    let pane = pane_of(&co);
    // autopilot's task, done: its agent ran in a pane whose id herdr gave the coordinator once it closed
    let old = w.add("Done long ago");
    let link = json!({ "socket": w.socket, "pane": pane, "terminal": "term_closed", "agent": "sw-task-old", "reviewer": null });
    set(&old, |t| { t.put("stage", "done"); t.put("autopilot", json!({ "coordinator": "project:proj" })); t.put("link", link.clone()); });
    assert!(sw::cli::caller().is_none(), "the coordinator taken for the done task's agent");
    let said = ok(&["add", "Next step", "--repo", "proj"]);
    assert!(said.contains(": added"), "{said}");
    // archived, and a reviewer of it whose pane id went the same way: neither
    set(&old, |t| { t.put("archivedAt", 1); t.obj_mut("link").put("pane", "w9:p9"); t.obj_mut("link").put("reviewer", json!({ "pane": pane, "terminal": "term_closed2" })); });
    assert!(sw::cli::caller().is_none());
    // a reviewer link from an older version, with no terminal saved: an ended task's is not taken either
    set(&old, |t| { t.obj_mut("link").put("reviewer", json!({ "pane": pane })); });
    assert!(sw::cli::caller().is_none());
    in_pane(&w, "");
}

#[test]
fn a_live_tasks_agent_is_still_its_tasks_and_a_done_one_in_its_own_terminal_too() {
    let w = pilot_world();
    let id = w.add("Live");
    w.script(json!({ "worker": [{ "files": { "a.txt": "a\n" }, "commit": "a", "note": "Did a." }] }));
    delegate(&id);
    let t = settle(&id);
    assert_ne!(t.s("stage"), "done", "{}", t.s("msg"));
    let pane = t.at("link").t("pane");
    assert_eq!(t.at("link").t("terminal"), terminal_of(&w, &pane));
    in_pane(&w, &pane);
    let (me, role) = sw::cli::caller().expect("the live task's agent");
    assert_eq!((me.s("id"), role), (id.as_str(), "agent"));
    // done, its agent still in its own terminal (a chat after the work is done): still its agent
    set(&id, |t| t.put("stage", "done"));
    assert_eq!(sw::cli::caller().map(|c| c.0.t("id")), Some(id.clone()));
    // the pane closed and its id given to another terminal: no longer, done or not
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["terminal_id"] = json!("term_someone_else"); } } });
    assert!(sw::cli::caller().is_none());
    set(&id, |t| t.put("stage", "approval"));
    assert!(sw::cli::caller().is_none());
    // herdr cannot say what runs there: a live task keeps its pane, a done one does not
    set(&id, |t| t.obj_mut("link").put("pane", "w9:p9"));
    in_pane(&w, "w9:p9");
    assert_eq!(sw::cli::caller().map(|c| c.0.t("id")), Some(id.clone()));
    set(&id, |t| t.put("stage", "done"));
    assert!(sw::cli::caller().is_none());
    in_pane(&w, "");
}

#[test]
fn the_hook_leaves_a_done_task_alone_when_its_pane_id_went_to_a_coordinator() {
    let w = pilot_world();
    let co = in_wake(&w);
    let pane = pane_of(&co);
    let old = w.add("Done long ago");
    let link = json!({ "socket": w.socket, "pane": pane, "terminal": "term_closed", "agent": "sw-task-old", "reviewer": null });
    set(&old, |t| { t.put("stage", "done"); t.put("link", link.clone()); t.put("last", json!({ "status": "done", "seq": 1, "term": "term_closed" })); });
    busy(&w, &co);
    sw::hook::on_pane(&w.socket, &pane);
    let t = task(&old);
    assert_eq!((t.s("stage"), t.at("last").s("status"), t.at("last").s("term")), ("done", "done", "term_closed"), "the coordinator's reading was taken for the done task's");
    // the same task with the coordinator's terminal on record too: still the coordinator's, its reading not the task's
    let term = terminal_of(&w, &pane);
    set(&old, |t| { t.obj_mut("link").put("terminal", term.clone()); t.put("terms", json!([term.clone()])); t.obj_mut("last").put("term", term.clone()); });
    sw::hook::on_pane(&w.socket, &pane);
    assert_eq!(task(&old).at("last").s("status"), "done", "the coordinator's own terminal taken for the task's");
    // the coordinator gone from that terminal, the task's agent in it: the reading is its own
    sw::coordinator::state::change(&co, |c| { c.put("terminal", "term_before"); true }).unwrap();
    sw::hook::on_pane(&w.socket, &pane);
    assert_eq!(task(&old).at("last").s("status"), "working");
    in_pane(&w, "");
}

/// The coordinator's pane and terminal, as a task has them on record.
fn its_record(w: &World, co: &Co) -> Value {
    let pane = pane_of(co);
    json!({ "socket": w.socket, "pane": pane, "terminal": terminal_of(w, &pane), "agent": "sw-task-old", "reviewer": null })
}

#[test]
fn a_coordinator_in_the_pane_and_terminal_a_task_has_on_record_is_the_coordinator() {
    let w = pilot_world();
    let co = in_wake(&w);
    let link = its_record(&w, &co);
    let term = link.t("terminal");
    assert!(!term.is_empty());
    // seen on a machine: autopilot's task, done and cleaned up, with the coordinator's pane and terminal as its own
    let old = w.add("Done long ago");
    set(&old, |t| { t.put("stage", "done"); t.put("autopilot", json!({ "coordinator": "project:proj" })); t.put("link", link.clone()); t.put("terms", json!([term.clone()])); });
    assert!(sw::cli::caller().is_none(), "the coordinator taken for the done task's agent");
    let said = ok(&["add", "Next step", "--repo", "proj"]);
    assert!(said.contains(": added"), "{said}");
    // the same record on a task not done yet: still the coordinator's pane
    set(&old, |t| t.put("stage", "approval"));
    assert!(sw::cli::caller().is_none());
    // the coordinator gone from that terminal (its pane's id given to a task's agent since): the task's again
    sw::coordinator::state::change(&co, |c| { c.put("terminal", "term_before"); true }).unwrap();
    assert_eq!(sw::cli::caller().map(|c| c.0.t("id")), Some(old.clone()));
    in_pane(&w, "");
}

#[test]
fn a_refusal_names_the_task_the_caller_was_taken_for() {
    let w = pilot_world();
    let id = w.add("Live");
    w.script(json!({ "worker": [{ "files": { "a.txt": "a\n" }, "commit": "a", "note": "Did a." }] }));
    delegate(&id);
    let t = settle(&id);
    set(&id, |t| t.put("autopilot", json!({ "coordinator": "project:proj" })));
    in_pane(&w, &t.at("link").t("pane"));
    let said = refused(&["add", "More", "--repo", "proj"]);
    assert!(said.contains(&format!("You are taken as {id}'s agent.")), "{said}");
    assert!(refused(&["autopilot", "on", "--project", "proj"]).contains(&format!("You are taken as {id}'s agent.")));
    in_pane(&w, "");
}

#[test]
fn a_task_cleaned_up_is_not_linked_again_to_the_coordinator_in_its_repos_folder() {
    let w = pilot_world();
    let co = in_wake(&w);
    in_pane(&w, "");
    // its worktree removed and its pane closed: the folder its agent would be in is the repo's, the coordinator's
    let old = w.add("Done long ago");
    let gone = w.base.join("worktree-removed").display().to_string();
    let link = json!({ "socket": w.socket, "pane": "w9:p1", "terminal": "term_closed", "agent": "sw-task-old", "reviewer": null });
    set(&old, |t| { t.put("stage", "done"); t.put("worktree", gone.clone()); t.put("link", link.clone()); });
    sw::hook::reconcile(&task(&old), &sw::herdr::Herdr::new(Some(w.socket.clone())));
    let t = task(&old);
    assert_eq!((t.at("link").s("pane"), t.at("link").s("terminal")), ("w9:p1", "term_closed"), "linked to the coordinator's pane");
    assert!(t.at("last").b("gone"));
    // nor does the hook give a task's not done yet the coordinator's readings in its repo's folder
    let rec = its_record(&w, &co);
    set(&old, |t| { t.put("stage", "approval"); t.del("worktree"); t.put("link", json!({ "socket": w.socket, "pane": rec.s("pane"), "terminal": "term_closed", "agent": "sw-task-old", "reviewer": null })); t.put("terms", json!(["term_closed"])); });
    busy(&w, &co);
    sw::hook::on_pane(&w.socket, rec.s("pane"));
    let t = task(&old);
    assert_eq!(strings(t.at("terms")), vec!["term_closed".to_string()], "the coordinator's terminal taken for the task's");
    assert_ne!(t.at("last").s("status"), "working");
    // nor does the board's refresh, with the coordinator's pane and terminal on the task's record
    set(&old, |t| { t.put("link", rec.clone()); t.put("terms", json!([rec.t("terminal")])); t.del("last"); });
    sw::hook::reconcile(&task(&old), &sw::herdr::Herdr::new(Some(w.socket.clone())));
    let t = task(&old);
    assert_eq!(t.s("stage"), "approval", "the coordinator's work taken for the task's");
    assert_ne!(t.at("last").s("status"), "working");
    assert!(t.at("last").b("gone"));
}

//! A pane id herdr handed out again: a done task's old pane given to a coordinator is the coordinator's, and its
//! commands are not refused as that task's agent's; a live task's agent (and a done task's agent still in its own
//! terminal) is still its task's. The hook leaves a done task alone when another pane took its id.
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
    // the same task, its agent still in that pane's own terminal: the reading is its own
    let term = terminal_of(&w, &pane);
    set(&old, |t| { t.obj_mut("link").put("terminal", term.clone()); t.put("terms", json!([term.clone()])); t.obj_mut("last").put("term", term.clone()); });
    sw::hook::on_pane(&w.socket, &pane);
    assert_eq!(task(&old).at("last").s("status"), "working");
    in_pane(&w, "");
}

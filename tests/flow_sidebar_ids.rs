//! herdr hands a closed pane's id and its space's out again: a task's old link never names another's pane or space in
//! herdr's sidebar, and never a coordinator's, while a task's own pane, also after herdr restarted, still is written.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::flow::sidebar::{sync_sidebar, tell_sidebar};
use sw::json::*;
use sw::store::tasks::{update, Change};

fn sync() { sync_sidebar(&sw::flow::live::board(true), None, sw::clock::now()); }
fn label(w: &World, ws: &str) -> String { w.state().arr("workspaces").iter().find(|x| x.s("workspace_id") == ws).map(|x| x.t("label")).unwrap_or_default() }
fn writes(w: &World, pane: &str, ws: &str) -> usize { w.state().arr("metadata").iter().filter(|m| m.s("pane") == pane || m.s("workspace") == ws).count() }
fn retitle(id: &str, title: &str) { update(id, |t| { t.put("title", title); Ok(Change::Write) }).unwrap(); }
/// The pane under the task's pane id, as herdr now has it.
fn repane(w: &World, pane: &str, more: Value) { w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { for (k, v) in more.as_object().unwrap() { p[k] = v.clone(); } } } }); }

#[test]
fn a_done_tasks_old_pane_and_space_ids_handed_to_another_space_are_never_renamed_or_written_its_own_still_are() {
    let w = world();
    w.script(json!({ "worker": [{ "files": { "hello.txt": "hello\n" }, "note": "Added the greeting." }] }));
    let id = w.add("Add a greeting");
    delegate(&id);
    settle(&id);
    update(&id, |t| { t.put("stage", "done"); Ok(Change::Write) }).unwrap();
    let t = task(&id);
    let (pane, ws) = (t.at("link").t("pane"), t.at("link").t("workspace"));
    assert!(t.at("link").b("ownsWorkspace") && t.at("link").b("terminal"), "{}", t.at("link"));
    // The normal case: the task's own space is named after it.
    retitle(&id, "Say hello");
    sync();
    assert_eq!(label(&w, &ws), "PRJ-1 Say hello");
    // herdr restarted: the same pane under the same id in the task's own folder, in a new terminal. Still its own.
    repane(&w, &pane, json!({ "terminal_id": "term_after_restart" }));
    retitle(&id, "Say hello again");
    sync();
    assert_eq!(label(&w, &ws), "PRJ-1 Say hello again", "its own pane after a restart");
    // Its space was removed by hand and herdr opened another under the same ids: another terminal in another folder.
    let repo = sw::store::repos::folder_of(&id).display().to_string();
    w.patch(|st| { for x in st["workspaces"].as_array_mut().unwrap() { if x.s("workspace_id") == ws { x["label"] = json!("proj"); } } });
    repane(&w, &pane, json!({ "terminal_id": "term_someone_else", "cwd": repo, "agent": Value::Null }));
    retitle(&id, "Leak into the Coordinator");
    let n = writes(&w, &pane, &ws);
    sync();
    tell_sidebar(&task(&id));
    assert_eq!(label(&w, &ws), "proj", "not renamed");
    assert_eq!(writes(&w, &pane, &ws), n, "nothing written into that space or its pane");
    assert_eq!(sw::flow::live::board(true).iter().find(|r| r.s("id") == id).unwrap().at("live"), &json!({ "gone": true }), "its pane is gone");
}

#[test]
fn a_coordinators_pane_and_space_are_never_a_tasks_whatever_its_link_says() {
    let w = world();
    w.script(json!({ "worker": [{ "files": { "hello.txt": "hello\n" }, "note": "Added the greeting." }] }));
    let id = w.add("Add a greeting");
    delegate(&id);
    settle(&id);
    let t = task(&id);
    let (pane, ws) = (t.at("link").t("pane"), t.at("link").t("workspace"));
    // The Coordinator is where the task's link points: its pane and the space its tab is in.
    sw::coordinator::state::change(&w.socket, |c| { c.put("pane", pane.as_str()); c.put("space", ws.as_str()); true }).unwrap();
    retitle(&id, "Leak into the Coordinator");
    let (before, n) = (label(&w, &ws), writes(&w, &pane, &ws));
    sync();
    tell_sidebar(&task(&id));
    assert_eq!(label(&w, &ws), before, "not renamed");
    assert_eq!(writes(&w, &pane, &ws), n, "nothing written into the Coordinator's pane or space");
}

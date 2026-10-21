//! A finished task leaves herdr's sidebar: the repo space herdr opened goes with the last task, yours stays, unsaved
//! files stay on disk; and clean-up closes no space with something of yours running in it, nor one herdr cannot answer for.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::actions::{archive, filing, finish_line, talk};
use sw::json::*;

fn done(id: &str) { filing::report(id, filing::Report { state: "done".into(), ..Default::default() }).unwrap(); }
fn kept(t: &Obj) -> String { t.arr("kept").first().map(text).unwrap_or_default() }
fn ws_ids(w: &World) -> Vec<String> { w.state().arr("workspaces").iter().map(|x| x.t("workspace_id")).collect() }
fn open(w: &World, t: &Obj) -> bool { w.state().arr("workspaces").iter().any(|x| x.s("workspace_id") == t.at("link").s("workspace")) }
fn busy(w: &World, pane: &str) { w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["busy"] = json!(true); } } }); }
fn ready(w: &World, n: i64) -> Obj {
    w.script(json!({ "worker": [{ "files": { format!("g{n}.txt"): "x\n" }, "commit": format!("g{n}"), "note": "Did it." }] }));
    let id = w.add(&format!("Task {n}"));
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    t
}

#[test]
fn a_finished_task_leaves_herdrs_sidebar_the_repo_space_herdr_opened_goes_with_the_last_task_yours_stays_unsaved_files_stay_on_disk() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    // herdr opens the repo itself next to the first worktree; it is closed with the last task space, not before
    let a = to_approval(&w, "Work");
    w.script(json!({ "worker": [{ "files": { "w.txt": "w\n" }, "commit": "w", "note": "Did w." }] }));
    let b = w.add("Second");
    delegate(&b);
    assert_eq!(settle(&b).s("stage"), "approval");
    assert_eq!(w.state().arr("workspaces").len(), 3);
    finish_line::approve(&a, None, false).unwrap();
    assert_eq!(settle(&a).s("stage"), "done");
    let mut cwds: Vec<String> = w.state().arr("workspaces").iter().map(|x| x.t("cwd")).collect(); cwds.sort();
    let mut want = vec![w.root_str(), task(&b).t("worktree")]; want.sort();
    assert_eq!(cwds, want);
    finish_line::approve(&b, None, false).unwrap();
    let t = settle(&b);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(w.state().arr("workspaces").len(), 0);
    // a repo space that was open before the task is yours and stays; calling a task done cleans up like a merge
    let root = w.root_str();
    w.patch(|s| {
        s["workspaces"].as_array_mut().unwrap().push(json!({ "workspace_id": "mine", "label": "proj", "cwd": root, "worktree": { "checkout_path": root, "repo_root": root, "is_linked_worktree": false } }));
        s["panes"].as_array_mut().unwrap().push(json!({ "pane_id": "mine:p1", "workspace_id": "mine", "tab_id": "mine:t1", "terminal_id": "term_1", "cwd": root, "agent": null }));
    });
    w.script(json!({ "worker": [{ "files": { "c.txt": "c\n" }, "commit": "c", "note": "Did c." }] }));
    let c = w.add("Third");
    delegate(&c);
    assert_eq!(settle(&c).s("stage"), "approval");
    std::fs::write(Path::new(task(&c).s("worktree")).join("unsaved.txt"), "x\n").unwrap();
    done(&c);
    let t = settle(&c);
    assert!(kept(&t).contains("uncommitted changes"), "{}", kept(&t)); assert!(Path::new(t.s("worktree")).exists());
    assert_eq!(ws_ids(&w), vec!["mine"]);
    archive::clear(&c).unwrap();
    // clean work that landed nowhere, called done: the space goes, the worktree and branch stay, and a reply continues there
    w.script(json!({ "worker": [{ "files": { "d.txt": "d\n" }, "commit": "d", "note": "Did d." }] }));
    let d = w.add("Fourth");
    delegate(&d);
    assert_eq!(settle(&d).s("stage"), "approval");
    done(&d);
    let t = settle(&d);
    assert!(kept(&t).contains("unmerged work on work/"), "{}", kept(&t)); assert!(Path::new(t.s("worktree")).exists()); assert!(!t.b("cleaned"));
    assert_eq!(ws_ids(&w), vec!["mine"]);
    w.script(json!({ "worker": [{ "files": { "d2.txt": "d\n" }, "commit": "d2", "note": "Did more." }] }));
    let said = talk::tell(&d, "go on", false).unwrap().msg;
    assert!(said.contains("starting it again"), "{said}");
    let t = settle(&d);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(Path::new(t.s("worktree")).join("d.txt").exists());
    assert_ne!(sw::store::repos::cwd_of(&t), w.root);
}

#[test]
fn the_repo_space_is_left_alone_while_a_command_of_yours_runs_in_it_a_reply_to_cleaned_up_work_starts_a_new_round() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    // nothing to merge: done, cleaned up, and a reply starts a new round in a fresh worktree
    w.script(json!({ "worker": [{ "note": "Looked. Nothing to change." }] }));
    let id = w.add("Look");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(t.b("cleaned"));
    assert_eq!(w.state().arr("workspaces").len(), 0); assert_eq!(git(&w.root, &["branch", "--list", "work/*"]), "");
    w.script(json!({ "worker": [{ "files": { "n.txt": "n\n" }, "commit": "n", "note": "Did n." }] }));
    let said = talk::tell(&id, "then add n", false).unwrap().msg;
    assert!(said.contains("new round"), "{said}");
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(Path::new(t.s("worktree")).exists() && t.s("worktree") != w.root_str());
    assert!(prompts(&w, t.at("link").s("pane")).iter().any(|x| x.contains("then add n")));
    // you started something in the repo space herdr opened: it stays
    let root = w.root_str();
    let mine = w.state().arr("panes").iter().find(|p| p.s("cwd") == root).unwrap().t("pane_id");
    busy(&w, &mine);
    finish_line::approve(&id, None, false).unwrap();
    assert_eq!(settle(&id).s("stage"), "done");
    assert_eq!(w.state().arr("workspaces").iter().map(|x| x.t("cwd")).collect::<Vec<_>>(), vec![root]);
}

#[test]
fn clean_up_closes_no_space_with_something_of_yours_running_in_it_nor_one_herdr_cannot_answer_for() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let split = |t: &Obj| sw::herdr::Herdr::new(None).pane_split(t.at("link").s("pane"), t.s("worktree"), "right", &[]).unwrap().t("pane_id");
    // a pane you split off in the task's space runs a command: marking the task done leaves the whole space open
    let t = ready(&w, 1);
    let mine = split(&t);
    busy(&w, &mine);
    done(t.s("id"));
    let t = settle(t.s("id"));
    assert!(kept(&t).contains("its space is in use"), "{}", kept(&t)); assert!(open(&w, &t)); assert!(Path::new(t.s("worktree")).exists());
    assert!(w.state().arr("panes").iter().any(|p| p.s("pane_id") == mine));
    // the same after a merge: the worktree is not pulled from under your command
    let t = ready(&w, 2);
    let mine2 = split(&t);
    busy(&w, &mine2);
    finish_line::approve(t.s("id"), None, false).unwrap();
    let t = settle(t.s("id"));
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(kept(&t).contains("its space is in use"), "{}", kept(&t)); assert!(open(&w, &t)); assert!(Path::new(t.s("worktree")).exists());
    // herdr does not answer whether the agent works: unknown is not "nobody there"
    let t = ready(&w, 3);
    sw::store::tasks::update(t.s("id"), |c| { c.put("stage", "done"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    w.patch(|s| s["agentGetFails"] = json!(true));
    sw::land::cleanup_job(t.s("id"), "merged");
    w.patch(|s| s["agentGetFails"] = json!(false));
    let t = task(t.s("id"));
    assert!(kept(&t).contains("its space is in use"), "{}", kept(&t)); assert!(open(&w, &t)); assert!(Path::new(t.s("worktree")).exists());
}

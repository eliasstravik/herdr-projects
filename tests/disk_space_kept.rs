//! The edges of a task's clean-up (crates/flow/src/leftovers.rs): a done task whose agent stays open
//! (`workspace.cleanup` off) loses its temporary files all the same, all but the folders its program keeps for itself;
//! nothing is ever reached through a link in a temporary folder's place; and the build an approved plan starts waits
//! for room on a nearly full disk like any start.
use crate::common;
use common::*;
use serde_json::json;
use std::path::{Path, PathBuf};
use sw::json::*;

fn fill(p: &Path) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, vec![7u8; 1_000_000]).unwrap(); }
fn claude_files(w: &World, tree: &str) -> PathBuf { w.base.join("tmp").join(format!("claude-{}", sw::agents::scratch::uid())).join(sw::agents::scratch::slug(Path::new(tree))) }

#[test]
fn a_done_task_whose_agent_stays_open_loses_its_temporary_files_but_the_folders_its_program_keeps_for_itself() {
    let w = world_with(Opts { repo: true, settings: json!({ "workspace.cleanup": false, "review.when": "never" }), ..Default::default() });
    let id = to_approval(&w, "Kept open");
    let t = task(&id);
    let tmp = sw::temp::of(&t);
    let own = tmp.join(format!("claude-{}", sw::agents::scratch::uid())).join("-wt").join("sess-1");
    fill(&tmp.join("preview/node_modules/big.bin")); fill(&own.join("scratchpad/venv/big.bin")); fill(&own.join("tasks/out.txt"));
    let old = claude_files(&w, t.s("worktree"));
    fill(&old.join("sess-0/scratchpad/x.bin"));
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(agent_of(&w, t.at("link").s("pane")).is_object(), "its agent is still open");
    assert!(Path::new(t.s("worktree")).exists());
    assert!(!tmp.join("preview").exists() && !own.join("scratchpad/venv").exists() && !own.join("tasks/out.txt").exists());
    assert!(own.join("scratchpad").is_dir() && own.join("tasks").is_dir(), "the agent's own folders stay, empty");
    assert!(old.join("sess-0/scratchpad").is_dir() && !old.join("sess-0/scratchpad/x.bin").exists());
    assert!(t.s("msg").contains("Freed ") && t.s("msg").contains("of temporary files."), "{}", t.s("msg"));
    // what it made later goes in the sweep, once nobody talked to it for an hour, and so does its old build output
    fill(&tmp.join("again/x.bin"));
    std::fs::write(Path::new(t.s("worktree")).join(".gitignore"), "node_modules/\n").unwrap();
    fill(&Path::new(t.s("worktree")).join("node_modules/a.js"));
    sw::leftovers::sweep::sweep();
    assert!(tmp.join("again").exists(), "talked to within the hour");
    sw::clock::advance(8 * 86_400_000);
    sw::leftovers::sweep::sweep();
    assert!(!tmp.join("again").exists() && own.join("scratchpad").is_dir());
    assert!(!Path::new(t.s("worktree")).join("node_modules").exists());
    // at work again (a chat): nothing is touched
    fill(&tmp.join("busy/x.bin"));
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.at("agent").is_object() { p["agent"]["status"] = json!("working"); } } });
    sw::clock::advance(2 * 3_600_000);
    sw::leftovers::sweep::sweep();
    assert!(tmp.join("busy/x.bin").exists());
}

#[test]
fn real_programs_a_link_in_a_temporary_folder_s_place_is_never_followed() {
    let w = world();
    let id = w.add("Linked");
    let t = task(&id);
    let tmp = sw::temp::made(&t);
    // the task's folder replaced by a link to a folder of yours, with something of yours running in it
    let yours = w.base.join("home/projects/site");
    std::fs::create_dir_all(&yours).unwrap();
    std::fs::write(yours.join("index.html"), "mine\n").unwrap();
    let mut server = std::process::Command::new("sleep").arg("60").current_dir(&yours).spawn().unwrap();
    std::fs::remove_dir(&tmp).unwrap();
    std::os::unix::fs::symlink(&yours, &tmp).unwrap();
    assert_eq!(sw::leftovers::free(&t), 0);
    assert!(server.try_wait().unwrap().is_none(), "yours runs on");
    assert!(yours.join("index.html").exists());
    assert!(std::fs::symlink_metadata(&tmp).is_err(), "the link itself goes");
    // a link further up (the board's own folder of task folders) is not followed either
    let base = sw::temp::board_base();
    std::fs::rename(&base, w.base.join("moved")).unwrap();
    std::os::unix::fs::symlink(&yours, &base).unwrap();
    let inside = base.join(sw::temp::name_of(&t));
    std::fs::create_dir_all(yours.join(inside.file_name().unwrap())).unwrap();
    assert!(!sw::leftovers::removable(&inside));
    assert_eq!(sw::leftovers::free(&t), 0);
    assert!(yours.join(inside.file_name().unwrap()).exists() && server.try_wait().unwrap().is_none());
    // and the sweep, for a done task with such a link, stops nothing of yours
    std::fs::remove_file(&base).unwrap(); std::fs::rename(w.base.join("moved"), &base).unwrap();
    std::os::unix::fs::symlink(&yours, &tmp).unwrap();
    sw::store::tasks::update(&id, |t| { t.put("stage", "done"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    sw::leftovers::sweep::sweep();
    assert!(server.try_wait().unwrap().is_none() && yours.join("index.html").exists());
    server.kill().unwrap(); let _ = server.wait();
}

#[test]
fn the_build_an_approved_plan_starts_waits_for_room_on_a_nearly_full_disk() {
    let w = world_with(Opts { repo: true, settings: json!({ "plan.approve": "you", "worker.agent": "codex" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap") };
    w.patch(|s| s["pids"] = json!(true));
    w.script(json!({ "worker": [{ "plan": "# Plan\n\n1. Add f.txt.\n", "note": "The plan is written." }, { "status": "working" }] }));
    let id = w.add("Planned");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "plan");
    unsafe { std::env::set_var("HERDR_PROJECTS_FREE", "2000000000") };
    sw::actions::plan::approve_plan(&id).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "queued", "{}", t.at("failure"));
    assert!(t.s("msg").starts_with("Disk nearly full on ") && t.at("plan").b("approvedAt") && !t.at("plan").b("built"), "{}", t.s("msg"));
    unsafe { std::env::set_var("HERDR_PROJECTS_FREE", "1000000000000") };
    assert_eq!(sw::actions::start::start_queued(&w.socket), vec![id.clone()]);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert!(t.at("plan").b("built") && !t.b("diskWait"));
}

#[test]
fn a_link_reached_through_another_link_is_never_taken_away() {
    let w = world();
    let id = w.add("Linked twice");
    // your folder, with a link of yours in it; the task's folder named through a link to that folder
    let yours = w.base.join("home/stuff");
    std::fs::create_dir_all(yours.join("site")).unwrap();
    std::fs::write(yours.join("site/keep.txt"), "mine\n").unwrap();
    std::os::unix::fs::symlink(yours.join("site"), yours.join("shortcut")).unwrap();
    let base = sw::temp::board_base();
    std::fs::create_dir_all(&base).unwrap();
    std::os::unix::fs::symlink(&yours, base.join("parent")).unwrap();
    let tmp = base.join("parent/shortcut");
    sw::store::tasks::update(&id, |t| { t.put("tmp", tmp.display().to_string()); t.put("stage", "done"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    assert_eq!(sw::leftovers::free(&task(&id)), 0);
    sw::leftovers::sweep::sweep();
    assert!(std::fs::symlink_metadata(yours.join("shortcut")).map(|m| m.file_type().is_symlink()).unwrap_or(false), "your link stays");
    assert!(yours.join("site/keep.txt").exists());
}

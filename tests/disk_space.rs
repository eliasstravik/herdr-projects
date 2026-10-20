//! A task's temporary files (crates/core/src/temp.rs, crates/flow/src/leftovers.rs): its agent and reviewer start with
//! its own temporary folder as TMPDIR and are told to keep everything temporary there; when it ends (done, archived,
//! removed) that folder and the agents' files for its worktree go, a kept worktree stays, and its line says what was
//! freed; on a nearly full disk a start waits in the queue with a plain line, and starts once there is room.
use crate::common;
use common::*;
use serde_json::json;
use std::path::{Path, PathBuf};
use sw::actions::{archive, finish_line};
use sw::json::*;

fn uid() -> u32 { sw::agents::scratch::uid() }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
/// A file of `mb` megabytes at `p`, its folders made.
fn fill(p: &Path, mb: usize) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, vec![7u8; mb * 1_000_000]).unwrap(); }
/// Where Claude Code, started without the task's own folder, kept its files for this worktree.
fn claude_files(w: &World, tree: &str) -> PathBuf { w.base.join("tmp").join(format!("claude-{}", uid())).join(sw::agents::scratch::slug(Path::new(tree))) }

#[test]
fn the_agent_and_the_reviewer_start_with_the_task_s_own_temporary_folder_and_are_told_to_keep_temporary_things_there() {
    let w = world();
    let id = to_approval(&w, "Work");
    let t = task(&id);
    let tmp = sw::temp::of(&t);
    assert_eq!(t.s("tmp"), tmp.display().to_string());
    assert!(tmp.starts_with(w.base.join("tmp").join(format!("hp-{}", uid()))) && tmp.ends_with(short(&id)), "{}", tmp.display());
    assert!(tmp.display().to_string().len() < w.base.join("tmp").display().to_string().len() + 30, "short, for the sockets programs make in it");
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(std::fs::metadata(&tmp).unwrap().permissions().mode() & 0o777, 0o700);
    // herdr hands a worktree's space no environment: the pane's shell is given it right before the program starts
    let all = calls(&w);
    let export = all.iter().position(|c| c.starts_with("pane run") && c.contains(&format!("export TMPDIR={0} TMP={0} TEMP={0} CLAUDE_CODE_TMPDIR={0}", tmp.display()))).expect("the export");
    let start = all.iter().position(|c| c.starts_with("agent start sw-proj-1 ")).unwrap();
    assert!(export < start, "{all:?}");
    // the reviewer's tab has it from the start
    assert!(all.iter().any(|c| c.starts_with("tab create") && c.contains(&format!("--env TMPDIR={}", tmp.display()))), "{all:?}");
    let rules = sw::delegate::rules::rules(&t);
    assert!(rules.contains(&format!("in your own temporary folder, {} (your $TMPDIR), never loose in /tmp or in a home folder", tmp.display())), "{rules}");
    assert!(rules.contains("stop the servers and background processes you started, unless the human asked you to keep one running"), "{rules}");
    let ch = sw::store::settings::lookup::work(Some(&t));
    let brief = sw::finish::review_brief::review_brief(&t, &sw::finish::review_brief::Brief { text: "Done.", change: None, to_land: false, ch: &ch, earlier: None, check: "" });
    assert!(brief.contains(&format!("in your own temporary folder, {}", tmp.display())), "{brief}");
}

#[test]
fn a_done_task_s_temporary_folder_and_its_agents_files_go_with_its_clean_up_and_its_line_says_what_was_freed() {
    let w = world();
    let id = to_approval(&w, "Work");
    let t = task(&id);
    let (tmp, old) = (sw::temp::of(&t), claude_files(&w, t.s("worktree")));
    fill(&tmp.join("preview/node_modules/big.bin"), 2);
    fill(&old.join("e5fc/scratchpad/venv.bin"), 1);
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(!tmp.exists() && !old.exists());
    assert!(has(t.s("msg"), r"Freed \d+(\.\d)? MB of temporary files\.$"), "{}", t.s("msg"));
}

#[test]
fn a_kept_worktree_stays_but_its_temporary_files_go_and_archive_and_remove_take_them_too() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    // kept: unsaved work in it. The worktree stays, what was temporary goes.
    let id = to_approval(&w, "Kept");
    let t = task(&id);
    std::fs::write(Path::new(t.s("worktree")).join("unsaved.txt"), "x\n").unwrap();
    fill(&sw::temp::of(&t).join("smoke/install.bin"), 2);
    sw::actions::filing::report(&id, sw::actions::filing::Report { state: "done".into(), ..Default::default() }).unwrap();
    let t = settle(&id);
    assert!(t.arr("kept").iter().any(|k| text(k).contains("uncommitted changes")), "{:?}", t.at("kept"));
    assert!(Path::new(t.s("worktree")).join("unsaved.txt").exists());
    assert!(!sw::temp::of(&t).exists());
    assert!(t.s("msg").contains("Kept: ") && t.s("msg").contains("Freed "), "{}", t.s("msg"));
    // archived: its space closes with its agent, and its temporary files go
    w.script(json!({ "worker": [{ "status": "idle" }] }));
    let a = w.add("Archived");
    delegate(&a);
    let ta = settle(&a);
    fill(&sw::temp::of(&ta).join("x.bin"), 1);
    let ta = archive::archive(&a).unwrap();
    assert!(!sw::temp::of(&ta).exists());
    assert!(has(ta.s("msg"), r"Freed \d MB of temporary files\.$"), "{}", ta.s("msg"));
    // removed: the same, before its record goes
    w.script(json!({ "worker": [{ "status": "idle" }] }));
    let r = w.add("Removed");
    delegate(&r);
    let tr = settle(&r);
    let (tmp, old) = (sw::temp::of(&tr), claude_files(&w, tr.s("worktree")));
    fill(&tmp.join("x.bin"), 1); fill(&old.join("s/x.bin"), 1);
    archive::clear(&r).unwrap();
    assert!(!tmp.exists() && !old.exists());
}

#[test]
fn a_working_task_keeps_its_temporary_files_and_nothing_the_environment_points_into_is_ever_removed() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Working");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working");
    let (tmp, old) = (sw::temp::of(&t), claude_files(&w, t.s("worktree")));
    fill(&tmp.join("x.bin"), 1); fill(&old.join("s/x.bin"), 1);
    sw::clock::advance(30 * 86_400_000);
    sw::leftovers::sweep::sweep();
    assert!(tmp.exists() && old.exists());
    // a PATH entry of yours in an old scratchpad (a bun installed there) keeps that folder, whatever else is true
    let gone = claude_files(&w, &w.base.join("wt/proj/work-9-old").display().to_string());
    fill(&gone.join("s/bun/bin/bun"), 1);
    assert!(sw::leftovers::guarded(&old) == false);
    let path = std::env::var("PATH").unwrap_or_default();
    unsafe { std::env::set_var("PATH", format!("{}:{path}", gone.join("s/bun/bin").display())) };
    assert!(sw::leftovers::guarded(&gone));
    assert_eq!(sw::leftovers::remove(&gone), 0);
    unsafe { std::env::set_var("PATH", path) };
    assert!(gone.exists());
    // the home folder, the board's own folders and your repos are never removed
    for p in [sw::store::files::user_home(), sw::store::files::data_dir(), w.root.clone(), w.base.clone()] { assert!(sw::leftovers::guarded(&p), "{}", p.display()); }
}

#[test]
fn on_a_nearly_full_disk_a_start_sweeps_then_waits_in_the_queue_with_a_plain_line_and_starts_once_there_is_room() {
    let w = world();
    unsafe { std::env::set_var("HERDR_PROJECTS_FREE", "3100000000") };
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Big build");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "queued");
    let host = sw::remote::check::hostname();
    assert!(t.s("msg").starts_with(&format!("Disk nearly full on {host}: 3.1 GB free on ")), "{}", t.s("msg"));
    assert!(t.s("msg").contains("this task starts once 5.0 GB is free (board.diskFree). Free some space: herdr-projects sweep"), "{}", t.s("msg"));
    assert!(!t.b("worktree") && !calls(&w).iter().any(|c| c.starts_with("agent start")), "nothing was started");
    assert!(sw::store::files::data_dir().join("sweep.json").exists(), "the sweep ran first");
    // still no room: the queue starts no runner for it
    assert!(sw::actions::start::start_queued(&w.socket).is_empty());
    // room again: it starts by itself
    unsafe { std::env::set_var("HERDR_PROJECTS_FREE", "1000000000000") };
    assert_eq!(sw::actions::start::start_queued(&w.socket), vec![id.clone()]);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working");
    assert!(!t.b("diskWait"));
    // 0: never waits
    sw::store::settings::change::set_value("board.diskFree", &json!(0), &sw::store::settings::lookup::Scope::you()).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_FREE", "1") };
    assert!(sw::leftovers::room::low(&t).is_none());
}

#[test]
fn herdr_projects_machines_says_each_machine_s_free_disk() {
    let r = json!({ "here": { "label": "vm-str", "here": true, "ok": true, "version": "1.2.12", "shell": "/bin/bash", "free": 778_000_000u64 },
        "machines": [{ "label": "vm-a", "target": "me@vm-a", "ok": true, "version": "1.2.12", "tasks": 2, "free": 21_000_000_000u64 }, { "label": "vm-b", "target": "me@vm-b", "ok": true, "version": "1.2.0", "tasks": 1, "free": null }] });
    let said = sw::remote::check::say(&r);
    assert!(said.contains("vm-str  this machine: a login shell (/bin/bash) finds herdr-projects 1.2.12 · 778 MB free\n"), "{said}");
    assert!(said.contains("vm-a    answers · herdr-projects 1.2.12 · 2 tasks · 21 GB free  (me@vm-a)"), "{said}");
    assert!(said.contains("vm-b    answers · herdr-projects 1.2.0 · 1 task  (me@vm-b)"), "an older one says nothing of it: {said}");
    let w = world();
    assert_eq!(sw::remote::remote_list().at("free"), &json!(1_000_000_000_000u64));
    drop(w);
}

#[test]
fn a_board_command_a_task_s_agent_runs_leaves_that_task_s_temporary_folder_behind() {
    let w = world();
    let id = w.add("Agent's own");
    let tmp = sw::temp::made(&task(&id));
    let names = ["TMPDIR", "TMP", "TEMP", "CLAUDE_CODE_TMPDIR"];
    let had: Vec<_> = names.iter().map(|k| std::env::var_os(k)).collect();
    for (k, v) in sw::temp::env(&task(&id)) { unsafe { std::env::set_var(k, v) } }
    sw::temp::leave_task_env();
    for k in names { assert!(std::env::var_os(k).is_none(), "{k}"); }
    // a TMPDIR of yours stays (a path of its own: the tests' folder is in a task's when a task's agent runs them)
    let mine = std::path::PathBuf::from("/var/tmp/mine");
    unsafe { std::env::set_var("TMPDIR", &mine) };
    sw::temp::leave_task_env();
    assert_eq!(std::env::var_os("TMPDIR"), Some(mine.into()));
    for (k, v) in names.iter().zip(had) { match v { Some(v) => unsafe { std::env::set_var(k, v) }, None => unsafe { std::env::remove_var(k) } } }
    assert!(sw::temp::in_task_folder(&tmp.join("claude-1000")) && !sw::temp::in_task_folder(std::path::Path::new("/srv/proj")));
}

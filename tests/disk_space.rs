//! A task's temporary files (crates/core/src/temp.rs, crates/flow/src/leftovers.rs): its agent and reviewer start with
//! its own temporary folder as TMPDIR and are told to keep everything temporary there; when it ends (done, archived,
//! removed) that folder and the agents' files for its worktree go, a kept worktree stays, and its line says what was
//! freed; on a nearly full disk a start waits in the queue with a plain line, and starts once there is room. A small disk
//! (a `/tmp` of 3.4 GB) keeps free `board.diskFree`, never more than half of itself; a task that waits for room says
//! "waiting for disk space" everywhere, never "waiting for a free slot", and doctor names it beside its disk.
use crate::common;
use common::*;
use serde_json::json;
use std::path::{Path, PathBuf};
use common::board::open;
use sw::actions::{archive, finish_line};
use sw::doctor::disk::{self, Disk};
use sw::doctor::Found;
use sw::json::*;

fn uid() -> u32 { sw::agents::scratch::uid() }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
/// A file of `mb` megabytes at `p`, its folders made.
fn fill(p: &Path, mb: usize) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, vec![7u8; mb * 1_000_000]).unwrap(); }
/// Where Claude Code, started without the task's own folder, kept its files for this worktree.
fn claude_files(w: &World, tree: &str) -> PathBuf { w.base.join("tmp").join(format!("claude-{}", uid())).join(sw::agents::scratch::slug(Path::new(tree))) }
/// Every disk has `free` bytes free of `size` in all.
const GB: u64 = 1_000_000_000;
fn disk_is(free: u64, size: u64) { unsafe { std::env::set_var("HERDR_PROJECTS_FREE", free.to_string()); std::env::set_var("HERDR_PROJECTS_SIZE", size.to_string()) } }

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
    // herdr hands a worktree's space no environment: the pane's shell is given it right before the program starts, with
    // the task's id and its own folder among the reports
    let all = calls(&w);
    let reports = sw::research::report_folder(&t);
    let export = all.iter().position(|c| c.starts_with("pane run") && c.contains(&format!("export TMPDIR={0} TMP={0} TEMP={0} CLAUDE_CODE_TMPDIR={0} HERDR_PROJECTS_ID=proj/1 HERDR_PROJECTS_TASK_REPORTS={1}", tmp.display(), reports.display()))).expect("the export");
    let start = all.iter().position(|c| c.starts_with("agent start sw-proj-1 ")).unwrap();
    assert!(export < start, "{all:?}");
    // the reviewer's tab has it from the start
    let reviewer = all.iter().find(|c| c.starts_with("tab create") && c.contains(&format!("--env TMPDIR={}", tmp.display()))).unwrap_or_else(|| panic!("{all:?}"));
    assert!(reviewer.contains("--env HERDR_PROJECTS_ID=proj/1") && reviewer.contains(&format!("--env HERDR_PROJECTS_TASK_REPORTS={}", reports.display())), "{reviewer}");
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
    let names = ["TMPDIR", "TMP", "TEMP", "CLAUDE_CODE_TMPDIR", "HERDR_PROJECTS_ID", "HERDR_PROJECTS_TASK_REPORTS"];
    let had: Vec<_> = names.iter().map(|k| std::env::var_os(k)).collect();
    let env = sw::temp::env(&task(&id));
    assert_eq!(env.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(), names);
    for (k, v) in env { unsafe { std::env::set_var(k, v) } }
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

#[test]
fn a_disk_smaller_than_twice_board_disk_free_needs_half_of_itself_free_and_its_line_says_so() {
    let w = world();
    // 3.4 GB in all, 2 GB free: more than its half (1.7 GB), so the task starts.
    disk_is(2 * GB, 3_400_000_000);
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let a = w.add("Fits");
    delegate(&a);
    assert_eq!(settle(&a).s("stage"), "working");
    assert_eq!(sw::leftovers::room::need_on(Some(3_400_000_000)), 1_700_000_000);
    // 1.2 GB free: it waits, and its line says the amount that disk needs, and why it is less than board.diskFree.
    disk_is(1_200_000_000, 3_400_000_000);
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let b = w.add("Waits");
    delegate(&b);
    let t = settle(&b);
    assert_eq!(t.s("stage"), "queued");
    let host = sw::remote::check::hostname();
    assert!(t.s("msg").starts_with(&format!("Disk nearly full on {host}: 1.2 GB free on ")), "{}", t.s("msg"));
    assert!(regex::Regex::new(r"; this task starts once 1\.7 GB is free \(\S+ is only 3\.4 GB in all, so it needs 1\.7 GB free, not the 5\.0 GB of board\.diskFree\)\. Free some space: herdr-projects sweep").unwrap().is_match(t.s("msg")), "{}", t.s("msg"));
    assert!(t.at("diskWait").s("on").starts_with('/'), "the disk it waits for: {}", t.at("diskWait"));
    // A big disk still needs all of board.diskFree, and says nothing more.
    disk_is(3_100_000_000, 100 * GB);
    let l = sw::leftovers::room::low(&t).unwrap();
    assert_eq!(l.need, 5 * GB);
    assert!(l.line().contains("this task starts once 5.0 GB is free (board.diskFree). Free"), "{}", l.line());
    // 0: never waits, on a small disk too.
    sw::store::settings::change::set_value("board.diskFree", &json!(0), &sw::store::settings::lookup::Scope::you()).unwrap();
    disk_is(1, 3_400_000_000);
    assert!(sw::leftovers::room::low(&t).is_none());
}

#[test]
fn a_task_that_waits_for_disk_space_says_so_everywhere_and_one_that_waits_for_a_slot_still_says_a_free_slot() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.agents": 1, "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }] }));
    let a = w.add("First at work");
    delegate(&a);
    assert_eq!(settle(&a).s("stage"), "working");
    // No free slot: both wait for one, in the same words as ever.
    let d = w.add("Waits for disk");
    sw::clock::advance(1000);
    delegate(&d);
    let s = w.add("Waits for a slot");
    sw::clock::advance(1000);
    delegate(&s);
    assert_eq!(sw::shown::states::standing(&sw::shown::states::as_row(&row(&d))), "waiting for a free slot, Q#1");
    // A slot frees on a small disk without room: the first waits for disk space now, its place in the queue kept.
    disk_is(1_200_000_000, 3_400_000_000);
    let agents = |n: i64| sw::store::settings::change::set_value("board.agents", &json!(n), &sw::store::settings::lookup::Scope::you()).unwrap();
    agents(2);
    assert_eq!(sw::actions::start::start_queued(&w.socket), vec![d.clone()]);
    agents(1);
    let t = settle(&d);
    assert!(t.s("stage") == "queued" && t.b("diskWait"), "{t:?}");
    assert_eq!(task(&s).s("stage"), "queued");
    let r = sw::shown::states::as_row(&row(&d));
    assert_eq!(sw::shown::states::standing(&r), "waiting for disk space, Q#1");
    assert_eq!(sw::shown::states::standing(&sw::shown::states::as_row(&row(&s))), "waiting for a free slot, Q#2");
    // The board's row says the whole line under its name; the task panel's State line what it waits for.
    let mut bd = open(&w.root, 40);
    let lines = bd.screen(200);
    let at = lines.iter().position(|l| l.contains("Waits for disk")).unwrap();
    assert!(lines[at + 1].contains("Disk nearly full on"), "{}", lines.join("\n"));
    assert!(!lines.iter().any(|l| l.contains("Waits for disk") && l.contains("free slot")), "{}", lines.join("\n"));
    let panel = sw::board::view::panel::panel(&mut bd.s, &bd.theme, &r, 100, false, 0).iter().map(|l| l.text()).collect::<Vec<_>>().join("\n");
    assert!(panel.contains("◷ waiting for disk space, Q#1"), "{panel}");
    // The web board's row: the same line, and its place.
    let rows: Vec<Value> = common::board::web(&w, "board", json!({})).at("list").arr("groups").iter().flat_map(|g| g.arr("rows").iter().cloned()).collect();
    let wd = rows.iter().find(|x| x.s("id") == d).unwrap();
    assert!(wd.at("says").to_string().contains("Disk nearly full on") && wd.s("percent") == "Q#1", "{wd}");
    // `list` and `details`.
    let (_, said) = sw::cli::tasks::command("list", &sw::cli::flags::parse(&["list".to_string()])).unwrap();
    assert!(said.lines().any(|l| l.contains("Waits for disk") && l.contains("· waiting for disk space, Q#1")), "{said}");
    assert!(said.lines().any(|l| l.contains("Waits for a slot") && l.contains("· waiting for a free slot, Q#2")), "{said}");
    let det = sw::actions::finish_line::details(&d).unwrap();
    assert_eq!(det.at("queue").s("words"), "waiting for disk space, Q#1");
    // herdr's sidebar: what it waits for, then its place.
    let e = sw::flow::sidebar_entry::entry_of(&row(&d), "", sw::clock::now());
    assert_eq!((e.place.as_str(), e.progress.as_str()), ("waiting for disk space", "Q#1"));
    let e = sw::flow::sidebar_entry::entry_of(&row(&s), "", sw::clock::now());
    assert_eq!((e.place.as_str(), e.progress.as_str()), ("queued", "Q#2"));
    // doctor: the task beside the disk it waits for.
    let named = sw::store::keys::say_id(&d, &sw::store::keys::repo_keys());
    let on = disk::ask().into_iter().find(|x| !x.waiting.is_empty()).expect("a disk with a waiting task");
    assert_eq!(on.waiting, vec![named]);
}

#[test]
fn doctor_says_which_task_waits_for_room_on_a_disk_and_measures_a_small_disk_as_the_start_does() {
    let _w = world();
    let tmp = |free: u64, waiting: &[&str]| Disk { mount: "/tmp".into(), free, total: 3_400_000_000, holds: vec!["temporary files".into()], waiting: waiting.iter().map(|s| s.to_string()).collect() };
    // A 3.4 GB /tmp is low under its half, as the start check has it: never under doctor's 10 GB, which it cannot have.
    assert_eq!(disk::low_at(3_400_000_000), sw::leftovers::room::need_on(Some(3_400_000_000)));
    assert_eq!(disk::check(&[tmp(2 * GB, &[])]), [Found::ok("2.0 GB free of 3.4 GB on /tmp (temporary files)")]);
    let f = disk::check(&[tmp(1_200_000_000, &["HP-12"])]);
    assert!(f[0].what.starts_with("Only 1.2 GB free on /tmp, which holds temporary files: HP-12 waits for room on /tmp, and new tasks wait too"), "{f:?}");
    // A big disk: 10 GB, or more where board.diskFree asks for more.
    assert_eq!(disk::low_at(100 * GB), disk::LOW);
    sw::store::settings::change::set_value("board.diskFree", &json!(20), &sw::store::settings::lookup::Scope::you()).unwrap();
    assert_eq!(disk::low_at(100 * GB), 20 * GB);
}

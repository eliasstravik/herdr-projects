//! The sweep (crates/flow/src/leftovers/sweep.rs): what tasks that ended left behind goes, and only what is provably
//! the board's: the agents' files for a worktree that is gone, the board's task folders no task has, old build output
//! (what git ignores) in a kept worktree of a done task; never your repos', your home's or a loose folder of /tmp, never
//! one that changed in the last hour; `--dry` only says what would go. It runs when the board opens and once a day; what
//! runs from a folder is stopped before the folder goes.
use crate::common;
use common::*;
use serde_json::json;
use std::path::{Path, PathBuf};
use sw::actions::{filing, finish_line};
use sw::json::*;

fn fill(p: &Path) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, vec![7u8; 1_000_000]).unwrap(); }
fn claude_files(w: &World, tree: &Path) -> PathBuf { w.base.join("tmp").join(format!("claude-{}", sw::agents::scratch::uid())).join(sw::agents::scratch::slug(tree)) }
/// Changed `ms` from now by the board's clock.
fn touched(p: &Path, ms: i64) { let at = std::time::UNIX_EPOCH + std::time::Duration::from_millis((sw::clock::now() + ms) as u64); std::fs::File::open(p).unwrap().set_modified(at).unwrap(); }

#[test]
fn the_sweep_removes_what_ended_tasks_left_and_only_what_is_provably_the_board_s() {
    let w = world();
    let id = to_approval(&w, "Work");
    let tree = PathBuf::from(task(&id).s("worktree"));
    finish_line::approve(&id, None, false).unwrap();
    assert_eq!(settle(&id).s("stage"), "done");
    assert!(!tree.exists());
    let root = tree.parent().unwrap().to_path_buf();
    // left by an agent started without the task's own folder, after its worktree went
    let gone = claude_files(&w, &tree);
    fill(&gone.join("s1/scratchpad/preview.bin"));
    let orphan = sw::temp::board_base().join("PRJ-99"); // a task's folder whose task was removed
    fill(&orphan.join("venv.bin"));
    // never the board's: your repo's, your home's, another repo's worktree folder beside it, a name no task branch has, a loose folder
    let beside = root.parent().unwrap().join(format!("{}-agent", root.file_name().unwrap().to_str().unwrap()));
    std::fs::create_dir_all(&beside).unwrap();
    let keep = [claude_files(&w, &w.root), claude_files(&w, &sw::store::files::user_home()), claude_files(&w, &beside.join("work-3-other")), claude_files(&w, &root.join("notes")), w.base.join("tmp/gtm7-r3-preview")];
    for k in &keep { fill(&k.join("x.bin")); }
    // changed within the hour: perhaps a program that just started
    let fresh = claude_files(&w, &root.join("work-8-fresh"));
    fill(&fresh.join("x.bin"));
    // a task in a folder that holds all of it (your home folder, say) keeps only its own
    sw::actions::filing::add(&w.base, "In the folder above", Default::default()).unwrap();
    sw::clock::advance(2 * 3_600_000);
    touched(&fresh, 0);
    // --dry: what would go, and nothing goes
    let dry = sw::leftovers::sweep::sweep_with(true);
    assert!(sw::leftovers::sweep::say(&dry).starts_with("Would free "), "{dry}");
    assert_eq!(dry.arr("went").len(), 2, "{dry}");
    assert!(gone.exists() && orphan.exists());
    let r = sw::leftovers::sweep::sweep();
    assert!(!gone.exists() && !orphan.exists(), "{r}");
    for k in keep.iter().chain([&fresh]) { assert!(k.exists(), "{}", k.display()); }
    assert!(r.i("freed") >= 2_000_000, "{r}");
    let said = sw::leftovers::sweep::say(&r);
    assert!(said.starts_with("Freed ") && said.contains("(an agent's files for a worktree that is gone)") && said.contains("(temporary files of a task that is gone)"), "{said}");
    assert_eq!(sw::store::files::read_json(&sw::store::files::data_dir().join("sweep.json")).unwrap().at("freed"), r.at("freed"));
    // a second sweep finds nothing
    assert_eq!(sw::leftovers::sweep::say(&sw::leftovers::sweep::sweep()), "Nothing to remove: no leftovers of ended tasks.");
}

#[test]
fn old_build_output_goes_from_a_kept_worktree_of_a_done_task_after_board_old_builds_days_and_only_what_git_ignores() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { ".gitignore": "node_modules/\n/target/\n.next/\n", "docs/target/keep.md": "mine\n" }, "commit": "ignore builds", "note": "Did it." }] }));
    let id = w.add("Builds");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "approval");
    let tree = PathBuf::from(task(&id).s("worktree"));
    std::fs::write(tree.join("unsaved.txt"), "x\n").unwrap();
    for b in ["node_modules/a/i.js", "target/quick/big.bin", "web/.next/cache.bin"] { fill(&tree.join(b)); }
    filing::report(&id, filing::Report { state: "done".into(), ..Default::default() }).unwrap();
    let t = settle(&id);
    assert!(tree.exists() && t.arr("kept").len() == 1, "kept for its unsaved file");
    sw::leftovers::sweep::sweep();
    assert!(tree.join("node_modules").exists(), "not old yet");
    sw::clock::advance(8 * 86_400_000);
    sw::leftovers::sweep::sweep();
    for gone in ["node_modules", "target", "web/.next"] { assert!(!tree.join(gone).exists(), "{gone}"); }
    for kept in ["docs/target/keep.md", "unsaved.txt", ".gitignore"] { assert!(tree.join(kept).exists(), "{kept}"); }
    // 0: never
    fill(&tree.join("node_modules/b.js"));
    sw::store::settings::change::set_value("board.oldBuilds", &json!(0), &sw::store::settings::lookup::Scope::you()).unwrap();
    sw::clock::advance(30 * 86_400_000);
    sw::leftovers::sweep::sweep();
    assert!(tree.join("node_modules/b.js").exists());
}

#[test]
fn the_board_sweeps_when_it_opens_after_an_hour_and_from_the_housekeeping_pass_once_a_day() {
    let _w = world();
    unsafe { std::env::remove_var("HERDR_PROJECTS_NO_SWEEP") };
    let sweeps = || sw::runner::JOBS.with(|j| { let n = j.borrow().iter().filter(|(job, _, _)| job == "sweep").count(); j.borrow_mut().clear(); n });
    sw::leftovers::sweep::at_start();
    assert_eq!(sweeps(), 1);
    sw::leftovers::sweep::at_start(); sw::leftovers::sweep::daily();
    assert_eq!(sweeps(), 0);
    sw::clock::advance(2 * 3_600_000);
    sw::leftovers::sweep::daily();
    assert_eq!(sweeps(), 0, "the pass sweeps once a day");
    sw::leftovers::sweep::at_start();
    assert_eq!(sweeps(), 1, "the board opening, an hour on");
    sw::clock::advance(25 * 3_600_000);
    sw::leftovers::sweep::daily();
    assert_eq!(sweeps(), 1);
    assert_eq!(sw::jobs::run_job("sweep", "-", &[]), 0);
    assert!(sw::store::files::data_dir().join("sweep.json").exists());
    unsafe { std::env::set_var("HERDR_PROJECTS_NO_SWEEP", "1") };
}

#[test]
fn real_programs_what_runs_from_a_task_s_temporary_folder_is_stopped_before_the_folder_goes() {
    let w = world();
    let id = w.add("Server");
    let t = task(&id);
    let tmp = sw::temp::made(&t);
    std::fs::create_dir_all(tmp.join("preview")).unwrap();
    let mut server = std::process::Command::new("sleep").arg("60").current_dir(tmp.join("preview")).spawn().unwrap();
    assert_eq!(sw::leftovers::procs::in_folder(&tmp), vec![server.id() as i32]);
    assert!(sw::leftovers::procs::in_folder(&w.root).is_empty());
    sw::leftovers::free(&t);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while server.try_wait().unwrap().is_none() && std::time::Instant::now() < until { std::thread::sleep(std::time::Duration::from_millis(50)); }
    assert!(server.try_wait().unwrap().is_some(), "stopped");
    assert!(!tmp.exists());
}

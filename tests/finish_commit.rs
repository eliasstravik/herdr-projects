//! The finish line's commit of what the agent left uncommitted, while another Git process holds the worktree's index
//! (issue #214): a lock that goes is waited out, a lock that stays fails and is named. And a new round never shows the
//! failure of the one before while it is checked.
use crate::common;
use common::*;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::{cell::{Cell, RefCell}, rc::Rc};
use sw::json::*;

/// The worker leaves its work uncommitted: the board commits it at the finish line.
fn uncommitted(w: &World) -> String {
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "note": "Did v1." }], "reviewer": [{ "verdict": { "meets": true, "findings": [], "text": "Fine." } }] }));
    let id = w.add("Leaves it uncommitted");
    delegate(&id);
    id
}
/// The index lock of the task's worktree, there now.
fn lock(id: &str) -> PathBuf {
    let tree = PathBuf::from(task(id).t("worktree"));
    let file = tree.join(git(&tree, &["rev-parse", "--git-path", "index.lock"]));
    std::fs::write(&file, "").unwrap();
    file
}
/// `settle`, with the worktree's index locked just as the agent's turn ends (before the board's finish job runs), on
/// the board's fixed clock; `meanwhile` happens at each of the board's waits. The lock file.
fn settle_locked(id: &str, meanwhile: impl Fn(&Path) + 'static) -> (Obj, PathBuf) {
    sw::clock::set(Some(sw::clock::now()));
    let made: Rc<RefCell<Option<PathBuf>>> = Rc::default();
    { let made = made.clone(); sw::clock::meanwhile(move || if let Some(f) = made.borrow().as_ref() { meanwhile(f) }); }
    for _ in 0..12 {
        flush();
        sw::runner::drain_jobs(&|job, jid, extra| { if job == "finish" && made.borrow().is_none() { *made.borrow_mut() = Some(lock(id)); } sw::jobs::run_job(job, jid, extra); flush(); });
        if made.borrow().is_some() { break; }
        let t = task(id);
        if t.at("link").b("pane") { sw::hook::on_pane(&sw::herdr::cli::current_socket(), t.at("link").s("pane")); }
    }
    let file = made.borrow().clone().expect("the agent's turn ended");
    (settle(id), file)
}

#[test]
fn a_lock_that_goes_is_waited_out_and_the_task_goes_on_to_review() {
    let w = world();
    let id = uncommitted(&w);
    let waits = Rc::new(Cell::new(0));
    let (t, _) = { let waits = waits.clone(); settle_locked(&id, move |f| { waits.set(waits.get() + 1); let _ = std::fs::remove_file(f); }) };
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    assert!(t.at("failure").is_null());
    assert_eq!(waits.get(), 1, "one wait, then it committed");
    let tree = Path::new(t.s("worktree"));
    assert_eq!(git(tree, &["status", "--porcelain"]), "");
    assert_eq!(git(tree, &["show", "HEAD:f.txt"]), "v1");
}

#[test]
fn a_lock_that_stays_fails_the_commit_and_names_the_lock_file() {
    let w = world();
    let id = uncommitted(&w);
    let start = sw::clock::now();
    let waits = Rc::new(Cell::new(0));
    let (t, file) = { let waits = waits.clone(); settle_locked(&id, move |_| waits.set(waits.get() + 1)) };
    assert_eq!(t.s("stage"), "failed", "{}", t.s("msg"));
    let f = t.at("failure");
    assert_eq!(f.s("stage"), "commit");
    assert!(f.s("title").contains("index is locked"), "{}", f.s("title"));
    assert!(f.s("next").contains(&file.display().to_string()), "{}", f.s("next"));
    assert!(f.s("detail").contains("Tried 5 times"), "{}", f.s("detail"));
    assert_eq!(waits.get(), 4, "it waited before each new try");
    assert!(sw::clock::now() - start >= 3750, "the waits are on the board's clock");
}

#[test]
fn a_lock_whose_holder_committed_it_all_is_no_failure() {
    let w = world();
    let id = uncommitted(&w);
    // The other Git process was the agent's own commit: when the lock goes, nothing is left to commit.
    let own = id.clone();
    let (t, _) = settle_locked(&id, move |f| { let _ = std::fs::remove_file(f); let tree = PathBuf::from(task(&own).t("worktree")); git(&tree, &["add", "-A"]); git(&tree, &["commit", "-qm", "own"]); });
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(git(Path::new(t.s("worktree")), &["log", "-1", "--format=%s"]), "own");
}

#[test]
fn a_new_round_never_shows_the_failure_of_the_round_before_while_it_is_checked() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.rounds": 2 }), ..Default::default() });
    // The check keeps what the task's record said while it ran, one copy a round, then passes on the second round only.
    let seen = w.base.join("seen");
    std::fs::create_dir_all(&seen).unwrap();
    let rec = format!("{}/$HERDR_PROJECTS_ID/task.json", sw::store::files::tasks_dir().display());
    let cmd = format!("n=$(ls '{0}' | wc -l | tr -d '[:space:]'); cp \"{rec}\" '{0}'/round-$n.json; grep -q good f.txt", seen.display());
    sw::store::settings::change::set_value("check.command", &json!(cmd), &sw::store::settings::lookup::Scope::you()).unwrap();
    w.script(json!({ "worker": [{ "files": { "f.txt": "bad\n" }, "commit": "bad", "note": "Did it." }, { "files": { "f.txt": "good\n" }, "commit": "good", "note": "Fixed it." }],
        "reviewer": [{ "verdict": { "meets": true, "findings": [], "text": "Fine." } }] }));
    let id = w.add("Fails its check once");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    let round = |n: usize| -> Obj { serde_json::from_str(&read(&seen.join(format!("round-{n}.json")))).unwrap() };
    assert_eq!(round(0).s("stage"), "checking");
    // The second round's check ran after the first one's failure was sent to the agent: that failure is gone.
    let second = round(1);
    assert_eq!(second.s("stage"), "checking");
    assert!(second.at("failure").is_null(), "{}", second.at("failure"));
    assert!(t.at("failure").is_null());
}

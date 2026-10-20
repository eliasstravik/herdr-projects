//! Nothing is changed before a plan's yes: the worktree is read-only while the plan is written and while it waits (also
//! for files git ignores); a change that gets in all the same is put back and kept as a patch; in your own checkout a
//! change the board may not undo holds the build until it is undone. And every way out of the step takes the lock off:
//! a stop, a delegate with the plan taken off, and your yes to a plan whose step failed.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::json::*;
use sw::plan as P;
use sw::plan::lock::{is_locked, unlock};

const PLAN: &str = "# Plan\n\n1. Add f.txt with one line.\n2. Run the check.\n";
fn approve(id: &str) { sw::actions::finish_line::approve(id, None, false).unwrap(); }
fn file(tree: &str, rel: &str) -> Option<String> { std::fs::read_to_string(Path::new(tree).join(rel)).ok() }

#[test]
fn nothing_is_changed_before_the_yes_what_gets_in_is_put_back_and_a_change_the_board_may_not_undo_holds_the_build() {
    let w = world_with(Opts { repo: true, settings: json!({ "plan.approve": "you", "worker.agent": "codex", "review.when": "never" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap") };
    w.patch(|s| s["pids"] = json!(true));
    std::fs::write(w.root.join(".gitignore"), "ignored.txt\nkept.log\n").unwrap();
    git(&w.root, &["add", "-A"]); git(&w.root, &["commit", "-qm", "ignore"]);
    // An agent with full access that writes while it plans: nothing it does gets in, during its turn or after.
    w.script(json!({ "worker": [{ "files": { "a.txt": "changed before yes\n", "ignored.txt": "early\n", "new.txt": "x\n" }, "commit": "early", "refused": true, "plan": PLAN, "note": "Written." }, { "plan": PLAN, "note": "Written." }, { "status": "working" }] }));
    let id = w.add("Work");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "plan", "{}", t.at("failure"));
    assert!(!t.at("plan").b("approvedAt"));
    assert!(!t.at("plan").b("undone"), "nothing had to be put back: nothing got in");
    assert!(w.pane_of(&t).at("agent").arr("args").iter().any(|a| text(a) == "--dangerously-bypass-approvals-and-sandbox"));
    let tree = t.t("worktree");
    let at = |rel: &str| Path::new(&tree).join(rel);
    assert_eq!((file(&tree, "a.txt"), file(&tree, "ignored.txt"), file(&tree, "new.txt")), (Some("one\n".to_string()), None, None));
    assert_eq!(git(Path::new(&tree), &["rev-parse", "HEAD"]), t.s("baseSha"));
    assert!(is_locked(&t));
    assert!(std::fs::write(at("a.txt"), "x").is_err());
    assert!(common::worktrees::git(Path::new(&tree), &["commit", "--allow-empty", "-qm", "x"]).is_err());
    // Somebody takes the lock off and changes things: a tracked file, a new one, a commit, and files git ignores.
    unlock(&t);
    std::fs::write(at("a.txt"), "changed\n").unwrap();
    git(Path::new(&tree), &["add", "-A"]); git(Path::new(&tree), &["commit", "-qm", "early"]);
    std::fs::write(at("new.txt"), "x\n").unwrap(); std::fs::write(at("ignored.txt"), "early content\n").unwrap();
    let mut changed = sw::plan::untouched::changed(&task(&id)).unwrap_or_default();
    changed.sort();
    assert_eq!(changed, vec!["(1 commit added)", "a.txt", "ignored.txt (ignored by git, added)", "new.txt"]);
    sw::actions::talk::tell(&id, "Once more.", false).unwrap(); // the plan's next turn ends
    let t = settle(&id);
    assert_eq!(t.s("stage"), "plan", "{}", t.at("failure"));
    assert_eq!(t.at("plan").at("undone").i("n"), 4);
    assert!(t.s("msg").contains("4 changes in the repo made before the plan was approved were put back"), "{}", t.s("msg"));
    assert_eq!((file(&tree, "a.txt"), file(&tree, "new.txt"), file(&tree, "ignored.txt")), (Some("one\n".to_string()), None, None));
    assert_eq!(git(Path::new(&tree), &["rev-parse", "HEAD"]), t.s("baseSha"));
    assert!(read(&P::plan_folder(&t).join(sw::plan::untouched::PATCH)).contains("+changed"), "what was thrown away is kept beside the plan");
    assert!(is_locked(&t), "and it is read-only again");
    let mut b = open(&w.root, 90); b.press(&["alt+t"]);
    assert!(regex::Regex::new(r"Put back\s+4 changes made in the repo before the yes").unwrap().is_match(&b.text()), "{}", b.text());
    // While it waits, the same: put back right before the build starts, which then goes on in a writable worktree.
    unlock(&t); std::fs::write(at("kept.log"), "late\n").unwrap();
    approve(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert!(t.at("plan").b("built"));
    assert_eq!(file(&tree, "kept.log"), None);
    assert!(!is_locked(&t));
    std::fs::write(at("f.txt"), "built\n").unwrap();
    // A task in your own checkout: the board undoes nothing there, and nothing is built on a change.
    w.script(json!({ "worker": [{ "plan": PLAN, "note": "Written." }, { "status": "working" }] }));
    let own = w.add("In the checkout");
    sw::store::tasks::patch(&own, |t| t.put("startedOnce", true)).unwrap(); // (how a task comes to work in your checkout)
    delegate(&own);
    let u = settle(&own);
    assert_eq!(u.s("stage"), "plan", "{}", u.at("failure"));
    assert!(!u.b("worktree"));
    assert_eq!(u.at("plan").at("locked"), &json!(false));
    assert!(!is_locked(&obj(json!({ "worktree": w.root_str() }))));
    // Its writer runs in the mount too, your checkout read-only inside it; your own files keep their permissions.
    assert_eq!(w.pane_of(&u).at("agent").at("boxed"), &json!(true));
    let root = w.root_str();
    assert!(sw::plan::sandbox::boxed_command(&u, "x", "codex", &[]).contains(&format!("--ro-bind {root} {root} --die-with-parent -- codex")));
    std::fs::write(w.root.join("a.txt"), "changed while waiting\n").unwrap();
    assert_eq!(sw::plan::untouched::put_back(&u), None);
    approve(&own);
    let u = settle(&own);
    assert_eq!(u.s("stage"), "failed");
    assert!(!u.at("plan").b("built"));
    assert_eq!(u.at("failure").s("title"), "The repo was changed before the plan was approved");
    assert!(u.at("failure").s("detail").contains("- a.txt"), "{}", u.at("failure"));
    assert_eq!(prompts(&w, u.at("link").s("pane")).len(), 1, "the builder was told nothing");
    assert_eq!(read(&w.root.join("a.txt")), "changed while waiting\n");
    std::fs::write(w.root.join("a.txt"), "one\n").unwrap(); // undone: the start is tried again, and the yes still stands
    sw::actions::finish_line::retry(&own).unwrap();
    let u = settle(&own);
    assert_eq!(u.s("stage"), "working", "{}", u.at("failure"));
    assert!(u.at("plan").b("built"));
    assert_eq!(u.at("plan").s("approvedBy"), "you");
}

#[test]
fn a_stop_a_delegate_with_the_plan_taken_off_and_a_yes_to_a_failed_plan_step_each_leave_the_worktree_writable() {
    let w = world_with(Opts { repo: true, settings: json!({ "plan.approve": "you", "worker.agent": "codex", "review.when": "never" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap") };
    w.patch(|s| s["pids"] = json!(true));
    w.script(json!({ "worker": [{ "plan": PLAN, "note": "Written." }, { "status": "working" }, { "status": "working" }] }));
    let id = w.add("Work");
    delegate(&id);
    let t = settle(&id);
    assert!(t.s("stage") == "plan" && is_locked(&t), "{}", t.at("failure"));
    // Stopped in its plan step: writable, and the task says so.
    sw::actions::talk::stop(&id).unwrap();
    let t = task(&id);
    assert_eq!((t.s("stage"), t.at("plan").at("locked"), is_locked(&t)), ("stopped", &json!(false), false));
    // Delegated again, the step goes on: read-only once more, its writer in the mount.
    delegate(&id);
    let t = settle(&id);
    assert!(is_locked(&t) && t.at("plan").b("locked") && !t.at("plan").b("approvedAt"), "{}", t.at("plan"));
    assert_eq!(w.pane_of(&t).at("agent").at("boxed"), &json!(true));
    // The plan taken off, and delegated again: the plan written is built as it stands, writable.
    sw::actions::talk::stop(&id).unwrap();
    sw::store::settings::change::set_value("plan.approve", &json!("none"), &sw::store::settings::lookup::Scope::task(&task(&id))).unwrap();
    delegate(&id);
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.at("plan").s("approvedBy")), ("working", "nobody"), "{}", t.at("failure"));
    assert!(t.at("plan").b("built") && !is_locked(&t) && t.at("plan").at("locked") == &json!(false));
    assert!(!w.pane_of(&t).at("agent").b("boxed"));
    // What an older version left behind (#229): a plan written, its step failed, the worktree read-only. Your yes builds it.
    w.script(json!({ "worker": [{ "plan": PLAN, "note": "Written." }, { "status": "working" }] }));
    let old = w.add("Stuck");
    delegate(&old);
    assert_eq!(settle(&old).s("stage"), "plan");
    sw::store::tasks::patch(&old, |t| { t.put("stage", "failed"); t.put("failure", json!({ "stage": "plan", "title": "The plan's review failed", "detail": "The reviewer could not check the plan: sandbox-exec" })); }).unwrap();
    assert!(is_locked(&task(&old)));
    approve(&old);
    let t = settle(&old);
    assert_eq!((t.s("stage"), t.at("plan").s("approvedBy")), ("working", "you"), "{}", t.at("failure"));
    assert!(!is_locked(&t) && t.at("plan").at("locked") == &json!(false) && t.at("failure").is_null());
}

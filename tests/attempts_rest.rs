//! What tests/attempts.rs left of the old suite: the check as a filter keeps the others' worktrees and branches (also
//! one whose only unsaved file is ignored), only the chosen attempt is reviewed, each attempt's pane in the task's one
//! space, and a check changed while the task waits for your pick is run again on the chosen one.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::json::*;
use sw::store::reviews::review_history;

fn like(text: &str, re: &str) { assert!(regex::Regex::new(re).unwrap().is_match(text), "{re}\n{text}"); }

#[test]
fn the_check_filter_keeps_the_others_worktree_and_branch_and_an_ignored_unsaved_file_keeps_an_attempt() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never", "review.rounds": 0, "check.command": "grep -q good f.txt", "board.agents": 2 }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "bad\n" }, "commit": "a", "note": "A." }, { "files": { "f.txt": "good\n" }, "commit": "b", "note": "B." }] }));
    let id = w.add("One passes");
    delegate_with(&id, json!({ "attempts.by": 2 }));
    let t = settle_all(&id);
    assert_eq!(t.at("picked").s("letter"), "B", "{}", t.at("failure"));
    let a = kid(&id, "A");
    assert!(Path::new(a.s("worktree")).exists() && !a.at("link").b("pane"), "the other's pane closes, its worktree stays: {a:?}");
    // one whose only such file is ignored, which a plain `git status` does not name, whatever its date, stays too
    let dir = w.base.join("loser");
    git(&w.root, &["worktree", "add", "-q", "-b", "work/9a-x", &dir.display().to_string()]);
    std::fs::write(w.root.join(".git").join("info").join("exclude"), "draft.txt\n").unwrap();
    std::fs::write(dir.join("draft.txt"), "a draft\n").unwrap();
    let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    std::fs::File::options().write(true).open(dir.join("draft.txt")).unwrap().set_modified(old).unwrap();
    assert_eq!(git(&dir, &["status", "--porcelain"]), "");
    let task9 = obj(json!({ "id": "proj/9" }));
    let c = obj(json!({ "id": "proj/9a", "letter": "A", "worktree": dir.display().to_string(), "branch": "work/9a-x", "triedAt": sw::clock::now() }));
    let kept = sw::attempts::adopt::discard(&task9, &c);
    assert!(kept.contains("attempt A: files that were never committed"), "{kept}");
    assert_eq!(read(&dir.join("draft.txt")), "a draft\n");
    assert!(!git(&w.root, &["branch", "--list", "work/9a-x"]).is_empty());
    // with nothing but what was committed in it, the attempt goes, whole
    std::fs::remove_file(dir.join("draft.txt")).unwrap();
    assert_eq!(sw::attempts::adopt::discard(&task9, &c), "");
    assert!(!dir.exists() && git(&w.root, &["branch", "--list", "work/9a-x"]).is_empty());
    // removing the task never takes unsaved work with it: the attempt's branch stays beside its worktree
    std::fs::write(Path::new(a.s("worktree")).join("unsaved.txt"), "mine\n").unwrap();
    sw::actions::archive::clear(&id).unwrap();
    like(&git(&w.root, &["branch", "--list", "work/1a-*"]), r"work/1a-one-passes");
}

#[test]
fn in_a_council_only_the_chosen_attempt_is_reviewed_in_full() {
    let w = world_with(Opts { repo: true, settings: json!({ "merge.by": "itself", "merge.pr": false }), ..Default::default() });
    w.script(json!({ "worker": [step(1), step(2)], "picker": [{ "comparison": { "pick": "b", "why": "B is the smallest change." } }] }));
    let id = w.add("Council");
    delegate_with(&id, json!({ "attempts.by": "codex/gpt-5.5, pi", "attempts.pick": "reviewer" }));
    let t = settle_all(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(review_history(&t).len(), 1, "only the chosen attempt was reviewed in full");
}

#[test]
fn each_attempt_has_a_tiled_pane_in_the_tasks_one_space_and_a_check_changed_while_it_waits_is_run_again_on_the_chosen_one() {
    let w = world_with(Opts { repo: true, settings: json!({ "merge.pr": false, "merge.by": "itself", "check.command": "true" }), ..Default::default() });
    w.script(json!({ "worker": [step(1), step(2), step(3)], "picker": [{ "comparison": { "pick": "B", "why": "B handles the empty case.", "attempts": [{ "letter": "A", "note": "Misses the empty case." }, { "letter": "B", "note": "Complete." }] } }] }));
    let id = w.add("Greet");
    delegate_with(&id, json!({ "attempts.by": 3 }));
    let mut b = open(&w.root, 64);
    settle_all(&id);
    b.load();
    let cs = kids(&id);
    assert_eq!(cs.len(), 3, "task: {:?}; calls: {:?}", task(&id), calls(&w));
    let space = cs[0].at("link").t("workspace");
    let state = w.state();
    let tabs: Vec<_> = state.arr("tabs").iter().filter(|x| x.s("workspace_id") == space).collect();
    assert_eq!(tabs.len(), 1, "attempts share the task's tab; the Picker pane has closed");
    let panes: Vec<_> = state.arr("panes").iter().filter(|x| x.s("workspace_id") == space).collect();
    assert!(panes.iter().any(|p| p.t("pane_id") == cs[0].at("link").t("pane")));
    assert!(panes.iter().any(|p| p.t("pane_id") == cs[1].at("link").t("pane")));
    assert!(panes.iter().any(|p| p.t("pane_id") == cs[2].at("link").t("pane")));
    assert_eq!(sw::store::tasks::list(Default::default()).len(), 1, "no task of its own for an attempt");
    assert!(kid(&id, "A").at("diff").s("file").ends_with("/1a/artifacts/diff.txt"), "{}", kid(&id, "A").at("diff"));
    let shown = b.text();
    assert!(!shown.contains("Asks for your OK"), "{shown}");
    like(&shown, r"  C +claude");
    // ctrl+n moves on to the next attempt
    b.press(&["ctrl+n"]);
    let r = b.s.current().cloned().unwrap();
    assert_eq!(sw::board::update::actions::attempt_sel(&b.s, &r), "C");
    // the check was changed while it waited: the chosen attempt is checked again, with the new command
    sw::store::settings::change::set_value("check.command", &json!("touch \"$HERDR_PROJECTS_REPO/.git/checked-again\""), &sw::store::settings::lookup::Scope::task(&task(&id))).unwrap();
    b.press(&["ctrl+l", "y"]);
    assert_eq!(task(&id).at("picked").s("letter"), "C");
    sw::store::settings::change::set_value("merge.by", &json!("you"), &sw::store::settings::lookup::Scope::task(&task(&id))).unwrap();
    let done = settle_all(&id);
    assert_eq!(done.s("stage"), "approval", "{}", done.at("failure"));
    assert_eq!(review_history(&done).len(), 1);
    assert!(w.root.join(".git").join("checked-again").exists(), "the new check ran");
    b.load();
    let shown = b.text();
    assert!(shown.contains("Status ─"), "beside the list the part is short: {shown}");
    like(&shown, r"Attempts +A claude, B claude, C claude");
    assert!(shown.contains("C was chosen"), "{shown}");
    assert_eq!(row(&id).arr("tries").len(), 3);
}

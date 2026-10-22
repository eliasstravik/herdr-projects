//! A task started from a named branch (`workspace.from develop`) with no pull request (`merge.pr` off, no GitHub): after
//! your yes the board pushes the reviewed work to that branch at the remote and touches no checkout of yours. Two features
//! in one repo land on their own branches; a push git refuses because someone pushed first sends the task back to catch
//! up and loses nothing; a push origin refuses waits for you; `merge.by itself` and a failed review keep their gate.
use crate::common;
use super::they_push;
use common::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use sw::actions::finish_line;
use sw::json::*;

fn push_world(extra: Value) -> World {
    let mut s = json!({ "review.when": "never", "merge.pr": false });
    for (k, v) in extra.as_object().cloned().unwrap_or_default() { s[k] = v; }
    world_with(Opts { repo: true, origin: true, settings: s, ..Default::default() })
}
fn bare(w: &World) -> PathBuf { w.base.join("origin.git") }
/// The files a branch holds at the remote, sorted, with commas between.
fn at_remote(w: &World, branch: &str) -> String { git(&bare(w), &["ls-tree", "-r", "--name-only", branch]).replace('\n', ",") }
fn work(n: i64, file: &str) -> Value { json!({ "files": { file: format!("{n}\n") }, "commit": format!("v{n}"), "note": format!("Did v{n}.") }) }
fn verdict(findings: Value) -> Value { json!({ "verdict": { "meets": true, "text": "Looked at it.", "findings": findings } }) }
/// A task added and delegated from `from`, taken to its approval.
fn waiting(w: &World, title: &str, from: &str) -> String {
    let id = w.add(title);
    delegate_with(&id, json!({ "workspace.from": from }));
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    id
}
fn land(id: &str) -> Obj { finish_line::approve(id, None, false).unwrap(); settle(id) }
fn hook(dir: &Path, name: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    let h = dir.join("hooks").join(name);
    std::fs::create_dir_all(h.parent().unwrap()).unwrap();
    std::fs::write(&h, body).unwrap();
    std::fs::set_permissions(&h, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn it_lands_on_the_named_branch_at_the_remote_after_your_yes_and_never_touches_your_checkout() {
    let w = push_world(json!({}));
    let d1 = they_push(&w, "develop", "d.txt", "one\n");
    w.script(json!({ "worker": [work(1, "f.txt")] }));
    let id = waiting(&w, "On develop", "develop");
    let t = task(&id);
    let tree = t.t("worktree");
    assert!(!t.b("pr")); assert_eq!(sw::sync::lands_on(&t), "develop");
    assert_eq!(t.s("syncSeen"), d1, "kept up with develop at the remote though merge.pr is off");
    assert!(!gh_log(&w).contains("pr "), "{}", gh_log(&w));
    assert_eq!(git(&bare(&w), &["rev-parse", "develop"]), d1, "nothing lands before your yes");
    // your checkout is on another branch, with unsaved files: the push needs neither to be otherwise
    git(&w.root, &["checkout", "-q", "-b", "mine"]);
    std::fs::write(w.root.join("mine.txt"), "unsaved\n").unwrap();
    let (mine, main_at_remote) = (git(&w.root, &["rev-parse", "HEAD"]), git(&bare(&w), &["rev-parse", "main"]));
    let t = land(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.s("msg").contains("Merged into develop"), "{}", t.s("msg"));
    assert_eq!(at_remote(&w, "develop"), "a.txt,d.txt,f.txt");
    let tip = git(&bare(&w), &["rev-parse", "develop"]);
    assert_eq!(tip, t.s("mergeCommit"));
    assert_eq!(git(&bare(&w), &["rev-parse", &format!("{tip}^")]), d1, "squashed onto develop as it stood");
    assert_eq!(git(&bare(&w), &["rev-parse", "main"]), main_at_remote);
    assert_eq!((git(&w.root, &["branch", "--show-current"]), git(&w.root, &["rev-parse", "HEAD"])), ("mine".to_string(), mine));
    assert!(w.root.join("mine.txt").exists() && !w.root.join("f.txt").exists());
    assert_eq!(git(&w.root, &["rev-parse", "refs/remotes/origin/develop"]), tip, "the push brought origin's copy here along");
    assert!(!Path::new(&tree).exists(), "cleaned up");
    assert!(git(&w.root, &["branch", "--list", "work/*"]).is_empty());
    assert!(!gh_log(&w).contains("pr "), "{}", gh_log(&w));
}

#[test]
fn a_second_task_of_a_feature_sees_the_first_and_two_features_in_one_repo_never_cross() {
    let w = push_world(json!({}));
    they_push(&w, "feat/A", "base-a.txt", "a\n");
    they_push(&w, "feat/B", "base-b.txt", "b\n");
    w.script(json!({ "worker": [work(1, "a1.txt"), work(1, "b1.txt"), work(1, "a2.txt"), work(1, "a3.txt")] }));
    // three tasks wait at once; the two of feature A started from the same commit
    let a1 = waiting(&w, "A one", "feat/A");
    let b1 = waiting(&w, "B one", "feat/B");
    let a2 = waiting(&w, "A two", "feat/A");
    assert_eq!(land(&a1).s("stage"), "done");
    assert_eq!(land(&b1).s("stage"), "done");
    // A's second task catches up with the first one's landing, is checked again and lands under the yes it was given
    let t = land(&a2);
    assert_eq!(t.s("stage"), "done", "{} {}", t.at("failure"), t.s("msg"));
    // a task that starts later starts from what both landed
    let a3 = waiting(&w, "A three", "feat/A");
    let tree = task(&a3).t("worktree");
    assert!(Path::new(&tree).join("a1.txt").exists() && Path::new(&tree).join("a2.txt").exists() && !Path::new(&tree).join("b1.txt").exists());
    assert_eq!(land(&a3).s("stage"), "done");
    assert_eq!(at_remote(&w, "feat/A"), "a.txt,a1.txt,a2.txt,a3.txt,base-a.txt");
    assert_eq!(at_remote(&w, "feat/B"), "a.txt,b1.txt,base-b.txt");
    assert_eq!(at_remote(&w, "main"), "a.txt");
    assert_eq!(git(&w.root, &["branch", "--show-current"]), "main", "your checkout stays where it was");
}

#[test]
fn a_push_git_refuses_because_someone_pushed_first_sends_the_task_back_to_catch_up_and_loses_nothing() {
    let w = push_world(json!({}));
    they_push(&w, "develop", "d.txt", "one\n");
    // the other person's clone has its commit ready; it is pushed in the middle of the board's push, once
    let other = w.base.join("racer");
    git(&w.base, &["clone", "-q", &bare(&w).display().to_string(), &other.display().to_string()]);
    git(&other, &["checkout", "-q", "develop"]);
    std::fs::write(other.join("late.txt"), "late\n").unwrap();
    git(&other, &["add", "-A"]); git(&other, &["commit", "-qm", "late"]);
    let flag = w.base.join("raced");
    hook(&w.root.join(".git"), "pre-push", &format!("#!/bin/sh\n[ -e \"{f}\" ] && exit 0\ntouch \"{f}\"\n(unset GIT_DIR GIT_INDEX_FILE GIT_WORK_TREE GIT_PREFIX; cd \"{o}\" && git push -q origin HEAD:develop)\n", f = flag.display(), o = other.display()));
    w.script(json!({ "worker": [work(1, "f.txt")] }));
    let id = waiting(&w, "Races", "develop");
    let t = land(&id);
    assert!(flag.exists(), "the other push happened in the middle of ours");
    // what the other person pushed is still there, and so is the task's work on top of it
    assert!(at_remote(&w, "develop").contains("late.txt"), "{}", at_remote(&w, "develop"));
    assert_eq!(t.s("stage"), "done", "{} {}", t.at("failure"), t.s("msg"));
    assert_eq!(at_remote(&w, "develop"), "a.txt,d.txt,f.txt,late.txt");
    assert_eq!(git(&bare(&w), &["log", "--format=%s", "-3", "develop"]), "Races\nlate\ndevelop: d.txt", "ours on top of theirs, in one line");
}

#[test]
fn a_push_origin_refuses_for_another_reason_waits_for_you_with_gits_words_and_goes_through_once_it_takes_it() {
    let w = push_world(json!({}));
    let d1 = they_push(&w, "develop", "d.txt", "one\n");
    hook(&bare(&w), "pre-receive", "#!/bin/sh\necho 'develop is protected' >&2\nexit 1\n");
    w.script(json!({ "worker": [work(1, "f.txt")] }));
    let id = waiting(&w, "Refused", "develop");
    let t = land(&id);
    assert_eq!((t.s("stage"), t.at("failure").s("title")), ("approval", "Could not push to develop"), "{}", t.at("failure"));
    assert!(t.at("failure").s("detail").contains("develop is protected"), "{}", t.at("failure"));
    assert_eq!(git(&bare(&w), &["rev-parse", "develop"]), d1);
    assert!(Path::new(t.s("worktree")).exists(), "nothing is cleaned up that did not land");
    std::fs::remove_file(bare(&w).join("hooks/pre-receive")).unwrap();
    let t = land(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(at_remote(&w, "develop"), "a.txt,d.txt,f.txt");
}

#[test]
fn merge_by_itself_lands_a_passing_task_by_a_push_and_a_failed_review_still_asks() {
    let w = push_world(json!({ "review.when": "always", "merge.by": "itself", "review.rounds": 1 }));
    let d1 = they_push(&w, "develop", "d.txt", "one\n");
    w.script(json!({ "worker": [work(1, "f.txt")], "reviewer": [verdict(json!([{ "severity": "minor", "subject": "A name", "where": "f.txt", "proof": "read it" }]))] }));
    let ok = w.add("Passes");
    delegate_with(&ok, json!({ "workspace.from": "develop" }));
    let t = settle(&ok);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(at_remote(&w, "develop"), "a.txt,d.txt,f.txt");
    // a critical finding that outlasts its rounds is yours to decide: nothing is pushed, however often the board looks
    let before = git(&bare(&w), &["rev-parse", "develop"]);
    assert_ne!(before, d1);
    let crit = verdict(json!([{ "severity": "critical", "subject": "Deletes the data", "where": "g.txt", "proof": "ran it" }]));
    w.script(json!({ "worker": [work(1, "g.txt"), work(2, "g.txt")], "reviewer": [crit.clone(), crit] }));
    let bad = w.add("Fails");
    delegate_with(&bad, json!({ "workspace.from": "develop" }));
    let mut t = settle(&bad);
    for _ in 0..2 { if t.s("stage") != "approval" { t = settle(&bad); } }
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert_eq!(t.at("review").at("pass"), &json!(false));
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let t = settle(&bad);
    assert_eq!(t.s("stage"), "approval");
    assert_eq!(git(&bare(&w), &["rev-parse", "develop"]), before);
}

#[test]
fn merge_method_shapes_what_is_pushed_a_merge_commit_or_the_reviewed_commits_as_they_are() {
    let w = push_world(json!({}));
    let d1 = they_push(&w, "develop", "d.txt", "one\n");
    w.script(json!({ "worker": [work(1, "m.txt"), work(1, "r.txt")] }));
    let m = w.add("Merge commit");
    delegate_with(&m, json!({ "workspace.from": "develop", "merge.method": "merge" }));
    assert_eq!(settle(&m).s("stage"), "approval");
    let t = land(&m);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    let tip = git(&bare(&w), &["rev-parse", "develop"]);
    assert_eq!(git(&bare(&w), &["rev-list", "--parents", "-n", "1", &tip]).split(' ').count(), 3, "a merge of develop and the task's branch");
    assert_eq!(at_remote(&w, "develop"), "a.txt,d.txt,m.txt");
    let r = w.add("Rebase");
    delegate_with(&r, json!({ "workspace.from": "develop", "merge.method": "rebase" }));
    let reviewed = { assert_eq!(settle(&r).s("stage"), "approval"); git(Path::new(task(&r).s("worktree")), &["rev-parse", "HEAD"]) };
    assert_eq!(land(&r).s("stage"), "done");
    assert_eq!(git(&bare(&w), &["rev-parse", "develop"]), reviewed, "the reviewed commit itself, a fast-forward of develop");
    assert_eq!(at_remote(&w, "develop"), "a.txt,d.txt,m.txt,r.txt");
    assert_ne!(tip, d1);
}

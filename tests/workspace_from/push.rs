//! A task started from a named branch with `merge.pr` off (`workspace.from develop`): it starts, and lands by a push of
//! a commit built from the reviewed one to that branch at origin, your checkout untouched. Tasks on one branch build on
//! each other, two branches each move on their own, origin's main named lands there too, a branch that moved is caught
//! up with and checked again, `merge.method` and `merge.by` are kept, and a push origin refuses waits for you with
//! git's own words.
use crate::common;
use super::they_push;
use common::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use sw::actions::finish_line;
use sw::json::*;
use sw::store::settings::change::set_value;
use sw::store::settings::lookup::Scope;

fn push_world(more: Value) -> World {
    let mut s = json!({ "review.when": "never", "merge.pr": false });
    for (k, v) in more.as_object().cloned().unwrap_or_default() { s[k] = v; }
    world_with(Opts { repo: true, origin: true, settings: s, repo_settings: Some(json!({ "check.command": "echo run >> \"$HOME/checks.log\"" })) })
}
fn bare(w: &World) -> PathBuf { w.base.join("origin.git") }
fn origin(w: &World, b: &str) -> String { git(&bare(w), &["rev-parse", &format!("refs/heads/{b}")]) }
fn has(w: &World, b: &str, file: &str) -> Option<String> { common::worktrees::git(&bare(w), &["show", &format!("{b}:{file}")]).ok() }
fn checks(w: &World) -> usize { read(&w.base.join("home/checks.log")).lines().count() }
fn from(v: &str) { set_value("workspace.from", &json!(v), &Scope::you()).unwrap(); }
fn work(file: &str) -> Value { json!({ "files": { file: format!("{file}\n") }, "commit": file, "note": "Did it." }) }
fn approved(id: &str) -> Obj { finish_line::approve(id, None, false).unwrap(); settle(id) }
fn verdict(findings: Value) -> Value { json!({ "verdict": { "meets": true, "text": "Looked at it.", "findings": findings } }) }
fn hook(dir: &Path, name: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    let f = dir.join("hooks").join(name);
    std::fs::write(&f, format!("#!/bin/sh\n{body}")).unwrap();
    std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn it_starts_from_the_branch_lands_there_by_a_push_and_the_next_task_builds_on_it() {
    let w = push_world(json!({}));
    let d1 = they_push(&w, "develop", "d.txt", "one\n");
    from("develop");
    // your checkout: on a branch of your own, with unsaved work
    git(&w.root, &["checkout", "-q", "-b", "mine"]);
    std::fs::write(w.root.join("a.txt"), "yours\n").unwrap(); std::fs::write(w.root.join("new.txt"), "untracked\n").unwrap();
    let mine = git(&w.root, &["rev-parse", "HEAD"]);
    let main = origin(&w, "main");
    w.script(json!({ "worker": [work("f.txt")] }));
    let id = to_approval(&w, "On develop");
    let t = task(&id);
    assert_eq!((t.s("from"), t.s("baseSha")), ("origin/develop", d1.as_str()));
    assert_eq!(sw::sync::lands_on(&t), "develop"); assert!(!t.b("pr"));
    let (tree, branch) = (PathBuf::from(t.s("worktree")), t.t("branch"));
    let t = approved(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    let landed = origin(&w, "develop");
    assert_eq!((t.s("mergeCommit"), t.s("msg")), (landed.as_str(), "Merged into develop at origin."));
    assert_eq!(git(&bare(&w), &["rev-parse", "develop^"]), d1, "squashed onto develop: one commit");
    assert_eq!(has(&w, "develop", "f.txt").as_deref(), Some("f.txt"));
    assert!(git(&bare(&w), &["log", "-1", "--format=%B", "develop"]).starts_with("On develop\n\nTask: "));
    assert_eq!(origin(&w, "main"), main, "main is untouched");
    // your checkout is as you left it
    assert_eq!((git(&w.root, &["symbolic-ref", "--short", "HEAD"]), git(&w.root, &["rev-parse", "HEAD"])), ("mine".into(), mine));
    assert_eq!((read(&w.root.join("a.txt")), read(&w.root.join("new.txt"))), ("yours\n".into(), "untracked\n".into()));
    assert!(!w.root.join("f.txt").exists());
    // cleaned up, and no GitHub was asked anything
    assert!(!tree.exists());
    assert!(common::worktrees::git(&w.root, &["rev-parse", "--verify", "-q", &format!("refs/heads/{branch}")]).is_err());
    assert_eq!(gh_log(&w), "");
    // the next task on the branch starts from that work, and lands on top of it
    w.script(json!({ "worker": [work("g.txt")] }));
    let next = to_approval(&w, "Next on develop");
    let t = task(&next);
    assert_eq!(t.s("baseSha"), landed); assert_eq!(read(&Path::new(t.s("worktree")).join("f.txt")), "f.txt\n");
    let t = approved(&next);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(git(&bare(&w), &["rev-parse", "develop^"]), landed);
    assert_eq!((has(&w, "develop", "f.txt"), has(&w, "develop", "g.txt")), (Some("f.txt".into()), Some("g.txt".into())));
}

#[test]
fn two_feature_branches_at_once_each_push_moves_only_its_own_branch() {
    let w = push_world(json!({}));
    let (a0, b0) = (they_push(&w, "feature-a", "a0.txt", "a\n"), they_push(&w, "feature-b", "b0.txt", "b\n"));
    let (main, mine) = (origin(&w, "main"), git(&w.root, &["rev-parse", "main"]));
    let mut ids = vec![];
    for (b, file) in [("feature-a", "fa.txt"), ("feature-b", "fb.txt")] {
        w.script(json!({ "worker": [work(file)] }));
        let id = w.add(&format!("On {b}"));
        delegate_with(&id, json!({ "workspace.from": b }));
        assert_eq!(settle(&id).s("stage"), "approval", "{}", task(&id).at("failure"));
        ids.push(id);
    }
    for id in &ids { assert_eq!(approved(id).s("stage"), "done", "{}", task(id).at("failure")); }
    assert_eq!((git(&bare(&w), &["rev-parse", "feature-a^"]), git(&bare(&w), &["rev-parse", "feature-b^"])), (a0, b0));
    assert_eq!((has(&w, "feature-a", "fa.txt").is_some(), has(&w, "feature-a", "fb.txt").is_some()), (true, false));
    assert_eq!((has(&w, "feature-b", "fb.txt").is_some(), has(&w, "feature-b", "fa.txt").is_some()), (true, false));
    assert_eq!((origin(&w, "main"), git(&w.root, &["rev-parse", "main"])), (main, mine), "main is untouched");
}

#[test]
fn a_named_branch_that_is_origins_main_lands_there_too_never_in_your_checkout() {
    let w = push_world(json!({}));
    let d1 = they_push(&w, "develop", "d.txt", "one\n");
    // develop is origin's main branch, and your checkout is on main
    git(&bare(&w), &["symbolic-ref", "HEAD", "refs/heads/develop"]);
    git(&w.root, &["fetch", "-q", "origin"]); git(&w.root, &["remote", "set-head", "origin", "develop"]);
    from("develop");
    let mine = git(&w.root, &["rev-parse", "HEAD"]);
    w.script(json!({ "worker": [work("f.txt")] }));
    let id = to_approval(&w, "On origin's main");
    let t = task(&id);
    assert_eq!((t.s("from"), t.s("baseSha")), ("origin/develop", d1.as_str()));
    assert_eq!(sw::sync::lands_on(&t), "develop");
    let t = approved(&id);
    assert_eq!((t.s("stage"), t.s("msg")), ("done", "Merged into develop at origin."), "{}", t.at("failure"));
    assert_eq!(git(&bare(&w), &["rev-parse", "develop^"]), d1); assert!(has(&w, "develop", "f.txt").is_some());
    assert_eq!((git(&w.root, &["symbolic-ref", "--short", "HEAD"]), git(&w.root, &["rev-parse", "HEAD"])), ("main".into(), mine), "your checkout is untouched");
    assert!(!w.root.join("f.txt").exists() && git(&w.root, &["status", "--porcelain"]).is_empty());
}

#[test]
fn a_branch_that_moved_after_the_review_is_caught_up_with_checked_again_and_nothing_is_lost() {
    let w = push_world(json!({}));
    they_push(&w, "develop", "d.txt", "one\n");
    from("develop");
    w.script(json!({ "worker": [work("f.txt")] }));
    let id = to_approval(&w, "Moved");
    assert_eq!(checks(&w), 1);
    let theirs = they_push(&w, "develop", "e.txt", "theirs\n");
    let t = approved(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(checks(&w), 2, "checked again with their work in");
    assert_eq!(git(&bare(&w), &["rev-parse", "develop^"]), theirs, "on top of theirs");
    assert_eq!((has(&w, "develop", "e.txt"), has(&w, "develop", "f.txt")), (Some("theirs".into()), Some("f.txt".into())));
}

#[test]
fn a_branch_that_moved_while_it_was_pushed_is_refused_by_git_then_caught_up_with_checked_again_and_lands() {
    let w = push_world(json!({}));
    they_push(&w, "develop", "d.txt", "one\n");
    from("develop");
    w.script(json!({ "worker": [work("f.txt")] }));
    let id = to_approval(&w, "Raced");
    // someone else's commit, ready in a clone of their own: pushed while the board's push runs, once
    let racer = w.base.join("racer");
    git(&w.base, &["clone", "-q", "-b", "develop", &bare(&w).display().to_string(), &racer.display().to_string()]);
    std::fs::write(racer.join("r.txt"), "raced\n").unwrap(); git(&racer, &["add", "-A"]); git(&racer, &["commit", "-qm", "race"]);
    let theirs = git(&racer, &["rev-parse", "HEAD"]);
    let once = w.base.join("raced");
    hook(&w.root.join(".git"), "pre-push", &format!("[ -e '{}' ] && exit 0\ntouch '{}'\nunset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE\ngit -C '{}' push -q origin develop\n", once.display(), once.display(), racer.display()));
    let t = approved(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(once.exists(), "the race happened");
    assert_eq!(checks(&w), 2, "checked again with their work in");
    assert_eq!(git(&bare(&w), &["rev-parse", "develop^"]), theirs, "nothing forced over theirs");
    assert_eq!((has(&w, "develop", "r.txt"), has(&w, "develop", "f.txt")), (Some("raced".into()), Some("f.txt".into())));
}

#[test]
fn merge_method_squash_makes_one_commit_merge_two_parents_rebase_lands_the_reviewed_commit_itself() {
    let w = push_world(json!({}));
    they_push(&w, "develop", "d.txt", "one\n");
    from("develop");
    for how in ["squash", "merge", "rebase"] {
        set_value("merge.method", &json!(how), &Scope::you()).unwrap();
        w.script(json!({ "worker": [work(&format!("{how}.txt"))] }));
        let id = to_approval(&w, &format!("By {how}"));
        let (before, reviewed) = (origin(&w, "develop"), task(&id).t("reviewedHead"));
        assert!(!reviewed.is_empty());
        assert_eq!(approved(&id).s("stage"), "done", "{}", task(&id).at("failure"));
        let now = origin(&w, "develop");
        let parents: Vec<String> = git(&bare(&w), &["rev-list", "--parents", "-n1", &now]).split_whitespace().skip(1).map(String::from).collect();
        match how {
            "squash" => { assert_ne!(now, reviewed); assert_eq!(parents, vec![before]); }
            "merge" => assert_eq!(parents, vec![before, reviewed.clone()]),
            _ => assert_eq!(now, reviewed),
        }
        assert_eq!(git(&bare(&w), &["rev-parse", &format!("{now}^{{tree}}")]), git(&w.root, &["rev-parse", &format!("{reviewed}^{{tree}}")]), "{how}");
    }
}

#[test]
fn merging_by_itself_it_lands_on_its_own_and_work_whose_review_fails_does_not() {
    let w = push_world(json!({ "merge.by": "itself", "review.when": "repos", "review.rounds": 1 }));
    they_push(&w, "develop", "d.txt", "one\n");
    from("develop");
    w.script(json!({ "worker": [work("f.txt")], "reviewer": [verdict(json!([]))] }));
    let id = w.add("Itself");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(t.s("mergeCommit"), origin(&w, "develop")); assert!(has(&w, "develop", "f.txt").is_some());
    let before = origin(&w, "develop");
    let crit = verdict(json!([{ "severity": "critical", "subject": "Deletes the data", "where": "g.txt", "proof": "ran it" }]));
    w.script(json!({ "worker": [work("g.txt"), work("g.txt")], "reviewer": [crit.clone(), crit] }));
    let bad = w.add("Fails");
    delegate(&bad);
    let mut t = settle(&bad);
    for _ in 0..2 { if t.s("stage") != "approval" { t = settle(&bad); } }
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert_eq!(t.at("review").at("pass"), &json!(false));
    assert_eq!(origin(&w, "develop"), before, "nothing landed");
}

#[test]
fn a_push_origin_refuses_waits_for_you_with_gits_own_words_and_lands_once_allowed() {
    let w = push_world(json!({}));
    they_push(&w, "develop", "d.txt", "one\n");
    from("develop");
    hook(&bare(&w), "pre-receive", "echo 'GL-HOOK-ERR: develop is a protected branch' >&2\nexit 1\n");
    w.script(json!({ "worker": [work("f.txt")] }));
    let id = to_approval(&w, "Refused");
    let before = origin(&w, "develop");
    let t = approved(&id);
    assert_eq!((t.s("stage"), t.at("failure").s("title")), ("approval", "Could not push to develop at origin"), "{}", t.at("failure"));
    assert!(t.at("failure").s("detail").contains("develop is a protected branch") && t.b("mergeFailed"), "{}", t.at("failure"));
    assert_eq!(t.at("failure").at("auth"), &json!(false));
    assert_eq!(origin(&w, "develop"), before); assert!(Path::new(t.s("worktree")).exists(), "the work is kept");
    std::fs::remove_file(bare(&w).join("hooks").join("pre-receive")).unwrap();
    let t = approved(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(has(&w, "develop", "f.txt").is_some());
}

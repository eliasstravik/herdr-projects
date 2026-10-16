//! A task started from a named branch (`workspace.from develop`): fetched from origin on every start, an unknown one or
//! one without a pull request refused with the reason, kept up with that branch (not main), a conflict with it sent to
//! the agent, its pull request into it, the agent's own one taken only when it merges into it, and merged and cleaned up.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::Path;
use sw::actions::{finish_line, talk};
use sw::json::*;
use sw::store::settings::change::set_value;
use sw::store::settings::lookup::Scope;

fn pr_world() -> World { world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() }) }
/// Someone else pushes a commit to a branch at the remote (made from main when it is new there): the commit.
fn they_push(w: &World, branch: &str, file: &str, body: &str) -> String {
    let n = std::fs::read_dir(&w.base).map(|d| d.count()).unwrap_or(0);
    let other = w.base.join(format!("other-{n}"));
    git(&w.base, &["clone", "-q", &w.base.join("origin.git").display().to_string(), &other.display().to_string()]);
    if git(&other, &["ls-remote", "--heads", "origin", branch]).is_empty() { git(&other, &["checkout", "-q", "-b", branch]); } else { git(&other, &["checkout", "-q", branch]); }
    std::fs::write(other.join(file), body).unwrap(); git(&other, &["add", "-A"]); git(&other, &["commit", "-qm", &format!("{branch}: {file}")]); git(&other, &["push", "-q", "origin", branch]);
    git(&other, &["rev-parse", "HEAD"])
}
fn holds(dir: &Path, sha: &str) -> bool { common::worktrees::git(dir, &["merge-base", "--is-ancestor", sha, "HEAD"]).is_ok() }
fn tree(t: &Obj) -> &Path { Path::new(t.s("worktree")) }
fn from(v: &str) { set_value("workspace.from", &json!(v), &Scope::you()).unwrap(); }
fn started(w: &World, title: &str) -> Obj { w.script(json!({ "worker": [{ "status": "working" }] })); let id = w.add(title); delegate(&id); settle(&id) }
fn refused(t: &Obj, words: &str) { assert_eq!((t.s("stage"), t.at("failure").s("title")), ("failed", "Could not start from that branch"), "{}", t.at("failure")); assert!(t.at("failure").s("detail").contains(words), "{}", t.at("failure")); assert!(!t.b("worktree")); }

#[test]
fn a_named_branch_is_fetched_on_every_start_and_the_worktree_starts_from_it() {
    let w = pr_world();
    let d1 = they_push(&w, "develop", "d.txt", "one\n");
    from("develop");
    let t = started(&w, "On develop");
    assert_eq!((t.s("from"), t.s("baseSha")), ("origin/develop", d1.as_str()), "{}", t.at("failure"));
    assert_eq!(git(tree(&t), &["rev-parse", "HEAD"]), d1); assert_eq!(read(&tree(&t).join("d.txt")), "one\n");
    assert_eq!(git(&w.root, &["symbolic-ref", "--short", "HEAD"]), "main", "your checkout stays where it was");
    // fetched again on the next start, though the repo was fetched just now
    let d2 = they_push(&w, "develop", "d.txt", "two\n");
    let u = started(&w, "Later on develop");
    assert_eq!(u.s("baseSha"), d2);
    // the task panel names it
    let mut b = common::board::open(&w.root, 60); b.s.sel = u.t("id"); b.load();
    let line = b.screen(160).into_iter().find(|l| l.contains("Workspace")).unwrap_or_default();
    assert!(line.contains("from develop"), "{line}");
}

#[test]
fn an_unknown_branch_no_remote_or_merge_pr_off_is_refused_with_the_reason() {
    let w = pr_world();
    from("nope");
    refused(&started(&w, "Nowhere"), "origin has no branch nope");
    they_push(&w, "develop", "d.txt", "one\n");
    from("develop");
    set_value("merge.pr", &json!(false), &Scope::you()).unwrap();
    refused(&started(&w, "Local"), "merge.pr is off");
    let bad = |v: &str| set_value("workspace.from", &json!(v), &Scope::you()).unwrap_err().msg;
    assert!(bad("origin/develop").contains("Did you mean develop?"), "{}", bad("origin/develop"));
    assert!(bad("my branch").contains("Did you mean my-branch?"), "{}", bad("my branch"));
    drop(w);
    let w = world();
    from("develop");
    refused(&started(&w, "No remote"), "no remote named origin");
}

#[test]
fn it_keeps_up_with_that_branch_opens_its_pull_request_into_it_merges_there_and_cleans_up() {
    let w = pr_world();
    they_push(&w, "develop", "d.txt", "one\n");
    from("develop");
    let id = to_approval(&w, "On develop");
    let t = task(&id);
    assert!(gh_log(&w).contains("pr create --draft --head work/1-on-develop --title On develop") && gh_log(&w).contains("--base develop"), "{}", gh_log(&w));
    assert_eq!(t.s("syncSeen"), git(&w.root, &["rev-parse", "origin/develop"]));
    // develop moves at the remote, and main too: the task takes develop's commit, never main's
    let d2 = they_push(&w, "develop", "e.txt", "two\n");
    let m2 = they_push(&w, "main", "m.txt", "main\n");
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    sw::housekeeping::poll_prs();
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert!(holds(tree(&t), &d2) && !holds(tree(&t), &m2)); assert_eq!(t.s("syncedTo"), d2);
    assert_eq!(sw::sync::lands_on(&t), "develop");
    // GitHub cannot say which branch it merges into: nothing merges
    w.gh_says(json!({ "view": "not an object" }));
    w.gh.log.borrow_mut().clear();
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.at("failure").s("title")), ("approval", "Could not read which branch the pull request merges into"), "{}", t.at("failure"));
    assert!(!gh_log(&w).contains("pr merge"), "{}", gh_log(&w));
    w.gh_says(json!({ "noLogin": true }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.at("failure").s("title"), "Could not read which branch the pull request merges into", "{}", t.at("failure"));
    assert!(!gh_log(&w).contains("pr merge"), "{}", gh_log(&w));
    sw::store::tasks::patch(&id, |t| { t.put("stage", "approval"); t.put("failure", Value::Null); }).unwrap();
    // its base was changed on GitHub meanwhile: nothing merges
    w.gh_says(json!({ "view": { "state": "OPEN", "headRefOid": t.at("prHead"), "baseRefName": "main" } }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.at("failure").s("title")), ("approval", "The pull request merges into main, not develop"), "{}", t.at("failure"));
    assert!(t.at("failure").s("next").contains("gh pr edit 7 --base develop") && !gh_log(&w).contains("pr merge"), "{}", t.at("failure"));
    // approved: GitHub's checks are those of the pull request into develop, and it merges there
    w.gh_says(json!({ "view": { "state": "OPEN", "headRefOid": t.at("prHead"), "baseRefName": "develop", "statusCheckRollup": [{ "name": "build", "conclusion": "SUCCESS" }], "comments": [], "reviews": [], "author": { "login": "me" } } }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(gh_log(&w).contains("pr merge 7 --squash") && gh_log(&w).contains("branches/develop --jq"), "{}", gh_log(&w));
    assert!(!gh_log(&w).contains("branches/main") && !gh_log(&w).contains("commits/main"), "{}", gh_log(&w));
    assert!(!tree(&t).exists(), "cleaned up");
}

#[test]
fn a_conflict_with_that_branch_goes_to_the_agent_naming_it() {
    let w = pr_world();
    they_push(&w, "develop", "d.txt", "one\n");
    from("develop");
    w.script(json!({ "worker": [{ "note": "Which one?" }, { "files": { "a.txt": "from the agent\n" }, "commit": "agent", "note": "Changed a.txt." }, { "note": null, "status": "working" }] }));
    let id = w.add("Conflict");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "asks");
    let target = they_push(&w, "develop", "a.txt", "from develop\n");
    talk::tell(&id, "The first one.", false).unwrap();
    let t = settle(&id);
    assert_eq!(t.at("failure").s("title"), "Merge conflict with develop", "{}", t.at("failure"));
    let said = prompts(&w, t.at("link").s("pane")).last().cloned().unwrap_or_default();
    assert!(said.contains("conflicts with origin/develop") && said.contains(&format!("Target commit: {target}")), "{said}");
}

#[test]
fn the_agents_own_pull_request_is_taken_only_when_it_merges_into_that_branch() {
    let w = pr_world();
    they_push(&w, "develop", "d.txt", "one\n");
    from("develop");
    w.gh_says(json!({ "prs": [{ "number": 9, "url": "https://github.com/o/r/pull/9", "state": "OPEN", "baseRefName": "main" }] }));
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = w.add("Wrong base");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.at("failure").s("title"), "The agent's pull request merges into another branch", "{}", t.at("failure"));
    assert!(t.at("failure").s("detail").contains("merges into main") && t.at("failure").s("next").contains("--base develop"), "{}", t.at("failure"));
    assert!(!gh_log(&w).contains("pr create") && !t.b("pr"), "{}", gh_log(&w));
    w.gh_says(json!({ "prs": [{ "number": 9, "url": "https://github.com/o/r/pull/9", "state": "OPEN", "baseRefName": "develop" }] }));
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = to_approval(&w, "Right base");
    assert_eq!(task(&id).s("pr"), "https://github.com/o/r/pull/9");
    assert!(!gh_log(&w).contains("pr create"), "{}", gh_log(&w));
}

#[test]
fn with_merge_pr_turned_off_after_the_start_it_does_not_merge_locally() {
    let w = pr_world();
    they_push(&w, "develop", "d.txt", "one\n");
    from("develop");
    w.script(json!({ "worker": [{ "note": "Which one?" }, { "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
    let id = w.add("No PR");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "asks");
    set_value("merge.pr", &json!(false), &Scope::you()).unwrap();
    talk::tell(&id, "The first one.", false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert!(!t.b("pr")); assert_eq!(sw::sync::lands_on(&t), "develop");
    let main_before = git(&w.root, &["rev-parse", "main"]);
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.at("failure").s("title"), "Lands on develop only through a pull request", "{}", t.at("failure"));
    assert_eq!(git(&w.root, &["rev-parse", "main"]), main_before, "nothing merged into your checkout");
}

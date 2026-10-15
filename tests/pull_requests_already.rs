//! Work the board opens no pull request of its own for (finish/already.rs): a pull request the agent opened itself is
//! taken on (HSW-3, SKL-8); a branch that brings main nothing is done, saying why: no change needed (ELA-2, ELA-3),
//! already on main (HSW-213), already merged by the agent (HP-5); a repo the task archived is not pushed to (HSW-219).
//! Work that is not on main is never done.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::Path;
use sw::json::*;

fn pr_world() -> World { world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() }) }
fn says(id: &str) -> String { sw::shown::row_says::sentence(&row(id)).text() }
fn pushed(w: &World, branch: &str) -> bool { !git(&w.root, &["ls-remote", "--heads", "origin", branch]).is_empty() }
fn on_main(w: &World) -> String { git(&w.root, &["ls-remote", "origin", "refs/heads/main"]).split('\t').next().unwrap_or("").to_string() }
fn work(git: Value) -> Value { json!({ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1.", "git": git }) }
/// The agent squash-merges its own branch into main on GitHub, as a merged pull request does.
fn squash_to_main() -> Value {
    json!([["push", "-q", "origin", "HEAD"], ["fetch", "-q", "origin", "main"], ["checkout", "-q", "--detach", "FETCH_HEAD"], ["merge", "-q", "--squash", "@{-1}"],
        ["-c", "user.email=agent@test", "-c", "user.name=agent", "commit", "-qm", "Release (#133)"], ["push", "-q", "origin", "HEAD:main"], ["checkout", "-q", "-"]])
}
/// GitHub's archived repo, as git sees it: every push refused once the agent archived it.
fn archivable(w: &World) -> Value {
    let bare = w.base.join("origin.git");
    let hook = bare.join("hooks").join("pre-receive");
    std::fs::write(&hook, "#!/bin/sh\nif [ \"$(git config --get herdr.archived)\" = true ]; then echo 'ERROR: This repository was archived so it is read-only.' >&2; exit 1; fi\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    json!(["--git-dir", bare.display().to_string(), "config", "herdr.archived", "true"])
}

#[test]
fn a_pull_request_the_agent_opened_itself_is_taken_on_and_merged_as_usual() {
    let w = pr_world();
    let url = "https://github.com/o/r/pull/9";
    w.gh_says(json!({ "createFail": "a pull request for branch \"work/1-own-pr\" into branch \"main\" already exists:\nhttps://github.com/o/r/pull/9", "prs": [{ "number": 9, "url": url, "state": "OPEN" }] }));
    let id = to_approval(&w, "Own PR");
    let t = task(&id);
    assert_eq!(t.s("pr"), url);
    assert!(t.s("msg").contains(&format!("Pull request: {url}")), "{}", t.s("msg"));
    assert!(t.at("failure").is_null() && t.at("doneWhy").is_null());
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
    assert!(gh_log(&w).contains("pr merge 9"), "{}", gh_log(&w));
}

#[test]
fn a_task_with_nothing_to_change_is_done_without_a_pull_request() {
    let w = pr_world();
    w.script(json!({ "worker": [{ "note": "Nothing needed changing." }] }));
    let id = w.add("Nothing to do");
    delegate(&id);
    // main moved on meanwhile: the branch takes it in, and still brings nothing of its own
    git(&w.root, &["-c", "user.email=h@test", "-c", "user.name=h", "commit", "-q", "--allow-empty", "-m", "elsewhere"]);
    git(&w.root, &["push", "-q", "origin", "main"]);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(t.s("doneWhy"), "No change needed");
    assert!(t.s("msg").contains("No change needed"), "{}", t.s("msg"));
    assert!(!gh_log(&w).contains("pr create") && !pushed(&w, t.s("branch")), "{}", gh_log(&w));
    assert_eq!(says(&id), "No change needed");
}

#[test]
fn work_the_agent_put_on_main_itself_is_done_saying_so() {
    let w = pr_world();
    w.script(json!({ "worker": [work(json!([["push", "-q", "origin", "HEAD:main"]]))] }));
    let id = w.add("Straight to main");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(t.s("doneWhy"), "Already on main");
    assert!(!gh_log(&w).contains("pr create") && !pushed(&w, t.s("branch")));
    assert_eq!(says(&id), "Already on main");
}

#[test]
fn work_the_agent_merged_in_its_own_pull_request_is_done_naming_it() {
    let w = pr_world();
    w.gh_says(json!({ "prs": [{ "number": 133, "url": "https://github.com/o/r/pull/133", "state": "MERGED" }] }));
    w.script(json!({ "worker": [work(squash_to_main())] }));
    let id = w.add("Release");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(t.s("doneWhy"), "Already merged in #133");
    assert!(t.s("msg").contains("Already merged in #133"), "{}", t.s("msg"));
    assert!(!gh_log(&w).contains("pr create"), "{}", gh_log(&w));
    assert_eq!(git(&w.root, &["log", "-1", "--format=%s", &on_main(&w)]), "Release (#133)", "main is as the agent's merge left it");
    assert_eq!(says(&id), "Already merged in #133");
}

#[test]
fn work_that_is_not_on_main_is_never_done_whatever_was_merged_before() {
    let w = pr_world();
    // An older pull request of the branch was merged; the work since is not on main: a new one is opened.
    w.gh_says(json!({ "prs": [{ "number": 133, "url": "https://github.com/o/r/pull/133", "state": "MERGED" }] }));
    let id = to_approval(&w, "More after the merge");
    let t = task(&id);
    assert_eq!(t.s("pr"), "https://github.com/o/r/pull/7");
    assert!(t.at("doneWhy").is_null());
    assert!(says(&id) != "Already merged in #133");
}

#[test]
fn a_repo_the_task_archived_is_not_pushed_to() {
    let w = pr_world();
    // HSW-219: the work went to main, then the repo was archived. Done, and nothing is pushed.
    let archive = archivable(&w);
    w.gh_says(json!({ "archived": true }));
    w.script(json!({ "worker": [work(json!([["push", "-q", "origin", "HEAD:main"], archive]))] }));
    let id = w.add("Archive the repo");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(t.s("doneWhy"), "Already on main");
    assert!(!gh_log(&w).contains("pr create") && !pushed(&w, t.s("branch")));
}

#[test]
fn work_that_cannot_land_because_the_repo_is_archived_fails_saying_so() {
    let w = pr_world();
    let archive = archivable(&w);
    w.gh_says(json!({ "archived": true }));
    w.script(json!({ "worker": [work(json!([archive]))] }));
    let id = w.add("Not on main");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed", "{}", t.s("msg"));
    assert_eq!(t.at("failure").s("title"), "The repo is archived on GitHub");
    assert!(t.at("failure").s("detail").contains("its work is not on main"), "{}", t.at("failure"));
    assert!(t.at("doneWhy").is_null() && !pushed(&w, t.s("branch")));
    assert!(Path::new(t.s("worktree")).exists(), "its work is kept");
}

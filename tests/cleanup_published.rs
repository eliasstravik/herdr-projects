//! After a task ended, its checkout and branch go only once its commits are at the remote, asked of the remote itself
//! (land/published.rs): at its pull request's head, on main, or on the branch there. Until then both are kept, its pane
//! stays open (a push from there is all it needs), and it is looked at again once it has been quiet a while.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::housekeeping::poll_prs;
use sw::json::*;

fn pr_world() -> World { world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() }) }
fn kept(t: &Obj) -> String { t.arr("kept").first().map(text).unwrap_or_default() }
fn open(w: &World, t: &Obj) -> bool { w.state().arr("workspaces").iter().any(|x| x.s("workspace_id") == t.at("link").s("workspace")) }
fn has_branch(w: &World, t: &Obj) -> bool { !git(&w.root, &["branch", "--list", t.s("branch")]).is_empty() }
/// The commit origin lists for a ref, asked of origin itself; empty when it has none.
fn at_origin(w: &World, r: &str) -> String { git(&w.root, &["ls-remote", "origin", r]).split('\t').next().unwrap_or("").to_string() }
/// A ref of origin set by hand (`None`: deleted), as GitHub does when a pull request's branch is deleted on merge.
fn origin_ref(w: &World, r: &str, oid: Option<&str>) {
    let o = w.base.join("origin.git").display().to_string();
    match oid { Some(oid) => git(&w.base, &["--git-dir", &o, "update-ref", r, oid]), None => git(&w.base, &["--git-dir", &o, "update-ref", "-d", r]) };
}
/// One housekeeping pass, as the keeper or any hook runs it, and the clean-ups it starts for this task.
fn pass(w: &World, id: &str) -> usize {
    let _ = std::fs::remove_dir_all(sw::store::files::data_dir().join("stamps"));
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let jobs: Vec<(String, String, Vec<String>)> = sw::runner::JOBS.with(|j| std::mem::take(&mut *j.borrow_mut())).into_iter().filter(|(job, i, _)| job == "cleanup" && i == id).collect();
    for (_, _, extra) in &jobs { sw::land::cleanup_job(id, if extra.iter().any(|e| e == "--merged") { "merged" } else { "always" }); }
    jobs.len()
}
/// A pass after the quiet time: what was put off for a push is looked at again.
fn later(w: &World, id: &str) -> usize {
    unsafe { std::env::set_var("HERDR_PROJECTS_QUIET_MS", "0") };
    let n = pass(w, id);
    unsafe { std::env::remove_var("HERDR_PROJECTS_QUIET_MS") };
    n
}

#[test]
fn a_task_merged_through_its_pull_request_is_cleaned_up_as_github_keeps_the_pull_requests_head() {
    let w = pr_world();
    let id = to_approval(&w, "Merged here");
    let t = task(&id);
    let (head, branch) = (t.t("prHead"), format!("refs/heads/{}", t.s("branch")));
    assert_eq!(at_origin(&w, &branch), head, "pushed for its pull request");
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let c = settle(&id);
    assert_eq!(c.s("stage"), "done", "{}", c.at("failure"));
    assert!(gh_log(&w).contains("pr merge 7 --squash"), "{}", gh_log(&w));
    assert_eq!(at_origin(&w, "refs/pull/7/head"), head, "GitHub keeps the pull request's head");
    assert_eq!(at_origin(&w, &branch), "", "the branch there went with the merge");
    assert!(c.b("cleaned") && !c.b("kept") && !c.b("tidy"), "{}", kept(&c));
    assert!(!Path::new(t.s("worktree")).exists() && !has_branch(&w, &t) && !open(&w, &c));
}

#[test]
fn a_commit_at_no_ref_of_the_remote_keeps_the_checkout_and_the_branch_with_its_pane_open_until_it_is_pushed() {
    let w = pr_world();
    let id = to_approval(&w, "Merged elsewhere");
    let t = task(&id);
    let (tree, head, branch) = (Path::new(t.s("worktree")), t.t("prHead"), format!("refs/heads/{}", t.s("branch")));
    // merged on GitHub by hand at the pushed commit; the branch there was deleted with it, and this remote keeps no
    // pull request heads: no ref of it holds the commit any more
    origin_ref(&w, &branch, None);
    w.gh_says(json!({ "view": { "state": "MERGED", "headRefOid": head } }));
    poll_prs();
    let c = task(&id);
    assert_eq!(c.s("stage"), "done"); assert!(c.b("prMerged"));
    let why = format!("(unpublished commits on {}: push them or set workspace.cleanup false)", t.s("branch"));
    assert!(kept(&c).ends_with(&why) && kept(&c).starts_with(&sw::store::marks::tilde(t.s("worktree"))), "{}", kept(&c));
    assert!(c.s("msg").ends_with(&format!(" Kept: {}", kept(&c))), "{}", c.s("msg"));
    assert!(tree.exists() && has_branch(&w, &t), "nothing of it is removed");
    assert!(open(&w, &c) && c.b("tidy"), "its space stays open: a push from its own terminal is all it needs");
    assert!(c.at("tidy").i("quietFrom") > 0 && c.at("tidy").s("deleteBranch") == "always", "{}", c.at("tidy"));
    assert_eq!(pass(&w, &id), 0, "origin is not asked again every half minute");
    // origin cannot be asked: nothing goes, and it says so
    let (bare, gone) = (w.base.join("origin.git").display().to_string(), w.base.join("gone.git").display().to_string());
    git(&w.root, &["remote", "set-url", "origin", &gone]);
    assert_eq!(later(&w, &id), 1);
    let c = task(&id);
    assert!(kept(&c).contains(&format!("(origin could not be asked whether {} is published: ", t.s("branch"))), "{}", kept(&c));
    assert!(tree.exists() && has_branch(&w, &t) && open(&w, &c) && c.b("tidy"));
    git(&w.root, &["remote", "set-url", "origin", &bare]);
    // the pull request's head turns up at the remote after all (GitHub lists it a moment after the merge): published
    origin_ref(&w, "refs/pull/7/head", Some(&head));
    assert_eq!(later(&w, &id), 1);
    let c = task(&id);
    assert!(c.b("cleaned") && !c.b("kept") && !c.b("tidy"), "{}", kept(&c));
    assert!(!tree.exists() && !has_branch(&w, &t) && !open(&w, &c));
    assert_eq!(c.s("msg"), "Merged: https://github.com/o/r/pull/7", "what it kept is said no more");
    assert_eq!(pass(&w, &id), 0, "and then it is over");
}

#[test]
fn a_push_of_the_branch_from_its_own_terminal_is_enough_and_a_branch_with_no_commits_of_its_own_asks_nothing() {
    let w = pr_world();
    let id = to_approval(&w, "Pushed by hand");
    let t = task(&id);
    let (tree, head, branch) = (Path::new(t.s("worktree")), t.t("prHead"), format!("refs/heads/{}", t.s("branch")));
    origin_ref(&w, &branch, None);
    w.gh_says(json!({ "view": { "state": "MERGED", "headRefOid": head } }));
    poll_prs();
    let c = task(&id);
    assert!(kept(&c).contains("(unpublished commits on "), "{}", kept(&c)); assert!(open(&w, &c));
    // pushed again from its terminal, say: the branch at the remote holds the commit
    git(tree, &["push", "-q", "origin", &format!("HEAD:{branch}")]);
    assert_eq!(at_origin(&w, &branch), head);
    assert_eq!(later(&w, &id), 1);
    let c = task(&id);
    assert!(c.b("cleaned") && !c.b("kept"), "{}", kept(&c)); assert!(!tree.exists() && !has_branch(&w, &t) && !open(&w, &c));
    // a task whose branch holds no commit of its own (its work landed as another task's) has nothing to publish:
    // closed as before, with origin asked nothing
    w.script(json!({ "worker": [{ "note": "Nothing to do: that landed with the other task." }] }));
    let e = w.add("Empty");
    delegate(&e);
    let c = settle(&e);
    assert_eq!(c.s("stage"), "done", "{} {}", c.s("msg"), c.at("failure"));
    assert!(c.b("cleaned") && !c.b("kept"), "{}", kept(&c)); assert!(!open(&w, &c));
}

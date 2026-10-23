//! Landing on a branch named at the remote with no pull request (`workspace.from develop`, `merge.pr` off): a commit
//! built from the reviewed one, as `merge.method` says, pushed to that branch at origin. No checkout is touched and
//! nothing is forced: a branch that moved meanwhile is caught up with and checked again, under the same approval.
use super::merge::{fail, landed, reverify, Fail};
use crate::err::Res;
use crate::finish::auto_merge::called_off;
use crate::flow;
use crate::json::*;
use crate::runner::Guard;
use crate::store::failures::is_auth_failure;
use crate::store::keys::{repo_keys, task_ref};
use crate::store::repos::{git_out, git_slow, TIMED_OUT};
use crate::store::settings::lookup::setting_text;
use crate::store::settings::values::ms;
use crate::store::tasks::Task;
use crate::sync;
use std::path::Path;

/// Where `b` stood at origin when it was last fetched.
fn at_origin(tp: &Path, b: &str) -> Res<String> { git_out(tp, &["rev-parse", "--verify", &format!("refs/remotes/origin/{b}^{{commit}}")]) }
fn holds(tp: &Path, older: &str, newer: &str) -> bool { git_out(tp, &["merge-base", "--is-ancestor", older, newer]).is_ok() }
/// The commit that goes onto `parent`: the reviewed tree squashed into one, a merge of the two, or (rebase) the reviewed
/// commit itself. `parent` when the work brings it nothing.
fn build(tp: &Path, how: &str, parent: &str, head: &str, message: &str) -> Res<String> {
    let tree = format!("{head}^{{tree}}");
    if parent == head || how == "rebase" { return Ok(head.into()); }
    if how == "merge" { return git_out(tp, &["commit-tree", &tree, "-p", parent, "-p", head, "-m", message]); }
    if git_out(tp, &["rev-parse", &tree])? == git_out(tp, &["rev-parse", &format!("{parent}^{{tree}}")])? { return Ok(parent.into()); }
    git_out(tp, &["commit-tree", &tree, "-p", parent, "-m", message])
}
/// Runs once `sync::sync_main` found the task's worktree up to date with `origin/<b>`.
pub fn land(id: &str, g: &mut Guard, t: &Task, tree: &str, b: &str, how: &str) -> Res<()> {
    let tp = Path::new(tree);
    // Only the commit that was checked and reviewed lands.
    let head = { let v = t.at("mergeIntent").t("verifiedHead"); if v.is_empty() { flow::git::head_of(tp) } else { v } };
    let parent = match at_origin(tp, b) {
        Ok(p) => p,
        Err(e) => return fail(g, &format!("Could not read {b} at origin"), &format!("Worktree: {tree}\n\nWhat git said:\n{}\n\nNothing was merged.", e.msg), &format!("Make sure origin still has {b} (git fetch origin {b}), then approve again."), Fail { agent_can_fix: false, ..Default::default() }),
    };
    // It moved since the review: caught up with and checked again first, never pushed over.
    if !holds(tp, &parent, &head) { return reverify(id, g, t, &format!("Checking the work caught up with {b}"), None); }
    let message = format!("{}\n\nTask: {}", if t.b("title") { t.t("title") } else { id.to_string() }, task_ref(id, &repo_keys()));
    let new = match build(tp, how, &parent, &head, &message) {
        Ok(c) => c,
        Err(e) => return fail(g, &format!("Could not make the commit for {b}"), &format!("Worktree: {tree}\nOnto: {parent} (origin/{b})\nReviewed: {head}\n\nWhat git said:\n{}\n\nNothing was merged.", e.msg), "Fix what git said (git config user.name and user.email, for one), then approve again.", Fail { agent_can_fix: false, ..Default::default() }),
    };
    if new == parent { return landed(id, g, t, &format!("{b} at origin has this work already."), &head, &parent); }
    if called_off(g) { return Ok(()); }
    // A plain push: git itself refuses it when the branch moved at origin since it was fetched.
    let most = match ms(&setting_text("merge.pushTimeout", Some(t))) { 0 => u64::MAX, l => l as u64 };
    let said = match git_slow(tp, &["push", "origin", &format!("{new}:refs/heads/{b}")], most) {
        Ok(_) => return landed(id, g, t, &format!("Merged into {b} at origin."), &head, &new),
        Err(e) if e.msg.contains(TIMED_OUT) => format!("{} (merge.pushTimeout): a slow pre-push hook? Raise it with herdr-projects set merge.pushTimeout 10m --repo {}", e.msg, t.s("repo")),
        Err(e) => e.msg,
    };
    let auth = is_auth_failure(&said);
    // Someone pushed to it meanwhile: fetched, caught up with and checked again, then pushed under the same approval.
    if !auth && sync::fetch_main(tp, b).is_ok() && at_origin(tp, b).is_ok_and(|p| !holds(tp, &p, &head)) {
        return reverify(id, g, t, &format!("{b} moved at origin: catching up with it and checking again"), None);
    }
    let next = if auth { format!("Make sure git can push to origin without asking for a password (git push origin {b}), then approve again.") }
        else { format!("Origin refused the push (a protected branch, or a hook there): see what git said. Once origin takes pushes to {b} from you, approve again; or turn merge.pr on for this task (herdr-projects set merge.pr true --task {}) and finish again, to land through a pull request.", task_ref(id, &repo_keys())) };
    fail(g, &format!("Could not push to {b} at origin"), &format!("Branch at origin: {b}\nCommit: {new}\nReviewed: {head}\n\ngit push:\n{said}\n\nNothing was merged."), &next, Fail { agent_can_fix: false, auth: Some(auth), keep_intent: false })
}

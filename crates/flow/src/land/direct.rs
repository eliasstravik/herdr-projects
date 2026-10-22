//! Landing a task that started from a named branch (`workspace.from develop`) with no pull request (`merge.pr` off): the
//! commit that was checked and reviewed is built on origin's copy of that branch and pushed to it, so no checkout of yours
//! is touched and the branch need not be checked out anywhere. Git itself refuses a push that is not a fast-forward; the
//! task then catches up with the branch again and is merged under the approval it was given.
use super::merge::{fail, landed, reverify, Fail};
use crate::err::Res;
use crate::finish::auto_merge::called_off;
use crate::json::*;
use crate::runner::{is_aborted, Guard};
use crate::store::fetch;
use crate::store::keys::{repo_keys, task_ref};
use crate::store::repos::{git_out, git_slow};
use crate::store::settings::lookup::setting_text;
use crate::store::settings::values::ms;
use crate::store::tasks::Task;
use std::path::Path;

/// Origin's copy of `b`, when it is in the history of `head` (the push is then a fast-forward); None when it is not, or not there.
fn parent_in(tp: &Path, b: &str, head: &str) -> Option<String> {
    let p = git_out(tp, &["rev-parse", "--verify", &format!("refs/remotes/origin/{b}^{{commit}}")]).ok()?;
    git_out(tp, &["merge-base", "--is-ancestor", &p, head]).is_ok().then_some(p)
}
/// Runs with the task in stage "merging", after its last catch-up with `b`.
pub fn land(id: &str, g: &mut Guard, t: &Task, tree: &str, b: &str, how: &str) -> Res<()> {
    match push_it(id, g, t, tree, b, how) {
        Err(e) if is_aborted(&e) => Err(e),
        Err(e) => fail(g, "Merge failed", &e.msg, "Investigate the Git error in your own worktree, then finish again. Approve again only after the problem is fixed.", Fail::default()),
        ok => ok,
    }
}
fn push_it(id: &str, g: &mut Guard, t: &Task, tree: &str, b: &str, how: &str) -> Res<()> {
    let tp = Path::new(tree);
    // The commit that was checked and reviewed is the one landed, by its id.
    let head = { let v = t.at("mergeIntent").t("verifiedHead"); if v.is_empty() { git_out(tp, &["rev-parse", "HEAD"])? } else { v } };
    // b moved at the remote since the last catch-up: the task takes it in and is checked again.
    let Some(parent) = parent_in(tp, b, &head) else { return reverify(id, g, t, &format!("{b} moved at the remote: checking the branch updated with it"), None) };
    let message = format!("{}\n\nTask: {}", if t.b("title") { t.t("title") } else { id.to_string() }, task_ref(id, &repo_keys()));
    let tree_of_head = format!("{head}^{{tree}}");
    // What lands: the head itself (rebase), or one commit with its tree on b (squash), or on both (merge). The tree is the
    // same: the head has b in its history, so it holds exactly what b plus the task's work is.
    let new = match how {
        "rebase" => head.clone(),
        "merge" => git_out(tp, &["commit-tree", &tree_of_head, "-p", &parent, "-p", &head, "-m", &message])?,
        _ => git_out(tp, &["commit-tree", &tree_of_head, "-p", &parent, "-m", &message])?,
    };
    if called_off(g) { return Ok(()); }
    let limit = ms(&setting_text("merge.pushTimeout", Some(t)));
    let most = if limit == 0 { u64::MAX } else { limit as u64 };
    if let Err(e) = git_slow(tp, &["push", "origin", &format!("{new}:refs/heads/{b}")], most) {
        // Someone pushed to b first: asked again, and the branch is brought up to date, not failed.
        let _ = fetch::fetch_branch(tp, b);
        if parent_in(tp, b, &head).is_none() { return reverify(id, g, t, &format!("{b} moved at the remote while it was pushed: checking the branch updated with it"), None); }
        return fail(g, &format!("Could not push to {b}"), &format!("Task branch: {}\nPushed to: origin/{b}\n\nWhat git said:\n{}\n\nNothing was merged.", t.s("branch"), e.msg),
            &format!("Check that origin takes a push to {b} (a protected branch, a pre-push hook, a login), then approve again."), Fail { agent_can_fix: false, auth: None, keep_intent: false });
    }
    sync_local(tp, b);
    landed(id, g, t, &format!("Merged into {b} at the remote"), &head, &new)
}

/// Brings a local copy of `b` up to date with what was just pushed, when no worktree has it checked out. A branch that is
/// checked out is left alone (git refuses to update it): its own `git pull --ff-only` brings it forward. Best effort: a
/// local branch that has moved on its own is left as it is, by the same fast-forward check git makes.
pub fn sync_local(tp: &Path, b: &str) {
    if git_out(tp, &["rev-parse", "--verify", "-q", &format!("refs/heads/{b}")]).is_err() { return; }
    // One fetch updates origin/b and the local branch together, and refuses when b is checked out or not a fast-forward.
    let _ = fetch::with_repo_lock(tp, || Ok(git_out(tp, &["fetch", "origin", &format!("{b}:{b}")]).map(|_| ())));
}

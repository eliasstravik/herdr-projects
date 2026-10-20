//! Whether a task's commits are at the remote, asked of the remote itself, before its checkout or branch is removed
//! after it ended: its branch tip, or the commit its merge made of it (`mergeCommit`, a squash is no ancestor of
//! anything), must be reachable from a ref `git ls-remote origin` lists now: its pull request's head (`refs/pull/N/head`,
//! which GitHub keeps after the branch is deleted), the main it lands on, or the branch itself. A task with no commits
//! of its own, or one that lands in your checkout (no origin, `merge.pr` off), has nothing to publish. The local rule
//! (the tip is in your checkout or in what it started from: cleanup.rs `landed`) stays beside this one, never alone.
use crate::flow::git::{has_origin, pr_number};
use crate::json::*;
use crate::store::fetch::quiet_git;
use crate::store::repos::git_out;
use crate::store::tasks::Task;
use crate::sync;
use std::path::Path;

/// How long origin is given to list its refs, or to hand over one of them.
const ASK_MS: u64 = 30000;

/// Is `tip` (the branch's commit as clean-up first saw it) published? `Err`: origin could not be asked, with what it said.
pub fn published(root: &Path, t: &Task, tip: &str) -> Result<bool, String> {
    if tip.is_empty() || !t.b("branch") || !has_origin(root) || !sync::via_remote(root, t) { return Ok(true); }
    if git_out(root, &["rev-parse", "--verify", &format!("refs/heads/{}", t.s("branch"))]).is_err() { return Ok(true); } // gone already: nothing here to lose
    let own = !t.b("baseSha") || git_out(root, &["merge-base", "--is-ancestor", tip, t.s("baseSha")]).is_err();
    if !own { return Ok(true); }
    let mut want = vec![format!("refs/heads/{}", t.s("branch")), format!("refs/heads/{}", sync::main_branch(root, t))];
    if t.b("pr") { want.push(format!("refs/pull/{}/head", pr_number(t.s("pr")))); }
    let mut args = vec!["ls-remote", "origin"];
    args.extend(want.iter().map(|w| w.as_str()));
    let listed = quiet_git(root, &args, ASK_MS)?;
    let refs: Vec<(String, String)> = listed.lines().filter_map(|l| { let (oid, r) = l.split_once('\t')?; want.iter().any(|w| w == r.trim()).then(|| (r.trim().to_string(), oid.trim().to_string())) }).collect();
    let mine: Vec<&str> = [tip, t.s("mergeCommit")].into_iter().filter(|c| !c.is_empty()).collect();
    let holds_mine = |oid: &str| mine.iter().any(|c| holds(root, oid, c));
    if refs.iter().any(|(_, oid)| holds_mine(oid)) { return Ok(true); }
    // A ref that moved on at the remote since it was last fetched names a commit not known here: that one is fetched
    // (its objects only, into FETCH_HEAD) and looked at again.
    for (r, oid) in &refs {
        if git_out(root, &["cat-file", "-e", &format!("{oid}^{{commit}}")]).is_ok() { continue; }
        if quiet_git(root, &["fetch", "--no-tags", "origin", r], ASK_MS).is_ok() && holds_mine(oid) { return Ok(true); }
    }
    Ok(false)
}
/// Is `c` the commit `oid` or one before it? A commit this checkout has never seen is not known to hold anything.
fn holds(root: &Path, oid: &str, c: &str) -> bool { oid == c || git_out(root, &["merge-base", "--is-ancestor", c, oid]).is_ok() }

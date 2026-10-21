//! Making a task's worktree while another git command holds the repo's `.git/config` (its config.lock: another task's
//! start, a `git push -u` at another's finish): the board waits a moment and tries again, and a start that fails
//! anyway takes away the empty branch git made before it failed, so the next start begins clean.
use crate::err::Res;
use crate::json::*;
use crate::store::repos::git_out;
use std::path::Path;

/// The waits before each new try while the config file is held: under four seconds in all.
const WAITS_MS: [u64; 4] = [250, 500, 1000, 2000];

/// Git could not write the repo's config because another git command held its lock at that moment.
pub fn config_locked(said: &str) -> bool { said.contains("could not lock config file") }
/// Runs `make` (herdr's `worktree create` of `branch` from the commit `base`) until it no longer fails on a held
/// config lock, at most five times. After every failure the branch it left is removed when it is still empty.
pub fn making(root: &Path, branch: &str, base: &str, mut make: impl FnMut() -> Res<Value>) -> Res<Value> {
    let mut tries = 0;
    loop {
        let e = match make() { Ok(v) => return Ok(v), Err(e) => e };
        drop_empty_branch(root, branch, base);
        if !config_locked(&e.msg) || tries == WAITS_MS.len() { return Err(e); }
        crate::clock::pause(WAITS_MS[tries]);
        tries += 1;
    }
}
/// Removes `branch` when it has no commits of its own (it is still at `base`) and no worktree has it checked out.
pub fn drop_empty_branch(root: &Path, branch: &str, base: &str) {
    let r = format!("refs/heads/{branch}");
    if git_out(root, &["rev-parse", "--verify", "-q", &r]).ok().as_deref() != Some(base) { return; }
    if git_out(root, &["worktree", "list", "--porcelain"]).unwrap_or_default().lines().any(|l| l == format!("branch {r}")) { return; }
    let _ = git_out(root, &["update-ref", "-d", &r, base]); // the ref alone: git's config is not touched
}

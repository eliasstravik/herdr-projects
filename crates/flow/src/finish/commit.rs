//! Committing what the agent left uncommitted at the finish line: while another Git process holds the worktree's index
//! (its index.lock; a repo gate's `git status`, an editor) the board waits a moment and tries again, and only a lock
//! that stays is a failure, which names the lock file.
use super::shared::failure;
use crate::json::*;
use crate::store::repos::git_out;
use std::path::Path;

/// The waits before each new try while the index is held: under four seconds in all.
const WAITS_MS: [u64; 4] = [250, 500, 1000, 2000];

/// The lock file a Git error names when another Git process holds the index; None for any other error.
pub fn index_lock(said: &str) -> Option<String> {
    if !said.contains("index.lock") || !said.contains("File exists") { return None; }
    Some(said.split('\'').find(|p| p.ends_with("index.lock")).unwrap_or("index.lock").to_string())
}
/// Nothing is left to commit in `tree`. A status Git cannot give is not taken for nothing.
fn clean(tree: &Path) -> bool { git_out(tree, &["status", "--porcelain"]).map(|s| s.is_empty()).unwrap_or(false) }
/// Commits everything in `tree` as `msg`. Err: the failure to record.
pub fn commit_left(tree: &Path, msg: &str) -> Result<(), Obj> {
    let mut tries = 0;
    loop {
        let Err(e) = git_out(tree, &["add", "-A"]).and_then(|_| git_out(tree, &["commit", "-m", msg])) else { return Ok(()) };
        let Some(lock) = index_lock(&e.msg) else { return Err(failure("commit", "Could not commit the work", &e.msg, "Fix the Git error in your own worktree, commit the work and finish again.")) };
        if tries == WAITS_MS.len() {
            let waited: u64 = WAITS_MS.iter().sum();
            let detail = format!("{}\n\nTried {} times over {:.1} seconds: the lock stayed.", e.msg, tries + 1, waited as f64 / 1000.0);
            return Err(failure("commit", "Could not commit the work: the Git index is locked", &detail, &format!("Another Git process holds {lock}. Let it finish; if no Git process is running in that worktree, delete {lock}. Then finish again.")));
        }
        crate::clock::pause(WAITS_MS[tries]);
        tries += 1;
        // The other process may have committed it all meanwhile (the agent's own commit): nothing left is no failure.
        if clean(tree) { return Ok(()); }
    }
}

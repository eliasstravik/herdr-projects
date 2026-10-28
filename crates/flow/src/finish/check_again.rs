//! Which check a finish runs: the full one (`check.command`), or, on a later round of the same task, the shorter one
//! (`check.again`): only once the full check has passed on this branch, and only while GitHub's own checks guard the
//! merge. Anything else, or anything it cannot tell, runs the full check. docs/shorter-checks.md.
use crate::json::*;
use crate::store::repos::git_ok;
use crate::store::settings::values::command_of;
use std::path::Path;

/// Given to the shorter check: the commit the full check last passed on, for it to find what changed since.
pub const CHECKED_ENV: &str = "HERDR_PROJECTS_CHECKED";
/// The check a finish runs: its command as set, and for the shorter one the commit it starts from (empty: the full).
pub struct Chosen { pub cmd: String, pub since: String }
impl Chosen {
    pub fn short(&self) -> bool { !self.since.is_empty() }
    /// The command as it is said, to the agent and the reviewer: the commit written in for the variable.
    pub fn shown(&self) -> String {
        if !self.short() { return self.cmd.clone(); }
        self.cmd.replace(&format!("${{{CHECKED_ENV}}}"), &self.since).replace(&format!("${CHECKED_ENV}"), &self.since)
    }
}
/// `full`: the full check's command (`check.command`). `tree`: the task's worktree (empty: a plain folder).
pub fn choose(t: &Obj, ch: &Obj, full: &str, tree: &str, root: &Path) -> Chosen {
    let full_one = Chosen { cmd: full.to_string(), since: String::new() };
    let again = command_of(ch.at("check.again"));
    let last = t.at("checkPassed");
    if again.is_empty() || full.is_empty() || tree.is_empty() || last.s("head").is_empty() || last.s("with") != full { return full_one; }
    // The branch still holds what passed: rebuilt (rebased, started again), nothing is known of it.
    if !git_ok(Path::new(tree), &["merge-base", "--is-ancestor", last.s("head"), "HEAD"]) { return full_one; }
    if !guarded(t, ch, root) { return full_one; }
    Chosen { cmd: again, since: last.t("head") }
}
/// GitHub runs its checks on this work before it merges: through its pull request, which waits for them
/// (`merge.checks`), with a workflow of the branch that runs on each of its commits, and GitHub Actions on for the
/// repo. Then the full suite still runs before the merge.
fn guarded(t: &Obj, ch: &Obj, root: &Path) -> bool {
    if ch.s("merge.checks") != "github" || !t.b("pr") || !ch.b("merge.pr") { return false; }
    use crate::land::{expected, triggers::Runs};
    expected::flows_for(t, &crate::sync::main_branch(root, t)).iter().any(|(_, _, r)| *r == Runs::Always) && !expected::actions_off(root)
}
/// Kept on the task when the full check passed: the commit it passed on (with what the check wrote committed) and
/// the command, so that a later round may run the shorter one.
pub fn passed_patch(head: &str, full: &str) -> Value { if head.is_empty() { Value::Null } else { json!({ "head": head, "with": full }) } }

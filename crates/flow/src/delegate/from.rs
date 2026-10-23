//! Where a task's worktree starts (`workspace.from`): the remote's main, your checkout's commit, or a branch named at
//! the remote, fetched on every start. A start that cannot begin from a named branch is refused with the reason.
use crate::err::Res;
use crate::flow;
use crate::flow::git::has_origin;
use crate::json::*;
use crate::runner::Guard;
use crate::store::failures::failure_patch;
use crate::store::fetch;
use crate::store::repos::git_out;
use crate::store::settings::from::named;
use crate::store::settings::lookup::setting_text;
use std::path::Path;
use std::process::{Command, Stdio};

/// How long a start waits for origin to give a named branch: never asking for a password.
const FETCH_MS: u64 = 10000;
fn quiet_git(root: &Path, args: &[&str]) -> Command {
    let mut c = Command::new("git");
    c.args(args).current_dir(root).env("GIT_TERMINAL_PROMPT", "0").env("GIT_SSH_COMMAND", "ssh -oBatchMode=yes").stdin(Stdio::null());
    c
}
/// `main`: the remote's default branch, fetched first if the repo has not been fetched in a day (5 seconds at most).
/// With no remote, or with `here`: your checkout's current commit.
fn main_ref(root: &Path, from: &str) -> String {
    if from != "here" && has_origin(root) {
        let age = std::fs::metadata(root.join(".git").join("FETCH_HEAD")).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).map(|d| d.as_millis() as i64).unwrap_or(i64::MAX);
        if age > 24 * 3600 * 1000 {
            // One fetch at a time in a repo: every worktree of it writes the same remote-tracking refs.
            let _ = fetch::with_repo_lock(root, || { if let Ok(child) = quiet_git(root, &["fetch", "origin"]).stdout(Stdio::null()).stderr(Stdio::null()).spawn() { let _ = crate::herdr::cli::wait_with_timeout(child, 5000); } Ok(()) });
        }
        // Origin's main branch never named here (a remote added by hand): asked of origin, so the task keeps up with it.
        fetch::learn_origin_head(root);
        for r in ["origin/HEAD", "origin/main", "origin/master"] { if git_out(root, &["rev-parse", "--verify", &format!("refs/remotes/{r}")]).is_ok() { return r.into(); } }
    }
    "HEAD".into()
}
/// The commit-ish a task's worktree starts from: `origin/<branch>` for a named branch, fetched now. Refused, with the
/// reason, for a branch origin does not have or a repo with no origin. With `merge.pr` off it lands there by a push of
/// its own (land/direct.rs), never on your checkout's branch.
pub fn base_ref(root: &Path, t: &Obj) -> Res<String> {
    let from = setting_text("workspace.from", Some(t));
    let Some(b) = named(&from) else { return Ok(main_ref(root, &from)) };
    if !has_origin(root) { crate::refuse!("workspace.from is {b}, a branch at the remote, and this repo has no remote named origin. Add the remote, or set workspace.from to main or here."); }
    let spec = format!("+refs/heads/{b}:refs/remotes/origin/{b}");
    let ran = fetch::with_repo_lock(root, || Ok(quiet_git(root, &["fetch", "--no-tags", "origin", &spec]).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| e.to_string())
        .and_then(|c| crate::herdr::cli::wait_with_timeout(c, FETCH_MS).map_err(|e| if e.code == "timeout" { format!("origin did not answer within {} seconds", FETCH_MS / 1000) } else { e.message })))).unwrap_or_else(|e| Err(e.msg));
    let said = match ran { Ok(r) if r.status == 0 => return Ok(format!("origin/{b}")), Ok(r) => r.stderr.trim().to_string(), Err(e) => e };
    if said.to_lowercase().contains("couldn't find remote ref") {
        crate::refuse!("origin has no branch {b} (workspace.from), so the task cannot start from it. Push {b} first, or set workspace.from to a branch origin has (git ls-remote --heads origin lists them), main or here.");
    }
    // Origin cannot be reached now: the copy of the branch it last gave is where the work starts.
    if git_out(root, &["rev-parse", "--verify", "-q", &format!("refs/remotes/origin/{b}")]).is_ok() { return Ok(format!("origin/{b}")); }
    crate::refuse!("Could not fetch {b} (workspace.from) from origin, and it was never fetched here: {said}. Make sure origin can be reached without a password (git fetch origin {b}), then delegate again.")
}
/// Does this task's start name a branch (`workspace.from develop`)? Kept on it as `fromNamed`: it then lands on that
/// branch, also when that is origin's main (sync.rs).
pub fn named_start(t: &Obj) -> bool { named(&setting_text("workspace.from", Some(t))).is_some() }
/// A start refused before anything was made: the task waits, failed, with the reason and what to do.
pub fn refuse_start(g: &mut Guard, why: &str) -> Res<()> {
    let first = why.split(". ").next().unwrap_or(why).trim_end_matches('.').to_string();
    let t = g.set(|t| {
        t.put("stage", "failed"); t.put("msg", format!("Could not start: {first}"));
        for (k, v) in failure_patch(&obj(json!({ "stage": "start", "title": "Could not start from that branch", "detail": why, "agentCanFix": false, "auth": false,
            "next": "Change workspace.from or the remote as the reason says, then delegate again." }))) { t.insert(k, v); }
    })?;
    flow::sidebar::tell_sidebar(&t);
    Ok(())
}

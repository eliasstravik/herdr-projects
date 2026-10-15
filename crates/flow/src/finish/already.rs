//! Work the board opens no pull request of its own for: a branch that brings main nothing (nothing needed changing, the
//! agent merged its own pull request, or pushed to main itself), a pull request the agent already opened for the task's
//! branch, and a repo archived on GitHub. Seen on HSW-3, SKL-8, ELA-2, ELA-3, HSW-213, HP-5 and HSW-219.
use crate::flow::git::{change_of, gh};
use crate::json::*;
use crate::store::repos::git_out;
use crate::store::tasks::Task;
use std::path::Path;

/// The pull requests GitHub has for this head branch, newest first: { number, url, state }. None when gh cannot say.
fn prs_of(branch: &str, cwd: &Path) -> Vec<Value> {
    if branch.is_empty() { return vec![]; }
    let r = gh(&["pr", "list", "--head", branch, "--state", "all", "--limit", "10", "--json", "number,url,state"], cwd, 60000);
    if r.status != 0 { return vec![]; }
    serde_json::from_str::<Value>(r.stdout.trim()).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default()
}
/// The pull request the agent opened itself for the task's branch, still open: the board takes it as its own.
pub fn open_pr(branch: &str, cwd: &Path) -> Option<String> {
    prs_of(branch, cwd).into_iter().find(|p| p.s("state") == "OPEN").map(|p| p.t("url")).filter(|u| !u.is_empty())
}
/// Whether the task made commits of its own: those on its branch before main was brought in now, that neither the
/// branch it started from nor main as it was last brought in has.
pub fn made_commits(tree: &Path, base: &str, before: &str, prev_main: &str) -> bool {
    let mut args = vec!["rev-list", "--count", before, "--not", base];
    if !prev_main.is_empty() { args.push(prev_main); }
    git_out(tree, &args).ok().and_then(|n| n.parse::<i64>().ok()).unwrap_or(0) > 0
}
/// Why the task is done without a pull request, when its branch brings main nothing: main (brought in just before) has
/// everything on it. `ask`: GitHub may be asked for a pull request of the branch that was merged. None: there is work
/// to land.
pub fn nothing_to_land(t: &Task, tree: &Path, own: bool, ask: bool) -> Option<String> {
    let ch = change_of(t)?;
    if !ch.s("stat").is_empty() { return None; }
    let merged = if ask { prs_of(t.s("branch"), tree).into_iter().find(|p| p.s("state") == "MERGED") } else { None };
    Some(match merged {
        Some(p) => format!("Already merged in #{}", p.t("number")),
        None if own => "Already on main".into(),
        None => "No change needed".into(),
    })
}
/// The repo is archived on GitHub: read-only, nothing can be pushed to it or merged.
pub fn archived(cwd: &Path) -> bool {
    let r = gh(&["repo", "view", "--json", "isArchived", "--jq", ".isArchived"], cwd, 60000);
    r.status == 0 && r.stdout.trim() == "true"
}
/// The failure for work that cannot land because its repo is archived. It is not on main, so it is never done.
pub fn archived_failure(branch: &str, tree: &str) -> Obj {
    let mut f = super::shared::failure("push", "The repo is archived on GitHub", &format!("Branch: {branch}\nFolder: {tree}\n\nThe repo on GitHub is archived, so it is read-only: the branch cannot be pushed and its work is not on main."), "If this work should land, unarchive the repo on GitHub and finish again. If it is not needed, archive the task.");
    f.put("agentCanFix", false);
    f
}

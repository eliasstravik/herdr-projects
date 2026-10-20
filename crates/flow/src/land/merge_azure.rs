//! The merge through a pull request on Azure DevOps, as merge.rs does it through GitHub's: at the reviewed commit only,
//! marked ready, its policies passed (checks.rs, `merge.checks`), then completed by `merge.method` (squash, merge with
//! no fast-forward, rebase) with its source branch deleted. Completed, it waits for the base branch's pipeline when
//! `merge.pipeline` asks for it (main_pipeline.rs), else it is done.
use super::merge::{fail, landed, Fail};
use crate::azure::{pr, Azure};
use crate::err::Res;
use crate::finish::auto_merge::called_off;
use crate::flow;
use crate::flow::git::{CHECK_BAD, CHECK_GOOD};
use crate::json::*;
use crate::runner::Guard;
use crate::store::keys::{repo_keys, task_ref};
use crate::store::repos::{git_out, git_slow};
use crate::store::tasks::Task;
use std::path::Path;

pub fn through_pr(id: &str, g: &mut Guard, t: &Task, root: &Path, how: &str, a: &Azure) -> Res<()> {
    let pr = t.t("pr");
    let branch = if t.b("branch") { t.t("branch") } else { "unknown".into() };
    // Only the commit that was checked and reviewed here may merge.
    let reviewed = t.at("mergeIntent").t("verifiedHead");
    if !reviewed.is_empty() {
        let asked = || pr::view(a, root, &pr, "headRefOid").map(|v| v.t("headRefOid")).unwrap_or_default();
        let mut remote = asked();
        // Asked right after a push, Azure DevOps may still name the commit before it: asked again, a few times.
        let lag = std::env::var("HERDR_PROJECTS_PR_LAG_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(5000u64);
        let mut i = 0;
        while i < 6 && !remote.is_empty() && remote != reviewed && flow::git::remote_head(root, t.s("branch")) == reviewed { if lag > 0 { std::thread::sleep(std::time::Duration::from_millis(lag)); } remote = asked(); i += 1; }
        if !remote.is_empty() && remote != reviewed {
            return fail(g, "Pull request has commits that were not reviewed", &format!("Pull request: {pr}\nBranch: {branch}\nReviewed here: {reviewed}\nOn the pull request: {remote}"), &format!("Bring the pull request's commits into your own worktree (git pull origin {branch}), or push what was reviewed, then finish again so it is checked and reviewed before it merges."), Fail { auth: Some(false), ..Default::default() });
        }
    }
    if called_off(g) { return Ok(()); }
    pr::ready(a, root, &pr);
    // Its policies (`merge.checks`): still running it waits, failed it goes back (checks.rs).
    if !super::checks::before_merge(id, g, t, root)? { return Ok(()); }
    let message = format!("{}\n\nTask: {}", if t.b("title") { t.t("title") } else { id.to_string() }, task_ref(id, &repo_keys()));
    let m = pr::complete(a, root, &pr, &reviewed, how, &message);
    if m.status != 0 {
        // Its policies are still running, or a reviewer's vote is missing: the approval is remembered.
        let v = pr::view(a, root, &pr, "statusCheckRollup").unwrap_or(json!({}));
        if v.s("state") != "MERGED" {
            let states: Vec<String> = v.arr("statusCheckRollup").iter().map(super::checks::word).collect();
            let waiting = !states.iter().any(|s| CHECK_BAD.contains(&s.as_str())) && v.s("reviewDecision") != "CHANGES_REQUESTED" && (states.iter().any(|s| !CHECK_GOOD.contains(&s.as_str())) || v.s("reviewDecision") == "REVIEW_REQUIRED");
            if waiting { g.set(|t| { t.put("failure", Value::Null); t.put("autoMerge", true); t.put("autoMergeLocal", true); let m = format!("Approved: merges when Azure DevOps' policies pass ({})", t.s("pr")); t.put("msg", m); })?; return Ok(()); }
            return fail(g, "Pull request could not complete", &format!("Pull request: {pr}\nBranch: {branch}\n\nAzure DevOps (completing the pull request):\n{}\n\nNo policy is still running and no reviewer is awaited, so there is nothing to wait for.", m.stderr.trim()),
                "Inspect the pull request in Azure DevOps: its target branch, conflicts and policies. Resolve branch conflicts in your own worktree, or fix failed checks, then finish again. Approval is required before retrying the merge.", Fail::default());
        }
    }
    // Bring your checkout along when that is safe.
    if git_slow(root, &["fetch", "origin"], 60000).is_ok() && git_out(root, &["status", "--porcelain", "--untracked-files=no"]).map(|s| s.is_empty()).unwrap_or(false) { let _ = git_out(root, &["merge", "--ff-only", "@{upstream}"]); }
    // Accepted is not completed: Azure DevOps may still be merging it, or wait for an auto-complete.
    let seen = pr::view(a, root, &pr, "state").unwrap_or(Value::Null);
    // Watched from then on by its merge status, at most `check.timeout` (queued.rs).
    if seen.s("state") != "MERGED" { g.set(|t| { t.put("failure", Value::Null); t.put("autoMerge", true); t.put("autoMergeLocal", false); super::queued::arm(t); let m = format!("Approved: Azure DevOps completes it when its policies are through ({})", t.s("pr")); t.put("msg", m); })?; return Ok(()); }
    flow::git::drop_remote_branch(root, t.s("branch"), &reviewed);
    let head = if reviewed.is_empty() { t.t("prHead") } else { reviewed.clone() };
    let commit = seen.at("mergeCommit").t("oid");
    if super::main_pipeline::applies(t) {
        let base = seen.t("baseRefName");
        let d = g.set(|t| super::main_pipeline::arm(t, &commit, &base, "always", &head))?;
        flow::sidebar::tell_sidebar(&d);
        return Ok(());
    }
    landed(id, g, t, &format!("Merged: {pr}"), &head, &commit)
}

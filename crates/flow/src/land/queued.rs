//! A completion Azure DevOps accepted but has not carried out (merge_azure.rs): watched by its `mergeStatus` on every
//! poll. Its merge failed (a conflict, a policy that rejected it, a failure of Azure DevOps' own), or it is still not
//! completed after `check.timeout`: the task stops waiting and goes back to you with why, never waiting in silence.
use crate::clock::now;
use crate::json::*;
use crate::store::settings::lookup::setting_text;
use crate::store::settings::values::ms;

/// Remembered on the task when Azure DevOps accepted the completion: when, so that the wait has an end.
pub fn arm(t: &mut Obj) { t.put("completeQueued", json!({ "since": now() })); }

/// What went wrong with an accepted completion, as a failure's fields (`title`, `detail`, `next`, `agentCanFix`), or
/// None while it may still complete. `j` is the pull request in gh's shape, still open.
pub fn stalled(t: &Obj, j: &Value) -> Option<Value> {
    if !t.b("autoMerge") || t.b("autoMergeLocal") || !t.b("completeQueued") { return None; }
    let pr = t.t("pr");
    let branch = t.so("branch").unwrap_or("unknown").to_string();
    let status = j.t("mergeStatus");
    let failure = |title: &str, why: &str, next: &str, agent: bool| Some(json!({ "stage": "merge", "title": title, "detail": format!("Pull request: {pr}\nBranch: {branch}\nMerge status: {}\n\n{why}", if status.is_empty() { "not set" } else { &status }), "next": next, "agentCanFix": agent, "auth": false }));
    match status.as_str() {
        "conflicts" => return failure("Pull request could not complete: its branch conflicts", "Azure DevOps accepted the completion, but its merge into the target branch has conflicts. Nothing was merged.",
            &format!("Merge the target branch into {branch} in your own worktree, resolve the conflicts, then finish again so it is checked and reviewed before it merges."), true),
        "rejectedByPolicy" => return failure("Pull request could not complete: a policy rejected it", "Azure DevOps accepted the completion, but a branch policy rejected its merge. Nothing was merged.",
            "Inspect the pull request's policies in Azure DevOps, then approve again.", false),
        "failure" => return failure("Pull request could not complete", "Azure DevOps accepted the completion, but its merge failed. Nothing was merged.",
            "Inspect the pull request in Azure DevOps, then approve again.", false),
        _ => {}
    }
    let limit = ms(&setting_text("check.timeout", Some(t)));
    if limit > 0 && now() - t.at("completeQueued").i("since") >= limit {
        let timeout = setting_text("check.timeout", Some(t));
        return failure("Pull request was not completed", &format!("Azure DevOps accepted the completion, but the pull request is still active after {timeout} (check.timeout). Nothing was merged."),
            "Inspect the pull request in Azure DevOps: complete it there (the task is then done by itself), or approve again.", false);
    }
    None
}

//! The main pipeline's gate, on Azure DevOps: a pull request merged into the base branch is not done until the
//! pipeline run(s) of that branch for the merge commit have passed (`merge.pipeline`: `all`, pipelines by name or id,
//! or `none`). Meanwhile the task stays at stage "merging" with `pipelineWait`, its row saying how far the runs are,
//! looked at on the pull requests' cadence (housekeeping/prs.rs). Passed: done, and cleaned up. Failed or cancelled:
//! it waits for you with the reason and the end of the failing job's log, `fix` sends that to the agent, whose next
//! finish opens a new pull request; `retry` looks at the runs again. Longer than `merge.pipelineTimeout`: it waits for you.
use crate::azure::{self, runs};
use crate::clock::now;
use crate::flow::{git::drop_remote_branch, sidebar::tell_sidebar, spaces::tidy_mark};
use crate::json::*;
use crate::store::failures::{failure_patch, is_auth_failure};
use crate::store::repos::folder_of;
use crate::store::settings::lookup::setting_text;
use crate::store::settings::values::ms;
use crate::store::tasks::{get, update_wait, Change, Task};

/// With `merge.pipeline` at `all` and no run of the merge commit after this long, no pipeline runs for it.
const START_GRACE_MS: i64 = 10 * 60000;
/// Does the gate hold this task's merge? Its repo is at Azure DevOps and `merge.pipeline` is not `none`.
pub fn applies(t: &Obj) -> bool { setting_text("merge.pipeline", Some(t)) != "none" && azure::host(&folder_of(t.s("id"))).is_some() }
/// The task merged at `commit` into `base`: it waits for the pipeline. `mode`/`head`: how its clean-up deletes its
/// branch, and the commit that merged, for when it is done.
pub fn arm(t: &mut Obj, commit: &str, base: &str, mode: &str, head: &str) {
    t.put("stage", "merging"); t.put("mergedAt", now()); t.put("since", now());
    if !commit.is_empty() { t.put("mergeCommit", commit); }
    for k in ["failure", "prFailure", "mergeIntent", "lineAt", "lineTurn", "checksWait", "pipelineFailed"] { t.put(k, Value::Null); }
    for k in ["autoMerge", "autoMergeLocal", "mergeFailed", "requireApproval"] { t.put(k, false); }
    t.put("prMerged", true); t.put("prState", "merged");
    t.put("pipelineWait", json!({ "commit": commit, "base": base, "since": now(), "mode": mode, "head": head }));
    t.put("msg", format!("Merged: {}. Waiting for the main pipeline to start", t.s("pr")));
}
fn say(id: &str, msg: &str) {
    if let Ok(t) = update_wait(id, 10000, |t| { if !t.b("pipelineWait") || t.s("msg") == msg { return Ok(Change::Keep); } t.put("msg", msg); Ok(Change::Write) }) { tell_sidebar(&t); }
}
/// It is done: passed, or nothing to wait for. Cleaned up as a merged task is.
fn done(id: &str, words: &str) {
    let (mut mode, mut head) = ("always".to_string(), String::new());
    let Ok(t) = update_wait(id, 10000, |t| {
        if !t.b("pipelineWait") { return Ok(Change::Keep); }
        let w = t.at("pipelineWait").clone();
        mode = if w.s("mode").is_empty() { "always".into() } else { w.t("mode") }; head = w.t("head");
        let m = tidy_mark(t, &mode, &head);
        t.put("stage", "done"); t.put("tidy", m.unwrap_or(Value::Null)); t.put("pipelineWait", Value::Null);
        t.put("msg", format!("Merged: {}. {words}", t.s("pr")));
        Ok(Change::Write)
    }) else { return };
    if t.s("stage") != "done" { return; }
    tell_sidebar(&t);
    crate::land::cleanup::cleanup_job(id, &mode);
    let t = get(id).unwrap_or(t);
    if mode == "always" { drop_remote_branch(&folder_of(id), t.s("branch"), &head); }
}
/// It waits for you: the runs failed (`agent`: the agent may fix it), or did not finish in time.
fn stopped(id: &str, title: &str, detail: &str, next: &str, agent: bool) {
    let Ok(t) = update_wait(id, 10000, |t| {
        if !t.b("pipelineWait") { return Ok(Change::Keep); }
        let w = t.at("pipelineWait").clone();
        t.put("stage", "failed"); t.put("pipelineWait", Value::Null); t.put("msg", title);
        t.put("pipelineFailed", json!({ "commit": w.at("commit"), "base": w.at("base"), "mode": w.at("mode"), "head": w.at("head"), "pr": t.at("pr") }));
        for (k, v) in failure_patch(&obj(json!({ "stage": "pipeline", "title": title, "detail": detail, "next": next, "agentCanFix": agent, "auth": is_auth_failure(detail) && !agent }))) { t.insert(k, v); }
        Ok(Change::Write)
    }) else { return };
    tell_sidebar(&t);
}
fn overdue(t: &Task) -> bool { let l = ms(&setting_text("merge.pipelineTimeout", Some(t))); l > 0 && now() - t.at("pipelineWait").i("since") >= l }
/// One look at the runs of a task that waits for them.
pub fn look(id: &str) {
    let Some(t) = get(id) else { return };
    let w = t.at("pipelineWait").clone();
    if !w.is_object() || t.s("stage") != "merging" { return; }
    let root = folder_of(id);
    let setting = setting_text("merge.pipeline", Some(&t));
    let Some(a) = azure::host(&root).filter(|_| setting != "none") else { return done(id, "The main pipeline is not waited for (merge.pipeline is none).") };
    let (commit, base) = (w.t("commit"), w.t("base"));
    let timeout = setting_text("merge.pipelineTimeout", Some(&t));
    let all = match runs::of_commit(&a, &root, &base, &commit) {
        Ok(r) => r,
        Err(said) => {
            let why: String = said.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("no answer").chars().take(160).collect();
            if overdue(&t) { return stopped(id, "Could not read the main pipeline", &format!("Merge commit: {commit} on {base}\n\naz pipelines runs list:\n{said}"), &format!("{} Then retry the task to look at its pipeline again.", azure::cli::LOGIN), false); }
            return say(id, &format!("Merged. Waiting for the main pipeline · could not read its runs ({why})"));
        }
    };
    let wanted = runs::wanted(&all, &setting);
    let missing = runs::not_started(&all, &setting);
    let late = |what: &str| stopped(id, "The main pipeline did not finish", &format!("Merge commit: {commit} on {base}\nNot finished after {timeout} (merge.pipelineTimeout): {what}\n\nThe work is merged; the task is not done until its pipeline passed."),
        "Look at the pipeline in Azure DevOps: it may wait for an agent or an approval. Retry the task to wait for it again.", false);
    match runs::look(&a, &root, &wanted, &missing) {
        runs::Look::Passed(words) => done(id, &format!("Main pipeline passed: {words}")),
        runs::Look::Failed { why, detail } => stopped(id, &format!("Main pipeline failed: {why}"), &format!("{detail}\n\nMerge commit: {commit} on {base}. The work is merged; the task is not done until its pipeline passed."),
            &format!("Fix what failed on your branch (it has {base} with this merge in it), run the checks and finish again: a new pull request then brings the fix in. Or run the pipeline again in Azure DevOps and retry the task."), true),
        runs::Look::NotStarted(names) if names.is_empty() => {
            // `all`, and no run: a repo with no pipeline, or none that runs for this commit (its triggers, its paths).
            if runs::any_pipeline(&a, &root) == Some(false) { return done(id, "No pipeline in this repo to wait for."); }
            if now() - w.i("since") >= START_GRACE_MS { return done(id, &format!("No pipeline ran for the merge commit on {base}.")); }
            say(id, &format!("Merged: {}. Waiting for the main pipeline to start", t.s("pr")));
        }
        runs::Look::NotStarted(names) => { if overdue(&t) { return late(&format!("{} not started", names.join(", "))); } say(id, &format!("Merged. Waiting for the pipeline {} to start", names.join(", "))); }
        runs::Look::Running(words) => { if overdue(&t) { return late(&words); } say(id, &words); }
    }
}
/// `retry` on a task whose pipeline failed or did not finish: it waits for (and looks at) the runs again.
pub fn rearm(id: &str) -> crate::err::Res<Task> {
    let t = update_wait(id, 10000, |t| {
        let f = t.at("pipelineFailed").clone();
        if !f.is_object() { crate::fail!("{} has no pipeline to look at again.", t.s("id")); }
        arm(t, f.s("commit"), f.s("base"), f.s("mode"), f.s("head"));
        Ok(Change::Write)
    })?;
    look(id);
    Ok(get(id).unwrap_or(t))
}
/// Every task that waits for its main pipeline gets a look (housekeeping/prs.rs, on the pull requests' cadence).
pub fn pass(tasks: &[Task]) {
    for t in tasks { if t.b("pipelineWait") && t.s("stage") == "merging" && !t.b("archivedAt") && !crate::runner::runner_alive(t) { look(t.s("id")); } }
}

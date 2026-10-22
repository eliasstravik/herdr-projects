//! The merge itself, with the task in stage "merging": the last catch-up with main, then through the pull request (at
//! the reviewed commit only) or locally into the branch your checkout is on, squashed by default.
use super::cleanup::tidy;
use crate::clock::now;
use crate::err::Res;
use crate::finish::auto_merge::called_off;
use crate::finish::line::take_turn;
use crate::flow;
use crate::flow::git::{gh, pr_number, CHECK_BAD, CHECK_GOOD};
use crate::json::*;
use crate::runner::{claim, is_aborted, release, Guard};
use crate::store::failures::{failure_patch, is_auth_failure};
use crate::store::keys::{repo_keys, task_ref};
use crate::store::repos::{folder_of, git_out, git_slow};
use crate::store::settings::lookup::setting_text;
use crate::store::tasks::Task;
use crate::sync;
use std::path::Path;

pub fn land_job(id: &str) {
    let Ok(Some(t)) = claim(id, "land", |_| {}) else { return };
    if t.s("stage") != "merging" { release(id, false); return; }
    let mut g = Guard::new(id);
    g.expect(&t, &[]);
    if let Err(e) = land_core(id, &mut g) {
        if !is_aborted(&e) {
            let _ = g.set(|t| { t.put("stage", "approval"); t.put("mergeFailed", true); t.put("msg", "Merge failed"); for (k, v) in failure_patch(&obj(json!({ "stage": "merge", "title": "Merge failed", "detail": e.msg, "next": "Investigate the error, then approve again." }))) { t.insert(k, v); } });
        }
    }
    release(id, false);
}
pub(super) struct Fail { pub agent_can_fix: bool, pub auth: Option<bool>, pub keep_intent: bool }
impl Default for Fail { fn default() -> Self { Fail { agent_can_fix: true, auth: None, keep_intent: false } } }
/// A failed merge waits for you again, as plain "permission": the row, panel and details show the blocker.
pub(super) fn fail(g: &mut Guard, title: &str, detail: &str, next: &str, f: Fail) -> Res<()> {
    let auth = f.auth.unwrap_or_else(|| is_auth_failure(detail));
    let t = g.set(|t| {
        t.put("stage", if auth { "failed" } else { "approval" }); t.put("mergeFailed", true); t.put("msg", title);
        if !f.keep_intent { t.put("mergeIntent", Value::Null); }
        for (k, v) in failure_patch(&obj(json!({ "stage": "merge", "title": title, "detail": detail, "next": next, "agentCanFix": f.agent_can_fix, "auth": auth }))) { t.insert(k, v); }
    })?;
    flow::sidebar::tell_sidebar(&t);
    Ok(())
}
pub(super) fn no_fix() -> Fail { Fail { agent_can_fix: false, auth: Some(false), keep_intent: false } }
/// `head`: the commit that was merged. Nothing newer on the branch is taken for merged. `commit`: the commit it made on
/// the branch it landed on, what a task that waited for it is told (after/handed.rs).
fn landed(id: &str, g: &mut Guard, t: &Task, msg: &str, head: &str, commit: &str) -> Res<()> {
    let mark = flow::spaces::tidy_mark(t, "always", head);
    // The issues its work closes (`merge.closes`), only once something landed.
    let closed = if head.is_empty() && commit.is_empty() { String::new() } else { super::closes::after_landing(t, &folder_of(t.s("id")), commit) };
    let d = g.set(|t| {
        if !commit.is_empty() { t.put("mergeCommit", commit); }
        // What GitHub's checks could not be read of (land/checks.rs) is said with it.
        let said = if t.b("checksUnread") { format!("{msg}. {}", t.s("checksUnread")) } else { msg.to_string() };
        let said = if closed.is_empty() { said } else { format!("{}. {closed}", said.trim_end_matches('.')) };
        t.put("stage", "done"); t.put("mergedAt", now()); t.put("tidy", mark.clone().unwrap_or(Value::Null)); t.put("msg", said);
        for k in ["failure", "prFailure", "mergeIntent", "lineAt", "lineTurn", "checksWait"] { t.put(k, Value::Null); }
        for k in ["autoMerge", "autoMergeLocal", "mergeFailed", "requireApproval"] { t.put(k, false); }
        if t.b("pr") { t.put("prMerged", true); t.put("prState", "merged"); }
    })?;
    flow::sidebar::tell_sidebar(&d);
    tidy(id, "always");
    Ok(())
}
fn reverify(id: &str, g: &mut Guard, t: &Task, why: &str, synced: Option<&sync::Synced>) -> Res<()> {
    g.set(|t| t.put("msg", why))?;
    let summary = if t.b("summary") { t.t("summary") } else { "Branch updated with main.".into() };
    crate::finish::pipeline::pipeline(id, g, &summary, true, "sync", synced)
}
/// A task started from a named branch (`workspace.from develop`) lands there through its pull request only, and only
/// while that merges into it. False: it may not merge, and that was said on the task.
fn onto_named(id: &str, g: &mut Guard, t: &Task, root: &Path, tree: &str, b: &str) -> Res<bool> {
    if !t.b("pr") && !tree.is_empty() {
        fail(g, &format!("Lands on {b} only through a pull request"), &format!("This task started from {b} at the remote and keeps up with it. It has no pull request (merge.pr is off), and a local merge would land on the branch your checkout is on instead.\n\nNothing was merged."),
            &format!("Turn merge.pr on for this task (herdr-projects set merge.pr true --task {}) and tell its agent to finish again: the board then opens its pull request into {b}.", task_ref(id, &repo_keys())), no_fix())?;
        return Ok(false);
    }
    if !t.b("pr") { return Ok(true); }
    let r = gh(&["pr", "view", &pr_number(t.s("pr")), "--json", "baseRefName"], root, 60000);
    let base = if r.status == 0 { serde_json::from_str::<Value>(&r.stdout).map(|v| v.t("baseRefName")).unwrap_or_default() } else { String::new() };
    // Not known is not "into b": nothing merges until GitHub says which branch it merges into.
    if base.is_empty() {
        let said = [r.stderr.trim(), r.stdout.trim()].into_iter().find(|s| !s.is_empty()).unwrap_or("no branch named").to_string();
        fail(g, "Could not read which branch the pull request merges into", &format!("Pull request: {}\nThis task started from, and lands on: {b}\n\ngh pr view:\n{said}\n\nNothing was merged.", t.s("pr")),
            "Make sure gh is logged in and reaches GitHub (gh auth status), then approve again.", Fail { agent_can_fix: false, auth: Some(is_auth_failure(&said)), keep_intent: false })?;
        return Ok(false);
    }
    if base == b { return Ok(true); }
    fail(g, &format!("The pull request merges into {base}, not {b}"), &format!("Pull request: {}\nIt merges into: {base}\nThis task started from, and lands on: {b}\n\nNothing was merged.", t.s("pr")),
        &format!("Change the pull request's base back to {b} (gh pr edit {} --base {b}), then approve again.", pr_number(t.s("pr"))), no_fix())?;
    Ok(false)
}
/// Main could not be fetched right before the merge: nothing is merged, and nothing failed. The task waits for your
/// approval again as "behind main", which it cannot be given until the board's own retry brings it up to date.
fn behind(g: &mut Guard, r: &sync::Synced) -> Res<()> {
    let why = sync::held_why(r);
    let t = g.set(|t| {
        let p = sync::seen_patch(t, None); sync::apply(t, p);
        t.put("stage", "approval"); t.put("since", now()); t.put("mergeIntent", Value::Null); t.put("autoMerge", false); t.put("autoMergeLocal", false);
        t.put("syncBlocked", sync::blocked(&why, Some(r))); t.put("msg", sync::behind_msg(&why));
    })?;
    flow::sidebar::tell_sidebar(&t);
    Ok(())
}
/// Runs with the task in stage "merging" and this runner's guard standing on it.
pub fn land_core(id: &str, g: &mut Guard) -> Res<()> {
    if called_off(g) { return Ok(()); } // `merge.by` went back to `you` before this merge of the board's own began
    // One merge at a time in a repo: behind another task this one waits, and catches up with main when its turn comes.
    if !take_turn(id, g)? { return Ok(()); }
    let mut t = g.check()?;
    let root = folder_of(t.s("id"));
    let tree = sync::tree_of(&t);
    let how = setting_text("merge.method", Some(&t)); // read right before the merge. squash by default
    if let Some(b) = sync::named_base(&root, &t) { if !onto_named(id, g, &t, &root, &tree, &b)? { return Ok(()); } }
    if !tree.is_empty() {
        let tp = Path::new(&tree);
        let pre = (|| -> Res<Option<&'static str>> {
            if !t.b("pr") && !flow::git::dirty(&root).is_empty() { fail(g, "Merge needs a clean checkout", &format!("Checkout: {}\n{}", root.display(), flow::git::dirty(&root)), "Commit, stash or move your own unsaved files, then approve again. The agent must not alter your checkout.", no_fix())?; return Ok(Some("done")); }
            if git_out(tp, &["symbolic-ref", "--short", "HEAD"])? != t.s("branch") { fail(g, "Task is on the wrong branch", &tree, "Restore the task's own branch before retrying.", no_fix())?; return Ok(Some("done")); }
            // Approve refuses unless the worktree is clean and its HEAD is exactly the commit that was checked and reviewed.
            let vh = t.at("mergeIntent").t("verifiedHead");
            if !flow::git::dirty(tp).is_empty() || (!vh.is_empty() && flow::git::head_of(tp) != vh) { return Ok(Some("reverify")); }
            if !t.b("mergeIntent") { let h = flow::git::head_of(tp); t = g.set(|t| t.put("mergeIntent", json!({ "approvedAt": now(), "repairs": 0, "verifiedHead": h })))?; }
            Ok(None)
        })();
        match pre {
            Ok(Some("done")) => return Ok(()),
            Ok(Some(_)) => return reverify(id, g, &t, "Checking work that changed after its review", None),
            Ok(None) => {}
            Err(e) if is_aborted(&e) => return Err(e),
            Err(e) => return fail(g, "Could not read the task's worktree", &format!("Worktree: {tree}\n\nWhat git said:\n{}", e.msg), "Check that the worktree is still there and on the task's own branch, then approve again.", Fail::default()),
        }
        // The last safety net: the branch was brought up to date before its review and whenever main moved since.
        let r = sync::sync_main(&t, &tree);
        match r.status {
            "skip" => return fail(g, "Your checkout is on another branch", &format!("Checkout: {}\nIt is on: {}\nThis task lands on: {}\n\nNothing was merged.", root.display(), r.onto.as_deref().filter(|s| !s.is_empty()).unwrap_or("no branch"), r.want), &format!("Switch your checkout back to {}, then approve again.", r.want), no_fix()),
            "error" => return behind(g, &r),
            "conflict" => {
                sync::conflict_back(id, g, &tree, &r, &mut |g, failure, seen| {
                    if let Ok(f) = g.set(|t| { for (k, v) in failure_patch(&failure) { t.insert(k, v); } for (k, v) in seen { t.insert(k, v); } t.put("stage", "approval"); t.put("mergeFailed", true); t.put("msg", failure.at("title").clone()); }) { flow::sidebar::tell_sidebar(&f); }
                });
                return Ok(());
            }
            // A clean update changed the commit: it is checked again, then merged under the approval given.
            "merged" => return reverify(id, g, &t, "Checking the branch updated with main", Some(&r)),
            _ => { let p = sync::seen_patch(&t, Some(&r)); g.set(|t| sync::apply(t, p))?; }
        }
    }
    t = g.check()?;
    if t.b("pr") {
        // By number, not link: gh then uses the repo's own remote for the host.
        let pr = pr_number(t.s("pr"));
        // Only the commit that was checked and reviewed here may merge.
        let reviewed = t.at("mergeIntent").t("verifiedHead");
        if !reviewed.is_empty() {
            let asked = || serde_json::from_str::<Value>(&gh(&["pr", "view", &pr, "--json", "headRefOid"], &root, 60000).stdout).map(|v| v.t("headRefOid")).unwrap_or_default();
            let mut remote = asked();
            // Asked right after a push, GitHub may still name the commit before it: asked again, a few times.
            let lag = std::env::var("HERDR_PROJECTS_PR_LAG_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(5000u64);
            let mut i = 0;
            while i < 6 && !remote.is_empty() && remote != reviewed && flow::git::remote_head(&root, t.s("branch")) == reviewed { if lag > 0 { std::thread::sleep(std::time::Duration::from_millis(lag)); } remote = asked(); i += 1; }
            if !remote.is_empty() && remote != reviewed {
                let b = if t.b("branch") { t.t("branch") } else { "<branch>".into() };
                return fail(g, "Pull request has commits that were not reviewed", &format!("Pull request: {}\nBranch: {}\nReviewed here: {reviewed}\nOn the pull request: {remote}", t.s("pr"), if t.b("branch") { t.s("branch") } else { "unknown" }), &format!("Bring the pull request's commits into your own worktree (git pull origin {b}), or push what was reviewed, then finish again so it is checked and reviewed before it merges."), Fail { auth: Some(false), ..Default::default() });
            }
        }
        if called_off(g) { return Ok(()); }
        gh(&["pr", "ready", &pr], &root, 60000);
        // GitHub's own checks on it (`merge.checks`): still running it waits, failed it goes back (checks.rs).
        if !super::checks::before_merge(id, g, &t, &root)? { return Ok(()); }
        // The issues it closes (`merge.closes`) as said now: on its body, and the board's lines of those it no longer
        // names off it. A line that could not be added: closed with gh once it has merged. One that could not be taken
        // off would close an issue the task no longer names: nothing merges (closes.rs).
        match super::closes::onto_pr(&t, t.s("pr"), &root) {
            Ok(on) => { g.set(|t| t.put("closesOnPr", json!(on)))?; }
            Err(why) if !super::closes::stale(&t).is_empty() => {
                let gone = super::closes::stale(&t).iter().map(|n| format!("Closes #{n}")).collect::<Vec<_>>().join(", ");
                return fail(g, "Could not take an issue off the pull request", &format!("Pull request: {}\nmerge.closes no longer names: {gone}\n\ngh pr edit:\n{why}\n\nNothing was merged: GitHub would close that issue.", t.s("pr")), "Make sure gh is logged in and reaches GitHub (gh auth status), or take those lines off the pull request's description yourself, then approve again.", no_fix());
            }
            Err(_) => {}
        }
        let mut a = vec!["pr", "merge", pr.as_str()];
        let how_flag = format!("--{how}");
        a.push(&how_flag);
        if !reviewed.is_empty() { a.extend(["--match-head-commit", reviewed.as_str()]); }
        let m = gh(&a, &root, 120000);
        if m.status != 0 {
            // GitHub's checks are still running, or a required review is missing: the approval is remembered.
            let v: Value = super::readable::pr_view(&root, &pr, "reviewDecision").map(|r| r.view).unwrap_or(json!({}));
            let states: Vec<String> = v.arr("statusCheckRollup").iter().map(|c| if c.b("conclusion") { c.t("conclusion") } else { c.t("state") }.to_uppercase()).collect();
            let waiting = !states.iter().any(|s| CHECK_BAD.contains(&s.as_str())) && v.s("reviewDecision") != "CHANGES_REQUESTED" && (states.iter().any(|s| !CHECK_GOOD.contains(&s.as_str())) || v.s("reviewDecision") == "REVIEW_REQUIRED");
            if waiting { g.set(|t| { t.put("failure", Value::Null); t.put("autoMerge", true); t.put("autoMergeLocal", true); let m = format!("Approved: merges when GitHub's checks pass ({})", t.s("pr")); t.put("msg", m); })?; return Ok(()); }
            let out = [m.stdout.trim(), m.stderr.trim()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n");
            return fail(g, "Pull request could not merge", &format!("Pull request: {}\nBranch: {}\n\ngh pr merge:\n{out}\n\nNo check is still running and no review is awaited, so there is nothing to wait for.", t.s("pr"), if t.b("branch") { t.s("branch") } else { "unknown" }), "Inspect the pull request's base branch, conflicts and checks. Resolve branch conflicts in your own worktree, or fix failed checks, then finish again. Approval is required before retrying the merge.", Fail::default());
        }
        // Bring your checkout along when that is safe.
        if git_slow(&root, &["fetch", "origin"], 60000).is_ok() && git_out(&root, &["status", "--porcelain", "--untracked-files=no"]).map(|s| s.is_empty()).unwrap_or(false) { let _ = git_out(&root, &["merge", "--ff-only", "@{upstream}"]); }
        // gh answering "fine" is not "merged": on a repo with a merge queue it only lines the pull request up.
        let seen = serde_json::from_str::<Value>(&gh(&["pr", "view", &pr, "--json", "state,mergeCommit"], &root, 60000).stdout).unwrap_or(Value::Null);
        let state = seen.t("state");
        if state != "MERGED" { g.set(|t| { t.put("failure", Value::Null); t.put("autoMerge", true); t.put("autoMergeLocal", false); let m = format!("Approved: GitHub merges it when its checks and merge queue are through ({})", t.s("pr")); t.put("msg", m); })?; return Ok(()); }
        flow::git::drop_remote_branch(&root, t.s("branch"), &reviewed);
        let head = if reviewed.is_empty() { t.t("prHead") } else { reviewed.clone() };
        return landed(id, g, &t, &format!("Merged: {}", t.s("pr")), &head, &seen.at("mergeCommit").t("oid"));
    }
    // No pull request: merge into the branch your checkout is on.
    if tree.is_empty() { let m = if t.b("msg") && !t.s("msg").to_lowercase().starts_with("merging") { t.t("msg") } else { "Done".into() }; return landed(id, g, &t, &m, "", ""); }
    let local = (|| -> Res<Option<String>> {
        let mess = flow::git::dirty(&root);
        if !mess.is_empty() { fail(g, "Merge needs a clean checkout", &format!("Checkout: {}\n\nUncommitted or untracked files:\n{mess}", root.display()), "Commit, stash or move your own unsaved files, then approve again. The agent must not alter your checkout.", no_fix())?; return Ok(None); }
        let onto = git_out(&root, &["symbolic-ref", "--short", "HEAD"])?;
        let target = { let h = flow::git::head_of(&root); if h.is_empty() { git_out(&root, &["rev-parse", "HEAD"])? } else { h } };
        let conflict = |g: &mut Guard, e: crate::err::Error| -> Res<Option<String>> {
            let files = git_out(&root, &["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
            let detail = format!("{}Task branch: {}\nTarget branch: {onto}\nTarget commit: {target}\nCheckout: {}\n\nGit output:\n{}", if files.is_empty() { String::new() } else { format!("Conflicting files:\n{files}\n\n") }, t.s("branch"), root.display(), e.msg);
            // Save the evidence before restoring the initially clean checkout. Never delete untracked files.
            let restored = git_out(&root, &["merge", "--abort"]).is_ok() || git_out(&root, &["reset", "--merge", "HEAD"]).is_ok();
            fail(g, if files.is_empty() { "Merge failed" } else { "Merge conflict" }, &format!("{detail}{}", if restored { "\n\nThe merge attempt was rolled back; the human's checkout is unchanged." } else { "\n\nRollback failed. The human must inspect the checkout before retrying." }),
                &if restored { format!("In your own worktree on {}, bring in target commit {target} ({onto}), resolve the conflicts, run the checks and commit. Do not change the human's checkout. Finish again for review, then ask for merge approval.", t.s("branch")) } else { "Inspect and restore the human's checkout before retrying. Do not let the agent reset or discard it.".into() },
                Fail { agent_can_fix: restored, auth: Some(false), keep_intent: false })?;
            Ok(None)
        };
        let message = format!("{}\n\nTask: {}", if t.b("title") { t.t("title") } else { id.to_string() }, task_ref(id, &repo_keys()));
        // The commit that was checked and reviewed is the one merged, by its id.
        let head = { let v = t.at("mergeIntent").t("verifiedHead"); if v.is_empty() { git_out(&root, &["rev-parse", "--verify", &format!("refs/heads/{}", t.s("branch"))])? } else { v } };
        if called_off(g) { return Ok(None); }
        if how == "squash" {
            if let Err(e) = git_out(&root, &["merge", "--squash", &head]) { return conflict(g, e); }
            if !git_out(&root, &["diff", "--cached", "--name-only"])?.is_empty() { if let Err(e) = git_out(&root, &["commit", "-m", &message]) { return conflict(g, e); } }
        } else if let Err(e) = git_out(&root, &["merge", if how == "rebase" { "--ff" } else { "--no-ff" }, "-m", &message, &head]) { return conflict(g, e); }
        let made = git_out(&root, &["rev-parse", "HEAD"]).unwrap_or_default();
        Ok(Some(format!("Merged into {onto}.|{head}|{made}")))
    })();
    match local {
        Ok(Some(done)) => { let p: Vec<&str> = done.splitn(3, '|').collect(); landed(id, g, &t, p[0], p.get(1).unwrap_or(&""), p.get(2).unwrap_or(&"")) }
        Ok(None) => Ok(()),
        Err(e) if is_aborted(&e) => Err(e),
        Err(e) => fail(g, "Merge failed", &e.msg, "Investigate the Git error in your own worktree, then finish again. Approve again only after the problem is fixed.", Fail::default()),
    }
}

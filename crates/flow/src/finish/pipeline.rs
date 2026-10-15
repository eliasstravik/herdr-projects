//! The pipeline: commit, bring main in, set up, check, push, pull request, review, then wait for approval. The second
//! half (the review and what follows from it) is pipeline_end.rs.
use super::already;
use super::check_again::{choose, passed_patch, CHECKED_ENV};
use super::check_run::run_check;
use super::hand_over::hand_over;
use super::pipeline_end::{after_review, Ctx};
use super::research::{kept_report, research_only};
use super::shared::{fail, failure, first_line, FailHow};
use crate::clock::now;
use crate::err::Res;
use crate::flow;
use crate::json::*;
use crate::runner::{claim, is_aborted, release, sh, Guard};
use crate::store::failures::{bound_detail, failure_patch, is_auth_failure};
use crate::store::keys::{repo_keys, task_ref};
use crate::store::repos::{branch_of, cwd_of, folder_of, git_out, git_slow};
use crate::store::reviews::{current_review, passed_review};
use crate::store::settings::lookup::work;
use crate::store::settings::values::{command_of, ms};
use crate::store::tasks::{get, one_line};
use crate::sync;
use std::path::Path;

/// "Finish anyway" on a turn that left no closing note: the screen text is the summary.
pub fn pipeline_job(id: &str) {
    let Ok(Some(t)) = claim(id, "pipeline", |_| {}) else { return };
    if t.s("stage") != "checking" { release(id, false); return; }
    let mut g = Guard::new(id);
    g.expect(&t, &[]);
    let text = [t.s("said"), t.s("screen"), t.s("msg")].into_iter().find(|s| !s.is_empty()).unwrap_or("").to_string();
    if let Err(e) = pipeline(id, &mut g, &text, false, "", None) {
        if !is_aborted(&e) { let _ = fail(&mut g, failure("workflow", "The finish line failed", &e.msg, "Investigate the error, then finish again."), FailHow::default()); } else { super::review::close_reviewer(id); }
    }
    release(id, false);
}
/// `keep_intent`/`reason`: the turn this follows was the board's own (a wake). `synced`: the caller already brought the
/// branch up to date with main (a re-sync, or the sync after an approval) and this is its result.
pub fn pipeline(id: &str, g: &mut Guard, text: &str, keep_intent: bool, reason: &str, synced: Option<&sync::Synced>) -> Res<()> {
    // A task that starts with a plan, and the build has not started: this turn was the plan's (plan/written.rs).
    if crate::plan::pending(&g.check()?) { return crate::plan::written::plan_written(id, g, text); }
    let mut t = g.set(|t| { t.del("chat"); t.del("chatKept"); t.put("stage", "checking"); t.put("syncBlocked", Value::Null); t.put("msg", if synced.is_some() { "Checking the branch updated with main" } else { "Checking the result" }); t.put("watch", Value::Null); t.put("check", Value::Null); t.put("summary", text.trim()); })?;
    let (folder, root) = (cwd_of(&t), folder_of(t.s("id")));
    let prev_main = t.t("syncedTo"); // main as it was brought in before this turn's sync
    let tree = if flow::task::is_agent_task(&t) && t.b("worktree") && Path::new(t.s("worktree")).exists() { t.t("worktree") } else { String::new() };
    let env = vec![("HERDR_PROJECTS_ID".to_string(), t.t("id")), ("HERDR_PROJECTS_WORKTREE".into(), folder.display().to_string()), ("HERDR_PROJECTS_REPO".into(), root.display().to_string())];
    // The settings of this step, read now: where it is checked. The review reads its own when it starts.
    let ch = work(Some(&t));
    let (check, setup) = (command_of(ch.at("check.command")), command_of(ch.at("workspace.setup")));
    // A research-only task changes nothing in the repo: with a change it goes back (research.rs).
    let research = match research_only(id, g, &tree)? { Some(false) => return Ok(()), Some(true) => true, None => false };
    t = kept_report(g)?;
    // Commit leftovers. A plain folder has nothing to commit.
    if !tree.is_empty() && !research && !flow::git::dirty(Path::new(&tree)).is_empty() {
        let msg = format!("{}\n\nTask: {}", if t.b("title") { t.t("title") } else { id.into() }, task_ref(id, &repo_keys()));
        if let Err(e) = git_out(Path::new(&tree), &["add", "-A"]).and_then(|_| git_out(Path::new(&tree), &["commit", "-m", &msg])) {
            fail(g, failure("commit", "Could not commit the work", &e.msg, "Fix the Git error in your own worktree, commit the work and finish again."), FailHow::default())?;
            return Ok(());
        }
    }
    t = g.check()?;
    // Bring the branch up to date with main before anything is checked or reviewed, so the reviewer sees the work as it
    // will really land. The agent hears nothing unless there is a real conflict.
    let mut own_sync: Option<sync::Synced> = None;
    if synced.is_none() && !sync::tree_of(&t).is_empty() && !research {
        let on = branch_of(Path::new(&tree));
        if on != t.s("branch") {
            let mut f = failure("merge", "Task is on the wrong branch", &format!("Worktree: {tree}\nIt is on: {}\nThe task's own branch: {}\n\nNothing was checked, reviewed or merged.", if on.is_empty() { "no branch" } else { &on }, t.s("branch")), &format!("In {tree}, bring the work onto {0} and switch back to it (git switch {0}), then finish again. Do not alter the human's checkout.", t.s("branch")));
            f.put("phase", "branch"); f.put("agentCanFix", true); f.put("auth", false);
            fail(g, f, FailHow { keep_intent: true, ..Default::default() })?;
            return Ok(());
        }
        let r = sync::sync_main(&t, &tree);
        if r.status == "conflict" { sync::conflict_back(id, g, &tree, &r, &mut |g, failure, seen| { let _ = fail(g, failure, FailHow { keep_intent: true, patch: seen }); }); return Ok(()); }
        // Main could not be read: work that was not brought up to date is never reviewed or put up for approval.
        if r.status == "error" { let mut f = failure("merge", "Could not synchronize main", &format!("Branch: {}\nFolder: {tree}\n\n{}", t.s("branch"), r.detail), "Investigate the fetch/Git error and fix what you safely can without altering the human's checkout. Finish again so the board retries the sync."); f.put("phase", "fetch"); fail(g, f, FailHow { keep_intent: true, ..Default::default() })?; return Ok(()); }
        own_sync = Some(r);
    }
    let sync_r = synced.or(own_sync.as_ref());
    if let Some(r) = sync_r { t = g.set(|t| { let p = sync::seen_patch(t, Some(r)); sync::apply(t, p); if r.status == "current" || r.status == "merged" { sync::conflict_log::conflict_repaired(t); } })?; }
    // A clean update that touched none of the files the task changed leaves its review standing. So does work that
    // passed its review and has only taken main in since (sync::merge_only is the rule): the check proves the result.
    let mut kept: Option<Value> = None;
    let mut kept_how: Option<Value> = None;
    if let Some(r) = sync_r { if r.status == "merged" && r.overlap.as_ref().map(|o| o.is_empty()).unwrap_or(false) && t.s("reviewedHead") == r.before { kept = current_review(&t).cloned(); if kept.is_some() { kept_how = Some(json!({ "from": r.before, "merges": 1, "resolved": [] })); } } }
    if kept.is_none() && !check.is_empty() && !tree.is_empty() && ch.s("review.when") != "never" {
        if let (Some(r), Some((review, head, _))) = (sync_r.filter(|r| !r.target.is_empty()), passed_review(&t)) {
            let m = sync::merge_only(&tree, &head, &r.target);
            // Where the task merges with nobody's yes, only the board's own clean merge of main stands in for a review.
            if m.b("ok") && !(!m.arr("resolved").is_empty() && super::auto_merge::merges_itself(&t)) { kept = Some(review); kept_how = Some(json!({ "from": m.at("from"), "merges": m.at("merges"), "resolved": m.at("resolved") })); }
        }
    }
    if t.b("setupPending") && !setup.is_empty() {
        let alive = || g.check().is_ok();
        let s = sh(&setup, &folder, &env, 15.0, Some(&alive), None);
        if s.aborted { return Ok(()); }
        if !s.ok { fail(g, failure("setup", "Setup still fails", &format!("Command: {setup}\nFolder: {}\n\n{}", folder.display(), s.out), "Fix the setup problem and finish again. Setup must pass before the check."), FailHow::default())?; return Ok(()); }
        g.set(|t| t.put("setupPending", false))?;
    }
    // The chosen attempt of several: its check passed on this very commit, as that attempt. Not run twice.
    let proven = t.b("checkedHead") && !tree.is_empty() && t.s("checkedWith") == check && flow::git::head_of(Path::new(&tree)) == t.s("checkedHead");
    // On a later round, the shorter check where it may stand in for the full one (check_again.rs).
    let run = choose(&t, &ch, &check, &tree, &root);
    let ran = run.shown();
    if !check.is_empty() && !research && !proven {
        let mut env = env.clone();
        if run.short() { env.push((CHECKED_ENV.into(), run.since.clone())); }
        let (r, _) = run_check(g, id, &run.cmd, run.short(), &folder, &env, &root, ms(ch.s("check.timeout")) as f64 / 60000.0)?;
        if r.aborted { return Ok(()); }
        if !r.ok {
            let merged = if kept.is_some() { " The branch was only re-merged with main since its review passed, so the tests ran instead of a new review." } else { "" };
            let fails = get(id).map(|c| c.i("checkFails")).unwrap_or(0) + 1;
            let rounds = ch.i("review.rounds");
            let last = fails > rounds; // it goes back by itself `review.rounds` times
            let auth = is_auth_failure(&r.out);
            let tail = r.out.lines().filter(|l| !l.is_empty()).last().unwrap_or("").to_string();
            let mut f = failure("check", &format!("Check failed{}", if last && fails > 1 { format!(" after {fails} tries") } else { String::new() }), &format!("Command: {ran}\nFolder: {}\n\n{}", folder.display(), r.out), "Fix the failed check, run it again and finish for a fresh review.");
            f.put("auth", auth); f.put("agentCanFix", true);
            if last && fails > 1 { f.put("msg", format!("The check still fails after {fails} tries ({ran}): {tail}")); }
            // The agent had its rounds and the check still fails: the next one on `worker.fallback` takes over.
            if last && !auth && rounds > 0 && !get(id).map(|c| c.b("mergeIntent")).unwrap_or(false) && hand_over(id, g, &format!("the check still failed after {fails} tries"), &format!("The check still fails, so this is not done yet.{merged}\n\n{}", bound_detail(f.s("detail"))))? { return Ok(()); }
            if last || auth { fail(g, f, FailHow { patch: obj(json!({ "checkFails": fails })), ..Default::default() })?; return Ok(()); }
            g.check()?;
            // Straight back to the agent. The turn it causes is the board's own: an approved merge survives it.
            let mut patch = failure_patch(&f);
            patch.obj_mut("failure").put("sentAt", now());
            patch.put("checkFails", fails);
            let keep = keep_intent || get(id).map(|c| c.b("mergeIntent")).unwrap_or(false);
            flow::send::send_own(id, &format!("The check failed, so this is not done yet.{merged}\n\n{}\n\nFix it and finish again.", bound_detail(f.s("detail"))), "check", keep, patch, 0)?;
            return Ok(());
        }
    }
    // What a passing check wrote into files that are in git is part of what passed: committed.
    if !check.is_empty() && !tree.is_empty() && !research && !proven {
        let tp = Path::new(&tree);
        if git_out(tp, &["diff", "--name-only"]).map(|s| !s.is_empty()).unwrap_or(false) { let _ = git_out(tp, &["commit", "-am", &format!("Written by the check\n\nTask: {}", task_ref(id, &repo_keys()))]); }
        // The full check is where the shorter one of a later round starts from; the reviewer hears which one passed.
        let passed = if run.short() { None } else { Some(passed_patch(&flow::git::head_of(tp), &check)) };
        g.set(|t| { if let Some(p) = &passed { t.put("checkPassed", p.clone()); } t.put("checkShort", if run.short() { json!(ran) } else { Value::Null }); })?;
    } else { g.set(|t| t.put("checkShort", Value::Null))?; }
    t = g.check()?;
    // One of several attempts stops here: it passed, and waits to be chosen.
    if t.b("attemptOf") {
        let made = if !tree.is_empty() && !research { flow::git::change_of(&t) } else { None };
        let diff = made.as_ref().map(|m| crate::attempts::diff::keep_diff(&t, m.s("range")));
        let d = g.set(|t| {
            t.put("stage", "tried"); t.put("triedAt", now()); let fl = first_line(text); t.put("msg", one_line(if fl.is_empty() { "Finished" } else { &fl }, 2048)); t.put("failure", Value::Null);
            t.put("stat", made.as_ref().map(|m| m.t("stat")).unwrap_or_default());
            t.put("checkedHead", if !check.is_empty() { made.as_ref().map(|m| m.at("head").clone()).unwrap_or(Value::Null) } else { Value::Null });
            t.put("checkedWith", check.clone()); t.put("checked", !check.is_empty() && !research);
            t.put("diff", diff.clone().flatten().unwrap_or(Value::Null));
        })?;
        flow::sidebar::tell_sidebar(&d);
        return Ok(());
    }
    // Push and open a draft pull request, when an origin remote exists.
    let mut note = if research && !t.b("report") { " No report was found.".to_string() } else { String::new() };
    // A branch that brings main nothing is done without one: no change was needed, or the work is on main already (the
    // agent merged its own pull request, or pushed to main itself). Main was brought in just now, so that is where it is.
    let ask = ch.b("merge.pr") && flow::git::has_origin(&root);
    let why = if tree.is_empty() || research { None } else {
        let before = sync_r.map(|r| if r.status == "merged" { r.before.clone() } else { r.head.clone() }).filter(|h| !h.is_empty()).unwrap_or_else(|| flow::git::head_of(Path::new(&tree)));
        let base = [t.s("baseSha"), t.s("from")].into_iter().find(|s| !s.is_empty()).unwrap_or("HEAD").to_string();
        already::nothing_to_land(&t, Path::new(&tree), already::made_commits(Path::new(&tree), &base, &before, &prev_main), ask)
    };
    t = g.set(|t| t.put("doneWhy", why.clone().map(Value::from).unwrap_or(Value::Null)))?;
    if let Some(w) = &why { note = format!("{note} {w}."); }
    if !tree.is_empty() && ch.b("merge.pr") && !research && why.is_none() {
        let tp = Path::new(&tree);
        let base = [t.s("baseSha"), t.s("from")].into_iter().find(|s| !s.is_empty()).unwrap_or("HEAD").to_string();
        let pushed = (|| -> Res<Option<String>> {
            let has_commits = git_out(tp, &["rev-list", "--count", &format!("{base}..HEAD")])?.parse::<i64>().unwrap_or(0) > 0;
            if !(has_commits && flow::git::has_origin(&root)) { return Ok(None); }
            t = g.set(|t| { t.put("stage", "pushing"); t.put("msg", "Pushing the branch"); })?;
            // The branch is the board's own and an agent may have rebuilt it: overwrite what was pushed before, but only
            // while the remote still holds what was last seen here.
            git_slow(tp, &["push", "-u", "--force-with-lease", "origin", t.s("branch")], 120000)?;
            let ph = git_out(tp, &["rev-parse", &format!("refs/remotes/origin/{}", t.s("branch"))])?;
            g.set(|t| t.put("prHead", ph.clone()))?;
            let mut url = t.t("pr");
            if url.is_empty() {
                let body = format!("{}\n\nTask: {}", if text.is_empty() { t.t("title") } else { text.to_string() }, task_ref(id, &repo_keys()));
                let r = flow::git::gh(&["pr", "create", "--draft", "--head", t.s("branch"), "--title", if t.b("title") { t.s("title") } else { id }, "--body", &body], tp, 120000);
                url = r.stdout.split_whitespace().find(|w| w.starts_with("http://") || w.starts_with("https://")).unwrap_or("").to_string();
                // The agent opened one itself for this branch: the board goes on with that one.
                if url.is_empty() { url = already::open_pr(t.s("branch"), tp).unwrap_or_default(); }
                if url.is_empty() { return Err(crate::err::Error::new(format!("pr:{}", [r.stdout.trim(), r.stderr.trim()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n")))); }
                g.set(|t| { t.put("pr", url.clone()); t.put("prSeen", ""); t.put("prState", "waiting"); })?;
            }
            Ok(Some(url))
        })();
        match pushed {
            Ok(Some(url)) => note = format!(" Pull request: {url}"),
            Ok(None) => {}
            Err(e) if is_aborted(&e) => return Err(e),
            Err(_) if already::archived(Path::new(&tree)) => { fail(g, already::archived_failure(t.s("branch"), &tree), FailHow::default())?; return Ok(()); }
            Err(e) if e.msg.starts_with("pr:") => { fail(g, failure("pr", "Could not open the pull request", &format!("Branch: {}\nFolder: {tree}\n\ngh pr create:\n{}", t.s("branch"), &e.msg[3..]), "Inspect the GitHub error, fix what you safely can, then finish again. The board retries opening the pull request; do not push or ship manually."), FailHow::default())?; return Ok(()); }
            Err(e) => { fail(g, failure("push", "Could not push the branch", &format!("Branch: {}\nFolder: {tree}\n\n{}", t.s("branch"), e.msg), "Inspect the Git error and fix what you safely can without altering the human's checkout. Finish again so the board retries the push."), FailHow::default())?; return Ok(()); }
        }
    }
    after_review(Ctx { id, text, keep_intent, reason, sync: sync_r, research, tree: &tree, folder: &folder, root: &root, check: &check, note, kept, kept_how }, g)
}

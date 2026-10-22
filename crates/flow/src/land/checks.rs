//! GitHub's checks on a pull request, looked at right before it merges (`merge.checks`): still running, the merge
//! waits for them (at most `check.timeout`, then for you); failed, nothing merges and the failure goes back to the
//! agent with each failing check's name and the end of its log, as a failed check of the board's own does, within
//! `review.rounds`, then waits for you; passed, or none at all, it merges (none listed where checks are coming is
//! waited for, and so is one that is gone from the list, or a workflow's run not through: judge). `approve --anyway` merges without them.
//! A token that may not read GitHub's list of them (a fine-grained one) has them read from GitHub Actions and commit
//! statuses (readable.rs); what it cannot read is said on the task, never asked of you as a login.
use super::{expected, readable};
use super::merge::{fail, no_fix};
use crate::clock::now;
use crate::err::Res;
use crate::flow;
use crate::flow::git::{gh, pr_number, CHECK_BAD, CHECK_GOOD};
use crate::json::*;
use crate::runner::Guard;
use crate::store::failures::{bound_detail, is_auth_failure};
use crate::store::settings::lookup::{setting, setting_text};
use crate::store::settings::values::ms;
use crate::store::tasks::Task;
use std::path::Path;

/// How GitHub words one check's result (a check run's conclusion, else a status's state, else how far it got).
pub fn word(c: &Value) -> String { [c.s("conclusion"), c.s("state"), c.s("status")].into_iter().find(|s| !s.is_empty()).unwrap_or("").to_uppercase() }
pub fn name(c: &Value) -> String { if c.b("name") { c.t("name") } else { c.t("context") } }
pub enum Look { Passed, Running(Vec<String>), Failed(Vec<Value>) }
/// One failed check is enough: the others are not waited for.
pub fn sort(checks: &[Value]) -> Look {
    let failed: Vec<Value> = checks.iter().filter(|c| CHECK_BAD.contains(&word(c).as_str())).cloned().collect();
    if !failed.is_empty() { return Look::Failed(failed); }
    let running = unique(checks.iter().filter(|c| !CHECK_GOOD.contains(&word(c).as_str())).map(name));
    if running.is_empty() { Look::Passed } else { Look::Running(running) }
}
/// Names as said and remembered: each once, whatever its case (a workflow "Check" and its job "check", its runs for a
/// push and for the pull request).
pub fn unique(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for n in names { if !out.iter().any(|o| o.eq_ignore_ascii_case(&n)) { out.push(n); } }
    out
}
/// What the row says while GitHub's checks run: how far they are (`· 2 of 3 passed`), never their names, which say
/// little and repeat (a workflow `Check` and its job `check`, its runs for the push and for the pull request). Each
/// check counts once, whatever its case, by its own result: a workflow of the same name never makes it pending. A
/// workflow (its run read from Actions, or one waited for by name: `flows`) stands for its jobs until they are listed,
/// and for those still to come once all listed have passed. `waited`: what is still waited for (running, gone from
/// the list, a workflow not through).
pub fn waiting_words(listed: &[Value], waited: &[String], flows: &[String]) -> String {
    let is = |a: &str, b: &str| a.eq_ignore_ascii_case(b);
    let (runs, checks): (Vec<&Value>, Vec<&Value>) = listed.iter().partition(|c| c.b("run"));
    let workflows = unique(runs.iter().map(|c| name(c)).chain(flows.iter().cloned()));
    let a_workflow = |n: &str| workflows.iter().any(|f| is(f, n));
    let waits = |n: &str| waited.iter().chain(flows).any(|w| is(w, n));
    let running = |n: &str| checks.iter().any(|c| is(&name(c), n) && !CHECK_GOOD.contains(&word(c).as_str()));
    let stands = |wf: &str| { let jobs: Vec<String> = checks.iter().filter(|c| is(c.s("workflowName"), wf)).map(|c| name(c)).collect(); jobs.is_empty() || (waits(wf) && !jobs.iter().any(|j| running(j))) };
    // Each: still waited for, or passed. A listed check by its own result (or waited for under a name no workflow has).
    let mut all: Vec<bool> = unique(checks.iter().map(|c| name(c))).iter().map(|n| running(n) || (waits(n) && !a_workflow(n))).collect();
    all.extend(workflows.iter().filter(|f| stands(f)).map(|f| waits(f)));
    let listed_name = |n: &str| checks.iter().any(|c| is(&name(c), n));
    all.extend(unique(waited.iter().filter(|w| !listed_name(w) && !a_workflow(w)).cloned()).iter().map(|_| true));
    let left = all.iter().filter(|p| **p).count();
    if left == 0 { return "Waiting for GitHub checks to start".into(); }
    format!("Waiting for GitHub checks · {} of {} passed", all.len() - left, all.len())
}
/// Every check GitHub listed on this commit while the board waited for it (`checksWait.seen`), and every one it waited
/// for (`checksWait.names`), read the same way (`via`, readable.rs): what was listed for an earlier commit, or read
/// another way (GitHub's list names jobs only, the one read from Actions names their runs too), is not looked for.
fn known(t: &Obj, head: &str, via: &str) -> Vec<String> { let w = t.at("checksWait"); if w.s("head") != head || w.s("via") != via { return vec![]; } unique(strings(w.at("seen")).into_iter().chain(strings(w.at("names")))) }
/// Those that no check listed now is, by its name or its workflow's, in any case: not passed, so still waited for.
pub fn missing(t: &Obj, head: &str, via: &str, listed: &[Value]) -> Vec<String> {
    let here = |n: &str| listed.iter().any(|c| name(c).eq_ignore_ascii_case(n) || c.s("workflowName").eq_ignore_ascii_case(n));
    known(t, head, via).into_iter().filter(|n| !here(n)).collect()
}
/// What judge decided: the look, the workflows still waited for by name (they are not checks: GitHub lists their
/// jobs), and the checks the branch requires that this token cannot read (`blind`).
pub struct Judged { pub look: Look, pub flows: Vec<String>, pub unread: Vec<String> }
/// What is listed, against what is to be listed (expected.rs). Still running: a check listed on this commit before and
/// gone now, a check the branch it merges into requires and not listed, a workflow that runs for this pull request
/// with its run on the commit not there yet or not through (all of its jobs, every member of a matrix). Failed: a
/// check, or such a run. None listed and nothing of that: "none to wait for", unless an outside CI reports in this
/// repo, or the pull request had checks before (`prChecksSeen`). Read without the Checks permission (readable.rs),
/// another app's check is never listed: a required check not listed is waited for while anything readable is still out, then it is one that could not be read, not one to wait for.
pub fn judge(t: &Obj, head: &str, read: &readable::Read, root: &Path, base: &str) -> Judged {
    let (listed, blind) = (read.listed(), !read.unread.is_empty());
    let look = sort(listed);
    if let Look::Failed(_) = look { return Judged { look, flows: vec![], unread: vec![] }; }
    let mut flows = expected::flows_state(t, root, base, head);
    if !flows.failed.is_empty() { return Judged { look: Look::Failed(flows.failed), flows: vec![], unread: vec![] }; }
    let names: Vec<String> = listed.iter().map(name).collect();
    let mut wait = missing(t, head, read.via(), listed);
    let required: Vec<String> = expected::required(root, base).into_iter().filter(|n| !names.contains(n) && !wait.contains(n)).collect();
    let mut unread = vec![];
    // Read blind, a required check waits beside the workflows (by name, not as one seen listed), so that what is
    // readable passing is enough to look again.
    if !blind { wait.extend(required); }
    else if matches!(look, Look::Passed) && wait.is_empty() && flows.waiting.is_empty() { unread = required; }
    else { for n in required { if !flows.waiting.contains(&n) { flows.waiting.push(n); } } }
    let look = match look {
        Look::Running(r) => Look::Running(unique(r.into_iter().chain(wait))),
        Look::Passed if !wait.is_empty() => Look::Running(wait),
        Look::Passed if !flows.waiting.is_empty() || (listed.is_empty() && (t.b("prChecksSeen") || expected::outside_ci(root, base))) => Look::Running(vec![]),
        x => x,
    };
    Judged { look, flows: flows.waiting, unread }
}
/// What was not looked at, said on the task that merges: what the token cannot read, and the required checks not found
/// in what it read.
pub fn unread_note(unread: &[String], required: &[String]) -> String {
    let mut parts: Vec<String> = unread.to_vec();
    if !required.is_empty() { parts.push(format!("the required {} {} (listed by no GitHub Actions job and no commit status)", if required.len() == 1 { "check" } else { "checks" }, required.join(", "))); }
    if parts.is_empty() { String::new() } else { format!("Not read: {}", parts.join("; ")) }
}
/// Are GitHub's checks looked at for this task?
pub fn looked_at(t: &Obj) -> bool { setting_text("merge.checks", Some(t)) != "none" }
/// The short reason, the same from the land step and from the pull requests' watch (housekeeping/prs.rs).
pub fn failed_why(pr: &str, names: &[String]) -> String { format!("Checks failed on the pull request {pr}: {}. Look at them (gh pr checks) and fix them on your branch.", names.join(", ")) }
const LOG_LINES: usize = 40;
/// The end of a failing check's log, where GitHub Actions ran it (`gh run view --job <id> --log-failed`).
fn log_of(root: &Path, c: &Value) -> String {
    let url = [c.s("detailsUrl"), c.s("targetUrl")].into_iter().find(|s| !s.is_empty()).unwrap_or("").to_string();
    let job: String = url.split("/job/").nth(1).map(|r| r.chars().take_while(|c| c.is_ascii_digit()).collect()).unwrap_or_default();
    if job.is_empty() || !url.contains("/actions/runs/") { return if url.is_empty() { "GitHub gave no link to its log.".into() } else { format!("Its log is not one gh can read: {url}") }; }
    let r = gh(&["run", "view", "--job", &job, "--log-failed"], root, 60000);
    // A line reads `<job>\t<step>\t<time> <text>`: only the text is kept.
    let lines: Vec<String> = r.stdout.lines().map(|l| {
        let l = l.splitn(3, '\t').last().unwrap_or(l);
        let l = crate::re!(r"\x1b\[[0-9;]*[A-Za-z]").replace_all(l, "");
        crate::re!(r"^\d{4}-\d\d-\d\dT[0-9:.]+Z ?").replace(&l, "").trim_end().to_string()
    }).filter(|l| !l.trim().is_empty()).collect();
    if r.status != 0 || lines.is_empty() { return format!("Its log could not be read here ({}): {url}", r.stderr.trim().lines().last().unwrap_or("no output")); }
    format!("The end of its log ({url}):\n{}", lines[lines.len().saturating_sub(LOG_LINES)..].join("\n"))
}
/// What failed, each check with the end of its log: what the agent is sent, and what the task shows.
pub fn failed_detail(root: &Path, pr: &str, failed: &[Value]) -> String {
    let names: Vec<String> = failed.iter().map(name).collect();
    let mut s = format!("{}\n\nPull request: {pr}", failed_why(pr, &names));
    // A run read as itself (readable.rs) that failed with a failed job listed: the job's log says it.
    let told = |c: &Value| c.b("run") && failed.iter().any(|o| !o.b("run") && o.s("workflowName") == c.s("name"));
    for c in failed.iter().filter(|c| !told(c)) { s.push_str(&format!("\n\n{} ({}):\n{}", name(c), word(c).to_lowercase(), log_of(root, c))); }
    s
}
/// The pull request blocker GitHub's failed checks make: the same from the land step and the watch.
pub fn blocker(pr: &str, head: &str, names: &[String], detail: &str) -> Value {
    json!({ "stage": "pr", "remote": true, "remoteKind": "checks", "head": head, "checks": names, "why": failed_why(pr, names), "title": "Pull request checks failed", "detail": detail,
        "next": "Address this on your own branch and finish again. Local checks do not prove remote CI passed; wait for a fresh remote result.", "agentCanFix": true, "at": now(), "sentAt": null })
}
/// Failed checks recorded on another commit than `head`: they say nothing of it (#309).
pub fn stale(t: &Obj, head: &str) -> bool { let f = t.at("prFailure"); f.s("remoteKind") == "checks" && f.b("head") && !head.is_empty() && f.s("head") != head }
/// A pull request blocker that stands for `head`: one of an older commit's checks does not.
pub fn stands(t: &Obj, head: &str) -> bool { t.b("prFailure") && !stale(t, head) }
/// The checks that failed on the commit that would merge, by name: what `approve` refuses without `--anyway`.
pub fn failed_on(t: &Obj) -> Option<String> {
    let f = t.at("prFailure");
    if f.s("remoteKind") != "checks" || stale(t, t.s("prHead")) { return None; }
    let names = strings(f.at("checks"));
    Some(if names.is_empty() { "its pull request's checks failed on GitHub".into() } else { format!("GitHub's checks failed on its pull request: {}", names.join(", ")) })
}
/// The board has waited for GitHub's checks longer than `check.timeout`.
pub fn overdue(t: &Obj) -> bool { let l = ms(&setting_text("check.timeout", Some(t))); l > 0 && now() - t.at("checksWait").i("since") >= l }

/// Asked by the land step right before a pull request merges. True: it may merge now. False: it waits for the
/// checks, or they failed, and that was dealt with here.
pub fn before_merge(id: &str, g: &mut Guard, t: &Task, root: &Path) -> Res<bool> {
    if !looked_at(t) || t.at("mergeIntent").b("anyway") { return Ok(true); }
    let pr = t.t("pr");
    let read = match readable::pr_view(root, &pr, "headRefOid,baseRefName") {
        Ok(r) => r,
        Err(said) => {
            fail(g, "Could not read GitHub's checks", &format!("Pull request: {pr}\n\ngh pr view:\n{said}\n\nNothing was merged."), "Make sure gh is logged in and reaches GitHub (gh auth status), then approve again.", super::merge::Fail { agent_can_fix: false, auth: Some(is_auth_failure(&said)), keep_intent: false })?;
            return Ok(false);
        }
    };
    if !read.fault.is_empty() {
        // GitHub's list of checks is not for this token, and neither is what it is made of here.
        fail(g, "Could not read GitHub's checks", &format!("Pull request: {pr}\n\nThis GitHub token may not read the pull request's checks list, so they are read from GitHub Actions and commit statuses, and GitHub refused those too:\n{}\n\nNothing was merged.", read.fault),
            "Give the GitHub token read access to Actions and to Commit statuses (a fine-grained token cannot have the Checks permission, and needs none: these two are enough), then approve again.", no_fix())?;
        return Ok(false);
    }
    let v = &read.view;
    let head = v.t("headRefOid");
    let listed = read.listed();
    if !listed.is_empty() && !t.b("prChecksSeen") { g.set(|t| t.put("prChecksSeen", true))?; }
    let base = if v.b("baseRefName") { v.t("baseRefName") } else { crate::sync::main_branch(root, t) };
    let Judged { look, flows, unread } = judge(t, &head, &read, root, &base);
    match look {
        Look::Passed => {
            // What could not be read is said on the task (merge.rs adds it to "Merged"), never asked of you as a login.
            let note = unread_note(&read.unread, &unread);
            // They passed on the commit that merges: a record of failed checks, of an older one, is over.
            let over = t.at("prFailure").s("remoteKind") == "checks";
            if t.b("checksWait") || t.s("checksUnread") != note || over { g.set(|t| { t.put("checksWait", Value::Null); t.put("checksUnread", if note.is_empty() { Value::Null } else { json!(note) }); if over { t.put("prFailure", Value::Null); } })?; }
            Ok(true)
        }
        Look::Running(checks) => {
            let names = unique(checks.iter().chain(&flows).cloned());
            // One wait per approval and commit: it counts from when the board first found them running.
            let (w, approved) = (t.at("checksWait"), t.at("mergeIntent").i("approvedAt"));
            let since = if w.s("head") == head && w.i("for") == approved { w.i("since") } else { now() };
            let stuck = { let mut x = t.clone(); x.put("checksWait", json!({ "since": since })); overdue(&x) };
            if stuck {
                g.set(|t| { t.put("checksWait", Value::Null); t.put("autoMerge", false); t.put("autoMergeLocal", false); })?;
                let timeout = setting_text("check.timeout", Some(t));
                let which = if names.is_empty() { "none has started (an outside CI reports in this repo: on the branch it merges into, or on its other pull requests)".to_string() } else { names.join(", ") };
                fail(g, "GitHub's checks did not finish", &format!("Pull request: {pr}\nStill not finished after {timeout} (check.timeout): {which}\n\nNothing was merged."),
                    &format!("Look at the pull request's checks on GitHub (gh pr checks {}): one may be stuck, or wait for a runner. Approve again to wait for them again, or merge without them: herdr-projects approve {id} --anyway.", pr_number(&pr)), no_fix())?;
                return Ok(false);
            }
            let msg = waiting_words(listed, &checks, &flows);
            let d = g.set(|t| { t.put("failure", Value::Null); t.put("autoMerge", true); t.put("autoMergeLocal", true); let seen = unique(known(t, &head, read.via()).into_iter().chain(listed.iter().map(name))); t.put("checksWait", json!({ "since": since, "head": head, "via": read.via(), "for": approved, "names": checks, "flows": flows, "seen": seen })); t.put("msg", msg.clone()); })?;
            flow::sidebar::tell_sidebar(&d);
            Ok(false)
        }
        Look::Failed(list) => {
            let names: Vec<String> = list.iter().map(name).collect();
            let detail = failed_detail(root, &pr, &list);
            let pf = blocker(&pr, &head, &names, &detail);
            let seen = format!("{head}|{}|{}", names.join(","), v.s("reviewDecision"));
            let went = t.i("prRounds");
            let off = json!({ "checksWait": null, "autoMerge": false, "autoMergeLocal": false, "prState": "failing", "prSeen": seen });
            if (went as f64) < number(&setting("review.rounds", Some(t))) {
                // Straight back to the agent, as a failed check of the board's own: the turn it causes is the board's,
                // so the merge that was approved survives it, and goes on once GitHub's checks pass on the repair.
                let mut sent = pf.clone(); sent["sentAt"] = json!(now());
                let mut patch = obj(off); patch.put("prFailure", sent); patch.put("prRounds", went + 1);
                flow::send::send_own(id, &format!("GitHub's checks failed on the pull request, so this is not merged yet.\n\n{}\n\nFix it on your branch, run the checks and finish again.", bound_detail(&detail)), "checks", true, patch, 0)?;
                return Ok(false);
            }
            // Its rounds are spent: it waits for you, and `approve` says why until `--anyway`.
            let msg = format!("GitHub's checks failed: {}. Send the details to the agent, or merge it anyway", names.join(", "));
            let d = g.set(|t| { for (k, v) in obj(off.clone()) { t.insert(k, v); } t.put("prFailure", pf.clone()); t.put("stage", "approval"); t.put("mergeFailed", true); t.put("mergeIntent", Value::Null); t.put("msg", msg.clone()); })?;
            flow::sidebar::tell_sidebar(&d);
            Ok(false)
        }
    }
}

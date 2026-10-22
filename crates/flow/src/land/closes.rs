//! The GitHub issues a task's work closes (`merge.closes`): a `Closes #N` line for each in its pull request's body (added
//! to one the agent opened itself, and taken off again for an issue the setting no longer names), and after a merge that
//! landed, `gh issue close` for each GitHub does not close by itself. A close that fails never fails the merge: the
//! task's line says what was closed, or why not. The task keeps the issues whose lines the board wrote in `closesOnPr`.
use crate::flow::git::{gh, pr_number};
use crate::json::*;
use crate::store::keys::{repo_keys, task_ref};
use crate::store::settings::issues::numbers;
use crate::store::settings::lookup::setting;
use crate::store::tasks::Task;
use std::path::Path;

/// The issues the task's work closes, as its setting says now.
pub fn issues(t: &Task) -> Vec<u64> { numbers(&setting("merge.closes", Some(t))) }
/// The issues a body already closes by one of GitHub's words (`Closes #12`, `fixes: #12`, `Resolved #12`).
fn closed_by(body: &str) -> Vec<u64> {
    crate::re!(r"(?i)\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)\s*:?\s+#(\d+)\b").captures_iter(body).filter_map(|c| c[1].parse().ok()).collect()
}
/// The body with a `Closes #N` line for each issue it does not close yet, and the issues it added a line for.
pub fn with_lines(body: &str, issues: &[u64]) -> (String, Vec<u64>) {
    let has = closed_by(body);
    let added: Vec<u64> = issues.iter().copied().filter(|n| !has.contains(n)).collect();
    let missing: Vec<String> = added.iter().map(|n| format!("Closes #{n}")).collect();
    let body = if missing.is_empty() { body.to_string() } else if body.trim().is_empty() { missing.join("\n") } else { format!("{}\n\n{}", body.trim_end(), missing.join("\n")) };
    (body, added)
}
/// The body without the board's own `Closes #N` line of each of `gone`.
fn without(body: &str, gone: &[u64]) -> String {
    if gone.is_empty() { return body.to_string(); }
    let lines: Vec<&str> = body.lines().filter(|l| !gone.iter().any(|n| l.trim() == format!("Closes #{n}"))).collect();
    lines.join("\n").trim_end().to_string()
}
/// The issues whose lines the board wrote on the pull request that the setting no longer names.
pub fn stale(t: &Task) -> Vec<u64> { let want = issues(t); numbers(t.at("closesOnPr")).into_iter().filter(|n| !want.contains(n)).collect() }
/// The pull request's body, None when gh cannot say.
fn body_of(pr: &str, cwd: &Path) -> Option<String> {
    let r = gh(&["pr", "view", &pr_number(pr), "--json", "body"], cwd, 60000);
    if r.status != 0 { return None; }
    serde_json::from_str::<Value>(&r.stdout).ok().filter(|v| v.is_object()).map(|v| v.t("body"))
}
fn said(r: &crate::flow::git::GhOut) -> String {
    [r.stderr.as_str(), r.stdout.as_str()].iter().flat_map(|s| s.lines()).map(str::trim).find(|l| !l.is_empty()).unwrap_or("gh said nothing").to_string()
}
/// The pull request's body as the setting says now: the lines it lacks added (the agent's own pull request, or one opened
/// before the setting was said), and the board's own lines of the issues it no longer names taken off. Ok: the issues
/// whose lines the board wrote there now, for `closesOnPr`. Err: why the body could not be read or changed.
pub fn onto_pr(t: &Task, pr: &str, cwd: &Path) -> Result<Vec<u64>, String> {
    let (want, gone) = (issues(t), stale(t));
    let mine: Vec<u64> = numbers(t.at("closesOnPr")).into_iter().filter(|n| want.contains(n)).collect();
    if (want.is_empty() && gone.is_empty()) || pr.is_empty() { return Ok(mine); }
    let Some(body) = body_of(pr, cwd) else { return Err("its body could not be read".into()) };
    let (new, added) = with_lines(&without(&body, &gone), &want);
    let mut written = mine;
    for n in added { if !written.contains(&n) { written.push(n); } }
    if new == body { return Ok(written); }
    let r = gh(&["pr", "edit", &pr_number(pr), "--body", &new], cwd, 60000);
    if r.status == 0 { Ok(written) } else { Err(said(&r)) }
}
fn list(ns: &[u64]) -> String { ns.iter().map(|n| format!("#{n}")).collect::<Vec<_>>().join(", ") }
/// After a merge that landed (`commit`: the commit it made), the words for the task's line. Through a pull request into
/// the default branch, GitHub closes those its body names; the rest (all, after a merge with no pull request or one into
/// another branch) are closed here, each with a comment naming the task and the commit.
pub fn after_landing(t: &Task, root: &Path, commit: &str) -> String {
    let want = issues(t);
    if want.is_empty() { return String::new(); }
    let by_github: Vec<u64> = if t.b("pr") && crate::sync::named_base(root, t).is_none() { body_of(t.s("pr"), root).map(|b| closed_by(&b)).unwrap_or_default() } else { vec![] };
    let (github, rest): (Vec<u64>, Vec<u64>) = want.iter().partition(|n| by_github.contains(n));
    let title = if t.b("title") { format!(" \"{}\"", t.s("title")) } else { String::new() };
    let landed = if commit.is_empty() { String::new() } else { format!(" in {}", &commit[..commit.len().min(12)]) };
    let comment = format!("Closed by task {}{title}: its work landed{landed}.", task_ref(t.s("id"), &repo_keys()));
    let (mut closed, mut failed): (Vec<u64>, Vec<(String, Vec<u64>)>) = (vec![], vec![]);
    for n in rest {
        let r = gh(&["issue", "close", &n.to_string(), "--comment", &comment], root, 60000);
        // An issue closed already is closed: gh may say so with an error.
        if r.status == 0 || said(&r).contains("already closed") { closed.push(n); continue; }
        let why = said(&r);
        match failed.iter_mut().find(|(w, _)| *w == why) { Some((_, ns)) => ns.push(n), None => failed.push((why, vec![n])) }
    }
    let mut words = vec![];
    if !github.is_empty() { words.push(format!("GitHub closes {}.", list(&github))); }
    if !closed.is_empty() { words.push(format!("Closed {}.", list(&closed))); }
    for (why, ns) in failed { words.push(format!("Could not close {}: {}.", list(&ns), why.trim_end_matches('.'))); }
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lines() {
        assert_eq!(with_lines("Did it.\n", &[3, 4]), ("Did it.\n\nCloses #3\nCloses #4".into(), vec![3, 4]));
        assert_eq!(with_lines("Did it. Fixes: #3", &[3]), ("Did it. Fixes: #3".into(), vec![]));
        assert_eq!(with_lines("Closes #34", &[3]).0, "Closes #34\n\nCloses #3");
        assert_eq!(without("Did it.\n\nCloses #3\nCloses #34\nFixes #3", &[3]), "Did it.\n\nCloses #34\nFixes #3");
        assert_eq!(closed_by("resolved #5, closes #6"), vec![5, 6]);
    }
}

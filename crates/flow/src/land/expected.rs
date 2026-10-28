//! The checks a pull request is to get before GitHub lists them, so that one not registered yet is waited for and
//! never taken for "none". GitHub Actions: each workflow of the branch that runs for this pull request (triggers.rs
//! decides it as GitHub does: its events, branches, paths and types) must have its run on the commit, finished and
//! passed; a run waits for all of its jobs, every member of a matrix too. One that may not run on this commit (its
//! types) is waited for a short while only, and none at all where the repo has GitHub Actions turned off. Each check the branch it merges into
//! requires (its protection, its rulesets) must be listed. An outside CI, with nothing listed: checks of its own on
//! the branch it merges into or on the repo's other pull requests. Used by checks.rs.
use super::triggers::{parse, runs_for, Flow, Pr, Runs};
use crate::clock::now;
use crate::flow::git::{gh, CHECK_BAD, CHECK_GOOD};
use crate::json::*;
use crate::store::repos::git_out;
use std::path::{Path, PathBuf};

/// What the workflows of the branch say of this commit.
#[derive(Default, Debug)]
pub struct Flows { pub waiting: Vec<String>, pub failed: Vec<Value> }
fn tree_of(t: &Obj) -> PathBuf { if t.b("worktree") { PathBuf::from(t.s("worktree")) } else { crate::store::repos::folder_of(t.s("id")) } }
/// The workflows in the task's worktree that run for this pull request, each with its file (`.github/workflows/…`) and
/// whether it runs on every head of it or only maybe.
pub fn flows_for(t: &Obj, base: &str) -> Vec<(String, Flow, Runs)> {
    let tree = tree_of(t);
    let Ok(dir) = std::fs::read_dir(tree.join(".github").join("workflows")) else { return vec![] };
    let files = git_out(&tree, &["diff", "--name-only", &format!("origin/{base}...HEAD")]).ok().map(|o| o.lines().map(String::from).collect());
    let pr = Pr { base, branch: t.s("branch"), files };
    let mut out: Vec<(String, Flow, Runs)> = dir.flatten().filter(|e| e.path().extension().map(|x| x == "yml" || x == "yaml").unwrap_or(false))
        .filter_map(|e| { let f = e.file_name().to_string_lossy().to_string(); std::fs::read_to_string(e.path()).ok().map(|y| (format!(".github/workflows/{f}"), parse(&f, &y))) })
        .map(|(f, w)| { let r = runs_for(&w, &pr); (f, w, r) }).filter(|(_, _, r)| *r != Runs::Never).collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}
/// How long a workflow that may not run on this commit is waited for to start: GitHub starts a run within seconds of
/// the event that brought the commit, or never (#420).
pub const MAYBE_MS: i64 = 90_000;
/// The board has waited for GitHub's checks on `head` for `ms` or longer.
pub fn waited_past(t: &Obj, head: &str, ms: i64) -> bool { let w = t.at("checksWait"); w.s("head") == head && w.i("since") > 0 && now() - w.i("since") >= ms }
/// GitHub Actions is turned off for the repo: GitHub says so (`enabled: false`). A token that may not ask, or any other
/// answer, is "not off", so that nothing merges unchecked for it (#416).
pub fn actions_off(root: &Path) -> bool {
    let r = gh(&["api", "repos/{owner}/{repo}/actions/permissions", "--jq", ".enabled"], root, 30000);
    r.status == 0 && r.stdout.trim() == "false"
}
/// Each workflow that runs for it, against its runs on this commit: none yet, or one not finished, is waited for (by the
/// workflow's name); one that finished and failed is a failed check. GitHub not answering: all of them are waited for.
/// One that may not run on this commit and has no run is waited for only `MAYBE_MS` since the board began waiting
/// (where GitHub gave the commit's runs: not given, it is waited for as the others are).
pub fn flows_state(t: &Obj, root: &Path, base: &str, head: &str) -> Flows {
    let flows = flows_for(t, base);
    if flows.is_empty() || actions_off(root) { return Flows::default(); }
    let runs = super::readable::runs(root, head);
    let given_up = runs.is_ok() && waited_past(t, head, MAYBE_MS);
    let runs = runs.unwrap_or_default();
    let mut out = Flows::default();
    for (file, flow, how) in flows {
        let mine: Vec<&super::readable::Run> = runs.iter().filter(|x| x.path.split('@').next() == Some(file.as_str())).collect();
        if mine.is_empty() && how == Runs::Maybe && given_up { continue; }
        let up = |s: &str| s.to_uppercase();
        if let Some(bad) = mine.iter().find(|x| up(&x.status) == "COMPLETED" && CHECK_BAD.contains(&up(&x.conclusion).as_str())) {
            out.failed.push(json!({ "name": flow.name, "conclusion": up(&bad.conclusion), "detailsUrl": bad.url }));
        } else if mine.is_empty() || mine.iter().any(|x| up(&x.status) != "COMPLETED" || !CHECK_GOOD.contains(&up(&x.conclusion).as_str())) {
            out.waiting.push(flow.name.clone());
        }
    }
    out
}
/// The checks the branch it merges into requires before anything merges into it: its protection's and its rulesets'.
/// Asked of GitHub; where it cannot say, nothing is added here.
pub fn required(root: &Path, base: &str) -> Vec<String> {
    if base.is_empty() { return vec![]; }
    let mut out: Vec<String> = vec![];
    for (path, jq) in [(format!("repos/{{owner}}/{{repo}}/branches/{base}"), ".protection.required_status_checks.contexts[]?"),
        (format!("repos/{{owner}}/{{repo}}/rules/branches/{base}"), ".[] | select(.type == \"required_status_checks\") | .parameters.required_status_checks[].context")] {
        let r = gh(&["api", &path, "--jq", jq], root, 30000);
        if r.status != 0 { continue; }
        for l in r.stdout.lines().map(str::trim).filter(|l| !l.is_empty()) { if !out.iter().any(|o| o == l) { out.push(l.to_string()); } }
    }
    out
}
/// With nothing listed: an outside CI (one not of GitHub Actions, which flows_state decides exactly) that reports on
/// the newest commit of the branch it merges into, or on one of the repo's last pull requests. A question GitHub cannot
/// answer counts as yes, so nothing merges unchecked for it; one the token may not ask (check runs, for a fine-grained
/// token: readable.rs) counts as no, as it would never be answered.
pub fn outside_ci(root: &Path, base: &str) -> bool {
    if base.is_empty() { return true; }
    let asks = [
        vec!["api".to_string(), format!("repos/{{owner}}/{{repo}}/commits/{base}/check-runs"), "--jq".into(), "[.check_runs[] | select(.app.slug != \"github-actions\")] | length".into()],
        vec!["api".to_string(), format!("repos/{{owner}}/{{repo}}/commits/{base}/status"), "--jq".into(), ".total_count".into()],
        ["pr", "list", "--state", "all", "--limit", "10", "--json", "statusCheckRollup", "--jq", "[.[].statusCheckRollup[] | select((.workflowName // \"\") == \"\")] | length"].iter().map(|s| s.to_string()).collect(),
    ];
    asks.iter().any(|a| {
        let r = gh(&a.iter().map(String::as_str).collect::<Vec<_>>(), root, 30000);
        if super::readable::refused(&r) { return false; }
        r.status != 0 || r.stdout.trim().parse::<i64>().map(|n| n > 0).unwrap_or(true)
    })
}

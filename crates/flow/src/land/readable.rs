//! A pull request's checks as the GitHub token can read them. GitHub's own list of them (`statusCheckRollup`, and the
//! check-runs API) needs the Checks permission, which a fine-grained token cannot have (GitHub gives it to GitHub Apps
//! only): where GitHub refuses it, the list is made of what such a token reads, the jobs of the commit's GitHub Actions
//! runs (every matrix member, with the link to its log) and its commit statuses (an outside CI, Vercel), in the same
//! shape, and what cannot be read that way (another app's check runs) is named, to be said on the task. A check run
//! again keeps its older runs in GitHub's list: only its newest one counts (#404). Used by
//! checks.rs, merge.rs and the pull requests' watch (housekeeping/prs.rs).
use crate::clock::now;
use crate::flow::git::{gh, pr_number, GhOut};
use crate::json::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// What a token without the Checks permission cannot read, said on the task that merged without it.
pub const UNREAD: &str = "check runs of apps other than GitHub Actions (this GitHub token cannot read them; their commit statuses were read)";
/// A pull request as gh gave it, its checks as `statusCheckRollup`. `unread`: what was not looked at (the token cannot
/// read it). `fault`: why its checks could not be read at all (they are then none here).
#[derive(Default)]
pub struct Read { pub view: Value, pub unread: Vec<String>, pub fault: String }
impl Read {
    pub fn listed(&self) -> &[Value] { self.view.arr("statusCheckRollup") }
    /// How they were read: GitHub's own list (`checks`, jobs only), or made of Actions runs, their jobs and statuses
    /// (`actions`). Names remembered one way are not looked for the other (checks.rs).
    pub fn via(&self) -> &'static str { if self.unread.is_empty() { "checks" } else { "actions" } }
}
/// GitHub said no for want of a permission: not a login gone, not the network.
pub fn refused(r: &GhOut) -> bool {
    let e = format!("{}\n{}", r.stderr, r.stdout).to_lowercase();
    r.status != 0 && (e.contains("not accessible by") || (e.contains("http 403") && !e.contains("rate limit")))
}
fn said(r: &GhOut) -> String { [r.stdout.trim(), r.stderr.trim()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n") }
/// Repos whose token GitHub refused the checks list, and when: asked again only after a while (a new token may read it).
static REFUSED: Mutex<Vec<(PathBuf, i64)>> = Mutex::new(Vec::new());
const ASK_AGAIN_MS: i64 = 15 * 60000;
fn refused_lately(root: &Path) -> bool { REFUSED.lock().map(|l| l.iter().any(|(p, at)| p == root && now() - at < ASK_AGAIN_MS)).unwrap_or(false) }
fn remember(root: &Path) { if let Ok(mut l) = REFUSED.lock() { l.retain(|(p, _)| p != root); l.push((root.to_path_buf(), now())); } }

/// `gh pr view <pr> --json <fields>`, with its checks: GitHub's own list where the token reads it, else the list made
/// here. Err: what gh said where it could not even give the pull request.
pub fn pr_view(root: &Path, pr: &str, fields: &str) -> Result<Read, String> {
    let n = pr_number(pr);
    if !refused_lately(root) {
        let r = gh(&["pr", "view", &n, "--json", &format!("{fields},statusCheckRollup")], root, 60000);
        if let Ok(mut v) = serde_json::from_str::<Value>(&r.stdout) { v["statusCheckRollup"] = json!(newest(root, v.arr("statusCheckRollup"))); return Ok(Read { view: v, ..Default::default() }); }
        if !refused(&r) { return Err(said(&r)); }
        remember(root);
    }
    let fields = if fields.split(',').any(|f| f == "headRefOid") { fields.to_string() } else { format!("{fields},headRefOid") };
    let r = gh(&["pr", "view", &n, "--json", &fields], root, 60000);
    let Ok(mut v) = serde_json::from_str::<Value>(&r.stdout) else { return Err(said(&r)) };
    // Merged or closed: its checks are not asked for.
    if !v.s("state").is_empty() && v.s("state") != "OPEN" { v["statusCheckRollup"] = json!([]); return Ok(Read { view: v, ..Default::default() }); }
    let (checks, fault) = match of_commit(root, &v.t("headRefOid")) { Ok(c) => (c, String::new()), Err(e) => (vec![], e) };
    v["statusCheckRollup"] = json!(checks);
    Ok(Read { view: v, unread: vec![UNREAD.to_string()], fault })
}

/// A number in a GitHub Actions link after `part` (`/actions/runs/<run>/job/<job>`): None for any other check.
fn link_id(c: &Value, part: &str) -> Option<u64> { c.s("detailsUrl").split(part).nth(1).and_then(|r| r.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()) }
/// The jobs of a workflow run's latest attempt, as GitHub gives them: None where it does not.
fn latest_jobs(root: &Path, run: u64) -> Option<Vec<u64>> {
    let r = every_page(root, &format!("repos/{{owner}}/{{repo}}/actions/runs/{run}/jobs?filter=latest&per_page=100"), ".jobs[].id");
    (r.status == 0).then(|| r.stdout.lines().filter_map(|l| l.trim().parse().ok()).collect())
}
/// GitHub's list with each check only as last run. A GitHub Actions job listed twice in its workflow run: the one of the
/// run's latest attempt, as GitHub says it (two jobs of one name in the same attempt are two checks). Another app's
/// check: one of the same name that began after it was over. What cannot be told (GitHub does not say, the times are not
/// known: not started yet, gh's `0001-…`) is kept, so that a failure is never dropped for a guess.
pub fn newest(root: &Path, checks: &[Value]) -> Vec<Value> {
    let run_of = |c: &Value| link_id(c, "/actions/runs/");
    let key = |c: &Value| (c.s("__typename").to_string(), c.s("workflowName").to_string(), if c.b("name") { c.t("name") } else { c.t("context") }, run_of(c));
    let mut latest: Vec<(u64, Option<Vec<u64>>)> = vec![];
    for c in checks.iter().filter(|c| checks.iter().filter(|o| key(o) == key(c)).count() > 1) {
        if let Some(r) = run_of(c) { if !latest.iter().any(|(x, _)| *x == r) { latest.push((r, latest_jobs(root, r))); } }
    }
    let known = |s: &str| !s.is_empty() && !s.starts_with("0001");
    // A status has one time, when it was set.
    let older = |a: &Value, b: &Value| { let (end, start) = (if a.b("completedAt") { a.s("completedAt") } else { a.s("startedAt") }, b.s("startedAt")); known(end) && known(start) && end < start };
    let gone = |c: &Value| match run_of(c) {
        Some(r) => {
            let Some((_, Some(ids))) = latest.iter().find(|(x, _)| *x == r) else { return false };
            let latest_job = |o: &Value| link_id(o, "/job/").map(|j| ids.contains(&j)).unwrap_or(false);
            link_id(c, "/job/").is_some() && !latest_job(c) && checks.iter().any(|o| key(o) == key(c) && latest_job(o))
        }
        None => checks.iter().any(|o| key(o) == key(c) && older(c, o)),
    };
    checks.iter().filter(|c| !gone(c)).cloned().collect()
}

/// One GitHub Actions run of a commit.
pub struct Run { pub id: String, pub path: String, pub status: String, pub conclusion: String, pub url: String, pub name: String }
/// Every page of a list GitHub gives at most 100 to a page, each item a line of `jq`.
fn every_page(root: &Path, path: &str, jq: &str) -> GhOut { gh(&["api", "--paginate", path, "--jq", jq], root, 60000) }
/// The commit's GitHub Actions runs (the Actions permission reads them), all of them.
pub fn runs(root: &Path, head: &str) -> Result<Vec<Run>, GhOut> {
    let r = every_page(root, &format!("repos/{{owner}}/{{repo}}/actions/runs?head_sha={head}&per_page=100"), ".workflow_runs[] | [.id, .path, .status, (.conclusion // \"\"), .html_url, .name] | @tsv");
    if r.status != 0 { return Err(r); }
    Ok(r.stdout.lines().filter(|l| !l.trim().is_empty()).map(|l| {
        let x: Vec<String> = l.split('\t').map(String::from).collect();
        let at = |i: usize| x.get(i).cloned().unwrap_or_default();
        Run { id: at(0), path: at(1), status: at(2), conclusion: at(3), url: at(4), name: at(5) }
    }).collect())
}
fn check_run(name: &str, status: &str, conclusion: &str, url: &str, flow: &str) -> Value {
    json!({ "__typename": "CheckRun", "name": name, "status": status.to_uppercase(), "conclusion": conclusion.to_uppercase(), "detailsUrl": url, "workflowName": flow })
}
/// The commit's checks, from what a fine-grained token reads, every page of them: each of its runs as itself (still
/// running or failed, whatever its jobs listed so far say; `run`), then each of the run's jobs, and each of its commit
/// statuses.
fn of_commit(root: &Path, head: &str) -> Result<Vec<Value>, String> {
    let mut out: Vec<Value> = vec![];
    let runs = runs(root, head).map_err(|r| format!("GitHub Actions runs (gh api …/actions/runs): {}", said(&r)))?;
    for run in runs {
        let mut me = check_run(&run.name, &run.status, &run.conclusion, &run.url, &run.name); me["run"] = json!(true);
        out.push(me);
        let r = every_page(root, &format!("repos/{{owner}}/{{repo}}/actions/runs/{}/jobs?per_page=100", run.id), ".jobs[] | [.name, .status, (.conclusion // \"\"), .html_url] | @tsv");
        if r.status != 0 { continue; }
        out.extend(r.stdout.lines().filter(|l| !l.trim().is_empty()).map(|l| {
            let x: Vec<&str> = l.split('\t').collect();
            let at = |i: usize| x.get(i).copied().unwrap_or("");
            check_run(at(0), at(1), at(2), at(3), &run.name)
        }));
    }
    let r = every_page(root, &format!("repos/{{owner}}/{{repo}}/commits/{head}/status?per_page=100"), ".statuses[] | [.context, .state, (.target_url // \"\")] | @tsv");
    if r.status != 0 { return Err(format!("commit statuses (gh api …/commits/{head}/status): {}", said(&r))); }
    for l in r.stdout.lines().filter(|l| !l.trim().is_empty()) {
        let x: Vec<&str> = l.split('\t').collect();
        let at = |i: usize| x.get(i).copied().unwrap_or("");
        out.push(json!({ "__typename": "StatusContext", "context": at(0), "state": at(1).to_uppercase(), "targetUrl": at(2) }));
    }
    Ok(out)
}

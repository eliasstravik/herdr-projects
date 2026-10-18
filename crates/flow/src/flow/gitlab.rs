//! GitLab, on gitlab.com and on hosts of your own (`board.gitlabHosts`), behind the board's gh calls: in a repo whose
//! `origin` is on GitLab, each call the board makes of gh (git.rs: `gh`) is answered through `glab api`, in gh's own
//! shape, so every step that opens, reads, waits for, comments on and merges a pull request does the same with a merge
//! request and its pipeline (each of its jobs a check). docs/gitlab.md.
use crate::flow::git::GhOut;
use crate::json::*;
use crate::store::repos::git_out;
use crate::store::settings::lookup::setting_text;
use crate::util::url_encode;
use std::path::Path;
use std::process::{Command, Stdio};

/// The host of a remote's link (`git@host:path`, `ssh://git@host:port/path`, `https://user:pw@host:port/path`), in
/// small letters. "" for a folder.
pub fn host_of(url: &str) -> String {
    let rest = match url.split_once("://") { Some((_, r)) => r, None if !url.starts_with('/') && url.contains(':') => url, None => return String::new() };
    let at = rest.split('/').next().unwrap_or("");
    at.rsplit('@').next().unwrap_or("").split(':').next().unwrap_or("").to_lowercase()
}
/// The GitLab host of the repo `cwd` is in, from its `origin` as written (an `insteadOf` does not change it): gitlab.com,
/// or one named in `board.gitlabHosts`. None: GitHub, as before.
pub fn gitlab_host(cwd: &Path) -> Option<String> {
    let host = host_of(&git_out(cwd, &["config", "--get", "remote.origin.url"]).ok()?);
    if host.is_empty() { return None; }
    let set = setting_text("board.gitlabHosts", None);
    let mine = set != "none" && set.split([',', ' ']).filter(|h| !h.is_empty()).any(|h| { let h = if h.contains("://") { host_of(h) } else { h.trim_end_matches('/').to_lowercase() }; h == host });
    (host == "gitlab.com" || mine).then_some(host)
}
pub fn is_gitlab(cwd: &Path) -> bool { gitlab_host(cwd).is_some() }

type Fake = std::rc::Rc<dyn Fn(&[String]) -> GhOut>;
thread_local! { static FAKE_GLAB: std::cell::RefCell<Option<Fake>> = const { std::cell::RefCell::new(None) }; }
/// A test answers for glab itself, so no program is started then.
pub fn set_fake_glab(f: Option<Fake>) { FAKE_GLAB.with(|x| *x.borrow_mut() = f); }
fn glab(a: &[String], cwd: &Path, ms: u64) -> GhOut {
    if let Some(f) = FAKE_GLAB.with(|x| x.borrow().clone()) { return f(a); }
    crate::runner::user_env();
    let child = Command::new("glab").args(a).current_dir(cwd).env("GLAB_NO_PROMPT", "1").env("GLAB_CHECK_UPDATE", "false").env("GIT_TERMINAL_PROMPT", "0").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    match child.map_err(|e| e.to_string()).and_then(|c| crate::herdr::cli::wait_with_timeout(c, ms).map_err(|e| e.message)) {
        Ok(r) => GhOut { status: r.status, stdout: r.stdout, stderr: r.stderr },
        Err(e) => GhOut { status: -1, stdout: String::new(), stderr: e },
    }
}
fn ok(out: impl Into<String>) -> GhOut { GhOut { status: 0, stdout: out.into(), stderr: String::new() } }
fn fail(m: String) -> GhOut { GhOut { status: 1, stdout: String::new(), stderr: m } }

/// GitLab's API for one repo, as glab reaches it from the repo's folder (`:id` is the repo, its host the remote's).
struct Api<'a> { cwd: &'a Path, ms: u64 }
impl Api<'_> {
    /// `raw` fields are sent as text (`-f`), `typed` ones as what they say (`-F`: true, false, numbers).
    fn call(&self, method: &str, path: &str, raw: &[(&str, &str)], typed: &[(&str, &str)]) -> GhOut {
        let mut a = vec!["api".to_string()];
        if method != "GET" { a.extend(["-X".into(), method.into()]); }
        a.push(path.into());
        for (flag, fields) in [("-f", raw), ("-F", typed)] { for (k, v) in fields { a.extend([flag.to_string(), format!("{k}={v}")]); } }
        glab(&a, self.cwd, self.ms)
    }
    fn json(&self, method: &str, path: &str, raw: &[(&str, &str)]) -> Result<Value, GhOut> {
        let r = self.call(method, path, raw, &[]);
        if r.status != 0 { return Err(r); }
        serde_json::from_str(&r.stdout).map_err(|e| fail(format!("glab api {path}: {e}")))
    }
    fn get(&self, path: &str) -> Result<Value, GhOut> { self.json("GET", path, &[]) }
}
fn mr_path(n: &str) -> String { format!("projects/:id/merge_requests/{n}") }
fn state_of(mr: &Value) -> &'static str { match mr.s("state") { "opened" => "OPEN", "merged" => "MERGED", _ => "CLOSED" } }

/// One gh call, answered by GitLab. What gh is asked that has no GitLab side (GitHub Actions, branch protection, check
/// runs) answers as nothing there.
pub fn answer(a: &[String], cwd: &Path, ms: u64) -> GhOut {
    let api = Api { cwd, ms };
    let s: Vec<&str> = a.iter().map(String::as_str).filter(|x| *x != "--paginate").collect();
    let flag = |f: &str| s.iter().position(|x| *x == f).and_then(|i| s.get(i + 1)).copied().unwrap_or("");
    let has = |f: &str| s.contains(&f);
    let n = s.get(2).copied().unwrap_or("");
    let done = |r: Result<GhOut, GhOut>| r.unwrap_or_else(|e| e);
    match (s.first().copied().unwrap_or(""), s.get(1).copied().unwrap_or("")) {
        ("api", "user") => done(api.get("user").map(|v| ok(v.t("username")))),
        // Does CI report on the branch it merges into (land/expected.rs: outside_ci): its pipelines there.
        ("api", p) if p.ends_with("/status") && flag("--jq") == ".total_count" => {
            let base = p.trim_end_matches("/status").rsplit("/commits/").next().unwrap_or("");
            done(api.get(&format!("projects/:id/pipelines?ref={}&per_page=1", url_encode(base))).map(|v| ok(v.as_array().map(|l| l.len()).unwrap_or(0).to_string())))
        }
        ("api", p) if p.ends_with("/check-runs") => ok("0"),
        ("api", _) => ok(""),
        ("pr", "view") => done(view(&api, n, flag("--json"))),
        ("pr", "list") if has("--head") => done(list(&api, flag("--head"))),
        ("pr", "list") => ok("0"),
        ("pr", "create") => done(create(&api, flag("--head"), flag("--base"), flag("--title"), flag("--body"))),
        ("pr", "comment") => api.call("POST", &format!("{}/notes", mr_path(n)), &[("body", flag("--body"))], &[]),
        ("pr", "ready") => done(ready(&api, n)),
        ("pr", "merge") if has("--disable-auto") => api.call("POST", &format!("{}/cancel_merge_when_pipeline_succeeds", mr_path(n)), &[], &[]),
        ("pr", "merge") => done(merge(&api, n, flag("--match-head-commit"), has("--squash"), has("--rebase"))),
        ("repo", "view") => done(api.get("projects/:id").map(|v| ok(if v.at("archived") == &json!(true) { "true" } else { "false" }))),
        // A failed job's log (land/checks.rs: log_of).
        ("run", "view") => { let mut r = api.call("GET", &format!("projects/:id/jobs/{}/trace", flag("--job")), &[], &[]); r.stdout = plain_trace(&r.stdout); r }
        _ => fail(format!("not done on GitLab: gh {}", a.join(" "))),
    }
}

fn checked(r: GhOut) -> Result<GhOut, GhOut> { if r.status == 0 { Ok(r) } else { Err(r) } }
/// `gh pr merge`, only at `sha` when one is given. `rebase`: the merge request is first rebased onto the branch it
/// merges into (GitLab does it in the background, waited for here), its pipeline not run again, as GitHub's rebase
/// merge runs none; then its rebased commits merge, without squashing, the way the project merges (fast-forward for a
/// linear history). Its source branch goes with it: the board's own delete leases the reviewed commit, which the
/// rebase replaced (git.rs: drop_remote_branch).
fn merge(api: &Api, n: &str, sha: &str, squash: bool, rebase: bool) -> Result<GhOut, GhOut> {
    let mut at = sha.to_string();
    if rebase {
        let mr = api.get(&format!("{}?include_diverged_commits_count=true", mr_path(n)))?;
        let was = mr.t("sha");
        if !sha.is_empty() && was != sha { return Err(fail(format!("Head branch was modified: the merge request is at {was}, not at {sha}. Review and try the merge again."))); }
        // Already on the branch it merges into: nothing to rebase (GitLab would leave its commit as it is).
        if mr.i("diverged_commits_count") > 0 {
            checked(api.call("PUT", &format!("{}/rebase", mr_path(n)), &[], &[("skip_ci", "true")]))?;
            // Right after it was asked, GitLab may not have started it: done once it is not running and the commit moved.
            let pause = std::env::var("HERDR_PROJECTS_PR_LAG_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(1000u64);
            let mut done = false;
            for _ in 0..120 {
                let m = api.get(&format!("{}?include_rebase_in_progress=true", mr_path(n)))?;
                if !m.s("merge_error").is_empty() { return Err(fail(format!("GitLab could not rebase it: {}", m.s("merge_error")))); }
                if m.at("rebase_in_progress") != &json!(true) && m.s("sha") != was { at = m.t("sha"); done = true; break; }
                std::thread::sleep(std::time::Duration::from_millis(pause));
            }
            if !done { return Err(fail("GitLab's rebase of the merge request did not finish within 2 minutes; nothing was merged.".into())); }
        }
    }
    let raw: Vec<(&str, &str)> = if at.is_empty() { vec![] } else { vec![("sha", at.as_str())] };
    let mut typed = vec![("squash", if squash { "true" } else { "false" })];
    if rebase { typed.push(("should_remove_source_branch", "true")); }
    checked(api.call("PUT", &format!("{}/merge", mr_path(n)), &raw, &typed))
}
/// A job's trace as its lines read: without a recent runner's time and stream mark at the start of each
/// (`2026-…Z 01O+`), its colours, and its section marks (`section_start:<time>:<name>`).
fn plain_trace(t: &str) -> String {
    t.lines().map(|l| {
        let l = crate::re!(r"\x1b\[[0-9;]*[A-Za-z]").replace_all(l, "");
        let l = crate::re!(r"^\d{4}-\d\d-\d\dT[0-9:.]+Z [0-9a-f]{2}[OE]\+? ?").replace(&l, "");
        crate::re!(r"section_(?:start|end):\d+:[^\r]*\r?").replace_all(&l, "").trim_end().to_string()
    }).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
}
/// `gh pr view <n> --json <fields>`: the merge request in a pull request's words. Its comments and checks are asked
/// for only when wanted.
fn view(api: &Api, n: &str, fields: &str) -> Result<GhOut, GhOut> {
    let mr = api.get(&mr_path(n))?;
    let want = |f: &str| fields.split(',').any(|x| x == f);
    let merged = [mr.t("merge_commit_sha"), mr.t("squash_commit_sha"), if state_of(&mr) == "MERGED" { mr.t("sha") } else { String::new() }].into_iter().find(|s| !s.is_empty());
    let decision = match mr.s("detailed_merge_status") { "not_approved" => "REVIEW_REQUIRED", "requested_changes" => "CHANGES_REQUESTED", _ => "" };
    let mut v = json!({ "number": mr.at("iid"), "url": mr.t("web_url"), "state": state_of(&mr), "headRefOid": mr.t("sha"), "baseRefName": mr.t("target_branch"), "isDraft": mr.at("draft") == &json!(true),
        "author": { "login": mr.at("author").t("username") }, "reviewDecision": decision, "reviews": [], "mergeCommit": merged.map(|m| json!({ "oid": m })).unwrap_or(Value::Null) });
    if want("comments") {
        let notes = api.get(&format!("{}/notes?sort=desc&per_page=100", mr_path(n)))?;
        v["comments"] = json!(notes.as_array().cloned().unwrap_or_default().iter().filter(|x| x.at("system") != &json!(true))
            .map(|x| json!({ "id": x.t("id"), "author": { "login": x.at("author").t("username") }, "body": x.t("body"), "updatedAt": x.t("updated_at") })).collect::<Vec<_>>());
    }
    if want("statusCheckRollup") { v["statusCheckRollup"] = json!(checks(api, &mr)?); }
    Ok(ok(v.to_string()))
}
/// The checks of the merge request's head: its pipeline as a whole (what its jobs do not show, a child pipeline's, is
/// in it) and each of its jobs. A pipeline of an older commit is none for this one yet.
fn checks(api: &Api, mr: &Value) -> Result<Vec<Value>, GhOut> {
    let p = mr.at("head_pipeline");
    if !p.is_object() || p.s("sha") != mr.s("sha") { return Ok(vec![]); }
    let jobs = api.get(&format!("projects/:id/pipelines/{}/jobs?per_page=100", p.t("id")))?;
    let mut out = vec![check("pipeline", p.s("status"), false, p.s("web_url"))];
    out.extend(jobs.as_array().cloned().unwrap_or_default().iter().map(|j| check(j.s("name"), j.s("status"), j.at("allow_failure") == &json!(true), j.s("web_url"))));
    Ok(out)
}
/// A pipeline's or a job's status in GitHub's words: a job allowed to fail, or one left to be started by hand, blocks nothing.
fn check(name: &str, status: &str, allowed: bool, url: &str) -> Value {
    let (st, end) = match status {
        "success" => ("COMPLETED", "SUCCESS"),
        "failed" if allowed => ("COMPLETED", "NEUTRAL"),
        "failed" => ("COMPLETED", "FAILURE"),
        "canceled" | "canceling" => ("COMPLETED", "CANCELLED"),
        "skipped" => ("COMPLETED", "SKIPPED"),
        "manual" => ("COMPLETED", "NEUTRAL"),
        _ => ("IN_PROGRESS", ""),
    };
    json!({ "__typename": "CheckRun", "name": name, "status": st, "conclusion": end, "detailsUrl": url, "workflowName": "GitLab CI" })
}
/// `gh pr list --head <branch>`: the merge requests of that source branch, newest first.
fn list(api: &Api, branch: &str) -> Result<GhOut, GhOut> {
    let v = api.get(&format!("projects/:id/merge_requests?source_branch={}&state=all&per_page=10&order_by=created_at&sort=desc", url_encode(branch)))?;
    let l: Vec<Value> = v.as_array().cloned().unwrap_or_default().iter().map(|m| json!({ "number": m.at("iid"), "url": m.t("web_url"), "state": state_of(m), "baseRefName": m.t("target_branch") })).collect();
    Ok(ok(json!(l).to_string()))
}
/// `gh pr create --draft`: a draft merge request into `base`, or into the repo's default branch. Prints its link.
fn create(api: &Api, head: &str, base: &str, title: &str, body: &str) -> Result<GhOut, GhOut> {
    let target = if base.is_empty() { api.get("projects/:id")?.t("default_branch") } else { base.to_string() };
    let title = format!("Draft: {title}");
    let mr = api.json("POST", "projects/:id/merge_requests", &[("source_branch", head), ("target_branch", &target), ("title", &title), ("description", body)])?;
    Ok(ok(format!("{}\n", mr.t("web_url"))))
}
/// `gh pr ready`: GitLab keeps a draft in its title, so the title loses its draft mark.
fn ready(api: &Api, n: &str) -> Result<GhOut, GhOut> {
    let title = api.get(&mr_path(n))?.t("title");
    let plain = crate::re!(r"(?i)^\s*(?:\[draft\]|\(draft\)|draft:|draft\s+-)\s*").replace(&title, "").to_string();
    if plain == title { return Ok(ok("")); }
    Ok(api.call("PUT", &mr_path(n), &[("title", &plain)], &[]))
}

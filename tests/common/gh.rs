//! A stand-in for the gh program, answering in the test's own thread (flow::git::set_fake_gh): canned answers set by
//! the test (`w.gh_says(json!({..}))`), every call logged (`gh_log(&w)`). No real gh, no network.
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use sw::flow::git::GhOut;
use sw::json::*;

/// What gh answers, as the old test/fake-gh.js: `user`, `createFail`, `head`, `disableFail`, `autoOk`, `mergeFail`,
/// `queue` (accepted, only lined up to merge later), `view` (what `pr view` gives), `late: { head, times }` (the first
/// `times` looks at the pull request still name `head`, the commit before a push), `log` (what `run view --log-failed`
/// prints of a failed job), `base` and `baseFail` (the checks of the branch it merges into, asked through `api`), `required` (the checks it requires), `runs` (a commit's workflow runs), `others`
/// (how many of the repo's last pull requests had checks). A fine-grained token: `noChecks` (GitHub refuses it the
/// checks list, GraphQL's `statusCheckRollup` and the check-runs API, as it does a token with no Checks permission),
/// `jobs` (each run's jobs), `statuses` (the commit's statuses), `actionsFail` (it may not read Actions either).
/// `noLogin`: gh is not logged in where it runs (a keeper started without the login shell's GH_TOKEN): every call fails.
/// `prs`: the pull requests GitHub has for a head branch (`pr list --head`), each { number, url, state }; none by
/// default. `archived`: the repo is archived on GitHub (`repo view`).
#[derive(Default)]
pub struct FakeGh { pub cfg: RefCell<Value>, pub log: RefCell<Vec<String>>, pub merged: Cell<bool>, late: Cell<i64> }
fn ok(out: &str) -> GhOut { GhOut { status: 0, stdout: if out.is_empty() { String::new() } else { format!("{out}\n") }, stderr: String::new() } }
fn rows(v: &Value) -> String { v.as_array().map(|a| a.iter().map(|r| strings(r).join("\t")).collect::<Vec<_>>().join("\n")).unwrap_or_default() }
fn fail(m: &str) -> GhOut { GhOut { status: 1, stdout: String::new(), stderr: format!("{m}\n") } }
impl FakeGh {
    pub fn run(&self, a: &[String]) -> GhOut {
        let c = self.cfg.borrow().clone();
        self.log.borrow_mut().push(a.join(" "));
        // Every page of a list is one answer here (`api --paginate`).
        let a: Vec<String> = a.iter().filter(|s| *s != "--paginate").cloned().collect();
        let (x, y) = (a.first().map(|s| s.as_str()).unwrap_or(""), a.get(1).map(|s| s.as_str()).unwrap_or(""));
        let has = |f: &str| a.iter().any(|s| s == f);
        let rollup = a.iter().any(|s| s.contains("statusCheckRollup"));
        if c.b("noLogin") { return GhOut { status: 4, stdout: String::new(), stderr: "To get started with GitHub CLI, please run:  gh auth login\nAlternatively, populate the GH_TOKEN environment variable with a GitHub API authentication token.\n".into() }; }
        if c.b("noChecks") && (rollup || a.iter().any(|s| s.contains("/check-runs"))) {
            return fail(if x == "api" { "gh: Resource not accessible by personal access token (HTTP 403)" } else { "GraphQL: Resource not accessible by personal access token (repository.pullRequest.statusCheckRollup)" });
        }
        match (x, y) {
            ("api", "user") => ok(if c.b("user") { c.s("user") } else { "me" }),
            ("pr", "create") => if c.b("createFail") { fail(c.s("createFail")) } else { ok("https://github.com/o/r/pull/7") },
            ("pr", "ready") | ("pr", "comment") => ok(""),
            ("pr", "merge") => {
                let only = a.iter().position(|s| s == "--match-head-commit").and_then(|i| a.get(i + 1)).cloned().unwrap_or_default();
                if c.b("head") && !only.is_empty() && only != c.s("head") && !has("--disable-auto") { return fail("Head branch was modified. Review and try the merge again."); }
                if has("--disable-auto") { return if c.b("disableFail") { fail("could not disable auto-merge") } else { ok("") }; }
                if has("--auto") { return if c.b("autoOk") { ok("") } else { fail("auto-merge is not allowed") }; }
                if c.b("mergeFail") { return fail(c.s("mergeFail")); }
                if !c.b("queue") { self.merged.set(true); }
                ok("")
            }
            ("pr", "view") => {
                if c.at("late").is_object() { let n = self.late.get(); self.late.set(n + 1); if n < c.at("late").i("times") { return ok(&json!({ "state": "OPEN", "headRefOid": c.at("late").at("head") }).to_string()); } }
                let mut v = if c.at("view").is_object() { c.at("view").clone() } else { json!({ "state": "OPEN", "statusCheckRollup": [], "comments": [], "reviews": [], "author": { "login": "me" } }) };
                if self.merged.get() { v["state"] = json!("MERGED"); }
                if !rollup { if let Some(o) = v.as_object_mut() { o.remove("statusCheckRollup"); } }
                ok(&v.to_string())
            }
            // Does the branch a pull request merges into have checks: `base: { "check-runs": n, "status": n }`, or
            // `baseFail` (GitHub cannot say). None by default.
            // A run's jobs: `jobs: { "<run id>": [[name, status, conclusion, url], …] }`.
            ("api", p) if p.contains("/actions/runs/") && p.contains("/jobs") => {
                let run = p.split("/actions/runs/").nth(1).unwrap_or("").split('/').next().unwrap_or("");
                ok(&rows(c.at("jobs").at(run)))
            }
            // A commit's workflow runs: `runs: [[path, status, conclusion, url, name?], …]`, given as id, path, status,
            // conclusion, url and name (the id from the url's `/runs/<id>`, the name "CI" if none).
            ("api", p) if p.contains("/actions/runs") => if c.b("actionsFail") { fail("gh: Resource not accessible by personal access token (HTTP 403)") } else {
                ok(&c.arr("runs").iter().enumerate().map(|(i, r)| {
                    let x = strings(r);
                    let at = |i: usize| x.get(i).cloned().unwrap_or_default();
                    let id = at(3).split("/runs/").nth(1).map(|r| r.chars().take_while(|c| c.is_ascii_digit()).collect::<String>()).filter(|s| !s.is_empty()).unwrap_or((i + 1).to_string());
                    [id, at(0), at(1), at(2), at(3), if x.len() > 4 { at(4) } else { "CI".into() }].join("\t")
                }).collect::<Vec<_>>().join("\n"))
            },
            // The pull request's commit's statuses: `statuses: [[context, state, url], …]`.
            ("api", p) if p.contains("/status") && a.iter().any(|s| s.contains(".statuses[]")) => ok(&rows(c.at("statuses"))),
            // The checks the branch it merges into requires: `required: [names]`.
            ("api", p) if p.contains("/branches/") => ok(&strings(c.at("required")).join("\n")),
            ("api", p) if p.starts_with("repos/") => if c.b("baseFail") { fail("HTTP 403") } else { let k = p.rsplit('/').next().unwrap_or(""); ok(&c.at("base").i(k).to_string()) },
            // How many of the repo's last pull requests had checks: `others`.
            ("pr", "list") if has("--head") => ok(&if c.at("prs").is_array() { c.at("prs").to_string() } else { "[]".into() }),
            ("pr", "list") => ok(&c.i("others").to_string()),
            ("repo", "view") => ok(if c.b("archived") { "true" } else { "false" }),
            ("run", "view") => if c.b("log") { ok(c.s("log")) } else { fail("log not found") },
            _ => fail(&format!("fake gh: unknown {}", a.join(" "))),
        }
    }
}

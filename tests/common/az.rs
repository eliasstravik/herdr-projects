//! A stand-in for the az program (the Azure CLI), answering in the test's own thread (azure::cli::set_fake_az): canned
//! answers set by the test (`w.az_says(json!({..}))`), every call logged (`az_log(&w)`), and what each `devops invoke`
//! sent kept (`sent`). No real az, no network. `at_azure(&w)` puts the world's repo at Azure DevOps.
use super::World;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use sw::flow::git::GhOut;
use sw::json::*;

/// What az answers: `head` (the pull request's last pushed commit), `merge` (its merge commit once completed),
/// `status` (`active`, `completed`, `abandoned`; completed once the board completed it), `reviewers`, `policies`
/// (each evaluation as az gives it), `prs` (`repos pr list`), `createFail`, `completeFail`, `noLogin`, `threads`,
/// `runs` (`pipelines runs list`), `pipelines` (`pipelines list`; one by default), `timeline` (by build id), `log`
/// (a build log's lines).
#[derive(Default)]
pub struct FakeAz { pub cfg: RefCell<Value>, pub log: RefCell<Vec<String>>, pub sent: RefCell<Vec<Value>>, pub completed: Cell<bool> }
fn ok(v: Value) -> GhOut { GhOut { status: 0, stdout: format!("{v}\n"), stderr: String::new() } }
fn fail(m: &str) -> GhOut { GhOut { status: 1, stdout: String::new(), stderr: format!("ERROR: {m}\n") } }
/// The URL the world's repo has at Azure DevOps.
pub const URL: &str = "https://acme@dev.azure.com/acme/Big%20Project/_git/shop";
pub const WEB: &str = "https://dev.azure.com/acme/Big%20Project/_git/shop";
/// The world's origin is at Azure DevOps (`URL`): git still pushes to its bare repo, through `insteadOf`.
pub fn at_azure(w: &World) {
    let bare = super::git(&w.root, &["config", "--get", "remote.origin.url"]);
    super::git(&w.root, &["remote", "set-url", "origin", URL]);
    super::git(&w.root, &["config", &format!("url.{bare}.insteadOf"), URL]);
}
impl FakeAz {
    fn pr(&self, c: &Value) -> Value {
        let status = if self.completed.get() { "completed".to_string() } else if c.b("status") { c.t("status") } else { "active".into() };
        json!({ "pullRequestId": 42, "status": status, "title": "t", "isDraft": false, "sourceRefName": "refs/heads/b", "targetRefName": "refs/heads/main",
            "lastMergeSourceCommit": { "commitId": c.at("head") }, "lastMergeCommit": { "commitId": if c.b("merge") { c.t("merge") } else { "m".repeat(40) } },
            "reviewers": if c.at("reviewers").is_array() { c.at("reviewers").clone() } else { json!([]) }, "createdBy": { "uniqueName": "me@acme.test" } })
    }
    pub fn run(&self, a: &[String]) -> GhOut {
        let c = self.cfg.borrow().clone();
        self.log.borrow_mut().push(a.iter().take_while(|s| *s != "--org").cloned().collect::<Vec<_>>().join(" "));
        if c.b("noLogin") { return GhOut { status: 1, stdout: String::new(), stderr: "ERROR: Please run 'az login' to setup account.\n".into() }; }
        let at = |i: usize| a.get(i).map(String::as_str).unwrap_or("");
        let has = |f: &str| a.iter().any(|s| s == f);
        let param = |k: &str| a.iter().find_map(|s| s.strip_prefix(&format!("{k}="))).unwrap_or("").to_string();
        match (at(0), at(1), at(2)) {
            ("account", "show", _) => GhOut { status: 0, stdout: "me@acme.test\n".into(), stderr: String::new() },
            ("repos", "pr", "create") => if c.b("createFail") { fail(c.s("createFail")) } else { ok(self.pr(&c)) },
            ("repos", "pr", "list") => ok(if c.at("prs").is_array() { c.at("prs").clone() } else { json!([]) }),
            ("repos", "pr", "show") => ok(self.pr(&c)),
            ("repos", "pr", "update") => ok(self.pr(&c)),
            ("repos", "pr", "policy") => ok(if c.at("policies").is_array() { c.at("policies").clone() } else { json!([]) }),
            ("pipelines", "runs", "list") => ok(if c.at("runs").is_array() { c.at("runs").clone() } else { json!([]) }),
            ("pipelines", "list", _) => ok(if c.at("pipelines").is_array() { c.at("pipelines").clone() } else { json!([{ "id": 1, "name": "CI" }]) }),
            ("devops", "invoke", _) => {
                let file = a.iter().position(|s| s == "--in-file").and_then(|i| a.get(i + 1)).cloned();
                let body = file.and_then(|f| std::fs::read_to_string(f).ok()).and_then(|s| serde_json::from_str::<Value>(&s).ok()).unwrap_or(Value::Null);
                let resource = a.iter().position(|s| s == "--resource").and_then(|i| a.get(i + 1)).cloned().unwrap_or_default();
                if !body.is_null() { self.sent.borrow_mut().push(json!({ "resource": resource, "body": body })); }
                match resource.as_str() {
                    "pullRequests" => { if c.b("completeFail") { return fail(c.s("completeFail")); } if !c.b("completeQueued") { self.completed.set(true); } ok(self.pr(&c)) }
                    "pullRequestThreads" => ok(if has("POST") { json!({ "id": 1 }) } else { json!({ "value": if c.at("threads").is_array() { c.at("threads").clone() } else { json!([]) } }) }),
                    "timeline" => ok(c.at("timeline").at(&param("buildId")).clone()),
                    "logs" => ok(json!({ "count": 2, "value": c.at("log") })),
                    _ => fail(&format!("fake az: unknown resource {resource}")),
                }
            }
            _ => fail(&format!("fake az: unknown {}", a.join(" "))),
        }
    }
}
/// A policy evaluation as az gives it: `kind` "Build" (with a build id), "Status" or a policy of people.
pub fn policy(kind: &str, name: &str, status: &str, build: Option<i64>) -> Value {
    let mut settings = json!({ "displayName": name });
    if kind == "Build" { settings["buildDefinitionId"] = json!(7); }
    json!({ "configuration": { "isBlocking": true, "isEnabled": true, "type": { "displayName": kind }, "settings": settings }, "context": { "buildId": build }, "status": status })
}
/// A run of a pipeline as `az pipelines runs list` gives it.
pub fn run(id: i64, pipeline: &str, commit: &str, status: &str, result: &str) -> Value {
    json!({ "id": id, "definition": { "id": id * 10, "name": pipeline }, "sourceVersion": commit, "status": status, "result": result })
}

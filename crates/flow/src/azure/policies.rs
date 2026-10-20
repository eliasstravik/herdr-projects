//! A pull request's policies on Azure DevOps (`az repos pr policy list`), read as gh's checks: each build validation
//! and status check is a check (its result as GitHub words one, a build with its id for its log: logs.rs); the other
//! blocking policies (reviewers, required reviewers, resolved comments, linked work items) are not checks the agent
//! can fix but a review still required. A policy that is not blocking never holds the merge up.
use super::cli::json_of;
use super::url::Azure;
use crate::json::*;
use std::path::Path;

/// What kind of policy an evaluation is of: a build, an outside status, or one of people.
#[derive(Debug, PartialEq, Eq)]
pub enum Kind { Build, Status, People }
pub fn kind(p: &Value) -> Kind {
    let c = p.at("configuration");
    let ty = c.at("type").s("displayName").to_lowercase();
    if c.at("settings").b("buildDefinitionId") || ty == "build" { Kind::Build } else if ty == "status" { Kind::Status } else { Kind::People }
}
/// Its name as the task says it: the one given to it, else what it checks.
pub fn name(p: &Value) -> String {
    let (c, s) = (p.at("configuration"), p.at("configuration").at("settings"));
    if s.b("displayName") { return s.t("displayName"); }
    // A build validation with no name of its own is called after its pipeline.
    if p.at("context").b("buildDefinitionName") { return p.at("context").t("buildDefinitionName"); }
    if kind(p) == Kind::Status && s.b("statusName") { return [s.s("statusGenre"), s.s("statusName")].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join("/"); }
    c.at("type").t("displayName")
}
/// An evaluation's status, as gh's `status` and `conclusion` of a check run. One not blocking counts, once through, as
/// neutral: it never fails the merge, and is never waited for.
pub fn words(status: &str, blocking: bool) -> (&'static str, &'static str) {
    match (status.to_lowercase().as_str(), blocking) {
        ("approved", _) => ("COMPLETED", "SUCCESS"),
        ("notapplicable", _) => ("COMPLETED", "SKIPPED"),
        (_, false) => ("COMPLETED", "NEUTRAL"),
        ("rejected", true) => ("COMPLETED", "FAILURE"),
        ("broken", true) => ("COMPLETED", "ERROR"),
        ("running", true) => ("IN_PROGRESS", ""),
        ("queued", true) => ("QUEUED", ""),
        _ => ("PENDING", ""),
    }
}
/// The evaluations as gh's checks, and whether a blocking policy of people is not met yet.
pub fn rollup(a: &Azure, evaluations: &[Value]) -> (Vec<Value>, bool) {
    let mut checks = vec![];
    let mut people = false;
    for p in evaluations.iter().filter(|p| p.at("configuration").at("isEnabled") != &json!(false)) {
        let blocking = p.at("configuration").at("isBlocking") != &json!(false);
        let st = p.s("status").to_lowercase();
        match kind(p) {
            Kind::People => { if blocking && st != "approved" && st != "notapplicable" { people = true; } }
            k => {
                let (status, conclusion) = words(&st, blocking);
                let build = p.at("context").t("buildId");
                let url = if !build.is_empty() { a.build_url(&build) } else { p.at("context").t("targetUrl") };
                let mut c = json!({ "__typename": "CheckRun", "name": name(p), "status": status, "conclusion": conclusion, "detailsUrl": url, "workflowName": "", "azure": true });
                if k == Kind::Build && !build.is_empty() { c["buildId"] = json!(build); }
                checks.push(c);
            }
        }
    }
    (checks, people)
}
/// The pull request's policies, read. Err: what az said.
pub fn read(a: &Azure, cwd: &Path, pr: &str) -> Result<(Vec<Value>, bool), String> {
    let v = json_of(a, cwd, &["repos", "pr", "policy", "list", "--id", &super::pr::id_of(pr)], 60000)?;
    Ok(rollup(a, v.as_array().map(|l| l.as_slice()).unwrap_or(&[])))
}

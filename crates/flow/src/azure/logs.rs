//! A build's timeline on Azure Pipelines (its stages, jobs and steps, through `az devops invoke`), and the end of the
//! log of what failed in it: the failing job's name and step, and its last lines, what the agent is sent and the task
//! shows, as `gh run view --log-failed` gives it for GitHub.
use super::cli::invoke;
use super::url::Azure;
use crate::json::*;
use std::path::Path;

const LOG_LINES: usize = 40;
/// One record of a build's timeline: a stage, a phase, a job or a step.
#[derive(Clone, Debug, Default)]
pub struct Record { pub id: String, pub parent: String, pub kind: String, pub name: String, pub state: String, pub result: String, pub log: String, pub order: i64 }
pub fn records_of(v: &Value) -> Vec<Record> {
    let mut out: Vec<Record> = v.arr("records").iter().map(|r| Record { id: r.t("id"), parent: r.t("parentId"), kind: r.t("type"), name: r.t("name"), state: r.t("state"), result: r.t("result"), log: r.at("log").t("id"), order: r.i("order") }).collect();
    out.sort_by_key(|r| r.order);
    out
}
/// The build's timeline. Empty when it cannot be read (yet).
pub fn timeline(a: &Azure, root: &Path, build: &str) -> Vec<Record> {
    invoke(a, root, "build", "timeline", &[("buildId", build)], "GET", None).map(|v| records_of(&v)).unwrap_or_default()
}
/// What failed in it: each failed step with a log, and the job it is in. A job that failed with no step to show is
/// taken as itself.
pub fn failed(recs: &[Record]) -> Vec<(String, Record)> {
    let job_of = |r: &Record| recs.iter().find(|j| j.id == r.parent && j.kind == "Job").map(|j| j.name.clone()).unwrap_or_default();
    let bad = |r: &&Record| r.result == "failed" || r.result == "canceled" || r.result == "abandoned";
    let steps: Vec<(String, Record)> = recs.iter().filter(bad).filter(|r| r.kind == "Task" && !r.log.is_empty()).map(|r| (job_of(r), r.clone())).collect();
    if !steps.is_empty() { return steps; }
    recs.iter().filter(bad).filter(|r| r.kind == "Job").map(|r| (r.name.clone(), r.clone())).collect()
}
/// A log's lines as they are read: its time stamps and colours off.
pub fn clean(lines: &[String]) -> Vec<String> {
    lines.iter().map(|l| {
        let l = crate::re!(r"\x1b\[[0-9;]*[A-Za-z]").replace_all(l, "");
        crate::re!(r"^\d{4}-\d\d-\d\dT[0-9:.]+Z ?").replace(&l, "").trim_end().to_string()
    }).filter(|l| !l.trim().is_empty()).collect()
}
fn lines_of(a: &Azure, root: &Path, build: &str, log: &str) -> Vec<String> {
    match invoke(a, root, "build", "logs", &[("buildId", build), ("logId", log)], "GET", None) {
        Ok(Value::String(s)) => clean(&s.lines().map(String::from).collect::<Vec<_>>()),
        Ok(v) => clean(&strings(v.at("value"))),
        Err(_) => vec![],
    }
}
/// The failing job's name and step, and the end of its log, for build `build`: what is said of a failed check or run.
pub fn tail(a: &Azure, root: &Path, build: &str) -> String {
    let url = a.build_url(build);
    let recs = timeline(a, root, build);
    let Some((job, step)) = failed(&recs).into_iter().next() else { return format!("Azure Pipelines named no failed job in its timeline: {url}") };
    let at = if job.is_empty() || job == step.name { format!("job {}", step.name) } else { format!("job {job}, step {}", step.name) };
    let lines = if step.log.is_empty() { vec![] } else { lines_of(a, root, build, &step.log) };
    if lines.is_empty() { return format!("Failed in {at}. Its log could not be read here: {url}"); }
    format!("Failed in {at}. The end of its log ({url}):\n{}", lines[lines.len().saturating_sub(LOG_LINES)..].join("\n"))
}

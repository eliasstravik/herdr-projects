//! The base branch's pipeline runs for a merge commit on Azure Pipelines (`az pipelines runs list`), the ones
//! `merge.pipeline` names, and what they say together: still to start, running (how far: `Main pipeline running · 2 of
//! 3 stages`, `Deploying staging`), passed, or failed with the failing job's name and the end of its log.
use super::cli::json_of;
use super::logs::{tail, timeline};
use super::url::Azure;
use crate::json::*;
use std::path::Path;

/// One run of a pipeline (a build).
#[derive(Clone, Debug, Default)]
pub struct Run { pub id: String, pub pipeline: String, pub pipeline_id: String, pub status: String, pub result: String, pub commit: String }
pub fn run_of(b: &Value) -> Run {
    Run { id: b.t("id"), pipeline: b.at("definition").t("name"), pipeline_id: b.at("definition").t("id"), status: b.t("status").to_lowercase(), result: b.t("result").to_lowercase(), commit: b.t("sourceVersion") }
}
/// The runs of `base` for `commit`, newest first: of this repo's pipelines only, as the project's list of runs holds
/// every repo's. Err: what az said.
pub fn of_commit(a: &Azure, root: &Path, base: &str, commit: &str) -> Result<Vec<Run>, String> {
    let ids = pipelines(a, root).unwrap_or_default();
    let branch = format!("refs/heads/{base}");
    let mut args = vec!["pipelines", "runs", "list", "--project", &a.project, "--branch", &branch, "--top", "50", "--query-order", "QueueTimeDesc"];
    if !ids.is_empty() { args.push("--pipeline-ids"); args.extend(ids.iter().map(String::as_str)); }
    let v = json_of(a, root, &args, 60000)?;
    Ok(v.as_array().map(|l| l.iter().map(run_of).filter(|r| r.commit.eq_ignore_ascii_case(commit)).collect()).unwrap_or_default())
}
/// The ids of the repo's pipelines. None when az cannot say.
pub fn pipelines(a: &Azure, root: &Path) -> Option<Vec<String>> {
    let v = json_of(a, root, &["pipelines", "list", "--project", &a.project, "--repository", &a.repo, "--repository-type", "tfsgit"], 60000).ok()?;
    Some(v.as_array().map(|l| l.iter().map(|p| p.t("id")).filter(|i| !i.is_empty()).collect()).unwrap_or_default())
}
/// Whether the repo has any pipeline at all. None when az cannot say.
pub fn any_pipeline(a: &Azure, root: &Path) -> Option<bool> { pipelines(a, root).map(|l| !l.is_empty()) }
/// `merge.pipeline` as a list: `all`, `none`, or pipelines by name or id (`CI, deploy`).
pub fn named(setting: &str) -> Vec<String> { setting.split([',', ' ']).map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect() }
/// The runs `merge.pipeline` waits for: all of them, or those of the pipelines it names. Of a pipeline run more than
/// once for the commit (run again by hand), its newest run counts.
pub fn wanted(runs: &[Run], setting: &str) -> Vec<Run> {
    let names = named(setting);
    let all = names.iter().any(|n| n.eq_ignore_ascii_case("all"));
    let mut out: Vec<Run> = vec![];
    for r in runs {
        if !all && !names.iter().any(|n| n.eq_ignore_ascii_case(&r.pipeline) || *n == r.pipeline_id) { continue; }
        if !out.iter().any(|o| o.pipeline_id == r.pipeline_id) { out.push(r.clone()); }
    }
    out
}
/// The pipelines named and not run for the commit yet.
pub fn not_started(runs: &[Run], setting: &str) -> Vec<String> {
    named(setting).into_iter().filter(|n| !n.eq_ignore_ascii_case("all") && !runs.iter().any(|r| n.eq_ignore_ascii_case(&r.pipeline) || *n == r.pipeline_id)).collect()
}
pub fn passed(r: &Run) -> bool { r.status == "completed" && (r.result == "succeeded" || r.result == "partiallysucceeded") }
pub fn failed(r: &Run) -> bool { r.status == "completed" && !passed(r) }
/// A stage of a run: its name, whether it is through, whether it is under way, and whether a deployment job of it
/// runs (a job named `Deploy…`, as a stage `Staging` with its job `Deploy` has).
#[derive(Clone, Debug, Default)]
pub struct Stage { pub name: String, pub done: bool, pub running: bool, pub deploying: bool }
pub fn stages_of(recs: &[super::logs::Record]) -> Vec<Stage> {
    // A job's stage: up its parents (job, phase, stage).
    let stage_of = |r: &super::logs::Record| { let mut at = r.parent.clone(); for _ in 0..4 { match recs.iter().find(|x| x.id == at) { Some(x) if x.kind == "Stage" => return x.id.clone(), Some(x) => at = x.parent.clone(), None => break } } String::new() };
    let deploying: Vec<String> = recs.iter().filter(|r| r.kind == "Job" && r.state == "inProgress" && r.name.to_lowercase().starts_with("deploy")).map(stage_of).collect();
    recs.iter().filter(|r| r.kind == "Stage").map(|r| Stage { done: r.state == "completed", running: r.state == "inProgress", deploying: deploying.contains(&r.id), name: r.name.clone() }).collect()
}
pub fn stages(a: &Azure, root: &Path, run: &str) -> Vec<Stage> { stages_of(&timeline(a, root, run)) }
/// How far the runs are, in the row's words: the stage that deploys (`Deploying staging`), else how many stages are
/// through (`Main pipeline running · 2 of 3 stages`), else that it runs.
pub fn progress(stages: &[Stage], runs: usize) -> String {
    let word = if runs > 1 { "Main pipelines" } else { "Main pipeline" };
    if let Some(s) = stages.iter().find(|s| s.running && s.name.to_lowercase().starts_with("deploy")) {
        let skip = if s.name.to_lowercase().starts_with("deployment") { 10 } else { 6 };
        let to = s.name[skip..].trim_start_matches(|c: char| c == '_' || c == '-' || c == ' ' || c == ':').trim();
        return if to.is_empty() { "Deploying".into() } else { format!("Deploying {to}") };
    }
    if let Some(s) = stages.iter().find(|s| s.running && s.deploying) { return format!("Deploying {}", s.name.replace('_', " ")); }
    if stages.is_empty() { return format!("{word} running"); }
    format!("{word} running · {} of {} stages", stages.iter().filter(|s| s.done).count(), stages.len())
}
/// What the runs say together.
pub enum Look { Passed(String), Running(String), Failed { why: String, detail: String }, NotStarted(Vec<String>) }
/// `runs`: those of the commit `merge.pipeline` waits for (`wanted`), `missing`: those it names not run yet.
pub fn look(a: &Azure, root: &Path, runs: &[Run], missing: &[String]) -> Look {
    if let Some(r) = runs.iter().find(|r| failed(r)) {
        let why = format!("The pipeline {} {} (run {})", r.pipeline, if r.result == "canceled" { "was cancelled" } else { "failed" }, r.id);
        return Look::Failed { detail: format!("{why}: {}\n\n{}", a.build_url(&r.id), tail(a, root, &r.id)), why };
    }
    if runs.is_empty() { return Look::NotStarted(missing.to_vec()); }
    let going: Vec<&Run> = runs.iter().filter(|r| !passed(r)).collect();
    if going.is_empty() && missing.is_empty() {
        return Look::Passed(runs.iter().map(|r| format!("{} {}", r.pipeline, if r.result == "partiallysucceeded" { "partly succeeded" } else { "passed" })).collect::<Vec<_>>().join(", "));
    }
    let all: Vec<Stage> = going.iter().flat_map(|r| stages(a, root, &r.id)).collect();
    Look::Running(if going.is_empty() { format!("Waiting for the pipeline {} to start", missing.join(", ")) } else { progress(&all, going.len()) })
}

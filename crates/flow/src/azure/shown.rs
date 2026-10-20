//! What a task shows of its pull request on Azure DevOps: one line for the task panel (its reviewers' votes and its
//! policies, kept on the task by the pull requests' watch as `prRemote`), and for `details` the pull request as Azure
//! DevOps says it now, with the main pipeline's runs of its merge commit while the task waits for them; and what a
//! task held at an approval there waits for.
use super::{host, pr, runs};
use crate::json::*;
use crate::store::repos::folder_of;

/// What the task waits for you to approve in Azure DevOps (a build of its checks, or its main pipeline, held at an
/// approval: land/checks.rs, land/main_pipeline.rs), in words. None: nothing is held there.
pub fn held(t: &Obj) -> Option<String> {
    if t.s("stage") != "merging" { return None; }
    [t.at("checksWait").s("approval"), t.at("pipelineWait").s("approval")].into_iter().find(|s| !s.is_empty()).map(String::from)
}
/// What yes says to such a task: the approval is Azure DevOps' to give, not the board's.
pub fn held_answer(id: &str, why: &str) -> String {
    let what = why.strip_prefix("Waiting for your approval in Azure DevOps: ").unwrap_or(why);
    format!("{id} waits for your approval in Azure DevOps ({what}), not for a yes here: approve it there and the board goes on by itself.")
}
/// A reviewer's vote in words.
pub fn vote(v: i64) -> &'static str { match v { 10 => "approved", 5 => "approved with suggestions", -5 => "waiting for the author", -10 => "rejected", _ => "no vote" } }
/// A check's result in words: passed, failed, running.
fn result(c: &Value) -> &'static str {
    let w = crate::land::checks::word(c);
    if crate::flow::git::CHECK_BAD.contains(&w.as_str()) { "failed" } else if crate::flow::git::CHECK_GOOD.contains(&w.as_str()) { "passed" } else { "running" }
}
/// The panel's line: `Reviewers: Ann approved · Policies: CI build passed`. Empty when there is nothing to say.
pub fn summary(v: &Value) -> String {
    let reviewers: Vec<String> = v.arr("reviewers").iter().map(|r| format!("{} {}{}", r.s("name"), vote(r.i("vote")), if r.b("required") { " (required)" } else { "" })).collect();
    let policies: Vec<String> = v.arr("statusCheckRollup").iter().map(|c| format!("{} {}", crate::land::checks::name(c), result(c))).collect();
    let mut parts = vec![];
    if !reviewers.is_empty() { parts.push(format!("Reviewers: {}", reviewers.join(", "))); }
    if !policies.is_empty() { parts.push(format!("Policies: {}", policies.join(", "))); }
    if v.s("reviewDecision") == "REVIEW_REQUIRED" { parts.push("a review is still required".into()); }
    parts.join(" · ")
}
/// For `details`: its pull request on Azure DevOps (status, draft, reviewers, policies) and the main pipeline's runs.
/// None for a task with no pull request there.
pub fn details(t: &Obj) -> Option<Value> {
    if !pr::is_azure(t.s("pr")) { return None; }
    let root = folder_of(t.s("id"));
    let a = host(&root)?;
    let mut out = obj(json!({ "pr": t.at("pr"), "summary": t.at("prRemote") }));
    match pr::view(&a, &root, t.s("pr"), "statusCheckRollup") {
        Ok(v) => { out.put("status", v.at("state").clone()); out.put("draft", v.at("isDraft").clone()); out.put("reviewDecision", v.at("reviewDecision").clone()); out.put("reviewers", v.at("reviewers").clone());
            out.put("policies", v.arr("statusCheckRollup").iter().map(|c| json!({ "name": crate::land::checks::name(c), "result": result(c), "url": c.at("detailsUrl") })).collect::<Vec<_>>()); out.put("summary", summary(&v)); }
        Err(e) => out.put("unread", e),
    }
    let w = if t.b("pipelineWait") { t.at("pipelineWait") } else { t.at("pipelineFailed") };
    if w.is_object() {
        let list = runs::of_commit(&a, &root, w.s("base"), w.s("commit")).unwrap_or_default();
        let wanted = runs::wanted(&list, &crate::store::settings::lookup::setting_text("merge.pipeline", Some(t)));
        out.put("pipeline", json!({ "commit": w.at("commit"), "base": w.at("base"), "waiting": t.b("pipelineWait"),
            "runs": wanted.iter().map(|r| json!({ "id": r.id, "pipeline": r.pipeline, "status": r.status, "result": r.result, "url": a.build_url(&r.id) })).collect::<Vec<_>>() }));
    }
    Some(Value::Object(out))
}

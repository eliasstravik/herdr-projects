//! A research-only task at the finish line: a change in the repo goes back to its agent, and every task's report is kept.
use super::shared::{fail, FailHow};
use crate::clock::now;
use crate::err::Res;
use crate::flow;
use crate::json::*;
use crate::research as r;
use crate::runner::Guard;
use crate::store::failures::{bound_detail, failure_patch};
use crate::store::repos::{cwd_of, is_repo};
use crate::store::settings::lookup::setting;
use crate::store::tasks::Task;

/// Is this task research only, and did it keep to that? None: an ordinary task. Some(true): research only and nothing
/// changed. Some(false) ("back"): it changed the repo: nothing is committed, pushed, opened or merged, and the task goes
/// back to its agent with what changed, as a round of the review loop; after `review.rounds` of them it waits for you.
pub fn research_only(id: &str, g: &mut Guard, tree: &str) -> Res<Option<bool>> {
    let mut t = g.check()?;
    if !r::is_research(&t) { return Ok(None); }
    if t.b("mergeIntent") { t = g.set(|t| t.put("mergeIntent", Value::Null))?; } // a merge approved before the task became research only
    let cwd = cwd_of(&t);
    let dir = if !tree.is_empty() { tree.to_string() } else if is_repo(&cwd) { cwd.display().to_string() } else { String::new() };
    let list = if dir.is_empty() { vec![] } else { r::changes_of(&t, std::path::Path::new(&dir)).unwrap_or_default() };
    if list.is_empty() { return Ok(Some(true)); }
    let rounds = t.i("reviewRounds");
    if rounds >= setting("review.rounds", Some(&t)).as_i64().unwrap_or(3) { fail(g, r::changed_failure(&t, &dir, &list, Some(rounds)), FailHow::default())?; return Ok(Some(false)); }
    let mut failure = failure_patch(&r::changed_failure(&t, &dir, &list, None)).o("failure").cloned().unwrap_or_default();
    let words = format!("This task is research only, and you changed the repo ({}), so it is not done yet.\n\n{}\n\n{}\n\nThen finish again.", r::changed_words(&list), bound_detail(failure.s("detail")), failure.s("next"));
    failure.put("sentAt", now());
    flow::send::send_own(id, &words, "research", false, obj(json!({ "reviewRounds": rounds + 1, "failure": failure })), 0)?;
    Ok(Some(false))
}
/// The report the agent left in its folder among the reports, for any task: given its link.
pub fn kept_report(g: &mut Guard) -> Res<Task> {
    let t = g.check()?;
    let report = r::keep_report(&t, || crate::review_page::report_base(t.s("id"))).unwrap_or_else(|| t.at("report").clone());
    if report.is_null() && !t.b("report") { return Ok(t); }
    if report.is_object() && t.at("report").is_object() && report.at("file") == t.at("report").at("file") && report.at("link") == t.at("report").at("link") { return Ok(t); }
    g.set(|t| t.put("report", report.clone()))
}

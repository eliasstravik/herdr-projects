//! A research-only task at the finish line: a change in the repo goes back to its agent, and a reviewed report goes to the Mission Control Orchestrator for a human-facing conclusion.
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
    let report = r::keep_report(&t, || crate::review_page::artifact_base(t.s("id"))).unwrap_or_else(|| t.at("report").clone());
    if report.is_null() && !t.b("report") { return Ok(t); }
    if report.is_object() && t.at("report").is_object() && report.at("file") == t.at("report").at("file") && report.at("link") == t.at("report").at("link") { return Ok(t); }
    g.set(|t| t.put("report", report.clone()))
}

/// After a Mission Control research task has passed review, ask its visible coordinator for a concise conclusion and an explicit implementation gate.
pub fn mission_result(t: &Task, review: Option<&Value>) {
    if t.s("stage") != "done" || !r::is_research(t) || !t.at("report").is_object() || t.s("missionImplementationApproval") != "" { return; }
    let socket = t.at("link").s("socket");
    if socket.is_empty() || !crate::mission_control::is_active(&socket) { return; }
    let report = &t.at("report");
    let file = r::report_folder(t).join(report.s("file"));
    let review_result = review.map(|r| crate::gate::sentence(r, "accept")).unwrap_or_else(|| "Der Recherche-Task wurde abgeschlossen.".into());
    let prompt = format!("[MISSION-CONTROL-RESEARCH-RESULT]\nDer Recherche-Task {} („{}“) ist abgeschlossen und geprüft. Review: {}. Berichtdatei: {}. Berichtlink: {}. Lies den vollständigen Bericht und antworte dem Menschen auf Deutsch mit: (1) einem klaren Fazit in wenigen Sätzen, (2) den wichtigsten konkreten Befunden, (3) einem kurzen Vorschlag für die Umsetzung und (4) dem Berichtlink. Sage deutlich, dass bislang nichts umgesetzt wurde. Frage ausdrücklich: ‚Soll ich diese Umsetzung starten? Antworte in Mission Control mit „Go für Umsetzung“.‘ Lege bis zu dieser ausdrücklichen Freigabe keine Umsetzungstasks an, delegiere keine Codeänderung und verändere keine Dateien.", t.s("id"), t.s("title"), review_result, file.display(), report.s("link"));
    let co = crate::mission_control::project_coordinator(&crate::store::repos::folder_of(t.s("id")), &socket).unwrap_or_else(|_| crate::coordinator::co::Co::top(&socket));
    let folder = crate::store::repos::folder_of(t.s("id"));
    if crate::coordinator::requests::send(&folder, &prompt, false, t.s("id"), &co).is_ok() {
        let _ = crate::store::tasks::patch(t.s("id"), |task| task.put("missionImplementationApproval", "pending"));
    }
}

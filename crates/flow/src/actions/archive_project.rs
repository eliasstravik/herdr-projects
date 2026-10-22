//! Archiving a whole project, and restoring it: its row and its tasks leave the board, its coordinator stops, its
//! automations are turned off, and it waits in the archive; restored, its tasks come back as they were. A task at work
//! or waiting for you is stopped first. Nothing on disk or on GitHub is deleted (docs/board.md, "Archiving a project").
use super::archive::{archive, restore};
use super::archive_done::repo_named;
use super::talk::stop;
use crate::err::Res;
use crate::json::*;
use crate::runner::runner_alive;
use crate::shown::states::group_of;
use crate::shown::words::repo_label;
use crate::store::archived_repos::{archived_repos, change, is_archived};
use crate::store::files::SLOT_STAGES;

/// What archiving a repo does to its tasks, before it is done: how many of each kind it holds (`working`, `waiting`,
/// `to do`, `done`), and the one that holds it up (a merge under way), if any.
pub struct Plan { pub repo: String, pub kinds: Vec<(&'static str, usize)>, pub busy: Option<String> }
impl Plan {
    pub fn tasks(&self) -> usize { self.kinds.iter().map(|(_, n)| n).sum() }
    /// Whether to ask first: a task works, waits or is to do.
    pub fn asks(&self) -> bool { self.kinds.iter().any(|(k, n)| *k != "done" && *n > 0) }
    /// What happens to its tasks, in words: `1 working, 1 waiting and 4 to do: all 6 go to the archive with it, as they
    /// are, and come back with it; an agent on one stops.`
    pub fn says(&self) -> String {
        let n = self.tasks();
        if n == 0 { return "It has no tasks.".into(); }
        let kinds: Vec<String> = self.kinds.iter().map(|(k, n)| format!("{n} {k}")).collect();
        let kinds = match kinds.split_last() { Some((last, [])) => last.clone(), Some((last, rest)) => format!("{} and {last}", rest.join(", ")), None => String::new() };
        let all = if n == 1 { "it goes to the archive with it, as it is, and comes back with it".to_string() } else { format!("all {n} go to the archive with it, as they are, and come back with it") };
        let agents = if self.kinds.iter().any(|(k, _)| ["working", "waiting"].contains(k)) { "; an agent on one stops" } else { "" };
        format!("{kinds}: {all}{agents}.")
    }
}
/// A task's kind as the question counts it: `working`, `waiting` (for you, or for a yes), `to do` or `done`.
fn kind(t: &Obj) -> &'static str { match group_of(t) { "done" => "done", "working" => "working", "to do" => "to do", _ => "waiting" } }
/// The repo's tasks on the board (not archived).
fn rows_of(repo: &str) -> Vec<Obj> { crate::flow::live::board(false).into_iter().filter(|t| !t.b("archivedAt") && t.s("id").split('/').next() == Some(repo)).collect() }
/// A merge under way: the archive waits for it (the task alone would refuse too).
fn merging(t: &Obj) -> bool { ["pushing", "merging"].contains(&t.s("stage")) || t.b("autoMerge") || t.b("mergeIntent") || (t.at("runner").s("job") == "cleanup" && runner_alive(t)) }
/// What archiving `repo` would do to these of its tasks (the board's rows, as it last read them), without doing it.
pub fn plan_of<'a>(repo: &str, rows: impl IntoIterator<Item = &'a Obj>) -> Plan {
    let rows: Vec<&Obj> = rows.into_iter().filter(|t| !t.b("archivedAt")).collect();
    let kinds = ["working", "waiting", "to do", "done"].into_iter().map(|k| (k, rows.iter().filter(|t| kind(t) == k).count())).filter(|(_, n)| *n > 0).collect();
    let busy = rows.iter().find(|t| merging(t)).map(|t| t.t("id"));
    Plan { repo: repo.into(), kinds, busy }
}
/// What archiving `want` (its name on the board or its key) would do, without doing it.
pub fn plan(want: &str) -> Res<Plan> { let repo = repo_named(want)?; let rows = rows_of(&repo); Ok(plan_of(&repo, &rows)) }
/// Archive the project: stop what works or waits, archive every task, stop its coordinator, turn its automations off.
/// Its words, and the ids it archived.
pub fn archive_project(want: &str) -> Res<(String, Vec<String>)> {
    let p = plan(want)?;
    let (repo, name) = (p.repo.clone(), repo_label(&p.repo));
    if is_archived(&repo) { crate::fail!("{name} is archived already."); }
    if let Some(id) = &p.busy { crate::fail!("{id} is merging: {name} can be archived once that is done."); }
    let (mut tasks, mut stopped, mut failed) = (vec![], vec![], vec![]);
    for t in rows_of(&repo) {
        let id = t.t("id");
        let live = SLOT_STAGES.contains(&t.s("stage")) || ["queued", "attempts"].contains(&t.s("stage")) || runner_alive(&t);
        if live { match stop(&id) { Ok(_) => stopped.push(id.clone()), Err(e) => { failed.push(format!("{id} ({})", e.msg.trim_end_matches('.'))); continue; } } }
        match archive(&id) { Ok(_) => tasks.push(id), Err(e) => failed.push(format!("{id} ({})", e.msg.trim_end_matches('.'))) }
    }
    stop_coordinator(&repo);
    let mut autos = vec![];
    for a in crate::store::automations::automations().into_iter().filter(|a| a.s("repo") == repo && a.b("on")) {
        if crate::automations::list::turn(a.s("id"), false).is_ok() {
            let _ = crate::store::automations::change_automation(a.s("id"), |x| x.put("offWhy", "its project is archived"));
            autos.push(a.t("id"));
        }
    }
    let rec = json!({ "at": crate::clock::now(), "tasks": tasks, "stopped": stopped, "automations": autos });
    change(|all| { all.insert(repo.clone(), rec); })?;
    let n = tasks.len();
    let mut said = format!("Archived {name}{}.", match n { 0 => String::new(), 1 => " and its task".into(), n => format!(" and its {n} tasks") });
    if !stopped.is_empty() { said += &format!(" Stopped first: {}.", stopped.join(", ")); }
    if !autos.is_empty() { said += &format!(" {} turned off.", if autos.len() == 1 { "Its automation is".to_string() } else { format!("Its {} automations are", autos.len()) }); }
    if !failed.is_empty() { said += &format!(" Still on the board: {}.", failed.join("; ")); }
    Ok((said, tasks))
}
/// Its project coordinator stops, and its autopilot goes off: nothing of it runs while it is archived.
fn stop_coordinator(repo: &str) {
    let co = crate::coordinator::co::Co::project(&crate::herdr::cli::current_socket(), repo);
    if crate::autopilot::record::is_on(&co) { let _ = crate::autopilot::switch::off(&co); }
    if crate::coordinator::state::get(co.clone()).is_some_and(|c| c.b("run") && !c.b("stopped")) { let _ = crate::coordinator::requests::stop(co); }
}
/// Bring the project back: the tasks archived with it, as they were (one stopped then stays stopped), and the
/// automations turned off with it. Nothing starts.
pub fn restore_project(want: &str) -> Res<(String, Vec<String>)> {
    let all = archived_repos();
    let w = want.trim().to_lowercase();
    let repo = match all.keys().find(|k| k.to_lowercase() == w) { Some(r) => r.clone(), None => repo_named(want)? };
    let name = repo_label(&repo);
    let Some(rec) = all.get(&repo).and_then(|v| v.as_object()).cloned() else { crate::fail!("{name} is not archived.") };
    let mut back = vec![];
    for id in strings(rec.at("tasks")) { if restore(&id).is_ok() { back.push(id); } }
    let autos: Vec<String> = strings(rec.at("automations")).into_iter().filter(|id| crate::automations::list::turn(id, true).is_ok()).collect();
    change(|all| { all.remove(&repo); })?;
    let n = back.len();
    let mut said = format!("Restored {name}{}.", match n { 0 => String::new(), 1 => " and its task".into(), n => format!(" and its {n} tasks") });
    let stopped: Vec<String> = strings(rec.at("stopped")).into_iter().filter(|id| back.contains(id)).collect();
    if !stopped.is_empty() { said += &format!(" Stopped when it was archived, and still: {}.", stopped.join(", ")); }
    if !autos.is_empty() { said += &format!(" {} on again.", if autos.len() == 1 { "Its automation is".to_string() } else { format!("Its {} automations are", autos.len()) }); }
    Ok((said, back))
}
fn strings(v: &Value) -> Vec<String> { v.as_array().map(|a| a.iter().map(text).collect()).unwrap_or_default() }

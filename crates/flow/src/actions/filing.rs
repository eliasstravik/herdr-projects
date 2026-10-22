//! Adding and editing a task: add, rename, describe, pin, and a report of where it stands.
use super::shared::is_agent_stage;
use super::talk::stop;
use crate::err::Res;
use crate::flow;
use crate::json::*;
use crate::research::RESEARCH;
use crate::runner::{runner_alive, spawn_runner};
use crate::store::failures::failure_patch;
use crate::store::files::KINDS;
use crate::store::repos::folder_for;
use crate::store::settings::{change::set_many, lookup::Scope};
use crate::store::tasks::{assert_unarchived, create, find, get, minutes, one_line, remove, update, Change, Task};
use crate::store::titles::titled;
use std::path::Path;

#[derive(Default)]
pub struct AddOpts { pub description: String, pub repo: String, pub research: Option<bool>, pub settings: Vec<(String, Value)>, pub from: String }
/// `settings`: work settings of the task's own, from the start. `research`: the short form of research.only. `from`:
/// the task whose agent adds it: a task an automation made hands that on, so the new task never fires it.
pub fn add(folder: &Path, title: &str, o: AddOpts) -> Res<Task> {
    let mut own = o.settings.clone();
    if let Some(r) = o.research { own.push((RESEARCH.into(), json!(r))); }
    let t = create(&folder_for(&o.repo, folder)?, title, &o.description, None)?;
    // A refused setting adds nothing: the task is taken away again.
    if !own.is_empty() { if let Err(e) = set_many(&own, &Scope::task(&t)) { remove(t.s("id")); return Err(e); } }
    if let Some(a) = crate::automations::told::inherited(&o.from) { update(t.s("id"), |t| { t.put("automation", a); Ok(Change::Write) })?; }
    get(t.s("id")).ok_or_else(|| "the task is gone".into())
}
/// A new title follows the same limit as a first one: too long, it is shortened and its full text opens the description.
fn retitle(t: &mut Task, title: &str) {
    let now = titled(title, t.s("description"));
    if !now.title.is_empty() { t.put("title", now.title); t.put("description", now.description); t.del("subject"); }
}
pub fn rename(id: &str, title: &str) -> Res<Task> { let cur = find(id)?; update(cur.s("id"), |t| { assert_unarchived(t)?; retitle(t, title); Ok(Change::Write) }) }
pub fn describe(id: &str, text: &str) -> Res<Task> { let cur = find(id)?; update(cur.s("id"), |t| { assert_unarchived(t)?; t.put("description", text.replace('\r', "").trim()); Ok(Change::Write) }) }
pub fn pin(id: &str) -> Res<Task> { let cur = find(id)?; update(cur.s("id"), |t| { assert_unarchived(t)?; let p = !t.b("pinned"); t.put("pinned", p); Ok(Change::Write) }) }

#[derive(Default)]
pub struct Report { pub state: String, pub kind: String, pub msg: String, pub title: String, pub progress: Option<Value>, pub eta: Option<Value>, pub as_agent: bool }
fn clamp_pct(p: &Value) -> i64 { (number(p).round() as i64).clamp(0, 100) }
/// A report: the human (or any agent) says where a task stands. A delegated task's own agent is told to say only that
/// it is blocked (the finish line marks its work done); `as_agent` marks that the turn after its own "blocked" is not a finish.
pub fn report(id: &str, r: Report) -> Res<Task> {
    let cur = find(id)?;
    if !r.title.is_empty() && r.state.is_empty() { return rename(cur.s("id"), &r.title); }
    if !["idle", "working", "blocked", "done", "error"].contains(&r.state.as_str()) { crate::fail!("state must be one of: idle, working, blocked, done, error"); }
    // "Mark it idle" on a task whose agent is working means "stop it".
    if r.state == "idle" && flow::task::is_agent_task(&cur) && is_agent_stage(cur.s("stage")) { return stop(cur.s("id")).map(|s| s.task); }
    let delegated = flow::task::is_agent_task(&cur) && (is_agent_stage(cur.s("stage")) || ["asks", "nosummary", "approval", "reviewneeds", "failed", "plan"].contains(&cur.s("stage")));
    let mark = if r.state == "done" { flow::spaces::tidy_mark(&cur, "merged", "") } else { None };
    let t = update(cur.s("id"), |t| {
        assert_unarchived(t)?;
        if !r.title.is_empty() { retitle(t, &r.title); }
        if r.state == "blocked" {
            t.put("stage", "asks");
            t.put("kind", if KINDS.contains(&r.kind.as_str()) { r.kind.clone() } else { "question".into() });
            let m = one_line(&r.msg, 2048);
            t.put("msg", if m.is_empty() { "Needs you".into() } else { m });
            t.put("turnOpen", false);
            // The end of the turn it says this in is not a finish (finish/waiting.rs).
            if r.as_agent { let now = crate::clock::now(); t.put("selfBlocked", now); t.put("blocked", json!({ "at": now, "kind": t.at("kind"), "msg": t.at("msg") })); }
            return Ok(Change::Write);
        }
        if delegated && runner_alive(t) { crate::fail!("{} is in the middle of a workflow step. Stop it first.", t.s("id")); }
        t.del("kind");
        t.put("msg", one_line(&r.msg, 2048));
        if r.state != "error" { t.put("failure", Value::Null); }
        let agent = flow::task::is_agent_task(t);
        match r.state.as_str() {
            "idle" => { t.put("stage", if agent { "stopped" } else { "todo" }); if !agent { t.put("app", Value::Null); } }
            "working" => {
                if agent { crate::fail!("{} has an agent. Reply to it, or delegate it again, to make it work.", t.s("id")); }
                t.put("stage", "progress"); let owner = t.at("owner").clone(); t.put("app", owner); t.put("since", crate::clock::now());
                if r.progress.is_some() || r.eta.is_some() { // one given alone keeps the other
                    let p = r.progress.as_ref().map(|p| json!(clamp_pct(p))).unwrap_or_else(|| t.at("watch").at("progress").clone());
                    let e = r.eta.as_ref().map(|e| minutes(e).map(|m| json!(m)).unwrap_or(Value::Null)).unwrap_or_else(|| t.at("watch").at("eta").clone());
                    t.put("watch", json!({ "progress": p, "eta": e }));
                }
            }
            "done" => { t.put("stage", "done"); t.put("mergeIntent", Value::Null); t.put("tidy", if agent { mark.clone().unwrap_or(Value::Null) } else { Value::Null }); }
            _ => { t.put("stage", "failed"); let d = one_line(&r.msg, 2048); for (k, v) in failure_patch(&obj(json!({ "stage": "agent", "title": "Marked as failed", "detail": if d.is_empty() { "No details given.".into() } else { d }, "auth": false }))) { t.insert(k, v); } }
        }
        Ok(Change::Write)
    })?;
    // Called done by you: its space leaves herdr's sidebar as after a merge. Unmerged work stays on its branch.
    if r.state == "done" && flow::task::is_agent_task(&t) { flow::sidebar::tell_sidebar(&t); spawn_runner("cleanup", t.s("id"), &["--merged"]); }
    Ok(t)
}
/// The human's own tasks: "I am on it".
pub fn start_human(id: &str) -> Res<Task> { report(id, Report { state: "working".into(), ..Default::default() }) }

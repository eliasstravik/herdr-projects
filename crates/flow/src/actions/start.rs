//! Putting an agent on a task: the request, and starting queued tasks while slots are free.
use super::shared::is_agent_stage;
use crate::after::{self, Wait};
use crate::clock::now;
use crate::err::Res;
use crate::flow;
use crate::herdr::cli::current_socket;
use crate::json::*;
use crate::research::{turn_refusal, RESEARCH};
use crate::runner::{runner_alive, spawn_runner};
use crate::store::attempts::attempts_of;
use crate::store::files::{with_lock_wait, SLOT_STAGES};
use crate::store::handovers::fresh_list;
use crate::store::settings::change::checked;
use crate::store::settings::lookup::{setting, setting_text, Scope};
use crate::store::settings::values::agent_of;
use crate::store::tasks::{assert_unarchived, find, get, list, update, Change, ListOpts, Task};

/// What a task that is started afresh forgets: everything its last round left on it. One name per line: a new one
/// goes beside the names it belongs with, never at the end.
const FRESH: &[&str] = &[
    "startedOnce",
    "startedAgentAt",
    "cleaned",
    "worktree",
    "branch",
    "from",
    "baseSha",
    "pr",
    "prMerged",
    "prState",
    "prHead",
    "prSeen",
    "prFeedbackSeen",
    "prFailure",
    "doneWhy",
    "review",
    "reviewedHead",
    "syncedTo",
    "syncSeen",
    "syncBranch",
    "syncRepairs",
    "conflict",
    "syncRetryAt",
    "syncBlocked",
    "handled",
    "session",
    "said",
    "summary",
    "stat",
    "kept",
    "tidy",
    "exitedAt",
    "repoFile",
    "repoRefused",
    "researchRun",
    "researchTurned",
    "attempts",
    "picked",
    "as",
    "comparison",
    "comparing",
    "checkedHead",
    "checkedWith",
    "spaceDir",
    "plan",
];

/// Records the request (stage "queued") and lets the housekeeping pass start it when a slot is free. `asked`: work
/// settings for this task (research.only among them): all checked before anything is saved.
pub fn delegate(id: &str, asked: Vec<(String, Value)>) -> Res<Task> { delegate_with(id, asked, Wait::default()) }
/// The same with what it waits for (after.rs): a task whose tasks are not all done waits in stage "after" instead,
/// and one that waits already is only started early with `now`.
pub fn delegate_with(id: &str, asked: Vec<(String, Value)>, w: Wait) -> Res<Task> {
    let cur = find(id)?;
    if cur.s("stage") == after::STAGE && !w.now && w.after.is_none() && asked.is_empty() { crate::fail!("{}", after::still_waits(&cur)); }
    let deps = match &w.after { Some(r) => Some(after::checked(Some(cur.s("id")), r)?), None => None };
    let asked: Vec<(String, Value)> = asked.into_iter().filter(|(_, v)| !(v.is_null() || *v == json!(""))).collect();
    let values = checked(&asked, &Scope::task(&cur))?;
    let socket = current_socket();
    let mission_pane = if crate::mission_control::is_active(&socket) {
        crate::mission_control::pane_for_task(&cur, &socket)
    } else {
        crate::mission_layout::anchor(&socket).unwrap_or_else(|| if crate::herdr::cli::in_herdr() { crate::util::env_val("HERDR_PANE_ID").unwrap_or_default() } else { String::new() })
    };
    let mut stale: Vec<Task> = vec![];
    let mut old: Option<(String, String)> = None; // the last round's space, when its clean-up had not closed it yet
    let t = update(cur.s("id"), |t| {
        assert_unarchived(t)?;
        let id = t.t("id");
        if t.s("stage") == "attempts" { crate::fail!("{id} already has its attempts working on it."); }
        if t.s("stage") == "pick" { crate::fail!("{id} waits for you to pick one of its attempts: herdr-projects pick {id} <letter>."); }
        if is_agent_stage(t.s("stage")) || runner_alive(t) { crate::fail!("{id} already has an agent working on it."); }
        if t.b("autoMerge") { crate::fail!("{id} is waiting for an automatic merge. Reply to its agent to cancel that first."); }
        if t.at("link").b("socket") && t.at("link").s("socket") != socket { crate::fail!("{id} lives in another herdr session. Open the board there."); }
        if !values.is_empty() { let s = t.obj_mut("settings"); for (k, v) in &values { s.insert(k.clone(), v.clone()); } }
        if values.get("merge.by") == Some(&json!("itself")) { t.put("requireApproval", false); }
        let owns = |t: &Task| (t.at("link").b("workspace") && t.at("link").b("ownsWorkspace")).then(|| (t.at("link").t("socket"), t.at("link").t("workspace")));
        // Work that already landed and was cleaned up starts a new round in a fresh worktree, never in your checkout.
        if flow::git::cleaned_up(t) { old = owns(t); for k in FRESH { t.del(k); } t.put("link", json!({})); }
        t.put("on", Value::Null);
        fresh_list(t); // a new start is `worker.agent` again, with its whole fallback list before it
        // Attempts of an earlier round that none was chosen from: it starts over.
        if !t.arr("attempts").is_empty() && !t.b("picked") {
            stale = attempts_of(t);
            for k in ["attempts", "comparison", "comparing", "spaceDir"] { t.del(k); }
            if let Some(o) = owns(t) { old = Some(o); }
            let s = t.at("link").at("socket").clone();
            t.put("link", json!({ "socket": s }));
        }
        // Research only or not is decided when the agent starts, not here: until then the task follows the setting.
        let why = turn_refusal(t, setting(RESEARCH, Some(t)) == json!(true));
        if !why.is_empty() { crate::fail!("{why} To start it as it was: herdr-projects set {RESEARCH} false --task {id}"); }
        t.put("researchOpen", true);
        if crate::mission_control::is_active(&socket) {
            let pick = crate::mission_control::worker_choice(t, &socket)?;
            t.obj_mut("settings").put("worker.agent", pick);
        }
        let agent = agent_of(&setting_text("worker.agent", Some(t))).kind;
        t.put("agent", agent.clone()); t.put("app", agent);
        t.put("stage", "queued"); t.put("queuedAt", now()); t.put("msg", ""); t.del("kind"); t.put("since", now()); t.put("watch", Value::Null);
        t.obj_mut("link").put("socket", socket.clone());
        if !mission_pane.is_empty() { t.obj_mut("link").put("missionPane", mission_pane.clone()); }
        let ra = t.b("requireApproval") || t.at("review").is_object();
        t.put("mergeIntent", Value::Null); t.put("requireApproval", ra); t.put("failure", Value::Null); t.put("expectedWake", Value::Null); t.put("turnOpen", false); t.put("waiting", Value::Null);
        if t.at("review").is_object() { t.obj_mut("review").put("stale", true); }
        for k in ["checkFails", "reviewRounds", "prRounds", "syncRepairs"] { t.put(k, 0); }
        if let Some(d) = &deps { t.put("after", d.clone()); }
        after::on_delegate(t, w.now);
        Ok(Change::Write)
    })?;
    for c in &stale { let _ = crate::attempts::adopt::discard(&t, c); }
    if let Some((s, w)) = old { flow::spaces::close_later(&s, &w); }
    flow::sidebar::tell_sidebar(&t);
    start_queued(&socket);
    Ok(get(t.s("id")).unwrap_or(t))
}
/// Start queued tasks while slots are free. Counting and claiming happen under one lock, so two passes cannot both
/// take the last slot. A turn the human started in an agent's terminal counts toward the total.
pub fn start_queued(socket: &str) -> Vec<String> {
    // A task done several times at once is split into its attempts first: each then waits for a slot of its own.
    for w in list(ListOpts::default()) { if w.s("stage") == "queued" && w.at("link").s("socket") == socket { let _ = crate::attempts::fan_out::fan_out(w.s("id")); } }
    let mut started = vec![];
    let _ = with_lock_wait("slots", 5000, || {
        let all = list(ListOpts { attempts: true, ..Default::default() });
        let mut busy = all.iter().filter(|t| SLOT_STAGES.contains(&t.s("stage")) && !t.b("autoMerge")).count() as i64; // a remembered auto-merge runs nothing
        let waiting = crate::shown::queue::order(all.iter(), socket); // the order the board says (Q#1, Q#2 …)
        let max = setting("board.agents", None).as_i64().unwrap_or(4);
        for w in waiting {
            if busy >= max { break; }
            let t = update(w.s("id"), |t| { if t.s("stage") != "queued" { return Ok(Change::Keep); } t.put("stage", "starting"); t.put("msg", ""); Ok(Change::Write) })?;
            if t.s("stage") != "starting" { continue; }
            busy += 1;
            started.push(t.t("id"));
        }
        Ok(())
    });
    for id in &started { spawn_runner("delegate", id, &[]); }
    started
}

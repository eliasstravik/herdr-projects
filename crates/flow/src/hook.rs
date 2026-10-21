//! The event hook entry. herdr runs it on pane.agent_status_changed (and a few lifecycle events). It decides, starts
//! a detached runner when a turn has ended, and exits: hook commands must not hold herdr up.
//!
//! Verified on herdr 0.9.3: plugin hooks do fire on agent status changes; HERDR_PLUGIN_EVENT_JSON carries
//! {event, data: {pane_id, workspace_id, agent_status, agent}}; an agent exiting to its shell fires
//! pane.agent_detected (released) and a status change to "unknown"; herdr does not replay missed events.
mod reconcile;
pub use reconcile::{reconcile, same_dir};

use crate::agents::{adapter::find_session, installed::read_turn_of};
use crate::clock::now;
use crate::delegate::task_line;
use crate::flow::{git::{cancel_auto_merge, pr_number}, handled_turn::{reopened, same_turn}, live::see_terminal, pane_owner::agents_pane, sidebar::tell_sidebar, task::{is_agent_task, note_stat}};
use crate::leftovers::sweep::ended;
use crate::herdr::{cli::current_socket, Herdr};
use crate::json::*;
use crate::limits::run::granted;
use crate::runner::{runner_alive, spawn_runner};
use crate::store::{failures::failure_patch, files::{data_dir, has_tasks}, handovers::fresh_list, inbox::{pending_text, take_text, untake_text}, repos::cwd_of, tasks::{get, list, update_wait, Change, ListOpts, Task}};
use crate::submit::{hand, not_started, note_sent, rest, took};
use std::path::Path;

/// Stages in which a finished turn nobody saw start (a short turn, a herdr restart, a lock wait) still counts.
const WAITING: [&str; 6] = ["asks", "nosummary", "approval", "failed", "reviewneeds", "plan"];

pub fn hook_main() {
    // The keeper does the board's periodic work with the board closed. herdr's startup and every hook event start it
    // when it is not running; an agent's own commands never do.
    crate::keeper::ensure_keeper();
    let event = std::env::var("HERDR_PLUGIN_EVENT").unwrap_or_default();
    if event == "startup" {
        crate::webui::keep::ensure_door(); // the board in the herdr web UI starts with herdr
        crate::extras::sync_skill(); crate::extras::remove_popups(); crate::extras::refresh_sidebar();
    }
    if !has_tasks() { return; } // nothing of ours anywhere: do nothing, in any herdr session
    let socket = current_socket();
    let data = serde_json::from_str::<Value>(&std::env::var("HERDR_PLUGIN_EVENT_JSON").unwrap_or_default()).map(|j| j.at("data").clone()).unwrap_or(Value::Null);
    if event == "worktree.removed" { on_gone(&socket, |t| t.at("link").at("workspace") == data.at("workspace_id") || (t.b("worktree") && t.at("worktree") == data.at("worktree").at("path"))); }
    else if event == "pane.closed" { on_gone(&socket, |t| t.at("link").at("pane") == data.at("pane_id") || t.at("link").at("pane") == data.at("pane").at("pane_id")); }
    else if data.b("pane_id") { on_pane(&socket, data.s("pane_id")); }
    crate::housekeeping::housekeeping(&socket, crate::housekeeping::Force::No);
}
pub fn log(e: &str) { let _ = std::fs::OpenOptions::new().create(true).append(true).open(data_dir().join("hook.log")).and_then(|mut f| { use std::io::Write; writeln!(f, "{} {e}", crate::clock::iso(now())) }); }
fn mine(socket: &str) -> Vec<Task> { list(ListOpts { attempts: true, ..Default::default() }).into_iter().filter(|t| is_agent_task(t) && t.at("link").s("socket") == socket).collect() }
fn on_gone(socket: &str, matches: impl Fn(&Task) -> bool) { let api = Herdr::new(Some(socket.into())); for t in mine(socket) { if matches(&t) { reconcile(&t, &api); } } }

/// The hook looks the pane up by herdr socket path plus pane id (pane ids repeat across herdr sessions) and does
/// nothing when there is no match. Reviewer panes never trigger the finish line.
pub fn on_pane(socket: &str, pane: &str) {
    let named: Vec<Task> = mine(socket).into_iter().filter(|t| t.at("link").s("pane") == pane).collect();
    let Some(first) = named.iter().find(|t| !ended(t)).or(named.first()) else { return };
    let api = Herdr::new(Some(socket.into()));
    let Ok(Some(a)) = api.agent_get(pane) else { return reconcile(first, &api) };
    // A done task's old pane id, given to another pane since (a coordinator's): what runs there is not its agent.
    let Some(t) = named.into_iter().find(|t| !ended(t) || agents_pane(t, a.s("terminal_id"))) else { return };
    // Ids can change: a match also requires the pane to be in the task's working folder. No prompt is ever sent to a
    // pane that fails this check.
    let cwd = cwd_of(&t);
    if !same_dir(a.s("cwd"), &cwd) && !same_dir(a.s("foreground_cwd"), &cwd) { return; }
    apply(t.s("id"), &a, &api);
}

fn newest_terminal(t: Option<&Task>, term: &str) -> bool {
    let Some(t) = t else { return false };
    let terms: Vec<String> = if t.at("terms").is_array() { strings(t.at("terms")) } else { [t.at("handled").t("term"), t.at("link").t("terminal")].into_iter().filter(|s| !s.is_empty()).take(1).collect() };
    terms.last().map(|s| s == term).unwrap_or(false)
}
#[derive(PartialEq)]
enum Then { Nothing, Inbox, CancelAuto, Unlabel, Recover, Finish, FinishForce }

/// One live reading of a worker pane, applied to its task under the task lock.
pub fn apply(id: &str, a: &Value, api: &Herdr) {
    let status = a.t("agent_status");
    let seq = a.i("state_change_seq");
    let comp = a.at("completion_seq").clone();
    let term = a.t("terminal_id");
    // A terminal that is not the newest one known for this task is either the pane's new terminal after herdr
    // restarted, or an old one in a reading that arrives late. Arrival order cannot tell which: herdr is asked.
    let known = get(id);
    if !term.is_empty() && !newest_terminal(known.as_ref(), &term) {
        let pane = if a.b("pane_id") { a.t("pane_id") } else { known.as_ref().map(|k| k.at("link").t("pane")).unwrap_or_default() };
        let Ok(now_a) = api.agent_get(&pane) else { return };
        if now_a.map(|n| n.t("terminal_id")) != Some(term.clone()) { return; }
    }
    let mut then = Then::Nothing; // what to do after the lock is released: never a herdr call or a spawn under it
    let Ok(t) = update_wait(id, 10000, |t| {
        if t.b("archivedAt") || !is_agent_task(t) { return Ok(Change::Keep); }
        // A reading that is not the newest is not acted on at all: one from a terminal herdr has since replaced, or an
        // older state of this terminal than the last one applied (a hook that waited for the lock). Acting on an old
        // "done" could finish a task whose agent is working by now.
        let prev = t.at("last").clone();
        let prev_term = if prev.has("term") { prev.t("term") } else { t.at("link").t("terminal") };
        let same_term = term.is_empty() || prev_term.is_empty() || prev_term == term;
        if !see_terminal(t, &term) || (!term.is_empty() && prev.is_object() && same_term && !prev_term.is_empty() && seq < prev.i("seq")) { return Ok(Change::Keep); }
        // compAt: when this count of finished turns was first seen. A later reading of the same count is not a new turn's end.
        let comp_at = if same_term && prev.is_object() && *prev.at("comp") == comp { if prev.has("compAt") { prev.at("compAt").clone() } else { prev.at("at").clone() } } else { json!(now()) };
        t.put("last", json!({ "status": status, "seq": seq, "comp": comp, "term": if term.is_empty() { Value::Null } else { json!(term) }, "at": now(), "compAt": comp_at }));
        if status == "working" { t.put("workedAt", now()); t.del("owedTurn"); } // herdr has seen it work: an idle after this is the end of a turn, not the wait before one (state.rs)
        // What the board typed in has been taken: the agent works, or has worked since.
        if t.at("sent").b("busy") { let r = rest(t.at("sent"), a); t.put("sent", r); } // typed into a running turn: only what it does once at rest says
        else if t.at("sent").is_object() && took(t.at("sent"), Some(a), None) == Some(true) { t.put("sent", Value::Null); }
        // Told "not started", and now it works: you pressed enter in its terminal yourself.
        if t.b("unsent") && status == "working" && t.s("stage") == "failed" && t.at("failure").s("stage") == "send" { t.put("unsent", Value::Null); t.put("failure", Value::Null); }
        if a.at("agent_session").b("value") { let v = a.at("agent_session").at("value").clone(); t.put("session", v); }
        let prev_same = if same_term { Some(&prev) } else { None };
        if t.s("stage") == "stopped" {
            // A late hook cannot undo the stop: `working` re-arms only with a higher seq and an idle seen in between.
            if status != "working" { if seq > t.i("stopSeq") { t.put("sawIdleAfterStop", true); } return Ok(Change::Write); }
            if !(seq > t.i("stopSeq") && t.b("sawIdleAfterStop")) { return Ok(Change::Write); }
        }
        if ["todo", "progress", "done", "queued", "starting", "setup"].contains(&t.s("stage")) { return Ok(Change::Write); }
        if status == "working" {
            if t.b("selfBlocked") { return Ok(Change::Write); } // it reported itself blocked and has not stopped yet: still that same turn
            if t.b("expectedWake") { if t.s("stage") != "working" && !runner_alive(t) { t.put("stage", "working"); } return Ok(Change::Write); } // the plugin's own turn: flaps change nothing
            // The turn is the human's: typing in the agent's own terminal re-arms the pipeline.
            if !t.b("turnOpen") || t.s("stage") != "working" {
                if !after_handled(t, a, prev_same) { return Ok(Change::Write); } // still the turn the finish line handled, not a new one
                // A review under way goes on while you chat, unless the code changes (flow/chat.rs).
                if crate::flow::chat::begin(t, "terminal", runner_alive(t)) == crate::flow::chat::Began::During { granted(t); return Ok(Change::Write); }
                then = if t.b("autoMerge") { Then::CancelAuto } else { Then::Unlabel };
                if t.b("mergeIntent") || t.s("stage") == "approval" { t.put("requireApproval", true); }
                t.put("mergeIntent", Value::Null);
                stale_review(t);
                for k in ["checkFails", "reviewRounds", "prRounds", "syncRepairs"] { t.put(k, 0); }
                fresh_list(t);
                let w = t.at("workedAt").clone();
                t.put("armedAt", w); t.put("turnOpen", true); t.put("turnBy", "typed"); granted(t);
                t.put("stage", "working"); t.put("msg", ""); t.del("kind"); t.put("watch", Value::Null); t.put("since", now()); t.put("mergeFailed", false);
                if t.at("failure").b("auth") { t.put("failure", Value::Null); } // typing after a login problem is how "fixed" is said
            } else {
                // A second `working` in the same human turn (a flap) only moves armedAt if no closing note is newer than it.
                // Only a reading from after the handled turn counts: an older one that arrives late says nothing about this
                // turn, and must not make a turn that was re-armed by mistake look real (reopened) or move when it was armed.
                if !after_handled(t, a, prev_same) { return Ok(Change::Write); }
                if !t.b("turnBy") { t.put("turnBy", "typed"); } // herdr has seen the agent work in this turn
                if !note_stat(t).map(|n| n.0 > t.f("armedAt")).unwrap_or(false) { let w = t.at("workedAt").clone(); t.put("armedAt", w); }
            }
            return Ok(Change::Write);
        }
        if status == "idle" || status == "done" {
            // Undelivered text first, whatever the previous status was: a first prompt queued behind a trust dialog is
            // delivered when the dialog is answered. A turn herdr counted before that text went out is not the turn
            // the text starts (answering the dialog counts as one): it is marked handled. herdr may count it only
            // after the text went out (`owedTurn`): the finish line then takes one count with no sign of work.
            if !pending_text(id).is_empty() {
                if !comp.is_null() { t.obj_mut("handled").put("comp", comp.clone()); }
                let owed = if comp.is_null() || comp == *prev.at("comp") { json!(if t.b("startedAgentAt") { t.i("startedAgentAt") } else { 1 }) } else { Value::Null };
                if owed.is_null() { t.del("owedTurn"); } else { t.put("owedTurn", owed); }
                then = Then::Inbox;
                return Ok(Change::Write);
            }
            if t.b("selfBlocked") && t.s("stage") == "asks" { // it said it is blocked, then stopped: that is not a finish
                let note = note_stat(t).map(|n| json!(n.0)).unwrap_or_else(|| t.at("handled").at("note").clone());
                let h = t.obj_mut("handled");
                h.put("comp", comp.clone()); h.put("seq", a.at("state_change_seq").clone()); h.put("term", a.at("terminal_id").clone()); h.put("note", note); h.put("at", now());
                t.put("selfBlocked", Value::Null); t.put("turnOpen", false); t.put("expectedWake", Value::Null);
                return Ok(Change::Write);
            }
            if t.s("stage") == "working" {
                if t.b("sent") { return Ok(Change::Write); } // text typed in is still waiting to be taken: not the end of the turn that answers it (submit.rs)
                // completion_seq is absent until an agent's first completed turn: absent means "no finished turn yet".
                if !comp.is_null() && comp != *t.at("handled").at("comp") && !runner_alive(t) { then = Then::Finish; }
                else if !runner_alive(t) && reopened(t, &comp) { then = Then::Recover; } // the runner looks again itself, and only hands the task to the human
                return Ok(Change::Write);
            }
            if not_started(t) { return Ok(Change::Write); } // told "not started": a turn that ended before the text went in changes nothing
            let waiting = WAITING.contains(&t.s("stage")) && !runner_alive(t);
            if waiting && same_turn(t) { if !comp.is_null() { t.obj_mut("handled").put("comp", comp.clone()); } return Ok(Change::Write); } // herdr counted a turn the agent's own report already finished
            if waiting && missed_turn(t, &comp) { human_turn_ended(t); then = Then::FinishForce; }
            return Ok(Change::Write);
        }
        if status == "unknown" && t.s("stage") == "working" && !runner_alive(t) {
            // herdr cannot classify this agent (normal for Codex): a closing note newer than armedAt is the stop signal.
            let note = note_stat(t);
            if note.as_ref().map(|n| n.0 > t.f("armedAt") && json!(n.0) != *t.at("handled").at("note")).unwrap_or(false) { then = Then::Finish; }
            // So is its saved conversation ending in an error: a turn the provider cut off writes no closing note.
            else if ended_in_error(t) { then = Then::Finish; }
        }
        Ok(Change::Write)
    }) else { return };
    match then {
        Then::Inbox => deliver_inbox(&t, api),
        Then::CancelAuto => {
            // A failed cancellation must be seen: the merge may still happen by itself while the agent changes the work.
            if let Err(e) = cancel_auto_merge(&t) {
                log(&e.msg);
                let _ = update_wait(id, 10000, |t| {
                    let detail = format!("{}\n\nYou typed in the agent's terminal while GitHub was set to merge {} by itself. That is still set: it may merge when its checks pass.", e.msg, t.so("pr").unwrap_or("the pull request"));
                    let next = format!("Cancel it yourself: gh pr merge {} --disable-auto", pr_number(t.s("pr")));
                    for (k, v) in failure_patch(&obj(json!({ "stage": "merge", "title": "Could not cancel the pending auto-merge", "detail": detail, "next": next, "agentCanFix": false, "auth": false }))) { t.insert(k, v); }
                    Ok(Change::Write)
                });
            }
            tell_sidebar(&t);
        }
        Then::Unlabel => tell_sidebar(&t),
        Then::Recover => { spawn_runner("recover", id, &[]); }
        Then::Finish => { spawn_runner("finish", id, &[]); }
        Then::FinishForce => { spawn_runner("finish", id, &["--force"]); }
        Then::Nothing => {}
    }
}
fn stale_review(t: &mut Task) { if t.at("review").is_object() && !t.at("review").b("stale") { t.obj_mut("review").put("stale", true); } }
/// The working agent's saved conversation ends in an error newer than the turn that was handled last and than the
/// prompt that armed this one (what the finish line itself asks before it takes an error for the turn's end).
fn ended_in_error(t: &Task) -> bool {
    let cwd = cwd_of(t);
    let session = if t.b("session") { t.t("session") } else { find_session(t.s("agent"), &cwd, if t.b("startedAgentAt") { t.i("startedAgentAt") } else { t.i("armedAt") }, &task_line(t)).unwrap_or_default() };
    read_turn_of(t.s("agent"), t.so("transcript").map(Path::new), &session, &cwd, false).map(|turn| !turn.error.is_empty() && turn.pos > t.at("handled").i("pos") && turn.pos > t.i("armedAt")).unwrap_or(false)
}
/// Is this reading from after the handled turn? A `working` that is not belongs to the turn already handled: the
/// agent's own turn-end hook runs inside its turn, so herdr still says "working" when the finish line is done with
/// it, and a status hook may apply a reading it took earlier. Such a reading never re-arms the task.
/// herdr's state_change_seq is one counter for the whole server, so within one server run a new turn always has a
/// higher one than the reading the turn was handled with, however late the old reading arrives. The counter starts
/// over when herdr restarts, and so does terminal_id: a reading from another terminal than the handled turn's is
/// after it. Tasks saved before that reading was kept have neither: completion_seq and `prev` must both agree.
fn after_handled(t: &Task, a: &Value, prev: Option<&Value>) -> bool {
    let h = t.at("handled");
    if a.at("state_change_seq").is_null() { return true; }
    let seq = a.i("state_change_seq");
    let term = if h.has("term") { h.t("term") } else { t.at("link").t("terminal") };
    if h.is_object() && a.b("terminal_id") && !term.is_empty() && a.t("terminal_id") != term { return true; }
    if let Some(p) = prev { if p.has("seq") && seq <= p.i("seq") { return false; } }
    let was = if h.has("seq") { h.at("seq") } else { h.at("comp") };
    was.is_null() || seq as f64 > number(was)
}
/// A human turn that has already ended without the hook seeing it start: a completion newer than the last handled
/// one, a closing note newer than the last consumed one, or a worktree HEAD or dirty tree different from the
/// recorded reviewed commit.
fn missed_turn(t: &Task, comp: &Value) -> bool {
    let h = t.at("handled");
    if !comp.is_null() && h.is_object() && comp != h.at("comp") && number(comp) > h.f("comp") { return true; }
    if let Some(n) = note_stat(t) { if h.is_object() && n.0 > h.f("note") && n.0 > t.f("armedAt") { return true; } }
    if t.s("stage") == "approval" && t.b("reviewedHead") && t.b("worktree") && Path::new(t.s("worktree")).exists() {
        let wt = Path::new(t.s("worktree"));
        if crate::flow::git::head_of(wt) != t.s("reviewedHead") || !crate::flow::git::dirty(wt).is_empty() { return true; }
    }
    false
}
pub fn human_turn_ended(t: &mut Task) {
    crate::flow::chat::begin(t, "", false);
    if t.b("mergeIntent") || t.s("stage") == "approval" { t.put("requireApproval", true); }
    t.put("mergeIntent", Value::Null);
    stale_review(t);
    for k in ["checkFails", "reviewRounds", "prRounds", "syncRepairs"] { t.put(k, 0); }
    fresh_list(t);
    // armedAt is never hook wall-clock time here: it is the time of the last handled completion, so a note written
    // during a fast turn still counts.
    let armed = if t.at("handled").b("at") { t.at("handled").at("at").clone() } else if t.b("armedAt") { t.at("armedAt").clone() } else { json!(0) };
    t.put("armedAt", armed); t.put("expectedWake", Value::Null); t.put("turnOpen", false);
    t.put("stage", "working"); t.put("msg", ""); t.del("kind"); t.put("mergeFailed", false);
}
fn deliver_inbox(t: &Task, api: &Herdr) {
    let id = t.s("id");
    let items = take_text(id).unwrap_or_default();
    if items.is_empty() { return; }
    let wake = items.iter().filter_map(|m| m.at("wake").as_object().filter(|_| m.b("wake")).cloned()).last();
    // The send time arms the pipeline for the turn this starts, so it is written before the text goes out.
    let before: Vec<(&str, Value)> = ["armedAt", "expectedWake", "stage", "msg"].iter().map(|k| (*k, t.at(k).clone())).collect();
    let _ = update_wait(id, 10000, |t| {
        t.put("armedAt", now());
        if let Some(w) = &wake { t.put("expectedWake", w.clone()); }
        if ["asks", "nosummary", "failed", "approval", "reviewneeds", "plan"].contains(&t.s("stage")) { t.put("stage", "working"); t.put("msg", ""); }
        Ok(Change::Write)
    });
    let text = items.iter().map(|m| m.t("text")).collect::<Vec<_>>().join("\n\n");
    match hand(api, t.at("link").s("pane"), &text) {
        Ok(mark) => note_sent(id, mark, &text, items[0].b("first")),
        Err(e) => {
            let _ = untake_text(id, items.len());
            let _ = update_wait(id, 10000, |t| { for (k, v) in &before { t.put(k, v.clone()); } Ok(Change::Write) });
            if e.code != "agent_blocked" && e.code != "agent_not_ready" { log(&e.message); }
        }
    }
}

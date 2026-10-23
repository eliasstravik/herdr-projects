//! Live readings: a task as herdr sees it right now, and the board's list made from them. One `agent list` and one
//! `pane list` per herdr session answer for every task on the board.
use super::task::is_agent_task;
use crate::err::Res;
use crate::herdr::cli::{current_socket, session_name, socket_exists};
use crate::herdr::Herdr;
use crate::json::*;
use crate::research::is_research_among;
use crate::state::{show, Show};
use crate::store::attempts::attempts_of;
use crate::store::inbox::pending_text;
use crate::store::tasks::{list, shared, ListOpts, Task};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// One herdr session as it stands: its agents and panes by pane id, or down.
pub enum Session { Down, Up { agents: HashMap<String, Value>, panes: HashMap<String, Value> } }
pub fn read_session(socket: &str) -> Session {
    if crate::herdr::cli::fake().is_none() && !socket_exists(socket) { return Session::Down; }
    let api = Herdr::new(Some(socket.to_string()));
    match (api.agents(), api.panes(None)) {
        (Ok(a), Ok(p)) => Session::Up { agents: a.into_iter().map(|x| (x.t("pane_id"), x)).collect(), panes: p.into_iter().map(|x| (x.t("pane_id"), x)).collect() },
        _ => Session::Down,
    }
}
/// The task's worker pane right now (state.rs: show says what each shape means).
pub fn live_of(t: &Task, session: &Session, here: &str) -> Value {
    if !is_agent_task(t) { return Value::Null; }
    let sock = t.at("link").s("socket");
    if !sock.is_empty() && sock != here { return json!({ "away": session_name(sock) }); }
    let Session::Up { agents, panes } = session else { return json!({ "down": true }) };
    let pane_id = t.at("link").s("pane");
    let Some(pane) = panes.get(pane_id).filter(|p| owns_pane(t, p)) else { return json!({ "gone": true }) };
    match agents.get(pane_id) { Some(a) => json!({ "status": a.at("agent_status"), "agent": a, "pane": pane }), None => json!({ "status": null, "pane": pane }) }
}
/// Is the pane under the task's pane id still the task's own? herdr hands a closed pane's id (and its space's) out
/// again, after a restart or once a space was removed, but never a terminal_id: a pane in a terminal the task was never
/// seen in is another's, wherever it is. After herdr restarted, the task's pane is its own again once the hook has read
/// its agent in the new terminal and herdr said so (hook.rs: `apply`, which keeps it among the task's `terms`).
pub fn owns_pane(t: &Task, pane: &Value) -> bool {
    let (term, known) = (pane.t("terminal_id"), terminals(t));
    term.is_empty() || known.is_empty() || known.contains(&term)
}
/// The terminals the task's agent was seen in: its `terms` and the one saved with its link.
pub fn terminals(t: &Task) -> Vec<String> {
    let mut known = if t.at("terms").is_array() { strings(t.at("terms")) } else { vec![] };
    known.push(t.at("link").t("terminal"));
    known.retain(|s| !s.is_empty());
    known
}
/// The show's fields, under names that cannot collide with a task's own (a task has its own `review` and `msg`).
pub fn pick(out: &mut Obj, s: &Show) {
    out.put("state", s.state);
    match &s.kind { Some(k) => out.put("kind", k.clone()), None => out.del("kind") }
    out.put("note", s.msg.clone());
    if s.plain { out.put("plain", true); } else { out.del("plain"); }
    out.put("shownApp", s.app.clone());
    match s.progress { Some(p) => out.put("progress", p.round() as i64), None => out.del("progress") }
    match s.eta { Some(e) => out.put("eta", e), None => out.del("eta") }
    out.put("asReview", s.review);
    if s.stale { out.put("stale", true); } else { out.del("stale"); }
    if s.exited { out.put("exited", true); } else { out.del("exited"); }
    match &s.limits { Some(l) => out.put("limits", l.clone()), None => out.del("limits") }
}
/// A task as the board and `list` show it: the task plus its derived state. A task done several times at once carries
/// its attempts as `tries`, each as the board shows it.
pub fn shown(t: &Task, session: &Session, here: &str, seen: &mut BTreeMap<String, bool>) -> Obj {
    let live = live_of(t, session, here);
    let pending = is_agent_task(t) && !pending_text(t.s("id")).is_empty();
    let tries: Option<Vec<Value>> = (!t.arr("attempts").is_empty() && !t.b("attemptOf")).then(|| attempts_of(t).iter().map(|c| Value::Object(shown(c, session, here, seen))).collect());
    let mut out = t.clone();
    out.put("live", live.clone());
    if let Some(tr) = tries { out.put("tries", tr); }
    // The show is of the task with its attempts and whether text waits in its inbox: the row as it is so far (the show
    // reads no `live` of a task), with `pendingText` for this look alone. One copy of the task, not two.
    let had = out.insert("pendingText".into(), json!(pending));
    let sh = show(&out, &live, crate::clock::now());
    match had { Some(v) => { out.insert("pendingText".into(), v); } None => out.del("pendingText") }
    // Research only or not is worked out here, by the machine that owns the task: a row says it.
    out.put("research", is_research_among(t, seen));
    out.put("chatOnly", crate::chat_only::is_chat_among(t, seen));
    pick(&mut out, &sh);
    out
}
/// The same from what was saved alone, with no look at herdr: for whoever just changed a task and says so at once.
pub fn saved(t: &Task) -> Obj {
    let mut out = t.clone();
    let mut probe = t.clone();
    probe.put("pendingText", false);
    pick(&mut out, &show(&probe, &Value::Null, crate::clock::now()));
    if let Some(n) = queue_place(t) { out.put("queuePlace", n); }
    out
}
/// A waiting task's place in the queue for a free slot, from every task saved (shown/queue.rs).
pub fn queue_place(t: &Task) -> Option<i64> {
    if !crate::shown::queue::queued(t) { return None; }
    let all = list(ListOpts { attempts: true, ..Default::default() });
    crate::shown::queue::places(all.iter()).into_iter().find(|(id, _)| id == t.s("id")).map(|(_, n)| n)
}
fn session_for(tasks: &[Arc<Task>], socket: &str) -> Session {
    let needs = tasks.iter().any(|t| (is_agent_task(t) || !t.arr("attempts").is_empty()) && t.at("link").s("socket") == socket);
    if needs { read_session(socket) } else { Session::Down }
}
/// This machine's tasks as one herdr session sees them (the keeper serves several sessions from one data folder).
pub fn board_for(socket: &str) -> Vec<Obj> {
    let tasks = shared(ListOpts::default());
    let session = session_for(&tasks, socket);
    let mut seen = BTreeMap::new();
    let mut rows: Vec<Obj> = tasks.iter().map(|t| shown(t, &session, socket, &mut seen)).collect();
    crate::shown::queue::number(&mut rows);
    rows
}
pub fn board(include_archived: bool) -> Vec<Obj> {
    let here = current_socket();
    let tasks = shared(ListOpts { include_archived, ..Default::default() });
    let session = session_for(&tasks, &here);
    let mut seen = BTreeMap::new();
    let mut rows: Vec<Obj> = tasks.iter().map(|t| shown(t, &session, &here, &mut seen)).collect();
    crate::shown::queue::number(&mut rows);
    rows
}
/// Refuse a live action with the reason the board shows.
pub fn assert_live(t: &Task) -> Res<()> {
    if !is_agent_task(t) { crate::fail!("No agent has been on {} yet. Delegate it first.", t.s("id")); }
    let here = current_socket();
    if t.at("link").s("socket") != here { crate::fail!("{} lives in herdr session {}. Open the board there.", t.s("id"), session_name(t.at("link").s("socket"))); }
    if crate::herdr::cli::fake().is_none() && (!socket_exists(&here) || !Herdr::new(Some(here)).alive()) { crate::fail!("herdr is not running."); }
    Ok(())
}
/// The terminals a task's agent has been read in, oldest first. herdr gives a pane a new terminal_id when it
/// restarts and never an old one again, so a reading from any terminal but the newest was taken before that restart.
/// Returns false for such a reading; a terminal not seen before becomes the newest.
pub fn see_terminal(t: &mut Task, term: &str) -> bool {
    if term.is_empty() { return true; }
    let mut seen: Vec<String> = if t.at("terms").is_array() { strings(t.at("terms")) } else {
        [t.at("handled").t("term"), t.at("link").t("terminal")].into_iter().filter(|s| !s.is_empty()).take(1).collect()
    };
    if seen.iter().any(|s| s == term) { if seen.last().map(|s| s != term).unwrap_or(false) { return false; } } else { seen.push(term.into()); }
    let keep: Vec<String> = seen.iter().skip(seen.len().saturating_sub(8)).cloned().collect();
    t.put("terms", keep);
    true
}

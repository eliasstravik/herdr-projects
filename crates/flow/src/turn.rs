//! The agent's own turn-end report. `turn-ended` is run by the hook the board passes at launch, inside the agent's
//! own turn: it only records the event and starts the finish runner, exits 0, prints nothing, and never waits more
//! than two seconds. Every judgement is made by the finish runner (finish/). A plan writer's hook runs inside its
//! read-only mount: its runner is started by the keeper instead, outside (runner/outside.rs).
use crate::agents::{helpers, installed, kinds::kind, read_turn::Turn};
use crate::clock::now;
use crate::json::*;
use crate::runner::spawn_runner;
use crate::store::files::{data_dir, marker_alive};
use crate::store::marks::{pending_events, token_ok};
use crate::store::repos::cwd_of;
use crate::store::tasks::{get, update_wait, Change, Task};
use std::io::Read;
use std::path::Path;

fn log(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(data_dir().join("hook.log")) {
        let _ = writeln!(f, "{} turn-ended: {msg}", chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    }
}
/// The hook's input: Claude and pi send JSON on stdin, Codex appends it as the last argument.
fn input(extra: &[String]) -> Value {
    for a in extra { if a.starts_with('{') { if let Ok(v) = serde_json::from_str(a) { return v; } } }
    if unsafe { libc::isatty(0) } == 1 { return json!({}); }
    let mut s = String::new();
    let _ = std::io::stdin().read_to_string(&mut s);
    serde_json::from_str(if s.trim().is_empty() { "{}" } else { &s }).unwrap_or(json!({}))
}
pub fn turn_ended(id: &str, role: &str, token: &str, round: i64, extra: &[String]) {
    let mut t = if id.is_empty() { None } else { get(id) };
    // Started as one of several attempts and chosen since: its turns are the task's now.
    if let Some(c) = &t { if c.b("chosen") && c.b("attemptOf") { t = get(c.s("attemptOf")); } }
    let Some(t) = t else { return log(&format!("no task {id}")) };
    if !["agent", "reviewer"].contains(&role) || !token_ok(t.s("id"), role, token, round) { return log(&format!("refused for {id} ({role}): wrong token")); }
    let j = input(extra);
    // Codex also reports turns of side conversations: a real first prompt starts with the task line, so `side` marks
    // every other report for the runner to judge. And the turns of the agent's own helpers (agents/helpers.rs), which
    // carry its first prompt along: their conversation names the thread that started them.
    let first = j.arr("input-messages").first().map(text).unwrap_or_default();
    let me = if t.b("as") { t.t("as") } else { t.t("id") };
    let session = if j.b("session_id") { j.t("session_id") } else { j.t("thread-id") };
    let helper = j.b("thread-id") && helpers::is_helper("codex", &session);
    let mut ev = json!({ "at": now(), "role": role, "round": round, "session": session, "file": j.t("transcript_path"), "error": j.t("error"),
        "side": helper || (j.b("thread-id") && !first.trim_start().starts_with(&format!("Work board task {me},"))) });
    if helper { ev["helper"] = json!(true); }
    let mut start = false;
    let r = update_wait(t.s("id"), 2000, |t| {
        let seq = t.i("eventSeq") + 1;
        t.put("eventSeq", seq);
        let mut events = pending_events(t);
        let mut e = ev.as_object().cloned().unwrap_or_default(); e.put("n", seq);
        events.push(Value::Object(e));
        t.put("events", events);
        start = !marker_alive(t.at("runner"));
        Ok(Change::Write)
    });
    match r { Ok(_) => { if start { spawn_runner("finish", t.s("id"), &[]); } } Err(e) => log(&if e.kind == crate::err::Kind::Busy { format!("gave up on {id}: the task was busy") } else { e.msg }) }
}
/// Why did this pane stop: { when, why, turn, hooked, kind, version, can_read }. The one seam for "when and why", fed
/// by the hook's event and the saved conversation.
pub struct Stopped { pub when: i64, pub why: String, pub turn: Option<Turn>, pub hooked: bool, pub kind: String, pub version: String, pub can_read: bool }
pub fn why_stopped(t: &Task, role: &str, event: Option<&Value>) -> Stopped {
    let k = if role == "reviewer" { t.at("link").at("reviewer").t("kind") } else { t.t("agent") };
    let ev_session = event.map(|e| e.t("session")).filter(|s| !s.is_empty());
    let session = ev_session.unwrap_or_else(|| if role == "reviewer" { t.t("reviewSession") } else { t.t("session") });
    let ev_file = event.map(|e| e.t("file")).filter(|s| !s.is_empty());
    let file = ev_file.or_else(|| (role == "agent" && t.b("transcript")).then(|| t.t("transcript")));
    let turn = installed::read_turn_of(&k, file.as_deref().map(Path::new), &session, &cwd_of(t), false);
    let why = match &turn {
        Some(x) if !x.error.is_empty() => "error".to_string(),
        Some(x) if x.interrupted => "interrupted".into(),
        Some(x) if x.finished => if x.stop.is_empty() { "stop".into() } else { x.stop.clone() },
        _ => if event.is_some() { "stop".into() } else { String::new() },
    };
    let when = event.map(|e| e.i("at")).filter(|w| *w != 0).or_else(|| turn.as_ref().map(|x| x.pos).filter(|p| *p != 0)).unwrap_or(0);
    Stopped { when, why, hooked: event.is_some(), version: installed::version_of(&k), can_read: kind(&k).map(|a| a.reads_turn).unwrap_or(false), kind: k, turn }
}

//! Making sure that what the board hands an agent is really taken: nothing is typed before the agent's program is up,
//! and a prompt that was typed but not submitted gets its enter again. The text itself is never typed a second time.
//! Seen on herdr 0.9.3 with claude 2.1.296: `agent start` answers "ready" as soon as the program is the pane's
//! foreground process, which on a loaded machine is seconds before it has drawn anything; `agent prompt` pastes the
//! text and presses enter 300 ms later, and a program still starting keeps the text and drops the enter.
use crate::agents::hand_limits::hand_over;
use crate::agents::installed::said_to;
use crate::board::keys::key_name;
use crate::clock::now;
use crate::herdr::{cli::HerdrError, Herdr};
use crate::json::*;
use crate::store::failures::failure_patch;
use crate::store::files::marker_alive;
use crate::store::repos::cwd_of;
use crate::store::tasks::{get, update_wait, Change, Task};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

fn ms(name: &str, d: i64) -> i64 { std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(d) }
/// How often enter is pressed again.
pub const TRIES: i64 = 3;
/// How long the agent gets to start before enter is pressed again: 5 seconds, then twice as long each time.
fn wait_ms(m: Option<&Value>) -> i64 { ms("HERDR_PROJECTS_SUBMIT_MS", 5000) * 2_i64.pow(m.map(|m| m.i("tries")).unwrap_or(0).clamp(0, 10) as u32) }
fn step_ms() -> i64 { ms("HERDR_PROJECTS_READY_STEP_MS", 400) }
const QUIET_MAX: i64 = 4000;

// ---- before the first prompt: is the program up? ----------------------------------------------------------------------
fn flat(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace() && !"'\"\\".contains(*c)).collect() }
/// Until a program has drawn something, the bottom of its pane is still the command herdr typed to start it.
pub fn drawn(screen: &str, launch: &str) -> bool {
    let l = flat(launch);
    let tail: String = l.chars().skip(l.chars().count().saturating_sub(24)).collect();
    tail.is_empty() || !flat(screen).ends_with(&tail)
}
/// Wait, after `agent start`, until the program has drawn its screen and that screen stands still for a moment (or has
/// been there a few seconds). Gives up after a minute and says so with false.
pub fn settle(api: &Herdr, pane: &str, launch: &str) -> bool {
    let max = ms("HERDR_PROJECTS_READY_MS", 60000);
    if max <= 0 { return true; }
    let until = now() + max;
    let (mut prev, mut since) = (None::<String>, 0);
    loop {
        let Ok(screen) = api.pane_read(pane, "visible", None) else { return false };
        if drawn(&screen, launch) { if since == 0 { since = now(); } if prev.as_deref() == Some(screen.as_str()) || now() - since >= QUIET_MAX { return true; } } else { since = 0; }
        prev = Some(screen);
        if now() >= until { return false; }
        if step_ms() > 0 { crate::clock::sleep_ms(step_ms() as u64); }
    }
}

// ---- after a prompt: did the agent start on it? -----------------------------------------------------------------------
fn working(a: &Value) -> bool { ["working", "blocked"].contains(&a.s("agent_status")) }
fn at_rest(a: &Value) -> bool { a.is_object() && !working(a) }
/// What herdr says of an agent just before text is typed into it. `busy`: herdr saw it work when the text was typed.
pub fn mark_of(a: Option<&Value>) -> Option<Value> {
    let a = a?;
    Some(json!({ "at": now(), "seq": a.at("state_change_seq"), "comp": a.at("completion_seq"), "term": a.at("terminal_id"), "tries": 0, "busy": working(a) }))
}
/// A mark taken anew once a busy agent comes to rest: only the words in its saved conversation prove that they went in.
pub fn rest(mark: &Value, a: &Value) -> Value {
    if !(mark.b("busy") && at_rest(a)) { return mark.clone(); }
    let mut m = mark.as_object().cloned().unwrap_or_default();
    for (k, v) in mark_of(Some(a)).and_then(|x| x.as_object().cloned()).unwrap_or_default() { m.insert(k, v); }
    m.del("pressedAt"); m.put("rested", true);
    m.put("typedAt", if mark.b("typedAt") { mark.at("typedAt").clone() } else { mark.at("at").clone() });
    Value::Object(m)
}
/// Do the words stand in the agent's saved conversation? Some(true), Some(false), or None when there is nothing to read.
pub fn shown(kind: &str, file: Option<&Path>, session: &str, cwd: &Path, mark: &Value, text: &str) -> Option<bool> {
    let since = if mark.b("typedAt") { mark.i("typedAt") } else { mark.i("at") };
    said_to(kind, file, session, cwd, text, since)
}
/// "Not started": the board gave up on what it typed. What ended then is still not its answer.
pub fn not_started(t: &Task) -> bool { t.s("stage") == "failed" && t.at("failure").s("stage") == "send" && t.b("unsent") }
/// Is something typed into this task's agent still waiting to be taken? The finish line holds back while it is.
pub fn pending_sent(t: &Task, a: Option<&Value>) -> bool {
    let m = t.at("sent");
    not_started(t) || (m.is_object() && t.s("stage") == "working" && (a.is_none() || m.b("busy") || took(m, a, None) == Some(false)))
}
/// Type a prompt into an agent. Returns the mark to look again with (None when herdr said nothing of the agent before).
/// Fails with what herdr answers when nothing was typed (agent_blocked, agent_not_ready, anything unexpected).
/// A text too long to hand over in one go is written to a file, and a line naming it is typed instead (hand_limits.rs).
pub fn hand(api: &Herdr, pane: &str, text: &str) -> Result<Option<Value>, HerdrError> {
    let typed = hand_over(text).map_err(|e| HerdrError { code: "failed".into(), message: e.msg })?;
    let a = api.agent_get(pane).ok().flatten();
    if let Err(e) = api.agent_prompt(pane, &typed) { if !["agent_prompt_stalled", "timeout"].contains(&e.code.as_str()) { return Err(e); } } // these do not prove nothing was sent; never send twice
    Ok(mark_of(a.as_ref()))
}
/// Some(true): it started on the text. Some(false): it sits at its prompt exactly as before. None: herdr cannot say.
pub fn took(mark: &Value, a: Option<&Value>, said: Option<bool>) -> Option<bool> {
    let a = a?;
    if mark.b("busy") { return None; }
    if mark.b("term") && a.b("terminal_id") && a.at("terminal_id") != mark.at("term") { return None; } // herdr restarted
    if said == Some(true) || working(a) { return Some(true); }
    Some(a.at("state_change_seq") != mark.at("seq") || a.at("completion_seq") != mark.at("comp"))
}
/// One look at a prompt typed a while ago, for a sender that keeps the mark itself (the reviewer, the Coordinator):
/// "took" | "unknown" | "wait" | "pressed" | "lost".
pub fn look(api: &Herdr, pane: &str, mark: &mut Value, said: Option<&dyn Fn() -> Option<bool>>) -> &'static str {
    let Ok(a) = api.agent_get(pane) else { return "wait" };
    if mark.b("busy") { if let Some(a) = &a { *mark = rest(mark, a); return "wait"; } }
    match took(mark, a.as_ref(), None) { Some(true) => return "took", None => return "unknown", Some(false) => {} }
    let pressed = if mark.b("pressedAt") { mark.i("pressedAt") } else { mark.i("at") };
    if now() - pressed < wait_ms(Some(mark)) { return "wait"; }
    let proof = said.and_then(|f| f());
    if took(mark, a.as_ref(), proof) == Some(true) { return "took"; }
    if mark.i("tries") >= TRIES { return "lost"; }
    if let Some(m) = mark.as_object_mut() { let n = m.i("tries") + 1; m.put("tries", n); m.put("pressedAt", now()); }
    let _ = api.agent_keys(pane, &["enter"]);
    "pressed"
}
/// The same, waited for: for a detached job that has nothing else to do meanwhile. False when the text was lost.
pub fn confirm(api: &Herdr, pane: &str, mark: Option<Value>, said: Option<&dyn Fn() -> Option<bool>>) -> bool {
    let Some(mut mark) = mark else { return true };
    if mark.b("busy") { return true; }
    for _ in 0..600 {
        match look(api, pane, &mut mark, said) { "took" | "unknown" => return true, "lost" => return false, _ => {} }
        if wait_ms(None) > 0 { crate::clock::sleep_ms(500); }
    }
    true
}
/// A task's agent: the mark rides on the task, and the housekeeping pass looks again. `first`: the text is everything
/// a freshly started program was given. Text typed earlier that the agent has not started on is still in its box.
pub fn note_sent(id: &str, mark: Option<Value>, text: &str, first: bool) {
    crate::store::said_by::note_board(id, text);
    let _ = update_wait(id, 10000, |t| {
        let was = mark.as_ref().map(|m| json!({ "agent_status": if m.b("busy") { "working" } else { "idle" }, "state_change_seq": m.at("seq"), "completion_seq": m.at("comp"), "terminal_id": m.at("term") }));
        let old = if t.at("unsent").is_object() { Some(t.at("unsent").clone()) }
            else if t.at("sent").is_object() && mark.is_some() && (t.at("sent").b("busy") || took(t.at("sent"), was.as_ref(), None) == Some(false)) { Some(t.at("sent").clone()) } else { None };
        match &mark {
            Some(m) => {
                let mut s = m.as_object().cloned().unwrap_or_default();
                s.put("text", match &old { Some(o) => format!("{}\n\n{text}", o.s("text")), None => text.to_string() });
                s.put("first", match &old { Some(o) => o.b("first"), None => first });
                t.put("sent", s);
                t.put("unsent", Value::Null);
            }
            None => t.put("sent", Value::Null),
        }
        Ok(Change::Write)
    });
}
/// Task id -> when its saved conversation was last read for a text typed into a running turn.
static LOOKED: LazyLock<Mutex<HashMap<String, i64>>> = LazyLock::new(Default::default);
pub const NOT_STARTED: &str = "Not started: its instructions sit unsent in its box";
/// Called on every pass with the agent as herdr shows it now. Never waits: one look, at most one enter.
pub fn check_sent(id: &str, a: Option<&Value>, api: &Herdr) {
    let Some(t) = get(id) else { return };
    let m = t.at("sent").clone();
    if !m.is_object() { return; }
    let mine = |c: &Task| c.at("sent").is_object() && c.at("sent").at("at") == m.at("at") && c.at("sent").at("tries") == m.at("tries");
    let clear = || { let _ = update_wait(id, 10000, |c| { if !mine(c) { return Ok(Change::Keep); } c.put("sent", Value::Null); Ok(Change::Write) }); };
    if t.s("stage") != "working" { return clear(); } // stopped, or taken over by a step of the finish line
    let said = || -> Option<bool> {
        let session = if t.b("session") { t.t("session") } else { crate::agents::adapter::find_session(t.s("agent"), &cwd_of(&t), t.i("startedAgentAt"), &crate::delegate::task_line(&t)).unwrap_or_default() };
        shown(t.s("agent"), t.so("transcript").map(Path::new), &session, &cwd_of(&t), &m, m.s("text"))
    };
    // The board says "not started" instead of showing work that is not going on; the text is kept for retry.
    let lost = |detail: String, next: String| {
        let _ = update_wait(id, 10000, |c| {
            if !mine(c) || c.s("stage") != "working" { return Ok(Change::Keep); }
            c.put("sent", Value::Null);
            c.put("unsent", json!({ "text": m.at("text"), "first": m.b("first"), "at": if m.b("typedAt") { m.at("typedAt").clone() } else { m.at("at").clone() }, "typed": true }));
            c.put("stage", "failed"); c.put("msg", NOT_STARTED); c.put("turnOpen", false); c.put("watch", Value::Null);
            for (k, v) in failure_patch(&obj(json!({ "stage": "send", "title": "The agent has not started on its instructions", "auth": false, "agentCanFix": false, "detail": detail, "next": next }))) { c.insert(k, v); }
            Ok(Change::Write)
        });
    };
    let secs = (((now() - m.i("at")) as f64 / 1000.0).round() as i64).max(1);
    if a.is_none() && marker_alive(t.at("runner")) { return; } // a job of the board's own is starting its program again
    let Some(a) = a else {
        if said() == Some(true) { return clear(); }
        return lost(format!("The board typed the text into the agent's box {secs} seconds ago. The agent's program ended before it started on it: nothing was submitted."), format!("Retry ({}) starts the agent's program again and hands it the text once more.", key_name("yes")));
    };
    if m.b("busy") { // typed into a running turn: looked at from the moment the agent is at rest
        let last = LOOKED.lock().unwrap().get(id).copied().unwrap_or(0);
        if now() - last >= wait_ms(None) { LOOKED.lock().unwrap().insert(id.into(), now()); if said() == Some(true) { LOOKED.lock().unwrap().remove(id); return clear(); } }
        if at_rest(a) { let _ = update_wait(id, 10000, |c| { if !mine(c) { return Ok(Change::Keep); } let r = rest(c.at("sent"), a); c.put("sent", r); Ok(Change::Write) }); }
        return;
    }
    if took(&m, Some(a), None) != Some(false) { return clear(); }
    let pressed = if m.b("pressedAt") { m.i("pressedAt") } else { m.i("at") };
    if now() - pressed < wait_ms(Some(&m)) { return; }
    if took(&m, Some(a), said()) == Some(true) { return clear(); }
    if m.i("tries") < TRIES {
        // Counted before the key goes out: a pass that dies here presses once less, never without end.
        let mut go = false;
        let _ = update_wait(id, 10000, |c| { if !mine(c) || c.s("stage") != "working" { return Ok(Change::Keep); } go = true; let s = c.obj_mut("sent"); s.put("tries", m.i("tries") + 1); s.put("pressedAt", now()); Ok(Change::Write) });
        if go { let _ = api.agent_keys(t.at("link").s("pane"), &["enter"]); }
        return;
    }
    lost(format!("The board typed the text into the agent's box {secs} seconds ago and has pressed enter {TRIES} more times since. The agent still sits at its prompt: nothing was submitted, and no work is going on."),
        format!("Retry ({}) starts the agent's program again and hands it the text once more. Or go into the agent and press enter yourself.", key_name("yes")));
}
/// "Not started", and the words do stand in the agent's saved conversation: it took them after all.
pub fn took_after_all(t: &Task) -> Option<bool> {
    let u = t.at("unsent");
    if !u.is_object() { return None; }
    let since = if u.b("typed") { u.i("at") } else { (if u.b("at") { u.i("at") } else { i64::MAX }).min(t.i("startedAgentAt")) };
    said_to(t.s("agent"), t.so("transcript").map(Path::new), t.s("session"), &cwd_of(t), u.s("text"), since)
}

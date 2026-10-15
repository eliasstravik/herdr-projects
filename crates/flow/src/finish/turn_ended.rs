//! The finish runner: an agent's turn ended. Was it really the end, and what did it say? Then the pipeline. Started two
//! ways: by the agent's own turn-end hook (`turn-ended`), a definite stop judged here from the saved conversation; or by
//! herdr's status hook, the hook-less route, where herdr's "idle" is a guess from the screen and alone never finishes a
//! task. `force`: the status hook already decided a turn ended (missed-turn recovery). A finish runner that finds
//! another unhandled event when it is done keeps its marker and processes that one too.
use super::hand_over::hand_over;
use super::pipeline::pipeline;
use super::review::close_reviewer;
use super::shared::{fail, failure, last_lines, ms, FailHow};
use super::waiting::{stay, waits};
use crate::agents::{adapter, helpers::still_at_work, installed, kinds::kind, read_turn::Turn};
use crate::board::keys::key_name;
use crate::clock::now;
use crate::err::Res;
use crate::flow;
use crate::herdr::api::read_screen;
use crate::json::*;
use crate::limits::reached::{stop_at, tokens_over, StopHow};
use crate::runner::{is_aborted, release, Guard};
use crate::store::failures::{failure_patch, is_auth_failure};
use crate::store::files::{dir_of, marker, marker_alive};
use crate::store::handovers::{fresh_list, why_of};
use crate::store::marks::{count_mark, pending_events};
use crate::store::repos::cwd_of;
use crate::store::retries::{gave_up, retry_waits};
use crate::store::tasks::{get, one_line, update_wait, Change, Task};
use crate::submit::pending_sent;
use std::path::Path;

pub fn finish_job(id: &str, force: bool) {
    let mut first = true;
    loop {
        if !finish_once(id, first && force) { if !first { release(id, false); } return; } // a marker kept for an event that turned out not to be a worker's
        first = false;
        if !release(id, true) { return; }
    }
}
const IGNORES: [&str; 6] = ["todo", "progress", "done", "queued", "starting", "setup"];
fn newer(turn: Option<&Turn>, t: &Task) -> bool { turn.map(|x| x.pos > t.at("handled").i("pos")).unwrap_or(false) }
/// What the status hook does when the human's turn ended without it seeing the start: the old approval, review and
/// counters no longer describe the work.
fn human_turn(t: &mut Task) {
    flow::chat::begin(t, "", false);
    if t.b("mergeIntent") || t.s("stage") == "approval" { t.put("requireApproval", true); }
    t.put("mergeIntent", Value::Null);
    if t.at("review").is_object() && !t.at("review").b("stale") { t.obj_mut("review").put("stale", true); }
    for k in ["checkFails", "reviewRounds", "prRounds", "syncRepairs"] { t.put(k, 0); }
    fresh_list(t);
    let armed = if t.at("handled").b("at") { t.at("handled").i("at") } else { t.i("armedAt") };
    t.put("armedAt", armed); t.put("expectedWake", Value::Null); t.put("turnOpen", false);
    t.put("stage", "working"); t.put("msg", ""); t.del("kind"); t.put("mergeFailed", false);
    if t.at("failure").b("auth") { t.put("failure", Value::Null); }
}
/// Has this task's agent begun on anything since its program was started?
fn begun(t: &Task) -> bool { t.at("handled").b("at") || t.s("turnBy") == "typed" || t.i("workedAt") >= t.i("startedAgentAt").max(1) }
fn sleep(ms: i64) { std::thread::sleep(std::time::Duration::from_millis(ms.max(0) as u64)); }
fn finish_once(id: &str, force: bool) -> bool {
    let Some(t) = get(id) else { return false };
    if !flow::task::is_agent_task(&t) { return false; }
    // Text typed into a running turn, and not taken yet: the turn that ended is not its answer.
    if t.b("sent") || t.b("unsent") { let a = flow::task::h(&t).agent_get(t.at("link").s("pane")).ok().flatten(); if pending_sent(&t, a.as_ref()) { return false; } }
    let session0 = t.t("session");
    let mine0 = |e: &Value| e.s("role") == "agent" && !(e.b("side") && e.s("session") != session0);
    // 1. Without an event, give herdr's screen guess time to correct itself.
    if !pending_events(&t).iter().any(mine0) { sleep(ms("HERDR_PROJECTS_SETTLE_MS", 5000)); }
    let mut ev: Option<Value> = None;
    let mut mine = false;
    let Ok(t) = update_wait(id, 10000, |t| {
        if t.at("runner").is_object() && t.at("runner").i("pid") != std::process::id() as i64 && marker_alive(t.at("runner")) { return Ok(Change::Keep); }
        let pending = pending_events(t);
        let session = t.t("session");
        ev = pending.iter().filter(|e| e.s("role") == "agent" && !(e.b("side") && e.s("session") != session)).last().cloned();
        if pending.is_empty() && (t.s("stage") != "working" || t.b("archivedAt")) { return Ok(Change::Keep); }
        // Every event up to here is now judged: accepted below, or dropped as a repeat, a side chat or a stale round.
        if !pending.is_empty() { let n = pending.iter().map(|e| e.i("n")).max().unwrap_or(0); t.put("eventsHandled", n); t.put("events", json!([])); }
        let skip = t.b("archivedAt") || match &ev { Some(e) => IGNORES.contains(&t.s("stage")) || (t.s("stage") == "stopped" && e.i("at") < t.i("stoppedAt") + 3000), None => t.s("stage") != "working" };
        if skip { return Ok(Change::Write); }
        mine = true;
        let mut m = marker(); m.put("job", "finish"); t.put("runner", m);
        Ok(Change::Write)
    }) else { return false };
    if !mine { return false; }
    let mut g = Guard::new(id);
    if let Err(e) = finish_steps(id, &mut g, t, ev, force) {
        if !is_aborted(&e) { let _ = fail(&mut g, failure("workflow", "The finish line failed", &e.msg, "Investigate the error, then finish again."), FailHow::default()); }
        else { close_reviewer(id); }
    }
    true
}
fn finish_steps(id: &str, g: &mut Guard, mut t: Task, ev: Option<Value>, force: bool) -> Res<()> {
    let api = flow::task::h(&t);
    let k = t.t("agent");
    let ver = installed::version_of(&k);
    let pane = t.at("link").t("pane");
    let a = api.agent_get(&pane)?;
    let (comp, seq, term) = (a.as_ref().map(|a| a.at("completion_seq").clone()).unwrap_or(Value::Null), a.as_ref().map(|a| a.at("state_change_seq").clone()).unwrap_or(Value::Null), a.as_ref().map(|a| a.at("terminal_id").clone()).unwrap_or(Value::Null));
    let note = flow::task::note_stat(&t);
    let fresh_note = note.filter(|n| n.0 > t.f("armedAt") && json!(n.0) != *t.at("handled").at("note") && n.0 != t.at("handled").f("note"));
    let reads = kind(&k).map(|x| x.reads_turn).unwrap_or(false);
    let seen = || obj(json!({ "comp": comp, "seq": seq, "term": term }));
    #[allow(unused_assignments)] let mut turn: Option<Turn> = None;
    if let Some(e) = &ev {
        // 2a. The agent said its turn ended. The hook can fire before the last message is on disk: re-read for up to
        //     ten seconds until the file shows something newer than the last handled turn.
        let file = e.so("file").map(Path::new);
        let session = if e.b("session") { e.t("session") } else { t.t("session") };
        let until = now() + ms("HERDR_PROJECTS_READ_MS", 10000);
        loop {
            turn = installed::read_turn_of(&k, file, &session, &cwd_of(&t), false);
            if newer(turn.as_ref(), &t) && turn.as_ref().map(|x| x.finished || !x.error.is_empty()).unwrap_or(false) { break; }
            if now() >= until { break; }
            sleep(400);
        }
        if reads { let _ = count_mark(&format!("reader:{k}"), &ver, turn.is_some()); }
        let _ = count_mark(&format!("hook:{k}"), &ver, true);
        if let Some(x) = &turn { if !(newer(Some(x), &t) && ((x.finished && x.id != t.at("handled").s("msg")) || !x.error.is_empty())) { return Ok(()); } } // a repeat, or an earlier turn
        // Helpers it started are still at work (agents/helpers.rs): their answers start it again, and that turn's end counts.
        if let Some(x) = turn.as_ref().filter(|x| x.finished && x.error.is_empty()) { if still_at_work(&k, Some(x), t.i("startedAgentAt")) { return Ok(()); } }
        // It said that it waits for the human, then stopped: that is not a finish.
        if waits(&t, turn.as_ref()) { stay(id, &t, turn.as_ref(), seen(), e.s("session")); return Ok(()); }
        // A turn that ended in a stage other than "working" was the human's, typed in the agent's own terminal.
        if t.s("stage") != "working" { t = update_wait(id, 10000, |t| { human_turn(t); Ok(Change::Write) })?; flow::sidebar::tell_sidebar(&t); close_reviewer(id); }
    } else {
        // 2b. The hook-less route: no event (a crash, a kind without a hook, a hook marked not working).
        if let Some(a) = &a { if ["working", "blocked"].contains(&a.s("agent_status")) { return Ok(()); } }
        let mut session = t.t("session");
        if session.is_empty() && adapter::can_find_session(&k) {
            if let Some(s) = adapter::find_session(&k, &cwd_of(&t), if t.b("startedAgentAt") { t.i("startedAgentAt") } else { t.i("armedAt") }, &crate::delegate::task_line(&t)) { session = s.clone(); let _ = update_wait(id, 10000, |t| { t.put("session", s.clone()); Ok(Change::Write) }); }
        }
        turn = installed::read_turn_of(&k, t.so("transcript").map(Path::new), &session, &cwd_of(&t), false);
        let says = newer(turn.as_ref(), &t) && turn.as_ref().map(|x| x.pos > t.i("armedAt") && ((x.finished && x.id != t.at("handled").s("msg")) || !x.error.is_empty())).unwrap_or(false);
        match &a {
            None => { if !(says && turn.as_ref().map(|x| !x.error.is_empty()).unwrap_or(false)) { return Ok(()); } } // the agent exited: a crash still shows its real last error
            Some(a) if !says => {
                if turn.as_ref().map(|x| x.finished && x.id == t.at("handled").s("msg")).unwrap_or(false) && fresh_note.is_none() { let c = comp.clone(); let _ = update_wait(id, 10000, |t| { t.obj_mut("handled").put("comp", c.clone()); Ok(Change::Write) }); return Ok(()); } // a late "idle" for a turn already handled
                if a.s("agent_status") == "unknown" && fresh_note.is_none() { return Ok(()); } // herdr cannot tell, and nothing says it stopped
                let stays = if a.s("agent_status") == "unknown" || comp.is_null() { fresh_note.is_none() } else { comp == *t.at("handled").at("comp") && fresh_note.is_none() };
                if !force && stays { return Ok(()); }
                turn = turn.filter(|x| newer(Some(x), &t) && x.interrupted);
                // Only herdr's count says a turn ended, the agent has not begun, and its first text went out behind a
                // dialog whose answer herdr had not counted yet: this is that answer, not the end of any work.
                if !force && turn.is_none() && fresh_note.is_none() && t.i("owedTurn") == t.i("startedAgentAt").max(1) && t.has("owedTurn") && !begun(&t) {
                    let c = comp.clone();
                    let _ = update_wait(id, 10000, |t| { t.obj_mut("handled").put("comp", c.clone()); t.del("owedTurn"); Ok(Change::Write) });
                    return Ok(());
                }
            }
            Some(_) => if let Some(x) = turn.as_ref().filter(|x| x.finished) {
                // the same, seen through herdr alone: this "idle" is handled, the helper's answer brings the next one
                if still_at_work(&k, Some(x), t.i("startedAgentAt")) { let c = comp.clone(); let _ = update_wait(id, 10000, |t| { t.obj_mut("handled").put("comp", c.clone()); Ok(Change::Write) }); return Ok(()); }
                // The file must have stood still through the settle wait: a message followed by more work is not the end.
                let again = x.file.as_deref().and_then(|f| installed::read_turn_of(&k, Some(f), "", &cwd_of(&t), false));
                if again.map(|y| y.id != x.id || y.pos != x.pos).unwrap_or(true) { return Ok(()); }
                if t.b("hooked") { let _ = count_mark(&format!("hook:{k}"), &ver, false); }
            }
        }
        if waits(&t, turn.as_ref()) { stay(id, &t, turn.as_ref(), seen(), ""); return Ok(()); } // the same, seen through herdr alone
    }
    g.expect(&t, &[]);
    // 3. What the agent said, and why it stopped.
    let wake = t.at("expectedWake").clone();
    let handled = json!({ "comp": comp, "seq": seq, "term": term, "note": fresh_note.as_ref().map(|n| json!(n.0)).unwrap_or_else(|| t.at("handled").at("note").clone()), "at": now(),
        "msg": turn.as_ref().map(|x| json!(x.id)).unwrap_or_else(|| t.at("handled").at("msg").clone()), "pos": turn.as_ref().map(|x| json!(x.pos)).unwrap_or_else(|| if t.at("handled").has("pos") { t.at("handled").at("pos").clone() } else { json!(0) }) });
    let ev_session = ev.as_ref().map(|e| e.t("session")).unwrap_or_default();
    let keep = |t: &mut Task| {
        t.put("handled", handled.clone()); t.put("turnOpen", false); t.put("turnBy", Value::Null);
        flow::live::see_terminal(t, &text(&term));
        if let Some(x) = &turn {
            if !x.said.is_empty() { t.put("said", x.said.clone()); }
            if let Some(u) = &x.usage { t.put("usage", serde_json::to_value(u).unwrap_or(Value::Null)); }
            if !x.model.is_empty() { t.put("model", x.model.clone()); }
            t.put("transcript", x.file.as_ref().map(|f| json!(f.display().to_string())).unwrap_or(Value::Null));
        }
        if !ev_session.is_empty() { t.put("session", ev_session.clone()); }
    };
    if let Some(x) = turn.as_ref().filter(|x| !x.error.is_empty() || x.stop == "length" || x.stop == "refusal") {
        let why = if !x.error.is_empty() { x.error.clone() } else if x.stop == "length" { "The agent's reply was cut off: it ran out of room (context or output limit).".into() } else { "The model refused to continue.".into() };
        g.set(&keep)?;
        // A passing provider error is nobody's to fix: the board waits and tells the agent to carry on, a few times with
        // growing waits, before it gives up. A turn that was itself such a retry counts on from the one before.
        let tries = if wake.s("reason") == "retry" && wake.b("tries") { wake.i("tries") } else { t.at("cutRetry").i("tries") };
        let waits_ms = retry_waits();
        if x.passing && tries < waits_ms.len() as i64 {
            if let Some((limit, used)) = tokens_over(id, "agent") { stop_at(id, "tokens", &limit, used, StopHow::default()); return Ok(()); }
            let keep_intent = (wake.b("keepIntent") || t.at("cutRetry").b("keepIntent")) && t.b("mergeIntent");
            let mut f = failure("agent", "Agent cut off", &format!("{why}\n\nThe {}. That passes by itself: the board tells the agent to carry on, up to {} times with growing waits.", gave_up("agent", &x.reason, 1), waits_ms.len()), &format!("Nothing: the board tells the agent to continue by itself. Reply or press {} to do it now.", key_name("sendBack")));
            f.put("auth", false); f.put("agentCanFix", true);
            let w = g.set(|t| {
                t.put("stage", "failed"); t.put("msg", format!("Agent stopped: {}", one_line(&why, 200)));
                if !keep_intent { t.put("mergeIntent", Value::Null); }
                for (k, v) in failure_patch(&f) { t.insert(k, v); }
                let at = t.at("failure").at("at").clone();
                t.put("waiting", json!({ "who": "agent", "reason": x.reason, "error": one_line(&why, 400), "until": now() + waits_ms[tries as usize], "tries": tries + 1, "keepIntent": keep_intent, "for": at }));
            })?;
            flow::sidebar::tell_sidebar(&w);
            return Ok(());
        }
        // It cannot go on: the next agent on `worker.fallback` takes the task over; with nobody left it waits for you.
        let hw = if !x.error.is_empty() { why_of(&why, x.auth, x.passing, &x.reason, tries + 1, now()) } else if x.stop == "length" { "it ran out of room (context or output limit)".into() } else { "its model refused to continue".into() };
        if hand_over(id, g, &hw, "")? { return Ok(()); }
        if x.passing {
            let gu = format!("{}. ", gave_up("agent", &x.reason, tries + 1));
            let mut f = failure("agent", "Agent stopped", &format!("{}{}The board told it to carry on each time and has stopped trying.\n\nIts error:\n{why}", gu[..1].to_uppercase(), &gu[1..]), &format!("Wait for the provider to recover (or choose another model), then reply or press {} to tell the agent to continue.", key_name("sendBack")));
            f.put("auth", false); f.put("agentCanFix", true); f.put("msg", format!("Agent stopped: {gu}Its error: {}", one_line(&why, 200)));
            fail(g, f, FailHow::default())?;
            return Ok(());
        }
        let auth = !x.error.is_empty() && x.auth;
        let mut f = failure("agent", if x.auth { "Needs a login" } else { "Agent stopped" }, &format!("{why}{}", if !x.said.is_empty() && x.said != why { format!("\n\nIts last message:\n{}", x.said) } else { String::new() }), &format!("Fix what it reports, then reply or press {} to tell the agent to continue.", key_name("sendBack")));
        f.put("auth", auth); f.put("agentCanFix", true); f.put("msg", if x.auth { "Needs a login".to_string() } else { format!("Agent stopped: {}", one_line(&why, 200)) });
        fail(g, f, FailHow::default())?;
        return Ok(());
    }
    let finished = turn.as_ref().map(|x| x.finished).unwrap_or(false);
    if !finished && fresh_note.is_none() && wake.is_null() {
        // Nothing says the turn finished normally: commit nothing and hold, with what is on the screen as the message.
        let screen = last_lines(&crate::agents::screen_text::screen_text(&read_screen(&api, &pane, a.as_ref().map(|a| a.s("agent_status")), 60)), 12); // its words, not its frames
        if is_auth_failure(&screen) {
            g.set(&keep)?;
            if hand_over(id, g, "logged out", "")? { return Ok(()); }
            let mut f = failure("agent", "Needs a login", &screen, ""); f.put("auth", true); f.put("msg", "Needs a login"); f.del("next");
            fail(g, f, FailHow::default())?;
            return Ok(());
        }
        g.set(|t| { keep(t); t.put("stage", "nosummary"); let m = one_line(&last_lines(&screen, 2), 2048); t.put("msg", if m.is_empty() { "Stopped without a summary".into() } else { m }); t.put("screen", screen.clone()); })?;
        return Ok(());
    }
    if t.b("cutRetry") { g.set(|t| t.put("cutRetry", Value::Null))?; } // the turn ended by itself: a later cut-off counts again
    let said = if finished { turn.as_ref().map(|x| x.said.clone()).unwrap_or_default() } else if let Some(n) = &fresh_note { n.1.clone() } else { t.t("said") };
    if let Some(n) = &fresh_note { let _ = std::fs::write(dir_of(id).join("closing.md"), format!("{}\n", n.1)); }
    let agent_session = a.as_ref().map(|a| a.at("agent_session").t("value")).unwrap_or_default();
    t = g.set(|t| { keep(t); t.put("expectedWake", Value::Null); t.put("said", said.clone()); if !agent_session.is_empty() && !t.b("session") { t.put("session", agent_session.clone()); } })?;
    // Only a kind on the closing-note fallback asks by ending its note with a question: that is where its rules put one.
    if !finished && fresh_note.is_some() && said.trim_end().ends_with('?') {
        g.set(|t| { t.put("stage", "asks"); t.put("kind", "question"); let last = said.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("").to_string(); t.put("msg", one_line(&last, 2048)); })?;
        return Ok(());
    }
    // The task's tokens, counted now that the turn has ended: at its limit it stops here, with this turn's work kept.
    if let Some((limit, used)) = tokens_over(id, "agent") { stop_at(id, "tokens", &limit, used, StopHow::default()); return Ok(()); }
    let _ = &t;
    if super::chat::after_turn(g)? { return Ok(()); } // a chat with reviewed work: kept as it was, or reviewed once you are done
    pipeline(id, g, &said, wake.b("keepIntent"), wake.s("reason"), None)
}

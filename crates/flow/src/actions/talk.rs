//! Talking to a task's agent: a reply, what every human-started turn does to a task, stop.
use super::plan::{awaits_plan, plan_reply, sent_back};
use super::shared::{is_agent_stage, say, Said};
use super::start::delegate;
use crate::agents::installed::interrupt_key;
use crate::attempts::words::standing_of;
use crate::clock::now;
use crate::err::{Error, Res};
use crate::flow;
use crate::json::*;
use crate::limits::run::granted;
use crate::runner::{runner_alive, spawn_runner};
use crate::store::attempts::attempts_of;
use crate::store::files::marker_alive;
use crate::store::handovers::fresh_list;
use crate::store::inbox::queue_text;
use crate::store::tasks::{assert_unarchived, find, get, update, Change, Task};

/// Words a person typed for a task's agent (`by`: "you", or "coordinator"): noted as theirs, so its conversation says
/// who said them, then told.
pub fn reply(id: &str, text: &str, by: &str) -> Res<Said> {
    if let Ok(t) = find(id) { crate::store::said_by::note_person(t.s("id"), by, text.trim()); }
    tell_as(id, text, false, true)
}
/// Who runs this command: a coordinator where it runs in the Coordinator's own folder or in a coordinator's pane, else you.
pub fn asker() -> &'static str {
    let socket = crate::herdr::cli::current_socket();
    let home = crate::coordinator::state::home(&socket);
    let pane = crate::util::env_val("HERDR_PANE_ID").unwrap_or_default();
    let in_one = !pane.is_empty() && crate::coordinator::state::all(&socket).iter().any(|co| crate::coordinator::state::get(co).map(|c| c.s("pane") == pane).unwrap_or(false));
    if in_one || std::env::current_dir().map(|d| d.starts_with(&home)).unwrap_or(false) { "coordinator" } else { crate::status_words::who::YOU }
}
/// A reply from the board is a human turn: it re-arms the pipeline, revokes an approval, cancels a pending
/// auto-merge and marks the old review stale, exactly as typing in the agent's own terminal does.
pub fn tell(id: &str, text: &str, keep_intent: bool) -> Res<Said> { tell_as(id, text, keep_intent, false) }
/// `chat`: these are a person's own words (`reply`), so a chat with reviewed work re-reviews only what it changes
/// (flow/chat.rs). The board's own messages on your key (send the review back, retry, carry on) are no chat.
pub fn tell_as(id: &str, text: &str, keep_intent: bool, chat: bool) -> Res<Said> {
    let cur = find(id)?;
    assert_unarchived(&cur)?;
    let cid = cur.t("id");
    let typed = text.trim();
    if typed.is_empty() { crate::fail!("Nothing to send."); }
    // A reply to a plan that waits for you sends the plan back to its writer with these words.
    let back = awaits_plan(&cur);
    let words = if back { plan_reply(&cur, typed) } else { typed.to_string() };
    if back { crate::store::said_by::carry(&cid, typed, &cid, &words); } // your words, inside the board's: still yours
    // A reply to a task that never had an agent starts one with that message, through the normal delegation path.
    if !flow::task::is_agent_task(&cur) && ["todo", "progress"].contains(&cur.s("stage")) {
        queue_text(&cid, &words, Obj::new())?;
        return Ok(say(delegate(&cid, vec![])?, format!("{cid}: an agent is starting on it with your message")));
    }
    // Its attempts are under way: your words go to every one that is still in it.
    if cur.s("stage") == "attempts" {
        let n = now();
        let mut live: Vec<Task> = attempts_of(&cur).into_iter().filter(|c| ["works", "needs"].contains(&standing_of(c, n)) && flow::task::is_agent_task(c)).collect();
        if live.is_empty() { crate::fail!("{cid}: none of its attempts can take a message right now."); }
        let asking: Vec<Task> = live.iter().filter(|c| standing_of(c, n) == "needs").cloned().collect(); // an answer is for the ones that asked
        if !asking.is_empty() { live = asking; }
        for c in &live { crate::store::said_by::carry(&cid, &words, c.s("id"), &words); tell(c.s("id"), &words, keep_intent)?; }
        let msg = if live.len() == 1 { format!("Sent to attempt {}", live[0].s("letter")) } else { format!("Sent to {} attempts ({})", live.len(), live.iter().map(|c| c.t("letter")).collect::<Vec<_>>().join(", ")) };
        return Ok(say(get(&cid).unwrap_or(cur), msg));
    }
    if cur.s("stage") == "pick" { crate::fail!("{cid} waits for you to pick one of its attempts. Pick first: your words then go to that one's agent."); }
    cleaning_up(&cur)?;
    // A reply to work that ended and was cleaned up starts a new round with that message, in a fresh worktree.
    if cur.s("stage") == "done" && flow::git::cleaned_up(&cur) {
        queue_text(&cid, &words, Obj::new())?;
        return Ok(say(delegate(&cid, vec![])?, format!("{cid}: an agent is starting a new round with your message")));
    }
    flow::live::assert_live(&cur)?;
    if cur.s("stage") == "merging" && runner_alive(&cur) && !cur.b("autoMerge") { crate::fail!("Wait for the merge to finish before replying."); }
    flow::git::cancel_auto_merge(&cur)?; // a failure sends nothing and keeps the draft
    let live = flow::live::live_of(&cur, &flow::live::read_session(cur.at("link").s("socket")), &crate::herdr::cli::current_socket());
    if live.b("gone") || live.at("status").is_null() { // the agent has exited: start it again, with the words waiting for it
        queue_text(&cid, &words, Obj::new())?;
        update(&cid, |t| { rearm(t, keep_intent, false, chat)?; if back { sent_back(t); } Ok(Change::Write) })?;
        spawn_runner("restart", &cid, &[]);
        return Ok(say(get(&cid).unwrap_or(cur), "The agent had exited: starting it again with your message."));
    }
    let t = update(&cid, |t| { rearm(t, keep_intent, true, chat)?; if back { sent_back(t); } Ok(Change::Write) })?;
    flow::sidebar::tell_sidebar(&t);
    let how = flow::send::deliver(&t, &words, None, false)?;
    let goes_on = flow::chat::during(&t) && ["reviewing", "reviewneeds"].contains(&t.s("stage"));
    let msg = if how == "queued" { "queued: open the agent and answer its dialog first" } else if back { "The plan went back to its writer with your words" } else if goes_on { "Sent to the agent: the review goes on unless the code changes" } else { "Sent to the agent" };
    Ok(say(get(&cid).unwrap_or(t), msg))
}
/// A task that just ended is being cleaned up: a reply waits for that, or its work would go with the worktree.
fn cleaning_up(t: &Task) -> Res<()> {
    if t.s("stage") == "done" && runner_alive(t) { return Err(Error::busy(format!("{} is cleaning up after it finished. Try again in a moment.", t.s("id")))); }
    Ok(())
}
/// What every human-started turn does to a task. `chat` (your own words) with `live` (the agent is there to answer): a
/// review under way goes on while you chat with it (flow/chat.rs), the stage stays and the turn is the chat's.
pub fn rearm(t: &mut Task, keep_intent: bool, live: bool, chat: bool) -> Res<()> {
    cleaning_up(t)?;
    if !chat { t.del("chat"); }
    else if flow::chat::begin(t, "board", live && runner_alive(t)) == flow::chat::Began::During { granted(t); return Ok(()); }
    granted(t); // stopped at a limit: your word gives it the limit once more
    for k in ["tidy", "waiting", "cutRetry"] { t.put(k, Value::Null); }
    fresh_list(t);
    if t.at("failure").s("stage") == "send" { t.put("failure", Value::Null); } // what you say now goes into the same box
    t.put("armedAt", now()); t.put("expectedWake", Value::Null); t.put("turnOpen", true); t.put("turnBy", "reply"); t.put("selfBlocked", Value::Null); t.put("blocked", Value::Null);
    if t.at("failure").b("auth") { t.put("failure", Value::Null); } // a reply or a retry after a login problem is how "fixed" is said
    if !keep_intent { if t.b("mergeIntent") || t.s("stage") == "approval" { t.put("requireApproval", true); } t.put("mergeIntent", Value::Null); }
    if t.at("review").is_object() && !t.at("review").b("stale") { t.obj_mut("review").put("stale", true); }
    for k in ["checkFails", "reviewRounds", "prRounds", "syncRepairs"] { t.put(k, 0); }
    let app = if t.b("agent") { t.at("agent").clone() } else { t.at("app").clone() };
    t.put("stage", "working"); t.put("msg", ""); t.del("kind"); t.put("watch", Value::Null); t.put("app", app); t.put("since", now()); t.put("mergeFailed", false);
    Ok(())
}
fn kill_runner(t: &Task) {
    let r = t.at("runner");
    if r.is_object() && r.i("pid") != std::process::id() as i64 && marker_alive(r) {
        let pid = r.i("pid") as i32;
        unsafe { if libc::kill(-pid, libc::SIGTERM) != 0 { libc::kill(pid, libc::SIGTERM); } }
    }
}
/// Stop: the stage is set first and the pane's state_change_seq recorded, then the interrupt key is sent, so a late
/// hook cannot undo the stop.
pub fn stop(id: &str) -> Res<Task> {
    let cur = find(id)?;
    assert_unarchived(&cur)?;
    let cid = cur.t("id");
    if cur.s("stage") == "progress" { return update(&cid, |t| { t.put("stage", "todo"); t.put("app", Value::Null); t.put("msg", ""); Ok(Change::Write) }); }
    // Every attempt stops with its task; one that waits for a pick, or is being compared, simply stops there.
    if !cur.arr("attempts").is_empty() && !cur.b("picked") && ["attempts", "pick", "reviewing"].contains(&cur.s("stage")) {
        let n = now();
        for c in attempts_of(&cur) { if ["works", "needs", "waits"].contains(&standing_of(&c, n)) { let _ = stop(c.s("id")); } }
        let t = update(&cid, |t| { t.put("stage", "stopped"); t.put("stoppedAt", now()); t.put("msg", ""); t.put("comparing", Value::Null); t.put("watch", Value::Null); Ok(Change::Write) })?;
        kill_runner(&t);
        if t.at("link").at("reviewer").b("pane") { flow::tabs::close_reviewer_tab(&flow::task::h(&t), t.at("link").at("reviewer")); }
        return Ok(get(&cid).unwrap_or(t));
    }
    if !flow::task::is_agent_task(&cur) || ["queued", "after"].contains(&cur.s("stage")) {
        return update(&cid, |t| {
            let was_waiting = t.s("stage") == "after";
            if !is_agent_stage(t.s("stage")) && !was_waiting { crate::fail!("{} is not working.", t.s("id")); } // one that waits stops waiting
            let agent = flow::task::is_agent_task(t);
            t.put("stage", if agent { "stopped" } else { "todo" }); if !agent { t.put("app", Value::Null); } t.put("msg", "");
            if was_waiting { t.put("stoppedWaiting", true); } // a task that waits for this one is told it was stopped (after.rs)
            Ok(Change::Write)
        });
    }
    let a = if flow::live::assert_live(&cur).is_ok() { flow::task::h(&cur).agent_get(cur.at("link").s("pane")).ok().flatten() } else { None };
    let t = update(&cid, |t| {
        let seq = a.as_ref().map(|a| a.at("state_change_seq").clone()).filter(|v| !v.is_null()).unwrap_or_else(|| if t.at("last").has("seq") { t.at("last").at("seq").clone() } else { json!(0) });
        t.put("stage", "stopped"); t.put("stoppedAt", now()); t.put("stopSeq", seq); t.put("sawIdleAfterStop", false);
        for k in ["expectedWake", "watch", "mergeIntent", "waiting", "limitStop"] { t.put(k, Value::Null); }
        t.put("turnOpen", false); t.put("msg", "");
        Ok(Change::Write)
    })?;
    kill_runner(&t);
    if let Some(a) = &a { if ["working", "blocked", "unknown"].contains(&a.s("agent_status")) { let _ = flow::task::h(&t).agent_keys(t.at("link").s("pane"), &[interrupt_key(t.s("agent"))]); } }
    if t.at("link").at("reviewer").b("pane") { flow::tabs::close_reviewer_tab(&flow::task::h(&t), t.at("link").at("reviewer")); }
    flow::sidebar::tell_sidebar(&t);
    Ok(get(&cid).unwrap_or(t))
}

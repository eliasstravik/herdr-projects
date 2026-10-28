//! The end of an agent's turn in a chat with a reviewed task (flow/chat.rs), and the one review after it. No code
//! changed: the task goes back to where it was, with the same review, and nothing runs. Code changed: the board waits
//! until you are done (no new message from you and the agent at rest for `review.afterChat`, or you left the chat),
//! then runs the finish line once, on what the chat made of the work.
use super::pipeline::pipeline;
use super::shared::{fail, failure, FailHow};
use crate::clock::now;
use crate::err::Res;
use crate::flow::{self, chat};
use crate::herdr::Herdr;
use crate::json::*;
use crate::runner::{claim, is_aborted, release, runner_alive, spawn_runner, Guard};
use crate::store::reviews::{current_review, log_kept};
use crate::store::settings::{lookup::setting, values::ms};
use crate::store::tasks::{update_wait, Change, Task};

/// The agent's turn ended, the finish line has it: true when the chat decided what follows, and the pipeline does not
/// run now.
pub fn after_turn(g: &mut Guard) -> Res<bool> {
    let t = g.check()?;
    if !t.at("chat").is_object() { return Ok(false); }
    let head = t.at("chat").t("head");
    let same = chat::same_code(&t, &head);
    if same && t.at("chat").at("from").is_object() {
        let back = g.set(|t| {
            chat::put_back(t);
            if current_review(t).is_some() { log_kept(t, &json!({ "chat": true, "head": head })); } // the task's history says the review still stands
        })?;
        flow::sidebar::tell_sidebar(&back);
        return Ok(true);
    }
    if same { return Ok(false); } // nothing to put back (its review is gone): the finish line as always
    let h = g.set(|t| { t.obj_mut("chat").put("held", now()); t.put("msg", chat::HELD); })?;
    flow::sidebar::tell_sidebar(&h);
    Ok(true)
}
/// How long the board waits for you after a chat that changed the code.
fn quiet(t: &Task) -> i64 { ms(&text(&setting("review.afterChat", Some(t)))).max(1000) }
/// You are done with the chat: you left it, or said nothing more for the quiet time while the agent was at rest.
fn done_with(t: &Task, away: bool, n: i64) -> bool {
    let c = t.at("chat");
    let since = c.i("last").max(c.i("held"));
    away || c.i("left") >= since || n - since >= quiet(t)
}
/// The housekeeping pass's part: each chat that changed the code and that you are done with gets its one review.
pub fn chat_pass(tasks: &[Task], socket: &str) {
    let held: Vec<&Task> = tasks.iter().filter(|t| chat::held(t) && crate::herdr::cli::same_socket(t.at("link").s("socket"), socket) && !t.b("archivedAt") && !runner_alive(t) && !t.b("sent") && !t.b("pendingText")).collect();
    if held.is_empty() { return; }
    // A chat in the agent's own pane is left when herdr shows another space.
    let shown: Option<Vec<Value>> = if held.iter().any(|t| t.at("chat").s("via") == "terminal") { Herdr::new(Some(socket.into())).workspaces().ok() } else { None };
    let n = now();
    for t in held {
        let away = t.at("chat").s("via") == "terminal" && shown.as_ref().and_then(|w| w.iter().find(|w| w.at("workspace_id") == t.at("link").at("workspace"))).map(|w| w.at("focused") == &json!(false)).unwrap_or(false);
        if done_with(t, away, n) { spawn_runner("chatreview", t.s("id"), &[]); }
    }
}
/// You left the chat with this task on the board (another task selected): its review need not wait the quiet time.
pub fn left(id: &str) {
    let _ = update_wait(id, 10000, |t| { if !chat::held(t) || t.at("chat").b("left") { return Ok(Change::Keep); } t.obj_mut("chat").put("left", now()); Ok(Change::Write) });
}
/// The one review after a chat that changed the code: the finish line, on the work as the chat left it.
pub fn chat_review_job(id: &str) {
    let Ok(Some(t)) = claim(id, "chatreview", |_| {}) else { return };
    if !chat::held(&t) { release(id, false); return; } // you spoke again meanwhile
    let mut g = Guard::new(id);
    g.expect(&t, &[]);
    let said = t.t("said");
    if let Err(e) = pipeline(id, &mut g, &said, false, "", None) {
        if !is_aborted(&e) { let _ = fail(&mut g, failure("workflow", "The finish line failed", &e.msg, "Investigate the error, then finish again."), FailHow::default()); } else { super::review::close_reviewer(id); }
    }
    release(id, false);
}

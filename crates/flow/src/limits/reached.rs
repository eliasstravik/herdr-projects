//! A limit is reached: the agent is stopped, its work is kept as it is, nothing is merged, and the task waits for you
//! under "needs you" with one plain line (`stopped at its 2h limit`, `stopped at 2m tokens`). The stop is the board's
//! own stop (the stage is "stopped"), marked with what stopped it: `limitStop: { what, limit, role, used, at }`. From
//! there the task only moves when somebody says so: a reply or "continue", "take it as it is", or a higher limit,
//! which the tick sees and carries on with by itself.
use super::run::{limit_said, stop_line, stopped_at_limit, used_of};
use super::spent::count;
use crate::clock::now;
use crate::json::*;
use crate::store::settings::lookup::setting_text;
use crate::store::settings::values::{ms, tokens_of, NONE};
use crate::store::tasks::{get, update_wait, Change, Task};

#[derive(Default)]
pub struct StopHow { pub role: String, pub review: bool, pub ready: bool, pub done: String }
/// Stop a task at a limit. `review`: the reviewer's findings were kept and are what "continue" sends back. `ready`: the
/// work is finished and carrying on puts it up for approval. `done`: there is nothing to land, and carrying on ends it.
pub fn stop_at(id: &str, what: &str, limit: &str, used: f64, how: StopHow) -> Option<Task> {
    let role = if how.role.is_empty() { "agent".to_string() } else { how.role.clone() };
    count(id, &role, 2000); // what it used up to here, read while the task still names its conversation
    let _ = crate::actions::talk::board_stop(id);
    let (what, limit) = (what.to_string(), limit.to_string());
    let t = update_wait(id, 10000, move |t| {
        if t.s("stage") != "stopped" { return Ok(Change::Keep); }
        let mut ls = obj(json!({ "what": what, "limit": limit, "role": role, "used": used, "at": now() }));
        if how.review { ls.put("review", true); }
        if how.ready { ls.put("ready", true); }
        if !how.done.is_empty() { ls.put("done", how.done.clone()); }
        t.put("limitStop", ls);
        t.put("msg", stop_line(&what, &limit));
        t.put("runCarry", Value::Null);
        Ok(Change::Write)
    }).ok();
    if let Some(t) = &t { crate::flow::sidebar::tell_sidebar(t); }
    t
}
/// Has this task used its tokens? Counted now, at the end of a turn. Returns (limit, used) when it has.
pub fn tokens_over(id: &str, role: &str) -> Option<(String, f64)> {
    let t = if role.is_empty() { get(id) } else { count(id, role, 10000) };
    over_now(t.as_ref()?)
}
/// The same of a task as it stands, with nothing read: asked before anything merges by itself.
pub fn over_now(t: &Task) -> Option<(String, f64)> {
    if !limit_said(t) { return None; }
    let limit = setting_text("limit.tokens", Some(t));
    if limit == NONE { return None; }
    let used = used_of(t);
    (used >= tokens_of(&limit) as f64).then_some((limit, used))
}
/// Is a task that a limit stopped under that limit again, as the settings stand now? (It was raised, or taken away.)
pub fn lifted(t: &Task) -> bool {
    if !stopped_at_limit(t) { return false; }
    let ls = t.at("limitStop");
    let limit = setting_text(&format!("limit.{}", ls.s("what")), Some(t));
    if limit == NONE { return true; }
    if ls.s("what") == "time" { ms(&limit) as f64 > ls.f("used") } else { tokens_of(&limit) as f64 > used_of(t) }
}

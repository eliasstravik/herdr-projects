//! A firing: the board adds an ordinary task and puts an agent on it at once. The task's title is the automation's name,
//! its description the automation's prompt; the automation's own settings are put on it as that task's own. Every
//! firing is written down. The guards, one firing at a time on the machine: while the task of the firing before still
//! has an agent, the check or a reviewer on it, it is skipped; an event of a task this automation made never fires it;
//! more than RUNAWAY firings in an hour turns the automation off, and its row says so.
use super::events::event_named;
use super::list::{checked_settings, find};
use super::told::{description_of, made_by, Cause};
use super::triggers::say_trigger;
use crate::actions::shared::is_agent_stage;
use crate::err::Res;
use crate::json::*;
use crate::store::automations::{change_automation, note_firing};
use crate::store::files::with_lock_wait;
use crate::store::marks::tilde;
use crate::store::tasks::{create, get, update, Change, Task};
use std::path::Path;

/// The most tasks one automation may make in an hour. The next firing turns it off instead.
pub const RUNAWAY: usize = 10;
const HOUR: i64 = 3600000;
/// One firing at a time on the machine, whoever asks: the keeper, an open board, a command.
pub fn one_at_a_time<T>(wait_ms: i64, f: impl FnOnce() -> Res<T>) -> Res<T> { with_lock_wait("automations-fire", wait_ms, f) }
/// What fired it, in a few words: "every 2h", "proj/3 was merged", "run now".
pub fn say_cause(a: &Obj, c: &Cause) -> String {
    match c { Cause::Event { event, task } => format!("{} {}", task.s("id"), event_named(event).map(|e| e.what).unwrap_or(event)), Cause::Run => "run now".into(), Cause::Schedule => say_trigger(a.at("trigger")) }
}
/// The task of its last firing that made one is still being worked on.
pub fn busy_task(a: &Obj) -> Option<Task> { get(a.s("lastTask")).filter(|t| !t.b("archivedAt") && is_agent_stage(t.s("stage"))) }
/// The firing as written down: { at, cause, what, task } or { at, cause, what, skipped }. `unwritten`: an automation
/// that is off, which never fires.
pub fn fire(reference: &str, cause: Cause, now: i64) -> Res<Value> {
    one_at_a_time(5000, || {
        let a = find(reference)?;
        let base = obj(json!({ "at": now, "cause": cause.kind(), "what": say_cause(&a, &cause) }));
        let skip = |why: String| { let mut e = base.clone(); e.put("skipped", why); note_firing(a.s("id"), e) };
        if !a.b("on") { let mut e = base.clone(); e.put("skipped", "it is off"); e.put("unwritten", true); return Ok(Value::Object(e)); }
        if let Some(busy) = busy_task(&a) { return skip(format!("its last task, {}, is still being worked on", busy.s("id"))); }
        let count_from = a.i("countFrom");
        let made = a.arr("made").iter().filter(|at| number(at) as i64 > (now - HOUR).max(count_from)).count();
        if made >= RUNAWAY {
            let why = format!("Turned itself off: more than {RUNAWAY} firings in an hour");
            change_automation(a.s("id"), |x| { x.put("on", false); x.put("nextAt", 0); x.put("offWhy", why.clone()); })?;
            return skip(format!("{}{}", why[..1].to_lowercase(), &why[1..]));
        }
        if !Path::new(a.s("root")).exists() { return skip(format!("its folder is gone: {}", tilde(a.s("root")))); }
        let settings = match checked_settings(&a, a.o("settings").unwrap_or(&Obj::new())) { Ok(s) => s, Err(e) => return skip(format!("one of its settings is refused: {}", e.msg)) };
        // An event's task hands on the automations it came from, so two automations can never fire each other for ever.
        let mut chain: Vec<Value> = match &cause { Cause::Event { task, .. } => made_by(task).map(|m| m.arr("chain").to_vec()).unwrap_or_default(), _ => vec![] };
        if !chain.contains(a.at("id")) { chain.push(a.at("id").clone()); }
        let t = create(Path::new(a.s("root")), a.s("name"), &description_of(&a, &cause), None)?;
        update(t.s("id"), |t| { t.put("settings", settings.clone()); t.put("automation", json!({ "id": a.at("id"), "name": a.at("name"), "chain": chain, "cause": cause.kind(), "at": now })); Ok(Change::Write) })?;
        change_automation(a.s("id"), |x| { x.put("lastTask", t.t("id")); let mut m = x.arr("made").to_vec(); m.push(json!(now)); let keep: Vec<Value> = m.iter().skip(m.len().saturating_sub(RUNAWAY + 1)).cloned().collect(); x.put("made", keep); })?;
        let mut entry = base.clone();
        entry.put("task", t.t("id"));
        if let Err(e) = crate::actions::start::delegate(t.s("id"), vec![]) { entry.put("note", format!("added, but no agent could be put on it: {}", e.msg)); }
        note_firing(a.s("id"), entry)
    })
}

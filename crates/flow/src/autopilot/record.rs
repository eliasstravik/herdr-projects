//! Autopilot's record of one coordinator: the `autopilot` part of its state.json, changed under the coordinator's own
//! lock. `coordinator new`, `restart` and `stop` keep it as it is. It holds: on or off (`on`, since `since`); this
//! time's values (`once`); the day its counts are of (`day`), the tasks it added that day (`added`) and the wakes typed
//! in that day (`woke`); its tasks (`tasks`: each its id, how it came to be its (`by`: added or started), the retries
//! used and whether it is left to you); the reasons waiting (`pending`); the wake in the queue or under way (`wake`);
//! when its last turn ended (`endAt`); the free look's snapshot (`snap`); unanswered wakes in a row (`unanswered`,
//! `needsYou`); its last 20 wakes (`wakes`); why it last could not start (`couldNot`); and the board events seen.
use crate::coordinator::co::Co;
use crate::coordinator::state;
use crate::err::Res;
use crate::json::*;
use crate::store::settings::coord::Coord;
use crate::store::settings::lookup::{lookup, Scope};
use crate::store::settings::places::yours;

/// Its key in config.json's `coordinators`, and in a task's autopilot record: "main", "machine", "project:<repo>".
pub fn key_of(co: &Co) -> String { co.settings_key() }
/// The coordinator a key names, in this herdr session.
pub fn co_of(socket: &str, key: &str) -> Co { match key { "main" | "" => Co::top(socket), "machine" => Co::machine(socket), k => Co::project(socket, k.strip_prefix("project:").unwrap_or(k)) } }
/// Its record now (empty: never on).
pub fn get(co: &Co) -> Obj { state::get(co).and_then(|c| c.o("autopilot").cloned()).unwrap_or_default() }
pub fn is_on(co: &Co) -> bool { get(co).b("on") }
/// The one way the record changes: under the coordinator's lock, with its whole state at hand (its queue).
/// `f` returning false writes nothing.
pub fn change(co: &Co, f: impl FnOnce(&mut Obj, &mut Obj) -> bool) -> Res<Obj> {
    let mut out = Obj::new();
    state::change(co, |c| {
        let mut ap = c.o("autopilot").cloned().unwrap_or_default();
        let wrote = f(&mut ap, c);
        if wrote { c.put("autopilot", Value::Object(ap.clone())); }
        out = ap;
        wrote
    })?;
    Ok(out)
}
/// Whose settings, for the lookup: this coordinator's, with this time's values while it is on.
pub fn scope_of(co: &Co) -> Scope<'static> { let ap = get(co); Scope::coord(Coord { key: key_of(co), once: if ap.b("on") { ap.o("once").cloned().unwrap_or_default() } else { Obj::new() } }) }
/// One of its settings, as it reads now.
pub fn value(co: &Co, name: &str) -> Value { lookup(name, &scope_of(co), &yours()).map(|f| f.value).unwrap_or(Value::Null) }
pub fn words(co: &Co, name: &str) -> String { text(&value(co, name)) }
pub fn count(co: &Co, name: &str) -> i64 { value(co, name).as_i64().unwrap_or(0) }
/// The day a time is in, on this machine: what its daily counts are of.
pub fn day_of(at: i64) -> String {
    use chrono::TimeZone;
    chrono::Local.timestamp_millis_opt(at).single().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()
}
/// A new day: its counts start again.
pub fn roll(ap: &mut Obj, now: i64) -> bool {
    let d = day_of(now);
    if ap.s("day") == d { return false; }
    ap.put("day", d); ap.put("added", 0); ap.put("woke", 0);
    true
}
/// Its tasks, as recorded.
pub fn tasks(ap: &Obj) -> Vec<Obj> { ap.arr("tasks").iter().filter_map(|t| t.as_object().cloned()).collect() }
pub fn task_rec(ap: &Obj, id: &str) -> Option<Obj> { tasks(ap).into_iter().find(|t| t.s("id") == id) }
/// Changes one of its tasks' record (none: nothing).
pub fn with_task(ap: &mut Obj, id: &str, f: impl FnOnce(&mut Obj)) {
    let mut list = ap.arr("tasks").to_vec();
    if let Some(o) = list.iter_mut().find(|t| t.s("id") == id).and_then(|t| t.as_object_mut()) { f(o); }
    ap.put("tasks", list);
}
/// Records a task as its own: one it added, or one of yours it started.
pub fn own_task(ap: &mut Obj, id: &str, by: &str, now: i64) {
    if task_rec(ap, id).is_some() { return; }
    let mut list = ap.arr("tasks").to_vec();
    list.push(json!({ "id": id, "by": by, "at": now, "retries": 0, "left": false }));
    ap.put("tasks", list);
}
/// The words it goes by on the board: "Coordinator", "proj coordinator".
pub fn name(co: &Co) -> String { co.name() }

//! Other machines' coordinators on autopilot: each runs its autopilot on its own machine, with that machine's settings
//! and limits. This machine is sent what each shows (shown.rs: view) with that machine's tasks, and sends the start key and
//! `Ctrl+X` there as it sends messages. Every coordinator with a row is sent, one that never ran as off; a machine on an
//! older version sends no autopilot at all, and the keys say to update it.
use crate::json::*;

/// What this machine sends of its coordinators' autopilot with its list, by level ("" the main one, "machine",
/// "project:<repo>"): each that ran, and the machine's and every repo's it knows that never did, as off (the start key there
/// starts it, as a first message does).
pub fn sent(socket: &str, now: i64) -> Value {
    use crate::coordinator::co::Co;
    let mut all: Obj = crate::coordinator::state::all(socket).into_iter().map(|co| (co.to(), super::shown::view(&co, now))).collect();
    let never_ran = std::iter::once(Co::machine(socket)).chain(crate::store::repos::known_repos().into_iter().map(|(r, _)| Co::project(socket, &r)));
    for co in never_ran { all.entry(co.to()).or_insert_with(|| json!({ "on": false })); }
    Value::Object(all)
}
/// The autopilot key, the start key and the stop key on a coordinator's row of this machine, from another machine's
/// board (remote.rs: door).
pub fn door(req: &Value) -> crate::err::Res<Value> {
    let co = crate::coordinator::co::Co::of(&crate::herdr::cli::current_socket(), req.s("to"));
    let msg = match req.s("action") {
        "autopilot-on" => super::switch::on(&co, None, &[])?, "autopilot-off" => super::switch::off(&co)?,
        // Started and stopped as an agent on a task is (the board's start key and stop key).
        "coordinator-start" => { crate::coordinator::requests::restart(&co)?; format!("Starting the {}.", co.name()) }
        "coordinator-stop" => { crate::coordinator::requests::stop(&co)?; format!("Stopped the {}.", co.name()) }
        // Gone into from another machine's board: its herdr puts it in front (`focus`), or that board attaches to its pane.
        "coordinator-open" => {
            let v = if req.b("focus") { crate::coordinator::look::go_in(&co, 30000)? } else { crate::coordinator::look::look(&co) };
            if v.pane.is_empty() { crate::coordinator::requests::open(&co)?; return Ok(json!({ "ok": false, "msg": format!("Starting the {}: enter again in a moment.", co.name()) })); }
            return Ok(json!({ "ok": true, "msg": "", "pane": v.pane, "hasAgent": true, "focused": req.b("focus") }));
        }
        a => crate::fail!("Unknown action {a}"),
    };
    Ok(json!({ "ok": true, "msg": msg }))
}

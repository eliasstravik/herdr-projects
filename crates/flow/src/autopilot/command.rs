//! `herdr-projects autopilot …`: every coordinator with its state (no word after it), `on [--machine | --project
//! <repo>] [--goal "<words>"] [--autopilot.<name> <value> …]` and `off [--machine | --project <repo> | --all]`. The
//! coordinator is named as the `coordinator` commands name it: no flag for the main Coordinator.
use super::record::{get, key_of};
use super::shown::view;
use super::switch::{off, off_all, on};
use crate::clock::now;
use crate::coordinator::co::Co;
use crate::coordinator::state::all;
use crate::err::Res;
use crate::herdr::cli::current_socket;
use crate::json::*;

/// The coordinator the flags name: `--project <repo>` (its name in any letter case), `--machine`, or the main one.
pub fn target(flags: &Obj) -> Co {
    let socket = current_socket();
    if flags.b("project") {
        let repo = flags.t("project");
        let name = crate::store::repos::known_repos().keys().find(|k| k.to_lowercase() == repo.to_lowercase()).cloned().unwrap_or(repo);
        return Co::project(&socket, &name);
    }
    if flags.b("machine") { Co::machine(&socket) } else { Co::top(&socket) }
}
/// One coordinator as `autopilot` lists it.
fn line(co: &Co) -> (Value, String) {
    let v = view(co, now());
    let said = if v.b("on") { format!("{}: {}\n  goal: {} · today: {}", co.name(), v.s("line"), v.s("goal"), v.s("today")) } else { format!("{}: autopilot off", co.name()) };
    let mut j = obj(v); j.put("coordinator", key_of(co)); j.put("name", co.name());
    (Value::Object(j), said)
}
pub fn autopilot_command(pos: &[String], flags: &Obj) -> Res<(Value, String)> {
    let what = pos.first().map(|s| s.as_str()).unwrap_or("");
    let settings: Vec<(String, Value)> = flags.iter().filter(|(k, _)| k.contains('.')).map(|(k, v)| (k.clone(), v.clone())).collect();
    match what {
        "" | "status" | "list" => {
            let socket = current_socket();
            let (list, said): (Vec<Value>, Vec<String>) = all(&socket).iter().map(line).unzip();
            Ok((json!(list), format!("{}\n(this machine only: another machine's coordinators are shown on its own board)", said.join("\n"))))
        }
        "on" => {
            let co = target(flags);
            let goal = flags.get("goal").filter(|g| g.is_string()).map(text);
            let said = on(&co, goal.as_deref(), &settings)?;
            Ok((json!({ "coordinator": key_of(&co), "on": true, "autopilot": get(&co) }), said))
        }
        "off" => {
            if flags.b("all") { let said = off_all(&current_socket())?; return Ok((json!({ "off": "all" }), said)); }
            let co = target(flags);
            let said = off(&co)?;
            Ok((json!({ "coordinator": key_of(&co), "on": false }), said))
        }
        _ => crate::fail!("autopilot takes: on [--machine | --project <repo>] [--goal \"<words>\"] [--autopilot.<name> <value>], off [--machine | --project <repo> | --all], or nothing to list them. Try: herdr-projects help"),
    }
}

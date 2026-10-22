//! `herdr-projects autopilot …`: every coordinator with its state (no word after it), `show [--machine | --project
//! <repo>]` (one coordinator's autopilot as it stands now: what a coordinator reads first in each wake), `on [--machine |
//! --project <repo>] [--goal "<words>"] [--autopilot.<name> <value> …]` and `off [--machine | --project <repo> | --all]`.
//! The coordinator is named as the `coordinator` commands name it: no flag for the main Coordinator.
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
/// One coordinator's autopilot whole, as it stands now: its state, goal, today's counts, its tasks under way and
/// those that need looking after, and its settings in plain words. A wake says only why it woke: this is the rest.
fn show(co: &Co) -> (Value, String) {
    let (v, _) = line(co);
    if !v.b("on") { return (v, format!("{}: autopilot off.", co.name())); }
    let ap = get(co);
    let board = super::limits::board_tasks();
    let tag = |id: &str| crate::store::keys::say_id(id, &crate::store::keys::repo_keys());
    let busy: Vec<String> = super::record::tasks(&ap).iter().filter_map(|r| board.iter().find(|t| t.s("id") == r.s("id") && super::care::under_way(t)).map(|t| format!("{} ({})", tag(t.s("id")), t.s("stage")))).collect();
    let approve = super::record::words(co, "autopilot.approve") == "yes";
    let needs: Vec<String> = super::care::needing(&ap, &board, approve).into_iter().map(|(id, w)| format!("{} ({})", tag(&id), match w { "asks" => "has a question", "yes" => "is ready to approve", _ => "is stuck" })).collect();
    let none = |l: Vec<String>| if l.is_empty() { "none".to_string() } else { l.join(", ") };
    let mut said = vec![format!("{}: {}", co.name(), v.s("line")), format!("Now: {}", v.s("status")), format!("Goal: {}", v.s("goal")), format!("Today: {}", v.s("today")),
        format!("Under way: {}", none(busy)), format!("Needs looking after: {}", none(needs))];
    said.extend(super::shown::settings_of(co).into_iter().map(|(l, x, _)| format!("{l}: {x}")));
    (v, said.join("\n"))
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
        "show" => Ok(show(&target(flags))),
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
        _ => crate::fail!("autopilot takes: show [--machine | --project <repo>], on [--machine | --project <repo>] [--goal \"<words>\"] [--autopilot.<name> <value>], off [--machine | --project <repo> | --all], or nothing to list them. Try: herdr-projects help"),
    }
}

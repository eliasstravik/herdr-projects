//! `pin` and `unpin` on the command line: a task by its id (another machine's as `vm-b:web/7`), a machine by its name,
//! a project by its name or its key (`--on MACHINE`: another machine's project, pinned there); and `list --pinned`,
//! what is pinned on this machine: the machine, its projects, its tasks (docs/pinned.md).
use crate::actions::{filing, pins::pin_here};
use crate::after::circle::is_me;
use crate::err::Res;
use crate::json::*;
use crate::remote::pins::{command as pin_there, machine};

/// `pin` (or `unpin`) `pos[1]`: what --json prints and what a person reads. `shown`: a task as `list` gives it.
pub fn command(cmd: &str, pos: &[String], flags: &Obj, shown: &dyn Fn(&str) -> Value) -> Res<(Value, String)> {
    let on = cmd == "pin";
    let word = if on { "pinned" } else { "unpinned" };
    let Some(want) = pos.get(1).filter(|s| !s.trim().is_empty()).map(|s| s.trim().to_string()) else { crate::fail!("{cmd} needs a task's id, a machine's name or a project's name. Try: herdr-projects help") };
    if pos.len() > 2 { crate::fail!("{cmd} takes one task, machine or project: herdr-projects {cmd} <id | machine | project> [--on MACHINE]"); }
    let on_flag = flags.get("on").filter(|v| !v.is_boolean()).map(text);
    if flags.has("on") && on_flag.is_none() { crate::fail!("--on needs a machine's name: herdr-projects {cmd} PROJECT --on MACHINE"); }
    let project = |repo: &str| -> Res<(Value, String)> { let (name, msg) = pin_here(Some(repo), on)?; Ok((json!({ "repo": name, "pinned": on }), msg)) };
    // A project, named with its machine: another machine's is pinned there (remote/pins.rs).
    if let Some(label) = on_flag {
        if is_me(&label) { return project(&want); }
        let Some(m) = machine(&label) else { crate::fail!("{label} is not a machine the board knows (herdr machine list, or the board.machines setting).") };
        return pin_there(&m, &want, on);
    }
    // Another machine's task, named by its machine first: pinned there, as the board's pin key does it.
    if let Some((label, rid)) = crate::after::far::split(&want).filter(|(l, _)| !is_me(l)) {
        if let Some(m) = machine(&label) {
            return match crate::remote::ask(&m, "do", Some(&json!({ "action": "pin", "id": rid, "on": on })), 30000) {
                Err(why) => crate::fail!("{}", crate::remote::why::unanswered(&m.label, &why)),
                Ok(j) if j.b("ok") => { let said = format!("{}: {}", m.label, j.s("msg")); Ok((j, said)) }
                Ok(j) => crate::fail!("{}: {}", m.label, j.s("msg")),
            };
        }
    }
    // A task here.
    if let Ok(t) = crate::store::tasks::find(&want) { let t = filing::set_pin(t.s("id"), on)?; return Ok((shown(t.s("id")), format!("{}: {word}", t.s("id")))); }
    // A machine: this one, or another the board knows.
    if is_me(&want) || want.eq_ignore_ascii_case(&crate::remote::check::hostname()) { let (name, msg) = pin_here(None, on)?; return Ok((json!({ "machine": name, "pinned": on }), msg)); }
    if let Some(m) = machine(&want) { return pin_there(&m, "", on); }
    // A project here: a name the board does not know says which it does.
    project(&want).map_err(|e| format!("\"{want}\" is no task, machine or project the board knows. {}", e.msg).into())
}
/// `list --pinned`: this machine when it is pinned, its pinned projects, and its pinned tasks, one line each.
pub fn list(tasks: Vec<Obj>, line: &dyn Fn(&Obj) -> String, brief: &dyn Fn(&Obj) -> Value) -> (Value, String) {
    let pins = crate::store::pins::pins();
    let host = crate::remote::check::hostname();
    let tasks: Vec<Obj> = tasks.into_iter().filter(|t| t.b("pinned") && !t.b("archivedAt")).collect();
    let mut text = vec![];
    if pins.machine { text.push(format!("machine   {host}")); }
    for r in &pins.repos { text.push(format!("project   {}", crate::shown::words::repo_label(r))); }
    text.extend(tasks.iter().map(line));
    let v = json!({ "machine": if pins.machine { json!(host) } else { Value::Null }, "projects": pins.repos, "tasks": tasks.iter().map(brief).collect::<Vec<_>>() });
    (v, if text.is_empty() { "Nothing pinned.".into() } else { text.join("\n") })
}

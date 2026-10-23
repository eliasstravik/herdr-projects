//! Another machine, or a project of it, pinned from here (the pin key on its row, `pin <machine>`, `pin <project> --on
//! <machine>`), and the same asked of this machine: through its door (`do`, `pin-machine` / `pin-project`) it is pinned
//! there, with that machine's own actions/pins.rs, so every board sees the pin with that machine's tasks
//! (docs/pinned.md).
use super::Machine;
use crate::err::Res;
use crate::json::*;

pub fn is_pin_action(action: &str) -> bool { ["pin-machine", "pin-project"].contains(&action) }
/// The request for the machine to pin itself (`repo` empty) or its project `repo` (`on`), or to unpin it.
pub fn request(repo: &str, on: bool) -> Value { json!({ "action": if repo.is_empty() { "pin-machine" } else { "pin-project" }, "id": "", "repo": repo, "on": on }) }
/// Ask the machine `m` to pin (or unpin) itself or its project: its answer (`msg`), or why not, in a sentence.
pub fn ask_there(m: &Machine, req: &Value) -> Res<Value> {
    match super::ask(m, "do", Some(req), 20000) {
        Err(why) => crate::fail!("{}", super::why::unanswered(&m.label, &why)),
        // Each answer of this door says it is one: an older program reads the action as a task's and cannot do it.
        Ok(j) if !j.b("pin") => crate::fail!("{} runs an older version of herdr-projects that cannot pin a machine or a project: update it there first.", m.label),
        Ok(j) if j.b("ok") => Ok(j),
        Ok(j) => crate::fail!("{}: {}", m.label, j.s("msg")),
    }
}
/// A machine the board knows, by its label.
pub fn machine(label: &str) -> Option<Machine> { super::machines().into_iter().find(|m| m.label.eq_ignore_ascii_case(label)) }
/// The same from the command line, the machine named by its label: what it answered, and its words.
pub fn command(m: &Machine, repo: &str, on: bool) -> Res<(Value, String)> {
    let j = ask_there(m, &request(repo, on))?;
    let mut out = obj(json!({ "pinned": on, "on": m.label }));
    out.put(if repo.is_empty() { "machine" } else { "repo" }, j.t("name"));
    let word = if on { "pinned" } else { "unpinned" };
    Ok((Value::Object(out), if repo.is_empty() { format!("{}: {word}", m.label) } else { format!("{} on {}: {word}", j.t("name"), m.label) }))
}
/// The door's side: this machine or a project of it pinned or unpinned, for the machine that asked.
pub fn door(req: &Value) -> Res<Value> {
    let project = (req.s("action") == "pin-project").then(|| req.t("repo"));
    Ok(match crate::actions::pins::pin_here(project.as_deref(), req.b("on")) {
        Ok((name, msg)) => json!({ "ok": true, "pin": true, "msg": msg, "name": name, "pinned": req.b("on") }),
        Err(e) => json!({ "ok": false, "pin": true, "msg": e.msg }),
    })
}

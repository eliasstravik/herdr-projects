//! A project of another machine archived or restored from here (the archive key on its row, `archive --repo <name> --on
//! <machine>`), and the same asked of this machine: through its door (`do`, `project-archive` / `project-restore`) it is
//! done there with that machine's own archive_project / restore_project, so its coordinator stops and its automations go
//! off there. A board asks from the tasks it last read of that machine (`seen`): they are read again there first, and a
//! change that would have changed the question refuses it, saying why (docs/board.md, "Archiving a project").
use super::Machine;
use crate::actions::archive_project::{archive_project, plan, restore_project};
use crate::err::Res;
use crate::json::*;
use crate::shown::words::repo_label;

pub fn is_project_action(action: &str) -> bool { ["project-archive", "project-restore"].contains(&action) }
/// What the asking board saw of the project when it asked: how many tasks, and whether it asked first (you said yes).
#[derive(Clone, Copy, Debug)]
pub struct Seen { pub tasks: usize, pub asked: bool }

/// Ask the machine `m` to archive (`archive`) or restore its project `repo`: its answer (`msg`, the ids), or why not, in
/// a sentence. `seen`: None from the command line, which archives as it does here.
pub fn ask_there(m: &Machine, archive: bool, repo: &str, seen: Option<Seen>) -> Res<Value> {
    let mut req = obj(json!({ "action": if archive { "project-archive" } else { "project-restore" }, "id": "", "repo": repo }));
    if let Some(v) = seen { req.put("seen", json!({ "tasks": v.tasks, "asked": v.asked })); }
    match super::ask(m, "do", Some(&Value::Object(req)), 60000) {
        Err(why) => crate::fail!("{}", super::why::unanswered(&m.label, &why)),
        // Each answer of this door says it is one: an older program reads the action as a task's and cannot do it.
        Ok(j) if !j.b("project") => crate::fail!("{} runs an older version of herdr-projects that cannot {} a project from another machine: update it there first.", m.label, if archive { "archive" } else { "restore" }),
        Ok(j) if j.b("ok") => Ok(j),
        Ok(j) => crate::fail!("{}: {}", m.label, j.s("msg")),
    }
}
/// The same from the command line, the machine named by its label: what it answered, and its words.
pub fn command(label: &str, archive: bool, repo: &str) -> Res<(Value, String)> {
    let Some(m) = super::machines().into_iter().find(|m| m.label.eq_ignore_ascii_case(label)) else {
        let known: Vec<String> = super::machines().into_iter().map(|m| m.label).collect();
        crate::fail!("{label} is not a machine the board knows (herdr machine list, or the board.machines setting). Known: {}.", if known.is_empty() { "none".into() } else { known.join(", ") });
    };
    let j = ask_there(&m, archive, repo, None)?;
    let mut out = obj(json!({ "repo": j.t("repo"), "on": m.label }));
    out.put(if archive { "archived" } else { "restored" }, j.at("ids").clone());
    Ok((Value::Object(out), format!("{}: {}", m.label, j.s("msg"))))
}
/// The door's side: this machine's project archived or restored, for the machine that asked.
pub fn door(req: &Value) -> Res<Value> {
    let answer = |r: Res<(String, Vec<String>)>, repo: String| Ok(match r {
        Ok((msg, ids)) => json!({ "ok": true, "project": true, "msg": msg, "ids": ids, "repo": repo }),
        Err(e) => json!({ "ok": false, "project": true, "msg": e.msg }),
    });
    let repo = req.t("repo");
    if req.s("action") == "project-restore" { return answer(restore_project(&repo), repo); }
    if let Some(why) = req.at("seen").is_object().then(|| changed(&repo, req.at("seen"))).flatten() { return answer(Err(why.into()), repo); }
    answer(archive_project(&repo), repo)
}
/// Why the project is not as the asking board saw it, in a way that changes its question: a task of it works, waits or
/// is to do now where it asked nothing, or it holds more tasks than it counted. None: as it was (or a change that asks
/// no less, a task that finished).
fn changed(repo: &str, seen: &Value) -> Option<String> {
    let p = plan(repo).ok()?;
    let (name, host) = (repo_label(&p.repo), super::check::hostname());
    let again = "Nothing was archived: look again, then archive it.";
    if p.asks() && !seen.b("asked") { return Some(format!("{name} changed on {host} since your board read it: {} {again}", p.says())); }
    let n = seen.i("tasks").max(0) as usize;
    if p.tasks() > n { return Some(format!("{name} changed on {host} since your board read it: it holds {} tasks now, not {n}. {again}", p.tasks())); }
    None
}

//! A task added on another machine (`herdr-projects add … --on <machine>`): asked there through its door (`add`), where
//! it is added as that machine adds its own, with the asking machine's work settings carried as for a dispatch, its own
//! settings, and, when a coordinator's autopilot added it, its autopilot record (the adding machine and the
//! `autopilot.repoFileWaits` in force), which that machine enforces. A machine on an older version has no such door:
//! the add is refused, saying to update it there.
use crate::err::Res;
use crate::json::*;

/// Asked of the machine `label`: Ok with that machine's answer (`id`, `msg`), or why not, in a sentence.
pub fn add_there(label: &str, title: &str, desc: &str, repo: &str, own: &[(String, Value)], delegate: bool, autopilot: Value) -> Res<Value> {
    let Some(m) = super::machines().into_iter().find(|m| m.label.eq_ignore_ascii_case(label)) else {
        let known: Vec<String> = super::machines().into_iter().map(|m| m.label).collect();
        crate::fail!("{label} is not a machine the board knows (herdr machine list, or the board.machines setting). Known: {}.", if known.is_empty() { "none".into() } else { known.join(", ") });
    };
    let own: Obj = own.iter().cloned().collect();
    let req = json!({ "title": title, "desc": desc, "repo": repo, "own": own, "delegate": delegate, "autopilot": autopilot,
        "settings": crate::store::settings::lookup::to_send(repo, &crate::store::settings::places::yours()), "from": super::check::hostname() });
    match super::ask(&m, "add", Some(&req), 60000) {
        Err(why) => crate::fail!("{}", super::why::unanswered(&m.label, &why)),
        Ok(j) if j.b("ok") => Ok(j),
        Ok(j) if j.s("msg").starts_with("Unknown request") => crate::fail!("{} runs an older version of herdr-projects that cannot add a task from another machine: update it there first.", m.label),
        Ok(j) => crate::fail!("{}: {}", m.label, j.s("msg")),
    }
}
/// The door's side: the task added here, for the machine that asked.
pub fn door(req: &Value) -> Res<Value> {
    let repo = req.t("repo");
    let here = crate::store::repos::folder_for(&repo, &crate::store::files::user_home())?;
    let own: Vec<(String, Value)> = req.at("own").as_object().cloned().unwrap_or_default().into_iter().collect();
    let t = crate::actions::filing::add(&here, req.s("title"), crate::actions::filing::AddOpts { description: req.t("desc"), repo, settings: own, ..Default::default() })?;
    let id = t.t("id");
    super::carried_with(&id, req)?;
    if req.at("autopilot").is_object() {
        let mut rec = req.o("autopilot").cloned().unwrap_or_default();
        rec.put("machine", req.t("from").chars().take(64).collect::<String>());
        crate::store::tasks::set(&id, json!({ "autopilot": rec }))?;
    }
    let started = if req.b("delegate") { crate::actions::start::delegate(&id, vec![])?; ", an agent is on it" } else { "" };
    Ok(json!({ "ok": true, "id": id, "msg": format!("{id}: added on {}{started}", super::check::hostname()) }))
}

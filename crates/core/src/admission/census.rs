//! Strict task inventory and the admission snapshot: unreadable or unknown task data never becomes an empty board.
use crate::err::{Error, Res};
use crate::json::*;
use crate::store::files::{file_of, read_obj, tasks_dir, with_lock_wait, SLOT_STAGES, STAGES};
use crate::store::settings::lookup::setting;
use crate::store::tasks::Task;

pub fn tasks() -> Res<Vec<Task>> {
    let mut out = vec![];
    for repo in std::fs::read_dir(tasks_dir())? {
        let repo = repo?;
        if repo.file_type()?.is_symlink() { return Err(Error::new("Unknown task inventory link")); }
        if !repo.file_type()?.is_dir() { continue; }
        for number in std::fs::read_dir(repo.path())? {
            let number = number?;
            let name = number.file_name().to_string_lossy().into_owned();
            let id = format!("{}/{}", repo.file_name().to_string_lossy(), name);
            if name.is_empty() || !(name.bytes().all(|b| b.is_ascii_digit()) || crate::store::attempts::is_attempt_id(&id)) { continue; }
            if !number.file_type()?.is_dir() { return Err(Error::new("Unreadable task inventory entry")); }
            let t = read_obj(&file_of(&id)).ok_or_else(|| Error::new("Unreadable or invalid task in admission inventory"))?;
            if t.s("id") != id || !STAGES.contains(&t.s("stage")) { return Err(Error::new("Unknown task identity or stage in admission inventory")); }
            if !t.b("archivedAt") { out.push(t); }
        }
    }
    Ok(out)
}

pub fn occupies_slot(t: &Task) -> bool { SLOT_STAGES.contains(&t.s("stage")) && !t.b("autoMerge") }

pub fn status() -> Res<Value> {
    with_lock_wait("slots", 5000, || {
        let tasks = tasks()?;
        let active: Vec<Value> = tasks.iter().filter(|t| occupies_slot(t)).map(|t| json!({ "id": t.at("id"), "stage": t.at("stage") })).collect();
        let waiting: Vec<Value> = tasks.iter().filter(|t| ["queued", "after"].contains(&t.s("stage"))).map(|t| json!({
            "id": t.at("id"), "stage": t.at("stage"), "state": "queued",
            "reason": if t.b("resourceHold") { t.s("resourceHold") } else if t.s("stage") == "after" { "dependency" } else { "slot" }
        })).collect();
        Ok(json!({ "schema": 1, "limit": setting("board.agents", None), "active": active, "waiting": waiting }))
    })
}

//! A coordinator's saved state (one file per herdr session and coordinator) and the folder it chats in.
use super::co::{Co, Level};
use crate::err::Res;
use crate::herdr::cli::session_name;
use crate::json::*;
use crate::store::files::{data_dir, marker_alive, read_obj, with_lock_wait, write_json};
use std::path::PathBuf;

/// The top-level Coordinator's folder holds the others': `machine/`, and `projects/<repo>/` for each repo.
pub fn dir_of(co: impl Into<Co>) -> PathBuf {
    let co = co.into();
    let top = data_dir().join("coordinator").join(session_name(&co.socket));
    match &co.level { Level::Top => top, Level::Machine => top.join("machine"), Level::Project(_) => top.join("projects").join(co.suffix().trim_start_matches("-project-")) }
}
fn file_of(co: &Co) -> PathBuf { dir_of(co).join("state.json") }
pub fn get(co: impl Into<Co>) -> Option<Obj> { read_obj(&file_of(&co.into())) }
/// A pump (the detached job that starts it and types into it) is at work for this coordinator.
pub fn pumping(c: &Obj) -> bool { c.at("pump").is_object() && marker_alive(c.at("pump")) }
/// The one way its state changes: under its lock. `f` returning false writes nothing.
pub fn change(co: impl Into<Co>, f: impl FnOnce(&mut Obj) -> bool) -> Res<Obj> {
    let co = co.into();
    with_lock_wait(&format!("coordinator:{}{}", session_name(&co.socket), co.suffix()), 5000, || {
        let mut c = get(&co).unwrap_or_else(|| obj(json!({ "queue": [] })));
        if !f(&mut c) { return Ok(c); }
        if let Some(r) = co.repo() { c.put("repo", r); }
        write_json(&file_of(&co), &Value::Object(c.clone()))?;
        Ok(c)
    })
}
/// Where it runs. The top-level one and the machine's: a folder of their own (HERDR_PROJECTS_COORDINATOR_DIR names
/// another for the top-level one, used for nothing else); a folder nothing else chats in is how a conversation is told
/// from any other. A project coordinator: its repo's own folder, at its main level.
pub fn home(co: impl Into<Co>) -> PathBuf {
    let co = co.into();
    match &co.level {
        Level::Top => std::env::var("HERDR_PROJECTS_COORDINATOR_DIR").ok().filter(|s| !s.is_empty()).map(PathBuf::from).unwrap_or_else(|| dir_of(&co).join("chat")),
        Level::Machine => dir_of(&co).join("chat"),
        Level::Project(_) => co.root().unwrap_or_else(|| dir_of(&co).join("chat")),
    }
}
/// Every coordinator of this herdr session: the top-level one, and each other one that was ever talked to.
pub fn all(socket: &str) -> Vec<Co> {
    let mut out = vec![Co::top(socket)];
    if get(Co::machine(socket)).is_some() { out.push(Co::machine(socket)); }
    let projects = dir_of(Co::top(socket)).join("projects");
    let mut names: Vec<String> = std::fs::read_dir(&projects).map(|d| d.flatten().filter(|e| e.path().join("state.json").exists()).map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    names.sort();
    // The folder is named after the repo; the repo's own name is in its state.
    for n in names { let c = read_obj(&projects.join(&n).join("state.json")).unwrap_or_default(); let repo = if c.b("repo") { c.t("repo") } else { n }; out.push(Co::project(socket, &repo)); }
    out
}

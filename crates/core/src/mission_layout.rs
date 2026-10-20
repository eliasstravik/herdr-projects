//! Mission Control's persistent pane and its owned CLI panes, all tiled in one Herdr tab.
use crate::err::Res;
use crate::herdr::Herdr;
use crate::herdr::cli::session_name;
use crate::json::*;
use crate::store::files::{data_dir, marker, marker_alive, read_obj, with_lock_wait, write_json};

pub fn file(socket: &str) -> std::path::PathBuf { data_dir().join(format!("mission-control-{}.json", session_name(socket))) }
pub fn state(socket: &str) -> Obj { read_obj(&file(socket)).unwrap_or_default() }
pub fn anchor(socket: &str) -> Option<String> {
    if let Some(folder) = std::env::var("HERDR_CONTROL_PROJECT_PATH").ok() { return project_anchor(socket, &folder); }
    let c = state(socket);
    (c.b("pane") && marker_alive(c.at("owner"))).then(|| c.t("pane"))
}
pub fn project_anchor(socket: &str, folder: &str) -> Option<String> {
    let c = state(socket);
    let key = project_key(folder);
    let p = c.at("projects").get(&key)?;
    (p.b("pane") && marker_alive(p.at("owner"))).then(|| p.t("pane"))
}
pub fn register_project(socket: &str, folder: &str, workspace: &str, pane: &str) -> Res<()> {
    let mut c = state(socket);
    let key = project_key(folder);
    let mut projects = c.o("projects").cloned().unwrap_or_default();
    let mut p = projects.get(&key).and_then(Value::as_object).cloned().unwrap_or_default();
    if p.s("pane") != pane { p.put("children", json!([])); }
    p.put("folder", folder); p.put("workspace", workspace); p.put("pane", pane);
    p.put("owner", marker());
    projects.insert(key, Value::Object(p)); c.put("projects", Value::Object(projects));
    write_json(&file(socket), &Value::Object(c))
}
pub fn prepare_project(socket: &str, folder: &str, workspace: &str, pane: &str) -> Res<()> {
    let mut c = state(socket); let key = project_key(folder);
    let mut projects = c.o("projects").cloned().unwrap_or_default();
    let mut p = projects.get(&key).and_then(Value::as_object).cloned().unwrap_or_default();
    if p.s("pane") != pane { p.put("children", json!([])); p.put("owner", Value::Null); }
    p.put("folder", folder); p.put("workspace", workspace); p.put("pane", pane);
    projects.insert(key, Value::Object(p)); c.put("projects", Value::Object(projects));
    write_json(&file(socket), &Value::Object(c))
}
pub fn project_workspace(socket: &str, folder: &str) -> Option<String> {
    state(socket).at("projects").get(&project_key(folder)).map(|p| p.t("workspace")).filter(|s| !s.is_empty())
}
pub fn project_key(folder: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new(); folder.hash(&mut h); format!("{:016x}", h.finish())
}
pub fn register(socket: &str, pane: &str) -> Res<()> {
    let mut c = state(socket);
    if c.s("pane") != pane { c.put("children", json!([])); }
    c.put("pane", pane); c.put("owner", marker());
    write_json(&file(socket), &Value::Object(c))
}
pub fn selected(socket: &str, folder: &str) -> Res<()> {
    let mut c = state(socket); c.put("selected", folder); write_json(&file(socket), &Value::Object(c))
}
/// The left control pane keeps its width. Further CLIs split the largest owned pane on its right.
pub fn split(api: &Herdr, cwd: &str, env: &[(String, String)]) -> Res<Option<Value>> {
    let socket = api.socket_path();
    let folder = std::env::var("HERDR_CONTROL_PROJECT_PATH").unwrap_or_default();
    let Some(root) = (if folder.is_empty() { anchor(&socket) } else { project_anchor(&socket, &folder) }) else { return Ok(None) };
    with_lock_wait(&format!("mission-layout:{}", session_name(&socket)), 10000, || {
        let Some(control) = api.pane_get(&root) else { crate::fail!("Mission Control ist geschlossen. Bitte erneut öffnen.") };
        let layout = api.pane_layout(&root).unwrap_or(Value::Null);
        let mut c = state(&socket);
        let mut children: Vec<Value> = c.arr("children").iter().filter(|id| api.pane_get(&text(id)).map(|p| p.s("tab_id") == control.s("tab_id")).unwrap_or(false)).cloned().collect();
        let largest = layout.arr("panes").iter().filter(|p| children.iter().any(|id| text(id) == p.s("pane_id")))
            .max_by_key(|p| p.at("rect").i("width") * p.at("rect").i("height"));
        let (target, direction) = match largest {
            Some(p) => (p.s("pane_id"), if p.at("rect").i("width") > p.at("rect").i("height") * 2 { "right" } else { "down" }),
            None => (root.as_str(), "right"),
        };
        let pane = api.pane_split(target, cwd, direction, env)?;
        if !pane.b("pane_id") { crate::fail!("Herdr hat kein Terminal-Pane geöffnet.") }
        children.push(pane.at("pane_id").clone()); c.put("children", children);
        write_json(&file(&socket), &Value::Object(c))?;
        Ok(Some(pane))
    })
}

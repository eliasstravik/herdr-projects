//! Mission Control's persistent pane and its owned CLI panes, all tiled in one Herdr tab.
use crate::err::Res;
use crate::herdr::Herdr;
use crate::herdr::cli::session_name;
use crate::json::*;
use crate::store::files::{data_dir, marker, marker_alive, read_obj, with_lock_wait, write_json};

pub fn file(socket: &str) -> std::path::PathBuf { data_dir().join(format!("mission-control-{}.json", session_name(socket))) }
pub fn state(socket: &str) -> Obj { read_obj(&file(socket)).unwrap_or_default() }
pub fn anchor(socket: &str) -> Option<String> {
    let c = state(socket);
    (c.b("pane") && marker_alive(c.at("owner"))).then(|| c.t("pane"))
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
    let Some(root) = anchor(&socket) else { return Ok(None) };
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

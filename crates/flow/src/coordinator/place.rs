//! Where a coordinator lives in herdr: a tab of its own, named after it. The top-level Coordinator's (`Coordinator`)
//! and the machine coordinator's are in the home space (`~`); a project coordinator's is in its repo's own space. A
//! top-level one still in the space of its own it had before is moved over. Checked on herdr 0.9.3: a space is labelled after the folder of
//! its first tab's pane, so the space the tab was last put in is remembered; `pane move --new-tab` carries a running
//! agent over as it is, gives the pane a new id, and closes the space it left when that was its last pane.
use super::co::Co;
use super::state::{change, get, home, pumping};
use crate::err::Res;
use crate::flow::spaces::real;
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::files::user_home;

pub const COORDINATOR_TAB: &str = "Coordinator";
pub const HOME_SPACE: &str = "~";
/// `space` (the home space it was last in) is kept: it is how that space is found again under another label.
pub fn nowhere(c: &mut Obj) { for k in ["workspace", "tab", "pane", "terminal", "agent", "paneWas"] { c.put(k, Value::Null); } }
/// The home space's id: the one labelled `~`, else the one the tab was last in, else a new one. Never a task's space.
pub fn home_space(api: &Herdr, c: Option<&Obj>) -> Res<String> {
    let list: Vec<Value> = api.workspaces()?.into_iter().filter(|w| !w.at("worktree").is_object()).collect();
    if let Some(w) = list.iter().find(|w| w.s("label") == HOME_SPACE).or_else(|| c.and_then(|c| list.iter().find(|w| c.b("space") && w.s("workspace_id") == c.s("space")))) { return Ok(w.t("workspace_id")); }
    Ok(api.workspace_create(&user_home().display().to_string(), "", &[])?.at("workspace").t("workspace_id"))
}
/// A project coordinator's space: the one it was last put in, else the repo's own (labelled after it, as herdr labels a
/// space after its folder), else a new one at the repo's folder. The home folder's is the home space.
fn space_for(api: &Herdr, co: &Co, c: Option<&Obj>) -> Res<String> {
    let Some(root) = co.root().filter(|r| real(r) != real(&user_home())) else { return home_space(api, c) };
    let list: Vec<Value> = api.workspaces()?.into_iter().filter(|w| !w.at("worktree").is_object()).collect();
    let base = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let repo = co.repo().unwrap_or_default();
    if let Some(w) = list.iter().find(|w| c.map(|c| c.b("space") && w.s("workspace_id") == c.s("space")).unwrap_or(false)).or_else(|| list.iter().find(|w| w.s("label") == repo || w.s("label") == base)) { return Ok(w.t("workspace_id")); }
    Ok(api.workspace_create(&root.display().to_string(), repo, &[])?.at("workspace").t("workspace_id"))
}
/// Not written down (the state file was lost) but there: the pane in the coordinator's own folder. A project
/// coordinator's folder is the human's repo: only a pane in a tab named after it.
fn adopt(api: &Herdr, co: &Co) -> Option<Value> {
    let h = real(&home(co));
    let tab = co.name();
    let label = |p: &Value| api.tab_get(p.s("tab_id")).map(|t| t.t("label")).unwrap_or_default();
    let here: Vec<Value> = api.panes(None).ok()?.into_iter().filter(|p| real(std::path::Path::new(p.s("cwd"))) == h && (co.repo().is_none() || label(p) == tab)).collect();
    let alone = |p: &Value| api.tab_get(p.s("tab_id")).map(|t| t.s("label") == tab && t.i("pane_count") == 1).unwrap_or(false);
    here.iter().find(|p| p.b("agent")).cloned().or_else(|| here.iter().find(|p| api.pane_idle(p.s("pane_id")) && alone(p)).cloned())
}
fn put(c: &mut Obj, co: &Co, space: Value, pane: &Value, was: Value) {
    c.put("socket", co.socket.as_str()); c.put("workspace", Value::Null); c.put("space", space); c.put("tab", pane.at("tab_id").clone()); c.put("pane", pane.at("pane_id").clone()); c.put("terminal", pane.at("terminal_id").clone()); c.put("paneWas", was);
}
/// Its tab, found or opened: one pane, in its own folder. Returns the pane.
pub fn open_tab(api: &Herdr, co: &Co, env: &[(String, String)]) -> Res<Value> {
    let mut pane = adopt(api, co);
    let mut old = None;
    if let Some(p) = pane.as_ref().filter(|_| co.is_top()) { if let Some(w) = api.workspace_get(p.s("workspace_id")) { if w.s("label") == COORDINATOR_TAB && !w.at("worktree").is_object() && w.i("pane_count") == 1 { old = Some(w.t("workspace_id")); } } }
    let space = match &pane { Some(p) => p.t("workspace_id"), None => space_for(api, co, get(co).as_ref())? };
    if pane.is_none() {
        let r = api.tab_create(&space, &home(co).display().to_string(), &co.name(), env)?;
        pane = if r.at("root_pane").is_object() { Some(r.at("root_pane").clone()) } else { api.panes(Some(&space))?.into_iter().find(|p| p.s("tab_id") == r.at("tab").s("tab_id")) };
        if pane.is_none() { if r.at("tab").b("tab_id") { api.tab_close(r.at("tab").s("tab_id")); } crate::fail!("herdr opened no terminal for the {}", co.name()); }
    }
    let pane = pane.unwrap();
    change(co, |c| {
        let sp = if old.is_some() { c.at("space").clone() } else { json!(space) };
        put(c, co, sp, &pane, Value::Null);
        if let Some(o) = &old { c.put("workspace", o.clone()); } // moved over by whoever looks next (move_over)
        if !c.b("run") && pane.b("agent") { c.put("run", json!({ "kind": pane.at("agent") })); } // found running
        true
    })?;
    Ok(pane)
}
/// From before it lived in a tab: the pane is moved into a `Coordinator` tab of the home space, with its program running
/// on. Returns the pane where it is now, or None. `quiet`: not while a pump is typing into it.
pub fn move_over(api: &Herdr, co: &Co, quiet: bool) -> Res<Option<Value>> {
    let mut pane = None;
    let mut err = None;
    change(co, |c| {
        if !c.b("workspace") || (quiet && pumping(c)) { return false; }
        let Some(old) = (if c.b("pane") { api.pane_get(c.s("pane")) } else { None }) else { nowhere(c); return true }; // closed meanwhile
        let space = match home_space(api, Some(c)) { Ok(s) => s, Err(e) => { err = Some(e); return false } };
        match api.pane_move(old.s("pane_id"), &space, COORDINATOR_TAB) {
            Ok(m) => { let p = m.at("pane").clone(); let was = c.at("pane").clone(); put(c, co, json!(space), &p, was); pane = Some(p); true }
            Err(e) => { err = Some(e.into()); false }
        }
    })?;
    if let Some(e) = err { return Err(e); }
    Ok(pane)
}
/// Close it: its tab, never the home space around it.
pub fn close_place(api: &Herdr, c: &Obj) {
    if c.b("workspace") { if c.b("pane") { api.pane_close(c.s("pane")); } return; }
    if c.b("tab") && c.b("pane") && api.pane_get(c.s("pane")).map(|p| p.s("tab_id") == c.s("tab")).unwrap_or(false) { api.tab_close(c.s("tab")); }
    else if c.b("pane") { api.pane_close(c.s("pane")); }
}

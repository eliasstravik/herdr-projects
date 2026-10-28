//! herdr spaces the board opened and owes closing: a repo's own space, a done task's clean-up, spaces left over.
//! Asked for a worktree of a repo that has no workspace open, herdr also opens the repo itself as one. Nobody asked
//! for that space, so it is remembered here and closed again with the repo's last task space. One that existed
//! before, or that you have since started using, is yours and is left alone.
use crate::herdr::cli::{args, same_socket, socket_exists};
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::files::{data_dir, read_json, read_obj, write_json};
use crate::store::repos::{folder_of, git_out};
use crate::store::settings::lookup::setting;
use std::path::{Path, PathBuf};

fn spaces_file() -> PathBuf { data_dir().join("spaces.json") }
pub fn real(p: &Path) -> PathBuf { std::fs::canonicalize(p).unwrap_or_else(|_| crate::store::files::absolute(p)) }
fn repo_spaces(api: &Herdr, root: &Path) -> Vec<Value> {
    api.workspaces().unwrap_or_default().into_iter().filter(|w| w.at("worktree").is_object() && !w.at("worktree").b("is_linked_worktree") && real(Path::new(w.at("worktree").s("checkout_path"))) == real(root)).collect()
}
/// Run a `herdr worktree create` or `open` and note the repo space it made, if it made one.
pub fn opening_worktree<T>(api: &Herdr, root: &Path, f: impl FnOnce() -> T) -> T {
    let before: Vec<String> = repo_spaces(api, root).iter().map(|w| w.t("workspace_id")).collect();
    let res = f();
    if let Some(made) = repo_spaces(api, root).into_iter().find(|w| !before.contains(&w.t("workspace_id"))) {
        let mut all = read_obj(&spaces_file()).unwrap_or_default();
        all.put(&format!("{}|{}", api.socket_path(), real(root).display()), made.at("workspace_id").clone());
        let _ = write_json(&spaces_file(), &Value::Object(all));
    }
    res
}
fn cleanup_on(task: Option<&Obj>) -> bool { setting("workspace.cleanup", task) == json!(true) }
/// The housekeeping pass looks again at the ones that could not be closed when their last task ended.
pub fn sweep_repo_spaces(socket: &str) {
    if !cleanup_on(None) { return; } // off for you: nothing is closed behind your back
    let all = read_obj(&spaces_file()).unwrap_or_default();
    let prefix = format!("{socket}|");
    let mine: Vec<String> = all.keys().filter(|k| k.starts_with(&prefix)).cloned().collect();
    if mine.is_empty() { return; }
    let api = Herdr::new(Some(socket.to_string()));
    for k in mine { close_repo_space(&api, Path::new(&k[prefix.len()..])); }
}
pub fn close_repo_space(api: &Herdr, root: &Path) {
    let key = format!("{}|{}", api.socket_path(), real(root).display());
    let mut all = read_obj(&spaces_file()).unwrap_or_default();
    let id = all.t(&key);
    if id.is_empty() { return; }
    let mut forget = || { all.shift_remove(&key); let _ = write_json(&spaces_file(), &Value::Object(all.clone())); };
    let Ok(list) = api.workspaces() else { return };
    let Some(w) = list.iter().find(|x| x.s("workspace_id") == id) else { return forget() };
    if !w.at("worktree").is_object() || w.at("worktree").b("is_linked_worktree") || real(Path::new(w.at("worktree").s("checkout_path"))) != real(root) { return forget(); }
    if list.iter().any(|x| x.at("worktree").b("is_linked_worktree") && real(Path::new(x.at("worktree").s("repo_root"))) == real(root)) { return; } // another task's space is still open
    if w.b("focused") { return; } // you are looking at it: the next task that ends closes it
    let Ok(panes) = api.panes(Some(&id)) else { return };
    let used = panes.len() != 1 || w.i("tab_count").max(1) > 1 || api.agent_get(panes[0].s("pane_id")).ok().flatten().is_some()
        || real(Path::new(if panes[0].b("foreground_cwd") { panes[0].s("foreground_cwd") } else { panes[0].s("cwd") })) != real(root) || !api.pane_idle(panes[0].s("pane_id"));
    if !used { let _ = api.workspace_close(&id); }
    forget();
}
/// A done task's clean-up is owed from the moment it is done: this note goes onto the task in the same write that
/// makes it done. `tip`: the commit this is about, when known; a branch that has moved past it holds work that landed
/// nowhere, and clean-up keeps it.
pub fn tidy_mark(t: &Obj, delete_branch: &str, tip: &str) -> Option<Value> {
    if !cleanup_on(Some(t)) || t.b("archivedAt") { return None; } // off: nothing is closed behind your back
    let mut tip = tip.to_string();
    if tip.is_empty() && t.b("branch") { tip = git_out(&folder_of(t.s("id")), &["rev-parse", "--verify", &format!("refs/heads/{}", t.s("branch"))]).unwrap_or_default(); }
    Some(json!({ "fresh": true, "deleteBranch": delete_branch, "tip": tip, "eventSeq": t.i("eventSeq"), "quietFrom": 0 }))
}
// ---- spaces left to close: a task that starts a new round while its old space could not be closed yet lets go of it here.
fn closing_file() -> PathBuf { data_dir().join("closing.json") }
pub fn close_later(socket: &str, workspace: &str) {
    if socket.is_empty() || workspace.is_empty() { return; }
    let mut all: Vec<Value> = read_json(&closing_file()).and_then(|v| v.as_array().cloned()).unwrap_or_default();
    all.retain(|x| !(same_socket(x.s("socket"), socket) && x.s("workspace") == workspace));
    all.push(json!({ "socket": socket, "workspace": workspace }));
    let keep: Vec<Value> = all.iter().skip(all.len().saturating_sub(200)).cloned().collect();
    let _ = write_json(&closing_file(), &Value::Array(keep));
    sweep_closing(socket);
}
pub fn sweep_closing(socket: &str) {
    if !cleanup_on(None) { return; }
    let all: Vec<Value> = read_json(&closing_file()).and_then(|v| v.as_array().cloned()).unwrap_or_default();
    if !all.iter().any(|x| same_socket(x.s("socket"), socket)) || (crate::herdr::cli::fake().is_none() && !socket_exists(socket)) { return; }
    let api = Herdr::new(Some(socket.to_string()));
    let closed = |ws: &str| -> bool {
        match api.call(args(&["workspace", "get", ws])) {
            Err(e) => e.code.contains("not_found"),
            Ok(_) => {
                let Ok(panes) = api.panes(Some(ws)) else { return false };
                // status "unknown" may be an agent at work
                let busy = panes.iter().any(|p| match api.agent_get(p.s("pane_id")) { Ok(Some(a)) => !["idle", "done"].contains(&a.s("agent_status")) || !api.pane_childless(p.s("pane_id"), a.s("agent")), Ok(None) => !api.pane_idle(p.s("pane_id")), Err(_) => true });
                if busy { return false; }
                api.workspace_close(ws).is_ok()
            }
        }
    };
    let left: Vec<Value> = all.iter().filter(|x| !same_socket(x.s("socket"), socket) || !closed(x.s("workspace"))).cloned().collect();
    if left.len() != all.len() { let _ = write_json(&closing_file(), &Value::Array(left)); }
}

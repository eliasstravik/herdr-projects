//! Where an attempt works: a worktree and a branch of its own (`work/12a-…`), and a tab of its own (`Agent A`) in the
//! task's one herdr space. The first attempt to start makes that space, as herdr makes any worktree's; each later one
//! gets a worktree beside it and a tab in the same space.
use crate::err::Res;
use crate::flow;
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::attempts::attempts_of;
use crate::store::files::{board_env, data_dir, with_lock_wait};
use crate::store::repos::{cwd_of, git_slow};
use crate::store::tasks::{get, update_wait, Change, Task};
use std::path::{Path, PathBuf};

/// Answers as `herdr worktree create` does: { workspace, tab, root_pane, worktree: { path } }. One attempt at a time
/// per task: two that start together must not both make the space.
pub fn open_place(api: &Herdr, root: &Path, t: &Task, branch: &str, from: &str, destination: Option<&Path>) -> Res<Value> {
    with_lock_wait(&format!("space:{}", t.s("attemptOf")), 180000, || {
        let p = get(t.s("attemptOf"));
        let space = p.as_ref().filter(|p| p.at("link").b("workspace")).and_then(|p| api.workspace_get(p.at("link").s("workspace")));
        let Some(space) = space else {
            let label: String = p.as_ref().map(|p| p.t("title")).unwrap_or_else(|| t.t("title")).chars().take(60).collect();
            let res = flow::spaces::opening_worktree(api, root, || api.worktree_create(&root.display().to_string(), branch, from, &label, destination))?;
            crate::worktree_destination::record(root, t, destination, branch, &res)?;
            let dir = if res.at("worktree").b("path") { res.at("worktree").t("path") } else { res.at("workspace").at("worktree").t("checkout_path") };
            let ws = res.at("workspace").t("workspace_id");
            update_wait(t.s("attemptOf"), 10000, |p| { p.obj_mut("link").put("workspace", ws.clone()); p.obj_mut("link").put("ownsWorkspace", true); p.put("spaceDir", Path::new(&dir).parent().map(|x| x.display().to_string()).unwrap_or_default()); Ok(Change::Write) })?;
            return Ok(res);
        };
        // Beside the first one's worktree, named as herdr names one: the branch with its slashes as dashes.
        let p = p.unwrap_or_default();
        let base: PathBuf = if p.b("spaceDir") { PathBuf::from(p.s("spaceDir")) } else {
            attempts_of(&p).iter().find(|c| c.b("worktree")).and_then(|c| Path::new(c.s("worktree")).parent().map(|x| x.to_path_buf())).unwrap_or_else(|| data_dir().join("worktrees").join(t.s("repo")))
        };
        let name: String = branch.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '-' }).collect();
        let dir = match destination { Some(p) => p.to_path_buf(), None => crate::worktree_destination::reserve(root, &base, &name)? };
        git_slow(root, &["worktree", "add", "--no-track", "-b", branch, &dir.display().to_string(), from], 120000)?;
        let ws = space.t("workspace_id");
        let r = api.tab_create(&ws, &dir.display().to_string(), &flow::tabs::agent_tab(Some(t)), &board_env())?;
        let tab = if r.at("tab").b("tab_id") { r.at("tab").t("tab_id") } else { r.at("root_pane").t("tab_id") };
        let pane = if r.at("root_pane").is_object() { Some(r.at("root_pane").clone()) } else { api.panes(Some(&ws))?.into_iter().find(|x| x.s("tab_id") == tab) };
        let Some(pane) = pane else { if !tab.is_empty() { api.tab_close(&tab); } crate::fail!("herdr opened no terminal for attempt {}", t.s("letter")) };
        Ok(json!({ "workspace": { "workspace_id": ws }, "tab": { "tab_id": if tab.is_empty() { pane.t("tab_id") } else { tab } }, "root_pane": pane, "worktree": { "path": dir.display().to_string() } }))
    })
}
/// A tab for an attempt whose terminal is gone, in the task's space, in its own worktree.
pub fn reopen_place(api: &Herdr, t: &Task) -> Res<Option<Value>> {
    let ws = if t.at("link").b("workspace") { t.at("link").t("workspace") } else { get(t.s("attemptOf")).map(|p| p.at("link").t("workspace")).unwrap_or_default() };
    if ws.is_empty() || api.workspace_get(&ws).is_none() { return Ok(None); }
    let r = api.tab_create(&ws, &cwd_of(t).display().to_string(), &flow::tabs::agent_tab(Some(t)), &board_env())?;
    if r.at("root_pane").is_object() { return Ok(Some(r.at("root_pane").clone())); }
    Ok(api.panes(Some(&ws))?.into_iter().find(|x| x.s("tab_id") == r.at("tab").s("tab_id")))
}

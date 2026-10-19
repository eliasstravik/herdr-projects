//! Where an attempt works: a worktree and branch of its own (`work/12a-…`) and a tiled pane in the task's workspace.
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
pub fn open_place(api: &Herdr, root: &Path, t: &Task, branch: &str, from: &str) -> Res<Value> {
    with_lock_wait(&format!("space:{}", t.s("attemptOf")), 180000, || {
        let p = get(t.s("attemptOf"));
        let space = p.as_ref().filter(|p| p.at("link").b("workspace")).and_then(|p| api.workspace_get(p.at("link").s("workspace")));
        let Some(space) = space else {
            let label: String = p.as_ref().map(|p| p.t("title")).unwrap_or_else(|| t.t("title")).chars().take(60).collect();
            let res = flow::spaces::opening_worktree(api, root, || api.worktree_create(&root.display().to_string(), branch, from, &label))?;
            let dir = if res.at("worktree").b("path") { res.at("worktree").t("path") } else { res.at("workspace").at("worktree").t("checkout_path") };
            let ws = res.at("workspace").t("workspace_id");
            let pane = res.at("root_pane").clone();
            update_wait(t.s("attemptOf"), 10000, |p| {
                let l = p.obj_mut("link");
                l.put("workspace", ws.clone()); l.put("ownsWorkspace", true);
                l.put("attemptAnchorPane", pane.at("pane_id").clone());
                p.put("spaceDir", Path::new(&dir).parent().map(|x| x.display().to_string()).unwrap_or_default());
                Ok(Change::Write)
            })?;
            return Ok(res);
        };
        // Beside the first one's worktree, named as herdr names one: the branch with its slashes as dashes.
        let p = p.unwrap_or_default();
        let base: PathBuf = if p.b("spaceDir") { PathBuf::from(p.s("spaceDir")) } else {
            attempts_of(&p).iter().find(|c| c.b("worktree")).and_then(|c| Path::new(c.s("worktree")).parent().map(|x| x.to_path_buf())).unwrap_or_else(|| data_dir().join("worktrees").join(t.s("repo")))
        };
        let name: String = branch.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '-' }).collect();
        let dir = base.join(name);
        if !dir.exists() { git_slow(root, &["worktree", "add", "--no-track", "-b", branch, &dir.display().to_string(), from], 120000)?; }
        let ws = space.t("workspace_id");
        let anchor = if p.at("link").b("attemptAnchorPane") { p.at("link").t("attemptAnchorPane") } else { p.at("link").t("pane") };
        let pane = api.pane_split(&anchor, &dir.display().to_string(), "down", &board_env())?;
        if !pane.b("pane_id") { crate::fail!("herdr opened no terminal pane for attempt {}", t.s("letter")) }
        Ok(json!({ "workspace": { "workspace_id": ws }, "tab": { "tab_id": pane.t("tab_id") }, "root_pane": pane, "worktree": { "path": dir.display().to_string() } }))
    })
}
/// A pane for an attempt whose terminal is gone, in the task's space and its own worktree.
pub fn reopen_place(api: &Herdr, t: &Task) -> Res<Option<Value>> {
    let ws = if t.at("link").b("workspace") { t.at("link").t("workspace") } else { get(t.s("attemptOf")).map(|p| p.at("link").t("workspace")).unwrap_or_default() };
    if ws.is_empty() || api.workspace_get(&ws).is_none() { return Ok(None); }
    let Some(parent) = get(t.s("attemptOf")) else { return Ok(None) };
    let anchor = if parent.at("link").b("attemptAnchorPane") { parent.at("link").t("attemptAnchorPane") } else { parent.at("link").t("pane") };
    let pane = api.pane_split(&anchor, &cwd_of(t).display().to_string(), "down", &board_env())?;
    Ok(pane.b("pane_id").then_some(pane))
}

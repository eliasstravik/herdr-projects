//! A task's worktree on Rex: Rex has no worktrees of its own, so the board makes it with git (`git worktree add`) in
//! the data folder (`worktrees/<repo>/<branch>`), then opens a space with its window in that folder (`-c`). Removing
//! it closes the space's windows and removes the worktree with git. The answers have herdr's shape.
use super::{registry, run, Rex};
use crate::herdr::cli::HRes;
use crate::json::*;
use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &str, a: &[&str]) -> Result<String, String> {
    let o = Command::new("git").arg("-C").arg(dir).args(a).output().map_err(|e| e.to_string())?;
    if o.status.success() { Ok(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { Err(String::from_utf8_lossy(&o.stderr).trim().to_string()) }
}
/// The repo's main folder, for any folder in it or in one of its worktrees.
pub fn repo_root(cwd: &str) -> Option<String> {
    let common = git(cwd, &["rev-parse", "--path-format=absolute", "--git-common-dir"]).ok()?;
    Some(common.strip_suffix("/.git").map(String::from).unwrap_or_else(|| git(cwd, &["rev-parse", "--show-toplevel"]).unwrap_or(common)))
}
fn dest(root: &str, branch: &str) -> PathBuf {
    let safe: String = branch.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '-' }).collect();
    crate::store::files::data_dir().join("worktrees").join(Path::new(root).file_name().unwrap_or_default()).join(safe)
}
fn linked(path: &str, root: &str) -> Value { json!({ "checkout_path": path, "repo_root": root, "is_linked_worktree": true }) }

impl Rex {
    pub fn tree_create(&self, cwd: &str, branch: &str, base: &str, label: &str) -> HRes<Value> {
        let root = repo_root(cwd).ok_or_else(|| run::err("worktree_create_failed", format!("{cwd} is not in a git repo")))?;
        let ds = dest(&root, branch).display().to_string();
        let exists = git(&root, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).is_ok();
        let base = if base.is_empty() { "HEAD" } else { base };
        let r = if exists { git(&root, &["worktree", "add", &ds, branch]) } else { git(&root, &["worktree", "add", "--no-track", "-b", branch, &ds, base]) };
        r.map_err(|e| run::err("worktree_create_failed", e))?;
        let mut made = self.space_create(&ds, label, &[], linked(&ds, &root))?;
        made["type"] = json!("worktree_created");
        made["worktree"] = json!({ "path": ds, "branch": branch, "open_workspace_id": made.at("workspace").at("workspace_id") });
        Ok(made)
    }
    /// A space on a worktree that is already there: the open one when there is one.
    pub fn tree_open(&self, _cwd: &str, path: &str, label: &str) -> HRes<Value> {
        let root = repo_root(path).ok_or_else(|| run::err("worktree_open_failed", format!("{path} is not in a git repo")))?;
        if let Some(w) = self.spaces()?.into_iter().find(|w| w.s("cwd") == path) {
            let p = self.blocks(Some(w.s("workspace_id")))?.into_iter().next().unwrap_or(Value::Null);
            return Ok(json!({ "type": "worktree_opened", "workspace": w, "root_pane": p, "already_open": true }));
        }
        let mut made = self.space_create(path, label, &[], linked(path, &root))?;
        made["type"] = json!("worktree_opened"); made["already_open"] = json!(false);
        Ok(made)
    }
    pub fn tree_remove(&self, space: &str, force: bool) -> HRes<Value> {
        let s = registry::space(&registry::read(self.srv()), space).filter(|s| s.at("worktree").b("is_linked_worktree"))
            .ok_or_else(|| run::err("workspace_not_found", "not a linked worktree space"))?;
        let mut a = vec!["worktree", "remove"]; if force { a.push("--force"); }
        let path = s.t("cwd"); a.push(&path);
        git(s.at("worktree").s("repo_root"), &a).map_err(|e| run::err("worktree_remove_failed", e))?;
        self.space_close(space)?;
        Ok(json!({ "type": "worktree_removed" }))
    }
}

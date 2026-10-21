//! The fake herdr's worktrees: made with git itself in the fake's own folder, opened as spaces, and removed.
use super::herdr::{arr, fail, ok, FakeHerdr as F, Out};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use sw::json::*;

pub fn git(cwd: &Path, a: &[&str]) -> Result<String, String> {
    let o = std::process::Command::new("git").args(a).current_dir(cwd).output().map_err(|e| e.to_string())?;
    if o.status.success() { Ok(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { Err(String::from_utf8_lossy(&o.stderr).into_owned()) }
}
/// The repo a folder belongs to: a checkout's own folder, or the main checkout of a worktree.
fn repo_of(cwd: &Path) -> PathBuf {
    git(cwd, &["rev-parse", "--path-format=absolute", "--git-common-dir"]).map(|d| PathBuf::from(d).parent().map(|p| p.to_path_buf()).unwrap_or_default()).unwrap_or_else(|_| cwd.to_path_buf())
}
pub fn worktree(f: &F, st: &mut Value, verb: &str, flag: &dyn Fn(&str) -> Option<String>, force: bool) -> Out {
    match verb {
        "create" => {
            let root = repo_of(Path::new(&flag("--cwd").unwrap_or_default()));
            let branch = flag("--branch").unwrap_or_default();
            let safe: String = branch.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '-' }).collect();
            let dest = f.dir.join("worktrees").join(root.file_name().unwrap_or_default()).join(safe);
            let ds = dest.display().to_string();
            let exists = git(&root, &["rev-parse", "--verify", &format!("refs/heads/{branch}")]).is_ok();
            let base = flag("--base").unwrap_or_else(|| "HEAD".into());
            // As herdr does: plain `-b`, so a branch made from a remote-tracking ref gets its upstream in .git/config.
            // `createLocked` fails that many creates as git does while another git command holds .git/config: the branch
            // is made, then the create refused.
            if st.i("createLocked") > 0 {
                st["createLocked"] = json!(st.i("createLocked") - 1);
                let _ = git(&root, &["branch", "--no-track", &branch, &base]);
                return fail("worktree_create_failed", "error: could not lock config file .git/config: File exists\nerror: unable to write upstream branch configuration");
            }
            let r = if exists { git(&root, &["worktree", "add", &ds, &branch]) } else { git(&root, &["worktree", "add", "-b", &branch, &ds, &base]) };
            if let Err(e) = r { return fail("worktree_create_failed", &e); }
            let rs = root.display().to_string();
            if !st.arr("workspaces").iter().any(|w| w.s("cwd") == rs) { F::new_workspace(st, &rs, root.file_name().and_then(|x| x.to_str()), json!({ "checkout_path": rs, "repo_root": rs, "is_linked_worktree": false })); }
            let made = F::new_workspace(st, &ds, flag("--label").as_deref(), json!({ "checkout_path": ds, "repo_root": rs, "is_linked_worktree": true }));
            let mut m = made.as_object().cloned().unwrap();
            m.insert("type".into(), json!("worktree_created"));
            m.insert("worktree".into(), json!({ "path": ds, "branch": branch, "open_workspace_id": made.at("workspace").at("workspace_id") }));
            ok(Value::Object(m))
        }
        "open" => {
            let dest = flag("--path").unwrap_or_default();
            let root = repo_of(Path::new(&dest)).display().to_string();
            if let Some(w) = st.arr("workspaces").iter().find(|x| x.s("cwd") == dest).cloned() {
                let p = st.arr("panes").iter().find(|p| p.at("workspace_id") == w.at("workspace_id")).cloned().unwrap_or(Value::Null);
                return ok(json!({ "type": "worktree_opened", "workspace": F::ws_out(st, &w), "root_pane": F::pane_out(&p), "already_open": true }));
            }
            let made = F::new_workspace(st, &dest, flag("--label").as_deref(), json!({ "checkout_path": dest, "repo_root": root, "is_linked_worktree": true }));
            let mut m = made.as_object().cloned().unwrap();
            m.insert("type".into(), json!("worktree_opened")); m.insert("already_open".into(), json!(false));
            ok(Value::Object(m))
        }
        "remove" => {
            let wid = flag("--workspace").unwrap_or_default();
            let Some(w) = st.arr("workspaces").iter().find(|x| x.s("workspace_id") == wid).cloned().filter(|w| w.at("worktree").b("is_linked_worktree")) else { return fail("workspace_not_found", "not a linked worktree workspace") };
            if st.b("removeFails") { return fail("timeout", "timed out"); }
            let mut a = vec!["worktree", "remove"]; if force { a.push("--force"); } a.push(w.s("cwd"));
            if let Err(e) = git(Path::new(w.at("worktree").s("repo_root")), &a) { return fail("worktree_remove_failed", &e); }
            arr(st, "workspaces").retain(|x| x.s("workspace_id") != wid);
            arr(st, "panes").retain(|p| p.s("workspace_id") != wid);
            F::drop_empty(st);
            ok(json!({ "type": "worktree_removed" }))
        }
        _ => fail("unknown_command", "worktree"),
    }
}

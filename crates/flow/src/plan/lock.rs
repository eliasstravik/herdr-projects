//! While a plan waits for its yes, the task's own worktree cannot be written to: its files, its folders and its git
//! index are read-only. Everything in the worktree loses its write permission, files git ignores too, and so does the
//! worktree's own git folder: nothing can be added, edited, deleted, staged or committed there. Reading and running
//! still work. Taken off when the build starts (before its agent is started, so a start that fails leaves it writable),
//! when the plan step is stopped, when the board itself puts something back, and before the worktree is removed; put on
//! again when the plan's writer starts once more. Only the task's own worktree is ever locked: your own checkout is
//! never made read-only.
use crate::json::*;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn tree_of(t: &Obj) -> Option<PathBuf> { let w = t.s("worktree"); (!w.is_empty() && Path::new(w).exists()).then(|| PathBuf::from(w)) }
/// The worktree's own git folder: what its `.git` file points at.
pub fn git_dir_of(tree: &Path) -> Option<PathBuf> {
    let s = std::fs::read_to_string(tree.join(".git")).ok()?;
    s.lines().find_map(|l| l.strip_prefix("gitdir:")).map(|p| tree.join(p.trim()))
}
/// Every file and folder under `dir`, and `dir` itself: the write bits taken off (`on` false) or the owner's put back.
/// Links are left alone. A folder is opened before it loses its write bit and after it has it back.
fn chmod(on: bool, dir: &Path) -> bool {
    let mut ok = true;
    fn set(p: &Path, mode: u32, on: bool, ok: &mut bool) { let m = if on { mode | 0o200 } else { mode & !0o222 }; if std::fs::set_permissions(p, std::fs::Permissions::from_mode(m)).is_err() { *ok = false; } }
    fn walk(p: &Path, on: bool, ok: &mut bool) {
        let Ok(s) = std::fs::symlink_metadata(p) else { return };
        if s.file_type().is_symlink() { return; }
        let mode = s.permissions().mode();
        if !s.is_dir() { set(p, mode, on, ok); return; }
        if on { set(p, mode, on, ok); }
        match std::fs::read_dir(p) { Ok(rd) => for e in rd.flatten() { walk(&e.path(), on, ok); }, Err(_) => *ok = false }
        if !on { set(p, mode, on, ok); }
    }
    walk(dir, on, &mut ok);
    ok
}
/// Returns true when the worktree is now read-only.
pub fn lock(t: &Obj) -> bool {
    let Some(tree) = tree_of(t) else { return false };
    let Some(git) = git_dir_of(&tree) else { return false }; // a folder that is no worktree: never locked
    chmod(false, &git) && chmod(false, &tree)
}
/// Returns true when the worktree is writable now (or there is none).
pub fn unlock(t: &Obj) -> bool {
    let Some(tree) = tree_of(t) else { return true };
    let a = chmod(true, &tree);
    git_dir_of(&tree).map(|g| chmod(true, &g)).unwrap_or(true) && a
}
/// The task's own worktree, and what the task says of it, as `on` asks: locked while its plan step is open, writable
/// when the build starts or the step is stopped. Returns whether the worktree is as asked.
pub fn set_locked(id: &str, on: bool) -> bool {
    let Some(t) = crate::store::tasks::get(id) else { return false };
    if !t.at("plan").is_object() { return true; }
    let ok = if on { lock(&t) } else { unlock(&t) };
    let locked = if on { ok } else { !ok };
    if t.at("plan").b("locked") != locked { let _ = crate::store::tasks::update_wait(id, 10000, |t| { if t.at("plan").is_object() { t.obj_mut("plan").put("locked", locked); } Ok(crate::store::tasks::Change::Write) }); }
    ok
}
pub fn is_locked(t: &Obj) -> bool {
    let Some(tree) = tree_of(t) else { return false };
    let c = std::ffi::CString::new(tree.display().to_string()).unwrap_or_default();
    unsafe { libc::access(c.as_ptr(), libc::W_OK) != 0 }
}

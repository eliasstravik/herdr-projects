//! A task's own temporary folder: what its agent and its reviewers make for a while (test projects, previews, venvs,
//! smoke installs) goes there, as their `TMPDIR`, and it goes when the task ends (docs/disk-space.md). Its path is
//! short (`/tmp/hp-1000/3f9a2c/HP-96`): programs make their sockets in it, a socket's path may not pass 108 bytes, and
//! Claude Code makes its own folder in it only when that stays under 44. The task's id and its report folder are given
//! with it.
use crate::agents::scratch::uid;
use crate::json::*;
use crate::store::files::{absolute, data_dir};
use crate::store::keys::{repo_keys, say_id};
use std::path::PathBuf;

/// The temporary folder of this machine as the board uses it (`HERDR_PROJECTS_TMP` names another, for a test).
pub fn root() -> PathBuf { std::env::var("HERDR_PROJECTS_TMP").ok().filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(std::env::temp_dir) }
/// The folder of this board's task folders: one per user and per board (several boards on one machine:
/// docs/several-boards.md), named after the board's data folder.
pub fn board_base() -> PathBuf {
    let h = sha1_smol::Sha1::from(absolute(&data_dir()).display().to_string()).digest().to_string();
    root().join(format!("hp-{}", uid())).join(&h[..6])
}
/// The name a task's folder gets: its short name (`HP-96`, an attempt `HP-96a`), in letters, digits and dashes.
pub fn name_of(t: &Obj) -> String { say_id(t.s("id"), &repo_keys()).chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' }).collect() }
/// The task's temporary folder: the one it was given (`tmp` on the task), or the one it gets.
pub fn of(t: &Obj) -> PathBuf { if t.b("tmp") { PathBuf::from(t.s("tmp")) } else { board_base().join(name_of(t)) } }
/// The folder, made where it is not there: for its owner only, as Claude Code wants its temporary folder.
pub fn made(t: &Obj) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let d = of(t);
    let _ = std::fs::create_dir_all(&d);
    for p in d.ancestors().take(3) { let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o700)); }
    d
}
/// The names a program reads its temporary folder from.
const NAMES: [&str; 4] = ["TMPDIR", "TMP", "TEMP", "CLAUDE_CODE_TMPDIR"];
/// The names that say which task it works on: the task's id (the name the hooks have it by) and the task's own folder
/// among the board's reports (not `HERDR_PROJECTS_REPORTS_DIR`, the folder of them all, which the board reads).
const TASK: [&str; 2] = ["HERDR_PROJECTS_ID", "HERDR_PROJECTS_TASK_REPORTS"];
/// Is this a task's temporary folder, or inside one?
pub fn in_task_folder(p: &std::path::Path) -> bool { let own = format!("hp-{}", uid()); p.components().any(|c| c.as_os_str() == own.as_str()) }
/// The board's own run, started by a task's agent (a `report`, an `add`): it does not keep that task's temporary
/// folder, or a task it starts would have its own folder inside it, and lose it when that task ends. Nor that task's id
/// and report folder: the board reads neither, and what it starts is given its own.
pub fn leave_task_env() {
    for k in NAMES { if std::env::var_os(k).map(|v| in_task_folder(std::path::Path::new(&v))).unwrap_or(false) { unsafe { std::env::remove_var(k) } } }
    for k in TASK { unsafe { std::env::remove_var(k) } }
}
/// The environment of a program the board starts for the task: every name programs read their temporary folder from,
/// then the task's id and its report folder.
pub fn env(t: &Obj) -> Vec<(String, String)> {
    let d = of(t).display().to_string();
    let mut all: Vec<(String, String)> = NAMES.iter().map(|k| (k.to_string(), d.clone())).collect();
    all.push((TASK[0].into(), t.t("id")));
    all.push((TASK[1].into(), crate::research::report_folder(t).display().to_string()));
    all
}
/// The board's own folders and the task's temporary folder (made now), for a tab or a space opened for the task.
pub fn task_env(t: &Obj) -> Vec<(String, String)> { made(t); [crate::store::files::board_env(), env(t)].concat() }
/// The line that gives a pane's shell the task's temporary folder, before its program starts there: herdr hands a
/// worktree's space no environment. A space first keeps it out of the shell's history where that is set.
pub fn export_line(t: &Obj) -> String {
    let q = |s: &str| if s.chars().all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c)) { s.to_string() } else { crate::util::shq(s) };
    format!(" export {}", env(t).iter().map(|(k, v)| format!("{k}={}", q(v))).collect::<Vec<_>>().join(" "))
}
/// What the task's agent and reviewers are told.
pub fn rule(t: &Obj) -> String {
    format!("Put every temporary thing you make (test projects, previews, venvs, smoke installs, downloads) in your own temporary folder, {} (your $TMPDIR), never loose in /tmp or in a home folder: the board removes it when this task ends, and stops whatever still runs from it then.", of(t).display())
}
/// What the task's agent is told besides.
pub const STOP_RULE: &str = "Before you finish, stop the servers and background processes you started, unless the human asked you to keep one running.";

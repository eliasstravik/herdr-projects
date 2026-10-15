//! While a plan waits for its yes, its writer's program runs where the task's worktree is mounted read-only: a barrier
//! the program cannot take off (bubblewrap: `bwrap --ro-bind`). A write, a chmod, a delete or a commit there is answered
//! "Read-only file system". The board starts the program itself in the pane, and herdr finds the agent there (checked on
//! herdr 0.9.3 with bubblewrap 0.9). After the yes the program is ended and started again the usual way. Where
//! bubblewrap is missing or may not be used, no plan is written in a repo at all.
use super::lock::git_dir_of;
use super::open;
use crate::herdr::{cli::HerdrError, Herdr};
use crate::json::*;
use crate::store::files::{data_dir, read_obj, write_json};
use crate::store::repos::{cwd_of, is_repo};
use std::path::PathBuf;
use std::sync::Mutex;

static KNOWN: Mutex<Option<(bool, i64)>> = Mutex::new(None);
/// Can a read-only mount be made here? `HERDR_PROJECTS_SANDBOX` says it outright (bwrap, none).
pub fn sandbox_ok() -> bool {
    if let Ok(said) = std::env::var("HERDR_PROJECTS_SANDBOX") { if !said.is_empty() { return said != "none"; } }
    let now = crate::clock::now();
    if let Some((ok, at)) = *KNOWN.lock().unwrap() { if now - at < 600000 { return ok; } }
    let file = data_dir().join("sandbox.json");
    if let Some(k) = read_obj(&file) { if now - k.i("at") < 600000 { *KNOWN.lock().unwrap() = Some((k.b("ok"), k.i("at"))); return k.b("ok"); } }
    let d = data_dir().display().to_string();
    let ok = std::process::Command::new("bwrap").args(["--dev-bind", "/", "/", "--ro-bind", &d, &d, "--", "true"]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false);
    *KNOWN.lock().unwrap() = Some((ok, now));
    let _ = write_json(&file, &json!({ "ok": ok, "at": now }));
    ok
}
fn q(s: &str) -> String { if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c)) { s.to_string() } else { crate::util::shq(s) } }
/// The folder that may not change: the task's own worktree, or the checkout it works in. None for a folder no repo.
pub fn guarded(t: &Obj) -> Option<PathBuf> { let dir = cwd_of(t); (dir.exists() && is_repo(&dir)).then_some(dir) }
/// Does this task's plan writer run in such a mount? In a repo, while the step is open.
pub fn boxed(t: &Obj) -> bool { open(t) && guarded(t).is_some() && sandbox_ok() }
pub const NO_MOUNT: &str = "This task starts with a plan (plan.approve), and its writer must run where the repo cannot be written: that needs bubblewrap (the `bwrap` program, with user namespaces allowed), which is missing or not usable on this machine. Nothing was started. Install it (for example: sudo apt install bubblewrap), or take the plan off for this task: herdr-projects set plan.approve none --task <id>.";
/// The command that starts a program there. The name it would have had from `agent start` rides along.
pub fn boxed_command(t: &Obj, name: &str, kind: &str, args: &[String]) -> String {
    let tree = guarded(t).map(|p| p.display().to_string()).unwrap_or_default();
    let git = git_dir_of(std::path::Path::new(&tree)); // a worktree's git folder is elsewhere; a checkout's is inside it
    let mut all: Vec<String> = vec!["env".into(), format!("HERDR_PROJECTS_AGENT={name}"), "bwrap".into(), "--dev-bind".into(), "/".into(), "/".into(), "--ro-bind".into(), tree.clone(), tree];
    if let Some(g) = git { let g = g.display().to_string(); all.extend(["--ro-bind".into(), g.clone(), g]); }
    all.extend(["--die-with-parent".into(), "--".into(), crate::agents::kinds::program(kind).into()]);
    all.extend(args.iter().cloned());
    all.iter().map(|s| q(s)).collect::<Vec<_>>().join(" ")
}
fn herr(code: &str, msg: String) -> HerdrError { HerdrError { code: code.into(), message: msg } }
/// Start an agent program in a pane: in the read-only mount while the task's plan waits, else as herdr starts one.
/// Answers and fails as `agent start` does: a program whose start-up dialog is in the way is "agent_not_ready".
pub fn start_program(api: &Herdr, t: &Obj, name: &str, kind: &str, pane: &str, args: &[String]) -> Result<(), HerdrError> {
    if open(t) && guarded(t).is_some() && !sandbox_ok() { return Err(herr("failed", NO_MOUNT.replace("<id>", t.s("id")))); } // never a plan writer in a repo without it
    if !boxed(t) { return api.agent_start(name, kind, pane, args, 60000).map(|_| ()); }
    api.integration_ready(kind)?;
    api.pane_run(pane, &boxed_command(t, name, kind, args))?;
    let step = std::env::var("HERDR_PROJECTS_READY_STEP_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(250u64);
    let until = crate::clock::now() + 60000;
    loop {
        if let Some(a) = api.agent_get(pane)? {
            if a.s("agent_status") == "blocked" { return Err(herr("agent_not_ready", format!("agent {name} is blocked during startup and is not ready for prompts"))); }
            return Ok(());
        }
        if crate::clock::now() >= until { return Err(herr("failed", format!("{kind} did not start in its read-only mount (bwrap). Look at the pane; HERDR_PROJECTS_SANDBOX=none starts it without one."))); }
        if step > 0 { std::thread::sleep(std::time::Duration::from_millis(step)); }
    }
}

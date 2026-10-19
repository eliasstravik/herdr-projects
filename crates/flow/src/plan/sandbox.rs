//! While a plan waits for its yes, its writer's program runs where the task's worktree cannot be written: a barrier the
//! program cannot take off, made by this platform's own tool (sandbox/tool.rs: bubblewrap on Linux, `sandbox-exec` on
//! macOS). A write, a chmod, a delete or a commit there is refused. The board starts the program itself in the pane, and
//! herdr finds the agent there (checked on herdr 0.9.3 with bubblewrap 0.9). After the yes the program is ended and
//! started again the usual way. Where no such mount can be made, no plan is written in a repo at all.
pub mod tool;

use super::lock::git_dir_of;
use super::open;
use crate::herdr::{cli::HerdrError, Herdr};
use crate::json::*;
use crate::store::files::{data_dir, read_obj, write_json};
use crate::store::repos::{cwd_of, is_repo};
use std::path::PathBuf;
use std::sync::Mutex;
pub use tool::Tool;

static KNOWN: Mutex<Option<(Option<Tool>, i64)>> = Mutex::new(None);
/// The tool that makes the read-only mount here, None where none can. `HERDR_PROJECTS_SANDBOX` says it outright
/// (`bwrap`, `sandbox-exec`, `none`); else this platform's own is tried, and what it found is kept for ten minutes.
pub fn tool() -> Option<Tool> {
    if let Ok(said) = std::env::var("HERDR_PROJECTS_SANDBOX") { if !said.is_empty() { return if said == "none" { None } else { Some(Tool::of(&said).unwrap_or(Tool::native())) }; } }
    let now = crate::clock::now();
    if let Some((found, at)) = *KNOWN.lock().unwrap() { if now - at < 600000 { return found; } }
    let file = data_dir().join("sandbox.json");
    let kept = |k: &Obj| k.b("ok").then(|| Tool::of(k.s("tool")).unwrap_or(Tool::native()));
    if let Some(k) = read_obj(&file) { if now - k.i("at") < 600000 { *KNOWN.lock().unwrap() = Some((kept(&k), k.i("at"))); return kept(&k); } }
    let native = Tool::native();
    let found = native.why_not(&data_dir()).is_none().then_some(native);
    *KNOWN.lock().unwrap() = Some((found, now));
    let _ = write_json(&file, &json!({ "ok": found.is_some(), "tool": native.name(), "at": now }));
    found
}
/// Can a read-only mount be made here?
pub fn sandbox_ok() -> bool { tool().is_some() }
fn q(s: &str) -> String { if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c)) { s.to_string() } else { crate::util::shq(s) } }
/// The folder that may not change: the task's own worktree, or the checkout it works in. None for a folder no repo.
pub fn guarded(t: &Obj) -> Option<PathBuf> { let dir = cwd_of(t); (dir.exists() && is_repo(&dir)).then_some(dir) }
/// Does this task's plan writer run in such a mount? In a repo, while the step is open.
pub fn boxed(t: &Obj) -> bool { open(t) && guarded(t).is_some() && sandbox_ok() }
/// Why the task cannot start where no mount can be made, naming this platform's tool.
pub fn no_mount(id: &str) -> String { tool().unwrap_or(Tool::native()).no_mount(id) }
/// The command that starts a program there. The name it would have had from `agent start` rides along.
pub fn boxed_command(t: &Obj, name: &str, kind: &str, args: &[String]) -> String {
    let tool = tool().unwrap_or(Tool::native());
    let tree = guarded(t).unwrap_or_default();
    let mut guard = vec![tree.clone()];
    guard.extend(git_dir_of(&tree)); // a worktree's git folder is elsewhere; a checkout's is inside it
    let mut all: Vec<String> = vec!["env".into(), format!("HERDR_PROJECTS_AGENT={name}")];
    all.extend(tool.wrap(&guard));
    all.push(crate::agents::kinds::program(kind).into());
    all.extend(tool.unnested(kind, args));
    all.iter().map(|s| q(s)).collect::<Vec<_>>().join(" ")
}
fn herr(code: &str, msg: String) -> HerdrError { HerdrError { code: code.into(), message: msg } }
/// Start an agent program in a pane: in the read-only mount while the task's plan waits, else as herdr starts one.
/// Answers and fails as `agent start` does: a program whose start-up dialog is in the way is "agent_not_ready".
pub fn start_program(api: &Herdr, t: &Obj, name: &str, kind: &str, pane: &str, args: &[String]) -> Result<(), HerdrError> {
    if open(t) && guarded(t).is_some() && !sandbox_ok() { return Err(herr("failed", no_mount(t.s("id")))); } // never a plan writer in a repo without it
    // These CLIs report live state through the user's installed Herdr plugins; launch them in the real pane TTY.
    // They do not use ACP, stdin prompt piping, or a one-shot/headless mode.
    if matches!(kind, "freebuff" | "codebuddy") {
        let command = if boxed(t) {
            boxed_command(t, name, kind, args)
        } else {
            let mut all = vec!["env".to_string(), format!("HERDR_PROJECTS_AGENT={name}"), crate::agents::kinds::program(kind).into()];
            all.extend(args.iter().cloned());
            all.iter().map(|s| q(s)).collect::<Vec<_>>().join(" ")
        };
        api.pane_run(pane, &command)?;
        let until = crate::clock::now() + 60000;
        loop {
            if api.agent_get(pane)?.is_some() { return Ok(()); }
            if crate::clock::now() >= until { return Err(herr("failed", format!("{kind} did not report its state in the pane; check that its Herdr plugin is installed and enabled."))); }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }
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
        if crate::clock::now() >= until { return Err(herr("failed", tool().unwrap_or(Tool::native()).not_started(kind, t.s("id")))); }
        if step > 0 { std::thread::sleep(std::time::Duration::from_millis(step)); }
    }
}

//! The only file that runs the herdr program. JSON in, JSON out. Verified on herdr 0.9.3 (docs/checked-on-herdr.md):
//! ids (workspace, tab, pane) survive a server restart, terminal_id does not; `agent get` answers agent_not_found for a
//! pane whose agent exited; `agent start` answers agent_not_ready when a dialog blocks the start; `agent prompt`
//! answers agent_blocked. A test answers for herdr itself (`set_fake`), so its many calls start no program.
use crate::json::*;
use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct HerdrError { pub code: String, pub message: String }
impl std::fmt::Display for HerdrError { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str(&self.message) } }
impl From<HerdrError> for crate::err::Error { fn from(e: HerdrError) -> Self { crate::err::Error::new(e.message) } }
pub type HRes<T> = Result<T, HerdrError>;
fn herr(code: &str, message: impl Into<String>) -> HerdrError { HerdrError { code: code.into(), message: message.into() } }

/// What a run of the program gave: its status, its output, its error output.
pub struct Ran { pub status: i32, pub stdout: String, pub stderr: String }
/// A stand-in for herdr, for tests: answers each command line as the program would.
pub trait Fake { fn run(&self, args: &[String], input: &str) -> Ran; fn kids(&self, _pid: i64) -> Vec<i64> { vec![] } fn wired_to(&self, pid: i64, group: &[i64]) -> Option<bool> { super::procs::real_wired_to(pid, group) } fn wiring(&self, pid: i64, group: &[i64]) -> super::procs::Wiring { use super::procs::Wiring as W; match self.wired_to(pid, group) { Some(true) => W::Agent, Some(false) => W::Elsewhere, None => W::Unknown } } fn command(&self, _pid: i64) -> Option<String> { None } fn kill(&self, _pid: i64) {} fn alive(&self, _pid: i64) -> Option<bool> { None } fn ask(&self, _method: &str, _params: &Value) -> Result<Value, HerdrError> { Err(herr("no_server", "no herdr")) } fn found(&self) -> bool { true } }
thread_local! { static FAKE: RefCell<Option<Rc<dyn Fake>>> = const { RefCell::new(None) }; }
/// Tests only: herdr is this stand-in on this thread (None: the real program again).
pub fn set_fake(f: Option<Rc<dyn Fake>>) { FAKE.with(|x| *x.borrow_mut() = f); }
pub fn fake() -> Option<Rc<dyn Fake>> { FAKE.with(|x| x.borrow().clone()) }

/// The herdr program: `HERDR_BIN_PATH`, else `herdr` on PATH, else where it is found without it (find.rs).
pub fn bin() -> String { std::env::var("HERDR_BIN_PATH").ok().filter(|s| !s.is_empty()).or_else(super::find::herdr_elsewhere).unwrap_or_else(|| "herdr".into()) }
/// Whether the herdr program `bin` names can be run from this process (a test's stand-in says so itself).
pub fn found() -> bool {
    if let Some(f) = fake() { return f.found(); }
    let b = bin();
    if b.contains('/') { super::find::runs(Path::new(&b)) } else { super::find::in_dirs(&b, &std::env::var("PATH").unwrap_or_default()).is_some() }
}
fn config_home() -> PathBuf { std::env::var("XDG_CONFIG_HOME").ok().filter(|s| !s.is_empty()).map(PathBuf::from).unwrap_or_else(|| crate::store::files::user_home().join(".config")) }
/// The session a plain command talks to: the pane it runs in, else the default session.
pub fn current_socket() -> String {
    if let Ok(s) = std::env::var("HERDR_SOCKET_PATH") { if !s.is_empty() { return s; } }
    let base = config_home().join("herdr");
    match std::env::var("HERDR_SESSION") { Ok(s) if !s.is_empty() => base.join("sessions").join(s).join("herdr.sock"), _ => base.join("herdr.sock") }.display().to_string()
}
pub fn session_name(socket: &str) -> String {
    let parts: Vec<&str> = socket.split('/').collect();
    let n = parts.len();
    if n >= 3 && parts[n - 1] == "herdr.sock" && parts[n - 3] == "sessions" { parts[n - 2].to_string() } else { "default".into() }
}
pub fn in_herdr() -> bool { std::env::var("HERDR_ENV").as_deref() == Ok("1") }
pub fn socket_exists(socket: &str) -> bool { use std::os::unix::fs::FileTypeExt; std::fs::metadata(socket).map(|m| m.file_type().is_socket()).unwrap_or(false) }

/// Runs the program: what it gave, or why it could not be run. `timeout_ms` ends a program that takes longer.
pub fn launch(socket: Option<&str>, args: &[String], input: &str, timeout_ms: u64) -> HRes<Ran> {
    // Linux refuses to start a program with a longer argument (E2BIG): said in words, also to a test's stand-in.
    if let Some(a) = args.iter().find(|a| a.len() > crate::agents::hand_limits::PROGRAM_ARG_MAX) { return Err(herr("too_long", format!("A text of {} bytes is too long to hand to herdr in one go: Linux takes at most {} bytes in one argument.", a.len(), crate::agents::hand_limits::PROGRAM_ARG_MAX))); }
    if let Some(f) = fake() { return Ok(f.run(args, input)); }
    let mut c = Command::new(bin());
    c.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    // Never let the caller's own pane leak in as an implicit target: every command names its pane.
    for k in ["HERDR_PANE_ID", "HERDR_TAB_ID", "HERDR_WORKSPACE_ID", "HERDR_SESSION"] { c.env_remove(k); }
    if let Some(s) = socket { c.env("HERDR_SOCKET_PATH", s); }
    let mut child = c.spawn().map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { herr("herdr_missing", "The herdr program was not found.") } else { herr("spawn_failed", e.to_string()) })?;
    if let Some(mut si) = child.stdin.take() { let _ = si.write_all(input.as_bytes()); }
    wait_with_timeout(child, timeout_ms)
}
/// Waits for a child, reading what it writes; kills it after `timeout_ms` (one too long to count: never).
pub fn wait_with_timeout(mut child: std::process::Child, timeout_ms: u64) -> HRes<Ran> {
    use std::io::Read;
    let mut out = child.stdout.take();
    let mut err = child.stderr.take();
    let t_out = std::thread::spawn(move || { let mut s = Vec::new(); if let Some(o) = out.as_mut() { let _ = o.read_to_end(&mut s); } s });
    let t_err = std::thread::spawn(move || { let mut s = Vec::new(); if let Some(o) = err.as_mut() { let _ = o.read_to_end(&mut s); } s });
    let until = std::time::Instant::now().checked_add(Duration::from_millis(timeout_ms));
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st.code().unwrap_or(-1),
            Ok(None) if until.is_some_and(|u| std::time::Instant::now() >= u) => { let _ = child.kill(); let _ = child.wait(); return Err(herr("timeout", "herdr did not answer in time")); }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(e) => return Err(herr("spawn_failed", e.to_string())),
        }
    };
    let stdout = String::from_utf8_lossy(&t_out.join().unwrap_or_default()).into_owned();
    let stderr = String::from_utf8_lossy(&t_err.join().unwrap_or_default()).into_owned();
    Ok(Ran { status, stdout, stderr })
}
/// One command: its JSON result, or the error herdr gave.
pub fn run(socket: Option<&str>, args: &[String], timeout_ms: u64, input: &str) -> HRes<Value> {
    let r = launch(socket, args, input, timeout_ms)?;
    if r.status != 0 {
        let (mut code, mut message) = ("failed".to_string(), if r.stderr.trim().is_empty() { r.stdout.trim().to_string() } else { r.stderr.trim().to_string() });
        for text in [&r.stderr, &r.stdout] {
            if let Ok(j) = serde_json::from_str::<Value>(text.trim().lines().last().unwrap_or("")) {
                if j.at("error").is_object() { if j.at("error").b("code") { code = j.at("error").t("code"); } if j.at("error").b("message") { message = j.at("error").t("message"); } break; }
            }
        }
        let low = message.to_lowercase();
        if code == "server_not_running" || (code == "failed" && ["no such file", "connection refused", "not running", "failed to connect"].iter().any(|w| low.contains(w))) { code = "no_server".into(); }
        if message.is_empty() { message = format!("herdr {} failed", args.join(" ")); }
        return Err(HerdrError { code, message });
    }
    match serde_json::from_str::<Value>(&r.stdout) {
        Ok(j) => {
            if j.at("error").is_object() { return Err(herr(j.at("error").s("code"), j.at("error").t("message"))); }
            Ok(if j.at("result").is_null() { j } else { j.at("result").clone() })
        }
        Err(_) => Ok(json!({ "text": r.stdout })),
    }
}
/// One command's raw output.
pub fn run_raw(socket: Option<&str>, args: &[String], timeout_ms: u64, input: &str) -> HRes<String> {
    let r = launch(socket, args, input, timeout_ms)?;
    if r.status != 0 { return Err(herr("failed", if r.stderr.trim().is_empty() { r.stdout.trim().to_string() } else { r.stderr.trim().to_string() })); }
    Ok(r.stdout)
}
/// The processes started by one process.
pub fn kids(pid: i64) -> Result<Vec<i64>, ()> {
    if let Some(f) = fake() { return Ok(f.kids(pid)); }
    let o = Command::new("pgrep").args(["-P", &pid.to_string()]).output().map_err(|_| ())?;
    if o.status.code().unwrap_or(2) > 1 { return Err(()); }
    Ok(String::from_utf8_lossy(&o.stdout).lines().filter_map(|l| l.trim().parse().ok()).collect())
}
pub fn args(a: &[&str]) -> Vec<String> { a.iter().map(|s| s.to_string()).collect() }
pub fn path_str(p: &Path) -> String { p.display().to_string() }

//! The only file that runs the rex program: its command line (`new`, `window new`, `split`, `send-key`) and its
//! server API (`api call METHOD -`, the arguments as JSON on its input). Its errors carry Rex's own code
//! (`code=session_not_found`). A test answers for Rex itself (`set_fake`), so its calls start no program.
use crate::herdr::cli::{wait_with_timeout, HRes, HerdrError, Ran};
use crate::json::*;
use std::cell::RefCell;
use std::io::Write;
use std::process::{Command, Stdio};
use std::rc::Rc;

/// A stand-in for the rex program, for tests: answers each command line as the program would.
pub trait RexFake { fn run(&self, args: &[String], input: &str) -> Ran; }
thread_local! { static FAKE: RefCell<Option<Rc<dyn RexFake>>> = const { RefCell::new(None) }; }
/// Tests only: rex is this stand-in on this thread (None: the real program again).
pub fn set_fake(f: Option<Rc<dyn RexFake>>) { FAKE.with(|x| *x.borrow_mut() = f); }
fn fake() -> Option<Rc<dyn RexFake>> { FAKE.with(|x| x.borrow().clone()) }

/// Where Rex's app keeps its program when it is not on the PATH.
const APP_BIN: &str = "/Applications/Rex Beta.app/Contents/Helpers/Rex Server.rex/Contents/MacOS/rex";
/// The rex program: `REX_BIN`, else `rex` on the PATH, else the one in Rex's app.
pub fn bin() -> String {
    if let Ok(b) = std::env::var("REX_BIN") { if !b.is_empty() { return b; } }
    let on_path = std::env::var("PATH").unwrap_or_default().split(':').any(|d| !d.is_empty() && std::path::Path::new(d).join("rex").is_file());
    if !on_path && std::path::Path::new(APP_BIN).is_file() { APP_BIN.into() } else { "rex".into() }
}
pub fn err(code: &str, message: impl Into<String>) -> HerdrError { HerdrError { code: code.into(), message: message.into() } }

/// One rex command line: its output, or the error Rex gave. `server`: a host's label or address, None the local one.
pub fn rex(server: Option<&str>, a: &[String], input: &str, timeout_ms: u64) -> HRes<String> {
    // Never start a server by accident: the board talks to the one that runs, or says it is not there.
    let mut full: Vec<String> = vec!["--autostart=false".into()];
    if let Some(s) = server { full.extend(["-S".to_string(), s.to_string()]); }
    full.extend(a.iter().cloned());
    let r = match fake() {
        Some(f) => f.run(&full, input),
        None => {
            let mut c = Command::new(bin());
            c.args(&full).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
            // Every call names its session and block: never one this process happens to run in.
            for k in ["REX_BLOCK", "REX_SESSION", "REX_WINDOW"] { c.env_remove(k); }
            let mut child = c.spawn().map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { err("rex_missing", "The rex program was not found: install Rex, or set REX_BIN.") } else { err("spawn_failed", e.to_string()) })?;
            if let Some(mut si) = child.stdin.take() { let _ = si.write_all(input.as_bytes()); }
            wait_with_timeout(child, timeout_ms).map_err(|e| if e.code == "timeout" { err("timeout", "Rex did not answer in time") } else { e })?
        }
    };
    if r.status != 0 { return Err(failure(&r.stderr, &r.stdout, a)); }
    Ok(r.stdout)
}
/// Rex's error, said as `Error: … (status=404 code=session_not_found)`, as a code and a message.
fn failure(stderr: &str, stdout: &str, a: &[String]) -> HerdrError {
    let text = if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() };
    let first = text.lines().next().unwrap_or("").trim_start_matches("Error: ").to_string();
    let code = text.split("code=").nth(1).map(|c| c.chars().take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_').collect::<String>()).filter(|c| !c.is_empty());
    let low = text.to_lowercase();
    let code = code.unwrap_or_else(|| if low.contains("connecting to rex server") || low.contains("connection refused") { "no_server".into() } else { "failed".into() });
    let message = if first.is_empty() { format!("rex {} failed", a.join(" ")) } else { first };
    err(&code, message)
}
/// One rex command line whose output is JSON.
pub fn rex_json(server: Option<&str>, a: &[String], timeout_ms: u64) -> HRes<Value> {
    let out = rex(server, a, "", timeout_ms)?;
    serde_json::from_str(out.trim()).map_err(|_| err("failed", format!("rex {} did not answer JSON", a.first().cloned().unwrap_or_default())))
}
/// One server API method. A method on a session attaches to it first (`-s`), as Rex asks.
pub fn api(server: Option<&str>, method: &str, params: &Value) -> HRes<Value> {
    let mut a = vec!["api".to_string(), "call".into()];
    if params.b("session_id") { a.extend(["-s".to_string(), params.t("session_id")]); }
    a.extend([method.to_string(), "-".into()]);
    let out = rex(server, &a, &params.to_string(), 30000)?;
    if out.trim().is_empty() { return Ok(json!({})); }
    serde_json::from_str(out.trim()).map_err(|_| err("failed", format!("Rex's {method} did not answer JSON")))
}
/// A method of one block (`process`, `program_status`, `format`, `write`, `size`).
pub fn block(server: Option<&str>, block: &str, method: &str, args: Value) -> HRes<Value> {
    let mut a = vec!["block".to_string(), "call".into(), "-b".into(), block.into(), method.into()];
    if !args.is_null() { a.push("-".into()); }
    let out = rex(server, &a, &if args.is_null() { String::new() } else { args.to_string() }, 15000)?;
    if out.trim().is_empty() { return Ok(json!({})); }
    serde_json::from_str(out.trim()).map_err(|_| err("failed", format!("Rex's {method} did not answer JSON")))
}

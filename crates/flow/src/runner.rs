//! Long jobs (delegate, finish line, land, restart) run in a detached runner so the board stays responsive and hook
//! commands never hold herdr up. A runner marks itself in task.json as `runner: {pid, start, started, job}`; a marker
//! whose process is gone is ignored. Each turn is a separate runner, so counters live in task.json. One asked for from
//! inside a sandbox is started from outside it (runner/outside.rs).
pub mod outside;

use crate::err::{Error, Res};
use crate::json::*;
use crate::store::files::*;
use crate::store::marks::pending_events;
use crate::store::tasks::{get, update_wait, Change, Task};
use std::cell::RefCell;
use std::process::{Command, Stdio};

pub use crate::login_env::user_env;
/// Tests run jobs in their own process: the jobs that would have been detached are kept here instead.
pub fn inline() -> bool { std::env::var("HERDR_PROJECTS_INLINE").as_deref() == Ok("1") }
thread_local! { pub static JOBS: RefCell<Vec<(String, String, Vec<String>)>> = const { RefCell::new(vec![]) }; }
/// What a detached process must not inherit: it is not a plugin command, nor an agent's pane.
const DROP_ENV: [&str; 8] = ["HERDR_PLUGIN_EVENT", "HERDR_PLUGIN_EVENT_JSON", "HERDR_PLUGIN_CONTEXT_JSON", "HERDR_PLUGIN_ENTRYPOINT_ID", "HERDR_PLUGIN_ACTION_ID", "HERDR_PANE_ID", "HERDR_TAB_ID", "HERDR_WORKSPACE_ID"];
/// A process of its own session, detached from this one: it outlives whoever started it.
pub fn detach(c: &mut Command) {
    use std::os::unix::process::CommandExt;
    unsafe { c.pre_exec(|| { libc::setsid(); Ok(()) }); }
}
/// Start a detached command and leave it to run: a small thread of this process waits for its end. `setsid` does not
/// change a process's parent, so one nobody waits for stays a zombie of a long-lived starter (the keeper, the board)
/// until that ends: thousands a day, until the user's process limit fails everything (#212). Only this one pid is
/// waited for, never any child (that would take the end of the commands others wait for themselves). Returns its pid,
/// 0 when it did not start.
pub fn start_reaped(c: &mut Command) -> u32 { c.spawn().map(reap).unwrap_or(0) }
/// Leave a started process to run, waited for by a small thread of this process when it ends (as `start_reaped`).
pub fn reap(mut ch: std::process::Child) -> u32 {
    let pid = ch.id();
    let _ = std::thread::Builder::new().name("reap".into()).stack_size(64 << 10).spawn(move || { let _ = ch.wait(); });
    pid
}
/// Start a detached runner for one job on one task. Returns at once.
pub fn spawn_runner(job: &str, id: &str, extra: &[&str]) -> u32 {
    if crate::plan::sandbox::inside::boxed() { return outside::ask(job, id, extra); }
    if inline() { JOBS.with(|j| j.borrow_mut().push((job.into(), id.into(), extra.iter().map(|s| s.to_string()).collect()))); return 0; }
    user_env();
    let dir = if id == "-" { data_dir() } else { dir_of(id) };
    let _ = std::fs::create_dir_all(&dir);
    let log = std::fs::OpenOptions::new().create(true).append(true).open(dir.join(if id == "-" { "jobs.log" } else { "runner.log" }));
    let mut c = Command::new(bin());
    c.arg("run").arg(job).arg(id).args(extra).current_dir(data_dir()).stdin(Stdio::null());
    for k in DROP_ENV { c.env_remove(k); }
    match log { Ok(f) => { let f2 = f.try_clone().ok(); c.stdout(f); if let Some(f2) = f2 { c.stderr(f2); } } Err(_) => { c.stdout(Stdio::null()).stderr(Stdio::null()); } }
    detach(&mut c);
    start_reaped(&mut c)
}
/// Tests drain the jobs that would have been detached.
pub fn drain_jobs(run: &dyn Fn(&str, &str, &[String])) {
    loop {
        let next = JOBS.with(|j| { let mut j = j.borrow_mut(); if j.is_empty() { None } else { Some(j.remove(0)) } });
        let Some((job, id, extra)) = next else { break };
        run(&job, &id, &extra);
    }
}
/// A shell command of the user's, started and left to run (`board.notify`). Returns its pid (0: kept for the test, or
/// it did not start).
pub fn detached_sh(cmd: &str, env: &[(&str, String)]) -> u32 {
    if inline() { JOBS.with(|j| j.borrow_mut().push(("notify".into(), cmd.into(), env.iter().map(|(k, v)| format!("{k}={v}")).collect()))); return 0; }
    user_env();
    let mut c = Command::new("sh");
    c.args(["-c", cmd]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    for (k, v) in env { c.env(k, v); }
    detach(&mut c);
    start_reaped(&mut c)
}
pub fn runner_alive(t: &Task) -> bool { marker_alive(t.at("runner")) }
fn mine(m: &Value) -> bool { m.i("pid") == std::process::id() as i64 }
fn others(t: &Task) -> bool { t.at("runner").is_object() && !mine(t.at("runner")) && marker_alive(t.at("runner")) }
/// Claim a task for this runner. Returns the task, or None when another live runner has it.
pub fn claim(id: &str, job: &str, patch: impl FnOnce(&mut Task)) -> Res<Option<Task>> {
    let mut ok = false;
    let t = update_wait(id, 10000, |t| {
        if others(t) { return Ok(Change::Keep); }
        ok = true;
        let mut m = marker(); m.put("job", job);
        t.put("runner", m);
        patch(t);
        Ok(Change::Write)
    })?;
    Ok(ok.then_some(t))
}
/// The one way a marker is given up: one update under the task lock that also looks at the turn-ended events. `keep`
/// (the finish runner): with an unhandled event, keep the marker and return true, so the caller processes it.
pub fn release(id: &str, keep: bool) -> bool {
    let mut more = false;
    let _ = update_wait(id, 10000, |t| {
        if !mine(t.at("runner")) { return Ok(Change::Keep); }
        more = !pending_events(t).is_empty();
        if more && keep { return Ok(Change::Keep); }
        t.put("runner", Value::Null);
        Ok(Change::Write)
    });
    if more && !keep { spawn_runner("finish", id, &[]); }
    more && keep
}
/// The world moved on under a runner: it stops what it was doing.
pub fn aborted(why: &str) -> Error { Error { msg: format!("aborted: {why}"), kind: crate::err::Kind::Plain } }
pub fn is_aborted(e: &Error) -> bool { e.msg.starts_with("aborted: ") }
/// Before every later step a runner re-reads the task under the lock and aborts if the stage is no longer the one it
/// set or `armedAt` moved, or the agent finished a new turn.
pub struct Guard { id: String, stages: Vec<String>, armed: Value, seen: f64, free: bool }
impl Guard {
    pub fn new(id: &str) -> Self { Guard { id: id.into(), stages: vec![], armed: Value::Null, seen: f64::INFINITY, free: false } }
    /// A guard that only changes the task, with no runner of its own to check (a start that is no job's).
    pub fn free(id: &str) -> Self { Guard { free: true, ..Guard::new(id) } }
    /// A turn of a chat that began while a review was under way, and changed nothing it looks at, is no new turn here.
    fn new_turn(&self, t: &Task) -> bool { t.arr("events").iter().any(|e| e.s("role") == "agent" && e.f("n") > self.seen) && !crate::flow::chat::review_goes_on(t) }
    /// Record the stage(s) this runner now stands in.
    pub fn expect(&mut self, t: &Task, also: &[&str]) -> Task {
        self.stages = std::iter::once(t.t("stage")).chain(also.iter().map(|s| s.to_string())).collect();
        self.armed = t.at("armedAt").clone();
        self.seen = t.f("eventSeq").max(t.f("eventsHandled"));
        t.clone()
    }
    fn still(&self, t: &Task) -> Res<()> {
        if !mine(t.at("runner")) { return Err(aborted("another runner took over")); }
        if !self.stages.iter().any(|s| s == t.s("stage")) { return Err(aborted(&format!("stage changed to {}", t.s("stage")))); }
        if *t.at("armedAt") != self.armed { return Err(aborted("a new turn started")); }
        Ok(())
    }
    pub fn check(&self) -> Res<Task> {
        let t = get(&self.id).ok_or_else(|| aborted("the task is gone"))?;
        if t.b("archivedAt") { return Err(aborted("archived")); }
        self.still(&t)?;
        if self.new_turn(&t) { return Err(aborted("the agent finished a new turn")); }
        Ok(t)
    }
    /// Change the task as this runner, only if it is still ours; remembers the new stage.
    pub fn set(&mut self, f: impl FnOnce(&mut Task)) -> Res<Task> {
        if self.free { return update_wait(&self.id, 10000, move |t| { f(t); Ok(Change::Write) }); }
        let (stages, armed) = (self.stages.clone(), self.armed.clone());
        let t = update_wait(&self.id, 10000, move |t| {
            if !mine(t.at("runner")) { return Err(aborted("another runner took over")); }
            if !stages.iter().any(|s| s == t.s("stage")) { return Err(aborted(&format!("stage changed to {}", t.s("stage")))); }
            if *t.at("armedAt") != armed { return Err(aborted("a new turn started")); }
            f(t);
            Ok(Change::Write)
        })?;
        self.stages = vec![t.t("stage")];
        self.armed = t.at("armedAt").clone();
        Ok(t)
    }
    pub fn allow(&mut self, more: &[&str]) { self.stages.extend(more.iter().map(|s| s.to_string())); }
}

/// What a shell command of the repo's own gave: whether it passed, and its output.
pub struct ShOut { pub ok: bool, pub out: String, pub aborted: bool }
/// A shell command of the repo's own (setup, check, teardown), run through a login shell. `alive` is asked every
/// second, so a runner that was superseded kills its check instead of finishing it; `tick` is called at every poll (the
/// check's progress is read there). What either says ends the command the same way.
pub fn sh(cmd: &str, cwd: &std::path::Path, env: &[(String, String)], timeout_min: f64, alive: Option<&dyn Fn() -> bool>, tick: Option<&mut dyn FnMut() -> Res<()>>) -> ShOut {
    use std::io::Read;
    user_env();
    let mut c = Command::new("sh");
    c.args(["-lc", cmd]).current_dir(cwd).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    for (k, v) in env { c.env(k, v); }
    detach(&mut c);
    let mut child = match c.spawn() { Ok(ch) => ch, Err(e) => return ShOut { ok: false, out: e.to_string(), aborted: false } };
    let pid = child.id() as i32;
    let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let mut readers = vec![];
    for stream in [child.stdout.take().map(|s| Box::new(s) as Box<dyn Read + Send>), child.stderr.take().map(|s| Box::new(s) as Box<dyn Read + Send>)].into_iter().flatten() {
        let b = buf.clone();
        readers.push(std::thread::spawn(move || {
            let mut s = stream;
            let mut chunk = [0u8; 8192];
            while let Ok(n) = s.read(&mut chunk) { if n == 0 { break; } let mut b = b.lock().unwrap(); b.extend_from_slice(&chunk[..n]); if b.len() > 8 << 20 { let cut = b.len() - (4 << 20); b.drain(..cut); } }
        }));
    }
    let kill = || unsafe { libc::kill(-pid, libc::SIGKILL); };
    let until = std::time::Instant::now() + std::time::Duration::from_secs_f64(timeout_min * 60.0);
    let mut last_poll = std::time::Instant::now();
    // The command is looked at every second (HERDR_PROJECTS_TICK_MS: a test of what a long check shows looks sooner).
    let every = std::time::Duration::from_millis(std::env::var("HERDR_PROJECTS_TICK_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(1000));
    let mut tick = tick;
    let (ok, why) = loop {
        match child.try_wait() {
            Ok(Some(st)) => break (st.success(), String::new()),
            Ok(None) => {}
            Err(e) => break (false, e.to_string()),
        }
        if std::time::Instant::now() >= until { kill(); let _ = child.wait(); break (false, format!("stopped after {} minutes", timeout_min)); }
        if last_poll.elapsed() >= every {
            last_poll = std::time::Instant::now();
            let stop = alive.map(|a| !a()).unwrap_or(false) || tick.as_mut().map(|t| t().is_err()).unwrap_or(false);
            if stop { kill(); let _ = child.wait(); break (false, "aborted".into()); }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    for r in readers { let _ = r.join(); }
    let mut out = String::from_utf8_lossy(&buf.lock().unwrap()).into_owned();
    if !why.is_empty() { out.push('\n'); out.push_str(&why); }
    ShOut { ok, out: out.trim().to_string(), aborted: why == "aborted" }
}

//! Where the board keeps things, reading and writing files, and locks.
use crate::clock::now;
use crate::err::{Error, Res};
use crate::json::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Every stage a task can be in. The board's five states are derived from these in state.rs.
/// "plan": its plan is written and waits for your yes. Several attempts: `attempts` (under way), `pick` (more than one
/// passed the check: you choose), `tried` (an attempt that passed). "after": delegated, and waits for other tasks to be
/// done (after.rs).
pub const STAGES: &[&str] = &["todo", "progress", "queued", "starting", "setup", "working", "stopped", "nosummary", "asks", "checking",
    "pushing", "reviewing", "reviewneeds", "plan", "approval", "merging", "failed", "done", "attempts", "pick", "tried", "after"];
/// A slot is held only while something is actually running; tasks waiting for the human never block the queue.
pub const SLOT_STAGES: &[&str] = &["starting", "setup", "working", "checking", "pushing", "reviewing", "merging"];
/// Stages owned by a runner (a long job of ours), as opposed to the agent's own turn.
pub const RUNNER_STAGES: &[&str] = &["starting", "setup", "checking", "pushing", "reviewing", "merging"];
pub const KINDS: &[&str] = &["permission", "question", "auth"];
/// The version of this program: what the marks and the keeper compare.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Everything the board keeps for a user is in one folder, the same on every machine: ~/.herdr-projects, with a
// sub-folder for each kind of thing (docs/home-folder.md). HERDR_PROJECTS_HOME names another one; each sub-folder
// has a variable of its own too. The XDG_* variables only say where an older version kept things (move_home.rs).
// Under the tests none of them may be the real one (never_real.rs).
pub const HOME_NAME: &str = ".herdr-projects";
fn env(k: &str) -> Option<String> { std::env::var(k).ok().filter(|v| !v.is_empty()) }
pub fn user_home() -> PathBuf { env("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/")) }
pub fn home_dir() -> PathBuf { env("HERDR_PROJECTS_HOME").map(PathBuf::from).unwrap_or_else(|| user_home().join(HOME_NAME)) }
/// Each part of the home folder: its own variable, else its place in the home folder.
const PARTS: [(&str, &str); 3] = [("HERDR_PROJECTS_DATA_DIR", "data"), ("HERDR_PROJECTS_CONFIG_DIR", "settings"), ("HERDR_PROJECTS_REPORTS_DIR", "reports")];
fn part((k, name): (&str, &str)) -> PathBuf { super::never_real::guard(env(k).map(PathBuf::from).unwrap_or_else(|| home_dir().join(name))) }
pub fn data_dir() -> PathBuf { part(PARTS[0]) }
pub fn config_dir() -> PathBuf { part(PARTS[1]) }
/// Pages and reports an agent makes for the human that belong in no repo.
pub fn reports_dir() -> PathBuf { part(PARTS[2]) }
/// The board's own folders as resolved here, for a process started somewhere that does not inherit this
/// environment (an agent's herdr space): the home folder, and a part's own variable only for a part kept elsewhere.
/// So HERDR_PROJECTS_HOME set to a scratch folder in front of a command there moves all of it: a part's variable
/// handed on would have kept that part in the real board (HP-172).
pub fn board_env() -> Vec<(String, String)> {
    let home = absolute(&home_dir());
    let mut out = vec![("HERDR_PROJECTS_HOME".to_string(), home.display().to_string())];
    for (k, name) in PARTS {
        let p = absolute(&part((k, name)));
        if p != home.join(name) { out.push((k.to_string(), p.display().to_string())); }
    }
    out
}
pub fn tasks_dir() -> PathBuf { data_dir().join("tasks") }
pub fn has_tasks() -> bool { tasks_dir().exists() }
pub fn dir_of(id: &str) -> PathBuf { let mut p = tasks_dir(); for part in id.split('/') { p.push(part); } p }
pub fn artifacts_dir(id: &str) -> PathBuf { dir_of(id).join("artifacts") }
/// This program, by its full path: what agents and hooks are handed, and what a process starts its successors from.
/// Once an update put a new file at that path, Linux names the running one "… (deleted)", a file nothing can be started
/// from: the path is the one without it, where the new program is.
pub fn bin() -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    let Ok(p) = std::env::current_exe() else { return PathBuf::from("herdr-projects") };
    match p.as_os_str().as_bytes().strip_suffix(b" (deleted)") { Some(b) => PathBuf::from(std::ffi::OsStr::from_bytes(b)), None => p }
}
pub fn absolute(p: &Path) -> PathBuf { if p.is_absolute() { p.to_path_buf() } else { std::env::current_dir().map(|d| d.join(p)).unwrap_or_else(|_| p.to_path_buf()) } }

/// The user's name, as the owner of a task.
pub fn me() -> String {
    let name = unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() { String::new() } else { std::ffi::CStr::from_ptr((*pw).pw_name).to_string_lossy().into_owned() }
    };
    let name = if name.is_empty() { env("USER").unwrap_or_default() } else { name };
    let clean: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || "_.+-".contains(*c)).take(32).collect();
    if clean.is_empty() { "you".into() } else { clean }
}

pub fn read_text(file: &Path) -> Option<String> { fs::read_to_string(file).ok() }
/// A JSON file, or None when it is missing or not JSON.
pub fn read_json(file: &Path) -> Option<Value> { read_text(file).and_then(|t| serde_json::from_str(&t).ok()) }
pub fn read_obj(file: &Path) -> Option<Obj> { match read_json(file) { Some(Value::Object(o)) => Some(o), _ => None } }
/// Written whole or not at all: a temporary file beside it, then renamed over it.
pub fn write_json(file: &Path, value: &Value) -> Res<()> {
    write_atomic(file, serde_json::to_string_pretty(&tidy(value.clone()))?.as_bytes())
}
pub fn write_atomic(file: &Path, bytes: &[u8]) -> Res<()> {
    if let Some(d) = file.parent() { fs::create_dir_all(d)?; }
    let tmp = PathBuf::from(format!("{}.{}.tmp", file.display(), std::process::id()));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, file)?;
    Ok(())
}
pub fn append_line(file: &Path, value: &Value) -> Res<()> {
    if let Some(d) = file.parent() { fs::create_dir_all(d)?; }
    let mut f = OpenOptions::new().create(true).append(true).open(file)?;
    f.write_all(format!("{}\n", serde_json::to_string(&tidy(value.clone()))?).as_bytes())?;
    Ok(())
}
/// One JSON value a line; a file that cannot be read is no lines.
pub fn read_lines(file: &Path) -> Vec<Value> {
    let Some(text) = read_text(file) else { return vec![] };
    let mut out = vec![];
    for l in text.split('\n').filter(|l| !l.is_empty()) {
        match serde_json::from_str(l) { Ok(v) => out.push(v), Err(_) => return vec![] }
    }
    out
}

pub fn pid_alive(pid: i64) -> bool {
    if pid <= 0 { return false; }
    let r = unsafe { libc::kill(pid as libc::pid_t, 0) };
    r == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}
/// A process marker is a pid plus that process's start time, so a pid reused after a reboot is not mistaken for it.
pub fn proc_start(pid: i64) -> Option<String> {
    let s = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let rest = &s[s.rfind(')')? + 2..];
    rest.split(' ').nth(19).map(|x| x.to_string())
}
pub fn marker() -> Obj {
    let pid = std::process::id() as i64;
    obj(json!({ "pid": pid, "start": proc_start(pid), "started": now() }))
}
pub fn marker_alive(m: &Value) -> bool {
    let (pid, start) = (m.i("pid"), m.t("start"));
    pid > 0 && pid_alive(pid) && (start.is_empty() || proc_start(pid).as_deref() == Some(start.as_str()))
}

// ---- locks ---------------------------------------------------------------------------------------------------------
// One kernel lock per name (flock), on a file of its own in data/locks. The kernel releases it when the process ends or
// crashes, so there are no stale owners to repair. A thread that takes a lock it already holds just goes on.
thread_local! { static HELD: RefCell<HashSet<PathBuf>> = RefCell::new(HashSet::new()); }

fn lock_file(name: &str) -> PathBuf {
    data_dir().join("locks").join(format!("{}.lock", base64url(name.as_bytes())))
}
pub fn base64url(bytes: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, b)| n | (*b as u32) << (16 - 8 * i));
        for i in 0..chunk.len() + 1 { out.push(A[(n >> (18 - 6 * i) & 63) as usize] as char); }
    }
    out
}
fn same_file(f: &File, path: &Path) -> bool {
    match (f.metadata(), fs::metadata(path)) { (Ok(a), Ok(b)) => a.ino() == b.ino() && a.dev() == b.dev(), _ => false }
}
fn flock(f: &File, wait: bool) -> bool {
    use std::os::fd::AsRawFd;
    unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | if wait { 0 } else { libc::LOCK_NB }) == 0 }
}
/// A lock taken: it is let go when this is dropped.
pub struct Held { file: Option<File>, path: PathBuf }
impl Drop for Held {
    fn drop(&mut self) { if self.file.take().is_some() { HELD.with(|h| h.borrow_mut().remove(&self.path)); } }
}
fn take_lock(name: &str, path: &Path, wait_ms: i64) -> Res<File> {
    if let Some(d) = path.parent() { fs::create_dir_all(d)?; }
    let until = now() + wait_ms.max(0);
    loop {
        let f = OpenOptions::new().create(true).append(true).open(path)?;
        let mut got = flock(&f, false);
        while !got && now() < until && same_file(&f, path) {
            std::thread::sleep(std::time::Duration::from_millis(15));
            got = flock(&f, false);
        }
        if !got {
            if !same_file(&f, path) && now() < until { continue; }
            return Err(Error::busy(format!("{name} is busy with another action. Try again in a moment.")));
        }
        // The file was opened before it was locked. If another file took its place at this path in between (the
        // one-time move puts two folders of locks together), what is held is a lock nobody else will ever ask for.
        if !same_file(&f, path) { continue; }
        return Ok(f);
    }
}
/// Runs `f` while holding the lock of this name, waiting up to two seconds for it.
pub fn with_lock<T>(name: &str, f: impl FnOnce() -> Res<T>) -> Res<T> { with_lock_wait(name, 2000, f) }
pub fn with_lock_wait<T>(name: &str, wait_ms: i64, f: impl FnOnce() -> Res<T>) -> Res<T> {
    let path = lock_file(name);
    if HELD.with(|h| h.borrow().contains(&path)) { return f(); } // nested on this thread
    let file = take_lock(name, &path, wait_ms)?;
    HELD.with(|h| h.borrow_mut().insert(path.clone()));
    let _held = Held { file: Some(file), path };
    f()
}
/// A lock held until the returned value is dropped, or the process ends (the keeper holds one for its whole life).
pub fn hold_lock(name: &str, wait_ms: i64) -> Res<Held> {
    let path = lock_file(name);
    let file = take_lock(name, &path, wait_ms)?;
    HELD.with(|h| h.borrow_mut().insert(path.clone()));
    Ok(Held { file: Some(file), path })
}
/// Whether somebody holds the lock of this name right now (a lock taken and let go at once).
pub fn lock_free(name: &str) -> bool {
    let path = lock_file(name);
    match OpenOptions::new().create(true).append(true).open(&path) { Ok(f) => flock(&f, false), Err(_) => true }
}
/// A task's lock: an attempt (`repo/12b`) shares its own id's.
pub fn task_key(id: &str) -> String { id.split('/').take(2).collect::<Vec<_>>().join("/") }
pub fn with_task_lock<T>(id: &str, wait_ms: i64, f: impl FnOnce() -> Res<T>) -> Res<T> {
    with_lock_wait(&format!("task:{}", task_key(id)), wait_ms, f)
}
/// Waits for a lock forever (the one-time moves and the keeper's start).
pub fn wait_lock(name: &str) -> Res<Held> {
    let path = lock_file(name);
    if let Some(d) = path.parent() { fs::create_dir_all(d)?; }
    let f = OpenOptions::new().create(true).append(true).open(&path)?;
    if !flock(&f, true) { return Err(Error::busy(format!("{name} is busy"))); }
    HELD.with(|h| h.borrow_mut().insert(path.clone()));
    Ok(Held { file: Some(f), path })
}

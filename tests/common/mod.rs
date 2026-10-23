//! What the tests share: a world of their own (a temporary board folder, a git repo, a fake herdr answering in the
//! test's thread, scripted agents), and running what would have been detached runners until nothing moves. No model
//! calls, no real herdr, no real board.
#![allow(dead_code)]
pub mod agents;
pub mod board;
pub mod endings;
pub mod fixtures;
pub mod gh;
pub mod herdr;
pub mod opener;
pub mod panes;
pub mod pilot;
pub mod worktrees;

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};
use sw::json::*;

/// A world sets this process's environment, which every test file shares (they are one test program): one test at a
/// time in all of them. A test that sets the environment without a world takes the same turn (`one_at_a_time`).
static ONE: Mutex<()> = Mutex::new(());
pub fn one_at_a_time() -> MutexGuard<'static, ()> { ONE.lock().unwrap_or_else(|e| e.into_inner()) }
static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
pub struct World { pub base: PathBuf, pub root: PathBuf, pub fake: Rc<herdr::FakeHerdr>, pub gh: Rc<gh::FakeGh>, pub opener: Rc<opener::FakeOpener>, pub socket: String, _listen: std::os::unix::net::UnixListener, _one: MutexGuard<'static, ()> }
#[derive(Default)]
pub struct Opts { pub repo: bool, pub settings: Value, pub repo_settings: Option<Value>, pub origin: bool }
pub fn git(cwd: &Path, a: &[&str]) -> String { worktrees::git(cwd, a).unwrap_or_else(|e| panic!("git {a:?}: {e}")) }
fn set(k: &str, v: &str) { unsafe { std::env::set_var(k, v) } }
pub fn world() -> World { world_with(Opts { repo: true, ..Default::default() }) }
pub fn world_with(o: Opts) -> World {
    let one = one_at_a_time();
    sw::clock::set(None);
    sw::board::key_style::system_is(Some(false)); // `auto` names keys as on a PC, whatever system runs the test
    let base = std::env::temp_dir().join(format!("sw-test-{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("home")).unwrap();
    let base = std::fs::canonicalize(&base).unwrap();
    let b = |p: &str| base.join(p).display().to_string();
    for k in ["HERDR_PANE_ID", "HERDR_TAB_ID", "HERDR_WORKSPACE_ID", "HERDR_PLUGIN_ID", "HERDR_PLUGIN_CONTEXT_JSON", "HERDR_ENV", "HERDR_SESSION", "TMUX", "NO_COLOR", "XDG_DATA_HOME", "HERDR_PROJECTS_DATA_DIR", "HERDR_PROJECTS_CONFIG_DIR", "HERDR_PROJECTS_REPORTS_DIR", "HERDR_PROJECTS_WEBUI", "HERDR_PROJECTS_PROGRESS", "HERDR_PROJECTS_SANDBOX", "HERDR_PROJECTS_BOXED", "HERDR_PROJECTS_RETRY_MS", "HERDR_PROJECTS_ENDED_MS", "HERDR_PROJECTS_QUIET_MS", "HERDR_PROJECTS_READY_MS", "HERDR_PROJECTS_SUBMIT_MS", "HERDR_PROJECTS_TICK_MS", "ELEVENLABS_API_KEY", "CLAUDE_CODE_TMPDIR", "HERDR_PROJECTS_SIZE"] { unsafe { std::env::remove_var(k) } }
    let socket = b("herdr.sock");
    // herdr's socket is there (the fake answers for herdr itself): what looks for a running herdr finds one.
    let listen = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    std::fs::create_dir_all(base.join("tmp")).unwrap();
    for (k, v) in [("HOME", b("home")), ("HERDR_PROJECTS_TMP", b("tmp")), ("HERDR_PROJECTS_HOME", b("sw")), ("HERDR_SOCKET_PATH", socket.clone()), ("XDG_CONFIG_HOME", b("home/.config")), ("XDG_STATE_HOME", b("home/.state")), ("XDG_DATA_HOME", b("home/.local/share")),
        ("CLAUDE_CONFIG_DIR", b("claude")), ("CODEX_HOME", b("codex")), ("PI_CODING_AGENT_DIR", b("pi")), ("HERDR_PROJECTS_CLAUDE_JSON", b("claude.json")), ("HERDR_PROJECTS_CODEX_TOML", b("codex.toml"))] { set(k, &v); }
    for (k, v) in [("HERDR_PROJECTS_INLINE", "1"), ("HERDR_PROJECTS_SETTLE_MS", "0"), ("HERDR_PROJECTS_PR_LAG_MS", "0"), ("HERDR_PROJECTS_FETCH_PAUSE_MS", "0"), ("HERDR_PROJECTS_READY_STEP_MS", "0"), ("HERDR_PROJECTS_POLL_MS", "20"),
        ("HERDR_PROJECTS_NO_KEEPER", "1"), ("HERDR_PROJECTS_NO_SWEEP", "1"), ("HERDR_PROJECTS_FREE", "1000000000000"), ("HERDR_PROJECTS_NO_HOOK", "1"), ("HERDR_PROJECTS_NO_MODEL_LIST", "1"), ("HERDR_PROJECTS_READ_MS", "0"), ("HERDR_PROJECTS_NO_SERVER", "1"), ("HERDR_PROJECTS_LINKS", "local"),
        ("HERDR_BIN_PATH", "/bin/false"), ("GIT_AUTHOR_NAME", "tester"), ("GIT_AUTHOR_EMAIL", "t@test"), ("GIT_COMMITTER_NAME", "tester"), ("GIT_COMMITTER_EMAIL", "t@test"), ("GIT_CONFIG_GLOBAL", "/dev/null"), ("GIT_CONFIG_NOSYSTEM", "1")] { set(k, v); }
    let data = sw::store::files::data_dir();
    let config = sw::store::files::config_dir();
    std::fs::create_dir_all(&data).unwrap(); std::fs::create_dir_all(&config).unwrap();
    // Every agent program counts as installed, without being run for its version.
    let now = sw::clock::now();
    let versions: serde_json::Map<String, Value> = sw::agents::kinds::all().map(|k| (k.name.to_string(), json!({ "v": "9.9.9", "at": now, "found": true }))).collect();
    std::fs::write(data.join("versions.json"), Value::Object(versions).to_string()).unwrap();
    let mut settings = json!({ "watcher.agent": "none", "coordinator.agent": "none" });
    for (k, v) in o.settings.as_object().cloned().unwrap_or_default() { settings[k] = v; }
    std::fs::write(config.join("config.json"), settings.to_string()).unwrap();
    let root = base.join("proj");
    std::fs::create_dir_all(&root).unwrap();
    if o.repo {
        git(&root, &["init", "-q", "-b", "main"]);
        std::fs::write(root.join("a.txt"), "one\n").unwrap();
        if let Some(rs) = &o.repo_settings { std::fs::write(root.join(".herdr-projects.json"), rs.to_string()).unwrap(); }
        git(&root, &["add", "-A"]); git(&root, &["commit", "-qm", "init"]);
        if o.origin { let bare = base.join("origin.git"); git(&base, &["init", "-q", "--bare", "-b", "main", &bare.display().to_string()]); git(&root, &["remote", "add", "origin", &bare.display().to_string()]); git(&root, &["push", "-q", "-u", "origin", "main"]); }
    }
    let fake = Rc::new(herdr::FakeHerdr { st: std::cell::RefCell::new(herdr::empty()), dir: base.join("fake"), script: std::cell::RefCell::new(json!({})), env: vec![] });
    std::fs::create_dir_all(base.join("fake")).unwrap();
    sw::herdr::cli::set_fake(Some(fake.clone()));
    FAKE.with(|f| *f.borrow_mut() = Some(fake.clone()));
    // gh too: never the real program, never the network.
    let gh = Rc::new(gh::FakeGh::default());
    { let g = gh.clone(); sw::flow::git::set_fake_gh(Some(Rc::new(move |a: &[String]| g.run(a)))); }
    // And the browser's opener: a link opened is only logged.
    let opener = Rc::new(opener::FakeOpener::default());
    { let o = opener.clone(); sw::opener::set_fake(Some(Rc::new(move |l: &str| o.run(l)))); }
    World { base, root, fake, gh, opener, socket, _listen: listen, _one: one }
}
impl Drop for World {
    // Out of the world's folder before it goes (a test may have gone into it): the other tests run in this same
    // process, and a program they start would find its folder gone.
    fn drop(&mut self) { let _ = std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")); FAKE.with(|f| *f.borrow_mut() = None); sw::webui::web::forget(); sw::herdr::cli::set_fake(None); sw::flow::git::set_fake_gh(None); sw::opener::set_fake(None); sw::clock::set(None); sw::runner::JOBS.with(|j| j.borrow_mut().clear()); let _ = std::process::Command::new("chmod").args(["-R", "u+w"]).arg(&self.base).status(); let _ = std::fs::remove_dir_all(&self.base); }
}
impl World {
    /// The scripted turns, by role: { "worker": [step, …], "reviewer": […], "picker": […], "coordinator": […] }.
    pub fn script(&self, s: Value) { *self.fake.script.borrow_mut() = s; }
    pub fn state(&self) -> Value { self.fake.st.borrow().clone() }
    pub fn patch(&self, f: impl FnOnce(&mut Value)) { f(&mut self.fake.st.borrow_mut()); }
    pub fn pane_of(&self, t: &Obj) -> Value { self.state().arr("panes").iter().find(|p| p.s("pane_id") == t.at("link").s("pane")).cloned().unwrap_or(Value::Null) }
    pub fn root_str(&self) -> String { self.root.display().to_string() }
    /// The rest of the turn of the task's agent (its step's `then`, agents.rs), now.
    pub fn go_on(&self, t: &Obj) { let pi = self.state().arr("panes").iter().position(|p| p.s("pane_id") == t.at("link").s("pane")); if let Some(pi) = pi { agents::go_on(&self.fake, pi); } }
    /// What the fake gh answers from now on (gh.rs).
    pub fn gh_says(&self, c: Value) { *self.gh.cfg.borrow_mut() = c; }
    pub fn add(&self, title: &str) -> String { sw::actions::filing::add(&self.root, title, Default::default()).unwrap().t("id") }
}
thread_local! { static FAKE: std::cell::RefCell<Option<Rc<herdr::FakeHerdr>>> = const { std::cell::RefCell::new(None) }; }
/// The closing notes of the turns that ended so far, written between the board's jobs (herdr.rs: notes).
pub fn flush() { FAKE.with(|f| if let Some(f) = f.borrow().as_ref() { f.notes(); }); }
/// A task as the board names it in words: its repo's key and its number (`PRJ-1` for proj/1).
pub fn short(id: &str) -> String { sw::store::keys::say_id(id, &sw::store::keys::repo_keys()) }
pub fn task(id: &str) -> Obj { sw::store::tasks::get(id).unwrap_or_else(|| panic!("no task {id}")) }
/// Run the jobs that would have been detached runners, then let the hook see the worker's pane, until nothing moves.
pub fn settle(id: &str) -> Obj {
    let seen = || { let t = sw::store::tasks::get(id).unwrap_or_default(); let mut h = t.at("handled").clone(); if let Some(o) = h.as_object_mut() { o.remove("at"); } let mut l = t.at("last").clone(); if let Some(o) = l.as_object_mut() { o.remove("at"); } json!([t.at("stage"), h, t.at("armedAt"), l]).to_string() };
    for _ in 0..12 {
        flush();
        sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); });
        flush();
        let before = seen();
        let t = sw::store::tasks::get(id).unwrap_or_default();
        if t.at("link").b("pane") { sw::hook::on_pane(&sw::herdr::cli::current_socket(), t.at("link").s("pane")); }
        if sw::runner::JOBS.with(|j| j.borrow().is_empty()) && seen() == before { break; }
    }
    task(id)
}
/// What a task's row on the board is (the flow's own view of it, with what herdr says).
pub fn row(id: &str) -> Obj { sw::flow::live::board(true).into_iter().find(|t| t.s("id") == id).unwrap_or_default() }
pub fn delegate(id: &str) { sw::actions::start::delegate(id, vec![]).unwrap_or_else(|e| panic!("delegate {id}: {}", e.msg)); }
pub fn read(p: &Path) -> String { std::fs::read_to_string(p).unwrap_or_default() }
/// The pane's agent, as the fake herdr has it now.
pub fn agent_of(w: &World, pane: &str) -> Value { w.state().arr("panes").iter().find(|p| p.s("pane_id") == pane).map(|p| p.at("agent").clone()).unwrap_or(Value::Null) }
pub fn prompts(w: &World, pane: &str) -> Vec<String> { agent_of(w, pane).arr("prompts").iter().map(text).collect() }
/// The agent in this pane moves to `status`, as herdr would see it; then the hook looks.
pub fn set_status(w: &World, pane: &str, status: &str) {
    w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["status"] = json!(status); p["agent"]["seq"] = json!(seq); } } });
    sw::hook::on_pane(&w.socket, pane);
}
pub fn on_pane(w: &World, pane: &str) { sw::hook::on_pane(&w.socket, pane); }
/// A task made, delegated and taken through its agent's one commit, its check and its review: waiting for approval.
pub fn to_approval(w: &World, title: &str) -> String {
    let mut s = w.fake.script.borrow().clone();
    if !s.at("worker").is_array() { s["worker"] = json!([{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }]); }
    w.script(s);
    let id = w.add(title);
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    id
}
/// Like `settle`, for a task done several times at once: every attempt's pane is looked at too, and the housekeeping
/// pass (which starts queued attempts and settles finished ones) runs between the rounds.
pub fn settle_all(id: &str) -> Obj {
    let all = || { let t = sw::store::tasks::get(id); let mut v: Vec<Obj> = t.iter().cloned().collect(); if let Some(t) = &t { v.extend(sw::store::attempts::attempts_of(t)); } v };
    let seen = || json!(all().iter().map(|t| { let mut h = t.at("handled").clone(); if let Some(o) = h.as_object_mut() { o.remove("at"); } let mut l = t.at("last").clone(); if let Some(o) = l.as_object_mut() { o.remove("at"); } json!([t.at("stage"), h, t.at("armedAt"), l, t.at("link").at("pane")]) }).collect::<Vec<_>>()).to_string();
    let socket = sw::herdr::cli::current_socket();
    for _ in 0..30 {
        flush();
        sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); });
        flush();
        let before = seen();
        for t in all() { if t.at("link").b("pane") { sw::hook::on_pane(&socket, t.at("link").s("pane")); } }
        sw::housekeeping::housekeeping(&socket, sw::housekeeping::Force::Yes);
        if sw::runner::JOBS.with(|j| j.borrow().is_empty()) && seen() == before { break; }
    }
    task(id)
}
pub fn kids(id: &str) -> Vec<Obj> { sw::store::attempts::attempts_of(&task(id)) }
pub fn kid(id: &str, letter: &str) -> Obj { kids(id).into_iter().find(|c| c.s("letter") == letter).unwrap_or_default() }
pub fn step(n: i64) -> Value { json!({ "files": { "f.txt": format!("v{n}\n") }, "commit": format!("v{n}"), "note": format!("Did v{n}.") }) }
pub fn delegate_with(id: &str, asked: Value) -> Obj { sw::actions::start::delegate(id, asked.as_object().cloned().unwrap_or_default().into_iter().collect()).unwrap_or_else(|e| panic!("delegate {id}: {}", e.msg)) }
/// A world whose agents keep their conversations inside it, whose waits are short, and whose agents are processes
/// of their own that the board can end.
pub fn cut_world(waits: &str, settings: Value) -> World {
    let w = world_with(Opts { repo: true, settings, ..Default::default() });
    for (k, v) in [("HERDR_PROJECTS_RETRY_MS", waits), ("HERDR_PROJECTS_ENDED_MS", "0"), ("HERDR_PROJECTS_QUIET_MS", "60")] { set(k, v); }
    w.patch(|s| s["pids"] = json!(true));
    w
}
/// The working agents started, by program, in order (not the reviewers).
pub fn starts(w: &World) -> Vec<String> { w.state().arr("calls").iter().map(text).filter(|c| c.starts_with("agent start") && !regex::Regex::new(r"^agent start \S+-r ").unwrap().is_match(c)).map(|c| c.split(' ').nth(4).unwrap_or("").to_string()).collect() }
/// Every call gh was asked, one per line.
pub fn gh_log(w: &World) -> String { w.gh.log.borrow().iter().map(|l| format!("{l}\n")).collect() }
pub fn calls(w: &World) -> Vec<String> { w.state().arr("calls").iter().map(text).collect() }

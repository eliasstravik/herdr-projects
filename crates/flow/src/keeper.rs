//! The keeper: a small always-on background process that does what the open board's tick does, with no screen, so
//! the board's information stays fresh and you are alerted with the board closed. One per data folder.
//!
//! Started detached from herdr's [[startup]] hook, from any herdr hook event and from the board opening, never from an
//! agent's own command (so it is never born inside an agent's process group). It looks after this machine's tasks only.
//!
//! Two processes: the keeper itself only holds the lock, rewrites its heartbeat once a second and watches for an
//! update of the program; a child does the ticking, so a slow herdr or gh call never stops the heartbeat. On an
//! update it starts its successor itself and goes only once that one beats, so a keeper runs on with no board open
//! and no agent's hook to start one.
use crate::clock::{iso, now};
use crate::err::Kind;
use crate::flow::{live::board_for, sidebar::sync_sidebar, task::is_agent_task};
use crate::herdr::{cli::{current_socket, socket_exists}, Herdr};
use crate::housekeeping::{housekeeping, loose, spawn_loose, Force};
use crate::json::*;
use crate::runner::{inline, user_env};
use crate::store::{files::{bin, data_dir, hold_lock, proc_start, read_obj, with_lock_wait, write_json, VERSION}, marks::due, repos::cwd_of, tasks::{list, update_wait, Change, ListOpts}};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

fn beat_file() -> PathBuf { data_dir().join("keeper.json") }
fn log(msg: &str) { let _ = std::fs::OpenOptions::new().create(true).append(true).open(data_dir().join("keeper.log")).and_then(|mut f| { use std::io::Write; writeln!(f, "{} {} {msg}", iso(now()), std::process::id()) }); }
/// Alive means the heartbeat file is less than five seconds old. Readers never touch the keeper's lock.
pub fn keeper_alive() -> bool { std::fs::metadata(beat_file()).and_then(|m| m.modified()).map(|m| m.elapsed().map(|e| e.as_millis() < 5000).unwrap_or(true)).unwrap_or(false) }
fn no_keeper() -> bool { std::env::var("HERDR_PROJECTS_NO_KEEPER").as_deref() == Ok("1") }
pub fn ensure_keeper() -> bool {
    if no_keeper() || inline() || keeper_alive() { return false; }
    if !due("keeper-start", 4000) { return false; } // several hook events at once start one, not five
    spawn_loose(&["keeper"]);
    true
}
/// The program that runs, as it is on disk now: a keeper started from an older one hands over to a successor.
pub fn code_stamp(exe: &Path) -> String {
    let m = std::fs::metadata(exe).and_then(|m| m.modified()).map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)).unwrap_or(0);
    format!("{VERSION}:{m}")
}
/// How often the keeper looks at the program on disk (HERDR_PROJECTS_STAMP_MS: the tests that update it look sooner).
fn stamp_every() -> i64 { std::env::var("HERDR_PROJECTS_STAMP_MS").ok().and_then(|s| s.parse().ok()).filter(|&n: &i64| n > 0).unwrap_or(5000) }

pub fn keeper_main() {
    if no_keeper() { return; }
    let Ok(lock) = hold_lock("keeper", 10000) else { return }; // another keeper has it
    let mut held = Some(lock);
    user_env();
    let exe = bin();
    let stamp = code_stamp(&exe);
    let pid = std::process::id() as i64;
    let info = json!({ "pid": pid, "start": proc_start(pid), "version": VERSION, "started": now() });
    let beat = || { let mut i = obj(info.clone()); i.put("at", now()); let _ = write_json(&beat_file(), &Value::Object(i)); };
    beat();
    log(&format!("started (plugin {VERSION})"));
    static ENDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    extern "C" fn on_signal(_: i32) { ENDING.store(true, std::sync::atomic::Ordering::SeqCst); }
    unsafe { for s in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] { libc::signal(s, on_signal as *const () as usize); } }
    let mut child: Option<std::process::Child> = None;
    let mut restart_at = 0i64;
    let why: Option<String>;
    let every = stamp_every();
    let (mut seen, mut look_at, mut tries) = (stamp.clone(), now() + every, 0i64);
    loop {
        if ENDING.load(std::sync::atomic::Ordering::SeqCst) { why = Some("a signal".into()); break; }
        if child.is_none() && now() >= restart_at {
            let mut c = std::process::Command::new(&exe);
            c.arg("keeper-work").current_dir(data_dir()).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
            child = c.spawn().ok();
            if child.is_none() { restart_at = now() + 2000; }
        }
        if let Some(ch) = child.as_mut() {
            if let Ok(Some(st)) = ch.try_wait() {
                child = None;
                if st.code() == Some(3) { why = Some("herdr is gone".into()); break; }
                restart_at = now() + 2000;
            }
        }
        beat();
        if now() >= look_at {
            look_at = now() + every;
            let now_stamp = code_stamp(&exe);
            // A new program, settled: the same file on two looks in a row (one cargo is still writing changes between
            // them). The successor is started and seen beating before this one goes; one that does not come up leaves
            // this one keeping on, and trying again a little later.
            if now_stamp != stamp && now_stamp == seen && exe.is_file() {
                log("ending: the plugin was updated");
                stop(&mut child);
                held = None;
                let _ = std::fs::remove_file(beat_file());
                let failed = match handover(pid) { Ok(()) => { why = None; break; } Err(e) => e };
                // Taken back at once: a keeper that has it now (started by a hook meanwhile) runs the new program.
                let Ok(lock) = hold_lock("keeper", 0) else { why = Some("another keeper took over".into()); break };
                held = Some(lock);
                tries += 1;
                look_at = now() + every * 6 * tries.min(20);
                restart_at = 0;
                beat();
                log(&format!("the new version did not start ({failed}): keeping on, trying again in {}s", (look_at - now()) / 1000));
            }
            seen = now_stamp;
        }
        std::thread::sleep(std::time::Duration::from_millis(every.clamp(1, 1000) as u64));
    }
    if let Some(why) = why { log(&format!("ending: {why}")); }
    stop(&mut child);
    if held.is_some() { let _ = std::fs::remove_file(beat_file()); }
    drop(held);
}
fn stop(child: &mut Option<std::process::Child>) { if let Some(mut ch) = child.take() { unsafe { libc::kill(ch.id() as i32, libc::SIGTERM); } let _ = ch.wait(); } }
/// Starts the keeper on the new program and waits for its heartbeat. A program that cannot be started (still being
/// written: "text file busy"; half written: "exec format error") or that ends at once is said, and nothing runs it.
fn handover(own: i64) -> Result<(), String> {
    let mut c = loose(&["keeper"]).spawn().map_err(|e| e.to_string())?;
    let until = now() + 15000;
    while now() < until {
        // Its own beat, or that of a keeper a hook started meanwhile (this one then gives up waiting for the lock).
        if keeper_alive() && read_obj(&beat_file()).map(|b| b.i("pid") != own).unwrap_or(false) { return Ok(()); }
        if let Ok(Some(st)) = c.try_wait() { return Err(format!("it ended: {st}")); }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    crate::runner::reap(c); // still running: waited for when it ends, as no zombie of this keeper
    Err("no heartbeat in 15s".into())
}

// ---- the tick: what the board's own tick does, once a second ----------------------------------------------------------
/// One ticker at a time: whoever ticks takes the short `tick` lock without waiting and skips when it is taken, so
/// housekeeping never runs twice at once during a handover between a board and a keeper.
pub fn tick_once(socket: &str, watch: bool) -> Option<Vec<Obj>> {
    let mut rows = None;
    let r = with_lock_wait("tick", 0, || {
        if socket == current_socket() { crate::automations::tick::automations_tick(now()); } // tasks made on a schedule or by a board event
        housekeeping(socket, Force::No);
        let rs = board_for(socket);
        sync_sidebar(&rs, Some(socket), now());
        let every: Vec<Obj> = rs.iter().flat_map(|t| std::iter::once(t.clone()).chain(t.arr("tries").iter().filter_map(|x| x.as_object().cloned()))).collect();
        refresh_said(&every, now());
        // A run at its time limit is stopped; a raised limit carries on (limits/tick.rs).
        crate::limits::tick::limits_tick(&every, now(), Some(&|id: &str| { let _ = crate::actions::limits::carry_on(id, true); }));
        crate::autopilot::tick::autopilot_tick(socket, now()); // coordinators on autopilot woken (autopilot/tick.rs)
        for co in crate::coordinator::state::all(socket) { crate::coordinator::pump::tick(&co); crate::coordinator::sidebar::tell_sidebar(&co); }
        rows = Some(rs);
        Ok(())
    });
    if let Err(e) = r { if e.kind != Kind::Busy { log(&e.msg); } }
    if let (Some(rs), true) = (&rows, watch) {
        // Each attempt of a task has an agent of its own to follow (attempts.rs).
        let every: Vec<Obj> = rs.iter().flat_map(|t| std::iter::once(t.clone()).chain(t.arr("tries").iter().filter_map(|x| x.as_object().cloned()))).collect();
        crate::watcher::watch_tick(&every, now());
    }
    rows
}
static SAID_AT: LazyLock<Mutex<HashMap<String, i64>>> = LazyLock::new(Default::default);
/// While an agent works, its last message is refreshed from the saved conversation, at most once every five seconds.
pub fn refresh_said(rows: &[Obj], now_ms: i64) {
    for t in rows {
        if t.s("stage") != "working" || !is_agent_task(t) || t.b("archivedAt") || t.at("live").at("status").is_null() { continue; }
        { let mut s = SAID_AT.lock().unwrap(); if now_ms - s.get(t.s("id")).copied().unwrap_or(0) < 5000 { continue; } s.insert(t.t("id"), now_ms); }
        let Some(turn) = crate::agents::installed::read_turn_of(t.s("agent"), t.so("transcript").map(Path::new), t.s("session"), &cwd_of(t), false) else { continue };
        let usage = turn.usage.as_ref().map(|u| serde_json::to_value(u).unwrap_or(Value::Null));
        let same_ctx = usage.as_ref().map(|u| u.at("context") == t.at("usage").at("context")).unwrap_or(true);
        if (turn.said.is_empty() || turn.said == t.s("said")) && same_ctx { continue; }
        let _ = update_wait(t.s("id"), 300, |c| {
            if c.s("stage") != "working" { return Ok(Change::Keep); }
            if !turn.said.is_empty() { c.put("said", turn.said.as_str()); }
            if let Some(u) = &usage { c.put("usage", u.clone()); }
            if !turn.model.is_empty() { c.put("model", turn.model.as_str()); }
            Ok(Change::Write)
        });
    }
}
pub fn keeper_work() -> i32 {
    let mut fails: HashMap<String, u32> = HashMap::new(); // socket -> failed calls in a row
    let first = current_socket();
    loop {
        let mut sockets = vec![first.clone()];
        for t in list(ListOpts::default()) { let s = t.at("link").t("socket"); if !s.is_empty() && !sockets.contains(&s) { sockets.push(s); } }
        let mut live = 0;
        for socket in &sockets {
            let ok = socket_exists(socket) && Herdr::new(Some(socket.clone())).alive();
            let f = fails.entry(socket.clone()).or_insert(0);
            *f = if ok { 0 } else { *f + 1 };
            if *f < 3 { live += 1; }
            if ok { tick_once(socket, true); }
        }
        // The board inside the herdr web UI: its front door is seen to here, every half minute (webui/keep.rs).
        if due("front-door", 30000) { crate::webui::keep::ensure_door(); }
        // On the computer you sit at: the helper that takes a call the terminal board on another machine starts.
        if due("voice-helper", 30000) { crate::calls::helper::ensure(); }
        // Voice calls turned off: a call still on ends (its page or helper then lets go of the microphone).
        if due("voice-off", 5000) { crate::voice::end_if_off(); }
        if live == 0 { return 3; } // every herdr it knows is gone
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}

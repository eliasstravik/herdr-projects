//! The Watcher: a hidden model call that gives whatever is running a status line and a progress guess (the board
//! works out the minutes left). About once a minute per working task (`watcher.every`; less often on a busy machine:
//! watcher/pace.rs) the last screen lines go to the helper, which answers a line and a percent. One call covers every
//! agent whose look is due, and only one call runs at a time on the machine (watcher/one.rs). A screen that says what
//! it said at the last look costs no call. What a look finds is kept on the task (`watch`); the sidebar is told by
//! flow/sidebar.rs. Also here: stopping an instruction sent to another machine's Coordinator.
pub mod one;
pub mod pace;
pub mod prompt;

use crate::agents::adapter::{one_shot, OneShot};
use crate::clock::now;
use crate::flow::{send::shim, task::is_agent_task};
use crate::herdr::Herdr;
use crate::json::*;
use crate::runner::{detach, user_env};
use crate::store::{files::data_dir, settings::{lookup::setting, values::{agent_of, ms}}, tasks::{update, Change}};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Tests replace the model call with a stand-in: a JSON argv; the prompt arrives on stdin.
fn stand_in() -> Option<Vec<String>> { std::env::var("HERDR_PROJECTS_HELPER_CMD").ok().and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok()).filter(|a| !a.is_empty()) }
fn helper_command(prompt: &str, system: &str, lean: bool) -> Option<OneShot> {
    if let Some(o) = stand_in() { return Some(OneShot { command: o[0].clone(), args: o[1..].to_vec(), stdin: Some(format!("{}{prompt}", if system.is_empty() { String::new() } else { format!("{system}\n\n") })) }); }
    let a = agent_of(setting("watcher.agent", None).as_str().unwrap_or(""));
    one_shot(&a.kind, &a.model, "", prompt, system, lean)
}
/// `watcher.agent: none`: no Watcher at all, whatever stands in for the model.
pub fn watcher_off() -> bool { setting("watcher.agent", None) == json!("none") }
pub fn helpers_ready() -> bool { !watcher_off() && (stand_in().is_some() || one_shot(&agent_of(setting("watcher.agent", None).as_str().unwrap_or("")).kind, "", "", "", "", false).is_some()) }

/// Helper calls are detached from the board's terminal (own session, no tty), so herdr does not mistake them for an
/// agent in the board's pane. They find `herdr-projects` on their PATH and never start the keeper.
fn run_helper(cmd: OneShot, extra_env: &[(&str, &str)], done: impl FnOnce(i32, String, String) + Send + 'static) {
    use std::io::{Read, Write};
    use std::process::{Command, Stdio};
    user_env();
    let mut c = Command::new(&cmd.command);
    c.args(&cmd.args).current_dir(data_dir()).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let path = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default();
    let path = std::env::join_paths(shim().parent().map(|p| p.to_path_buf()).into_iter().chain(path)).unwrap_or_default();
    c.env("PATH", path).env("HERDR_PROJECTS_NO_KEEPER", "1");
    for k in ["HERDR_PANE_ID", "HERDR_TAB_ID", "HERDR_WORKSPACE_ID", "HERDR_PLUGIN_ID", "HERDR_PLUGIN_CONTEXT_JSON", "HERDR_PLUGIN_EVENT", "HERDR_PLUGIN_EVENT_JSON", "HERDR_PLUGIN_ENTRYPOINT_ID"] { c.env_remove(k); }
    for (k, v) in extra_env { c.env(k, v); }
    detach(&mut c);
    let child = c.spawn();
    std::thread::spawn(move || {
        let Ok(mut child) = child else { return done(1, String::new(), String::new()) };
        if let Some(mut i) = child.stdin.take() { let _ = i.write_all(cmd.stdin.unwrap_or_default().as_bytes()); }
        let pid = child.id() as i64;
        let killer = std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_millis(one::CALL_MS as u64)); crate::os::kill_group(pid); });
        let mut out = String::new(); let mut err = String::new();
        if let Some(mut o) = child.stdout.take() { let _ = o.read_to_string(&mut out); }
        if let Some(mut e) = child.stderr.take() { let _ = e.read_to_string(&mut err); }
        let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(1);
        drop(killer);
        let n = err.chars().count();
        done(code, out, err.chars().skip(n.saturating_sub(4000)).collect());
    });
}

struct Seen { at: i64, role: String, words: Option<String> }
#[derive(Default)]
struct Pace { seen: HashMap<String, i64>, call_at: i64, running: bool, fails: u32, looks: HashMap<String, Seen>, plain: bool }
static PACE: LazyLock<Mutex<Pace>> = LazyLock::new(Default::default);
pub fn watcher_reset() { *PACE.lock().unwrap() = Pace::default(); }
/// The turn a look belongs to. Not armedAt: herdr's status flaps between tools move that every few seconds.
fn turn_of(c: &Obj) -> String { format!("{}:{}:{}", c.i("since"), c.at("handled").i("at"), c.i("reviewRound")) }
fn role_in(t: &Obj) -> &'static str { if t.s("stage") == "reviewing" { "reviewer" } else { "agent" } }
fn pane_of(t: &Obj, role: &str) -> String { if role == "reviewer" { t.at("link").at("reviewer").t("pane") } else { t.at("link").t("pane") } }

/// What a finished helper call hands back: its exit code, what it printed, what it said went wrong.
pub type Done = Box<dyn FnOnce(i32, String, String) + Send>;
/// Starting the helper's program with these environment variables; tests answer it by hand.
pub type Run<'a> = &'a dyn Fn(OneShot, &[(&str, &str)], Done);
/// One look, once per tick of whoever ticks (the keeper). `tasks` are rows as the board shows them.
pub fn watch_tick(tasks: &[Obj], now_ms: i64) { watch_tick_with(tasks, now_ms, None, &|cmd, env, done| run_helper(cmd, env, done)) }
/// The same, with the machine's load (None: read now) and the start of the helper given.
pub fn watch_tick_with(tasks: &[Obj], now_ms: i64, load: Option<f64>, run: Run) {
    let mut p = PACE.lock().unwrap();
    // The keeper hands over the same tasks once per herdr session, each time as that session sees them.
    let view = tasks.iter().find(|t| t.at("live").is_object() && !t.at("live").b("away")).map(|t| t.at("link").t("socket")).unwrap_or_default();
    if let Some(&seen) = p.seen.get(&view) { if now_ms >= seen && now_ms - seen < pace::TICK_MS { return; } }
    p.seen.insert(view, now_ms);
    if !helpers_ready() { return; }
    p.looks.retain(|id, _| tasks.iter().any(|t| t.s("id") == id));
    // Only an agent that is working is looked at: not one that is idle, waiting for you or finished.
    let mut busy: Vec<(usize, i64)> = vec![];
    for (i, t) in tasks.iter().enumerate() {
        let role = role_in(t);
        let working = !t.b("archivedAt") && is_agent_task(t) && t.s("state") == "working" && (t.s("stage") == "reviewing" || (t.s("stage") == "working" && t.at("live").s("status") == "working"));
        if !working || pane_of(t, role).is_empty() { continue; }
        let w = p.looks.entry(t.t("id")).or_insert(Seen { at: 0, role: role.into(), words: None });
        if w.role != role { *w = Seen { at: 0, role: role.into(), words: None }; } // a review is a new assignment: its first look is due at once
        busy.push((i, w.at));
    }
    if p.running || busy.is_empty() { return; }
    let slow = pace::slowdown(load.unwrap_or_else(pace::load_now)) * pace::after_failures(p.fails);
    if ((now_ms - p.call_at) as f64) < pace::GAP_MS as f64 * slow { return; }
    let e = ms(setting("watcher.every", None).as_str().unwrap_or(""));
    let every = (if e > 0 { e } else { 60000 }).max(5000) as f64 * slow;
    let mut batch: Vec<(usize, String, String, String)> = vec![]; // task index, role, lines, words
    let mut sessions: HashMap<String, Vec<Value>> = HashMap::new();
    for i in pace::riders(&busy, now_ms, every) {
        if batch.len() >= pace::BATCH { break; }
        let t = &tasks[i];
        let role = role_in(t);
        let socket = t.at("link").t("socket");
        let api = Herdr::new(Some(socket.clone()));
        // A reviewer counts as working while herdr says so of its own pane: asked once per herdr session, only when due.
        if role == "reviewer" {
            let agents = sessions.entry(socket.clone()).or_insert_with(|| api.agents().unwrap_or_default());
            if agents.iter().find(|a| a.s("pane_id") == pane_of(t, role)).map(|a| a.s("agent_status")) != Some("working") { continue; }
        }
        let Ok(screen) = api.agent_read(&pane_of(t, role), "visible", None) else { continue }; // never a deep read: it would scroll a working agent's view
        let lines = pace::last_lines(&screen);
        let words = pace::words_of(&lines);
        let w = p.looks.get_mut(t.s("id")).unwrap();
        if lines.is_empty() { w.at = now_ms; continue; }
        if w.words.as_deref() == Some(words.as_str()) && t.at("watch").s("role") == role { w.at = now_ms; continue; } // an unchanged screen costs no call
        batch.push((i, role.into(), lines, words));
    }
    if batch.is_empty() { return; }
    let looks: Vec<prompt::Look> = batch.iter().map(|b| prompt::Look { t: &tasks[b.0], role: &b.1, lines: b.2.clone() }).collect();
    let Some(cmd) = helper_command(&prompt::watcher_prompt(&looks, now_ms), prompt::SYSTEM, !p.plain) else { return };
    if !one::take_turn(now_ms) { return; } // another process's call is under way: this one waits its turn
    for b in &batch { if let Some(w) = p.looks.get_mut(tasks[b.0].s("id")) { w.at = now_ms; w.words = Some(b.3.clone()); } }
    p.call_at = now_ms; p.running = true;
    let jobs: Vec<(String, String, String, String)> = batch.iter().map(|b| (tasks[b.0].t("id"), b.1.clone(), tasks[b.0].t("stage"), turn_of(&tasks[b.0]))).collect();
    drop(p);
    // "Only the watcher should have autocompact 100k."
    run(cmd, &[("CLAUDE_CODE_AUTO_COMPACT_WINDOW", "100000")], Box::new(move |code, out, err| {
        let mut p = PACE.lock().unwrap();
        p.running = false; one::give_turn();
        let found = prompt::read_answer(&out, jobs.len());
        // A program too old for the lean start says so: from then on it is started the plain way, and these looks are due again.
        let old = crate::re!(r"(?i)unknown (option|argument)|unrecognized|unexpected argument");
        if found.is_empty() && !p.plain && code != 0 && old.is_match(&err) { p.plain = true; for j in &jobs { if let Some(w) = p.looks.get_mut(&j.0) { w.at = 0; w.words = None; } } return; }
        p.fails = if found.is_empty() { p.fails + 1 } else { 0 };
        let at = now();
        for (i, (id, role, stage, turn)) in jobs.iter().enumerate() {
            let Some(got) = found.get(&(i + 1)) else { if let Some(w) = p.looks.get_mut(id) { w.words = None; } continue }; // no answer: its next look is not skipped as unchanged
            let mut watch = got.as_object().cloned().unwrap_or_default();
            watch.put("at", at); watch.put("role", role.as_str());
            // A late answer cannot overwrite a newer turn or its result.
            let _ = update(id, |c| { if c.s("stage") != stage || turn_of(c) != *turn || watcher_off() { return Ok(Change::Keep); } c.put("watch", watch); Ok(Change::Write) });
        }
    }));
}

/// Instructions sent to another machine's Coordinator: a row on this board until that machine answers. Stopped here.
pub fn stop_dispatch(id: &str) {
    use crate::store::inbox::{clear_dispatch, get_dispatch};
    if let Some(d) = get_dispatch(id) {
        if crate::store::files::marker_alive(&json!({ "pid": d.at("pid"), "start": d.at("pidStart") })) {
            crate::os::term_group(d.i("pid"));
        }
    }
    clear_dispatch(id);
}

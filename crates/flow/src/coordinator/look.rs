//! What the board shows of a coordinator: its state, latest message and screen; going into its chat. Read on every
//! beat of the board: one small file, one `agent get`, and the end of the saved conversation when that file changed.
//! `state` is one of: none, starting, working, blocked, idle, exited.
use super::co::Co;
use super::pump::active;
use super::requests::{open, tab_note, typed_there};
use super::rules_place::intro_line_of;
use super::state::{get, home, pumping};
use super::who::{choice, describe, same_choice};
use crate::agents::adapter;
use crate::agents::installed::read_turn_of;
use crate::agents::kinds::kind;
use crate::agents::read_turn::Turn;
use crate::agents::talk::{talk_of, Said};
use crate::err::Res;
use crate::herdr::api::read_screen;
use crate::herdr::cli::socket_exists;
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::marks::due;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

/// The Coordinator as the board shows it.
#[derive(Clone, Debug, Default)]
pub struct Look {
    pub state: String, pub status: Option<String>, pub run: Option<Obj>, pub one_off: bool, pub queue: usize, pub waiting: Vec<String>, pub error: String, pub pane: String, pub workspace: String,
    pub agent: Value, pub turn: Option<Turn>, pub talk: Vec<Said>, pub readable: bool, pub said: String, pub last: String, pub sent: String, pub unseen: String, pub stale: bool, pub def: Option<Obj>,
}
/// A transcript's last read, by when it changed and its size: two writes inside one tick of the file system's clock
/// keep its time, never its size.
static TURNS: LazyLock<Mutex<HashMap<String, (PathBuf, (std::time::SystemTime, u64), Option<Turn>)>>> = LazyLock::new(Default::default);
static SCREENS: LazyLock<Mutex<HashMap<String, (String, i64, String)>>> = LazyLock::new(Default::default);
/// What you said, without what the board put around it on its way in.
fn bare(text: &str, co: &Co) -> String {
    let mut s = text.to_string();
    let intro = intro_line_of(co);
    if let Some(r) = s.strip_prefix(&intro) { s = r.trim().to_string(); }
    let note = tab_note();
    if let Some(r) = s.strip_suffix(&note) { s = r.trim().to_string(); }
    s
}
pub fn look(co: impl Into<Co>) -> Look {
    let co = co.into();
    let socket = co.socket.as_str();
    let c = get(&co);
    let def = choice();
    let Some(c) = c.filter(|c| c.b("run") || c.b("pane") || !c.arr("queue").is_empty() || c.b("want") || c.b("error")) else {
        return Look { state: "none".into(), run: def.clone(), def, ..Default::default() };
    };
    let agent = if c.b("pane") && (crate::herdr::cli::fake().is_some() || socket_exists(socket)) { Herdr::new(Some(socket.into())).agent_get(c.s("pane")).ok().flatten() } else { None };
    let status = agent.as_ref().map(|a| a.t("agent_status"));
    let run = c.o("run").cloned().or_else(|| c.o("pick").cloned()).or_else(|| c.at("want").o("pick").cloned()).or_else(|| def.clone());
    let busy = pumping(&c) || c.b("want");
    let (mut turn, mut talk) = (None, vec![]);
    if let Some(r) = c.o("run") {
        let k = r.t("kind");
        let session = if due(&co.mark("coordinator-session"), 3000) { active(&co, &c) } else { c.so("session").map(String::from) };
        let key = format!("{}|{}", co.key(), session.clone().unwrap_or_default());
        let mut cache = TURNS.lock().unwrap();
        let hit = cache.get(&key).cloned();
        let mut file = hit.as_ref().map(|h| h.0.clone());
        if let Some(s) = &session { if file.as_ref().map(|f| !f.exists()).unwrap_or(true) { file = adapter::transcript(&k, s, &home(&co)); } }
        if let Some(f) = &file {
            let mark = std::fs::metadata(f).ok().and_then(|m| Some((m.modified().ok()?, m.len())));
            if mark != hit.as_ref().map(|h| h.1) { cache.insert(key.clone(), (f.clone(), mark.unwrap_or((std::time::UNIX_EPOCH, 0)), read_turn_of(&k, Some(f), "", &home(&co), false))); }
            turn = cache.get(&key).and_then(|h| h.2.clone());
            drop(cache);
            let h = home(&co);
            talk = talk_of(&k, Some(f), "", &h, 12).unwrap_or_default().into_iter().map(|m| if m.you { Said { text: bare(&m.text, &co), ..m } } else { m }).filter(|m| !m.text.is_empty()).collect();
        }
    }
    let queue = c.arr("queue").len();
    let state = if c.b("starting") && pumping(&c) { "starting" } else { match status.as_deref() { Some("working") => "working", Some("blocked") => "blocked", Some(_) => "idle", None => if c.b("stopped") && !c.b("want") { "exited" } else if busy || queue > 0 { "starting" } else if c.b("run") { "exited" } else { "none" } } };
    let readable = run.as_ref().map(|r| kind(r.s("kind")).map(|k| k.reads_turn).unwrap_or(false)).unwrap_or(false);
    let said = turn.as_ref().map(|t| t.said.clone()).unwrap_or_default();
    // Your newest words while they are not answered yet: still waiting to go in, or gone in with nothing new said since.
    let sent = if queue > 0 { c.arr("queue")[queue - 1].t("said") } else if c.b("last") && readable && c.at("over").is_string() && c.s("over") == said && !turn.as_ref().map(|t| t.finished && !t.id.is_empty() && t.id != c.s("overId")).unwrap_or(false) { c.t("last") } else { String::new() };
    let unseen = turn.as_ref().filter(|t| t.finished && !t.id.is_empty() && !t.said.is_empty() && t.id != c.s("seen")).map(|t| t.id.clone()).unwrap_or_default();
    // The default changed since this one started: it keeps running as it is until you start a new one.
    let stale = c.o("run").map(|r| !r.b("oneOff") && def.as_ref().map(|d| !same_choice(r, d)).unwrap_or(false)).unwrap_or(false);
    Look { state: state.into(), status, one_off: c.at("run").b("oneOff"), queue, waiting: c.arr("queue").iter().map(|m| m.t("said")).collect(), error: c.t("error"), pane: c.t("pane"), workspace: c.t("workspace"),
        agent: agent.unwrap_or(Value::Null), turn, talk, readable, said, last: c.t("last"), sent, unseen, stale, def, run }
}
/// What is on its screen, for when there is no message to show (a dialog, a program without a readable conversation).
pub fn screen(v: &Look, co: impl Into<Co>) -> String {
    let co = co.into();
    let socket = co.socket.as_str();
    let Some(_) = &v.status else { return String::new() };
    if v.pane.is_empty() { return String::new(); }
    let now = crate::clock::now();
    if let Some((p, at, text)) = SCREENS.lock().unwrap().get(&co.key()) { if *p == v.pane && now - at < 2000 { return text.clone(); } }
    let raw = read_screen(&Herdr::new(Some(socket.into())), &v.pane);
    let mut text = raw.split('\n').map(|l| l.trim_end()).collect::<Vec<_>>().join("\n");
    while text.contains("\n\n\n") { text = text.replace("\n\n\n", "\n\n"); }
    let text = text.trim().to_string();
    SCREENS.lock().unwrap().insert(co.key(), (v.pane.clone(), now, text.clone()));
    text
}
/// Going into it: herdr moves to its real pane, as it does for any task's agent.
pub fn focus(v: &Look, co: impl Into<Co>) -> Res<()> { let co = co.into(); Herdr::new(Some(co.socket.clone())).agent_focus(&v.pane)?; typed_there(&co); Ok(()) }
/// "Go into the Coordinator's chat" from anywhere in herdr: no board in between. Not running: it is started with
/// nothing said first, and this waits for it.
pub fn go_in(co: impl Into<Co>, wait_ms: i64) -> Res<Look> {
    let co = co.into();
    let mut asked = false;
    let until = crate::clock::now() + wait_ms;
    loop {
        let v = look(&co);
        if v.status.is_some() && !v.pane.is_empty() { focus(&v, &co)?; return Ok(v); }
        if asked && !v.error.is_empty() { crate::fail!("The {} could not be started: {}", co.name(), v.error); }
        if !asked { open(&co)?; asked = true; }
        if crate::clock::now() > until { crate::fail!("The {} did not start in time.", co.name()); }
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
}
/// For the command line and for other machines.
pub fn summary(co: impl Into<Co>) -> Value {
    let co = co.into();
    let v = look(&co);
    json!({ "name": co.name(), "state": v.state, "agent": v.run.as_ref().map(|r| r.t("kind")).unwrap_or_default(), "model": v.run.as_ref().map(|r| r.t("model")).unwrap_or_default(), "effort": v.run.as_ref().map(|r| r.t("effort")).unwrap_or_default(),
        "oneOff": v.one_off, "default": describe(v.def.as_ref()), "pane": v.pane, "queued": v.queue, "error": v.error, "said": v.said })
}

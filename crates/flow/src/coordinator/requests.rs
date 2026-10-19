//! Requests to the Coordinator: written down here (send, restart, new, stop, open), carried out by the pump. What the
//! human types goes in word for word. Where they were (the repo, the task they had open) is kept beside it, and the
//! Coordinator asks for it with `coordinator where`, so one conversation serves every repo the board is opened in. A
//! message sent with tab carries a line under the words saying so; the chat shows the words alone.
use super::co::Co;
use super::state::{change, get, home};
use super::who::{choice, OFF_WORDS};
use crate::board::keys::key_name;
use crate::clock::now;
use crate::err::Res;
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::marks::tilde;
use crate::store::repos::repo_of;
use crate::store::tasks::{get as get_task, one_line};
use std::path::Path;

pub fn tab_note() -> String { format!("[Sent with {} on the board: start every task you add from this message right away, without asking. Add each with \"board add … --delegate\"; if \"board add\" answers only \"added\", run \"board delegate <id>\". If this message leads to no new task, delegate nothing extra.]", key_name("delegate")) }
pub fn context(cwd: &Path, delegate: bool, about: &str) -> Res<Value> {
    let (repo, root) = repo_of(cwd)?;
    let t = if about.is_empty() { None } else { get_task(about) };
    Ok(json!({ "repo": repo, "root": root.display().to_string(), "task": t.as_ref().map(|t| t.t("id")).unwrap_or_default(), "title": t.as_ref().map(|t| one_line(t.s("title"), 120)).unwrap_or_default(), "delegate": delegate }))
}
/// Of the messages dealt with, the last few stay; one not dealt with yet is never dropped.
const KEPT: usize = 5;
pub fn rest_ms() -> i64 { std::env::var("HERDR_PROJECTS_HANDLED_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(15000) }
pub fn trimmed(list: &[Value]) -> Vec<Value> { let cut = list.len().saturating_sub(KEPT); list.iter().enumerate().filter(|(i, x)| *i >= cut || !x.b("handled")).map(|(_, x)| x.clone()).collect() }
pub fn where_text(co: impl Into<Co>) -> String {
    let list = get(co).map(|c| c.arr("where").to_vec()).unwrap_or_default();
    if list.is_empty() { return "Nothing was sent from the board yet, so it is not known which repo the human is in: ask them, or see \"board list\".".into(); }
    let one = |w: &Value| [format!("\"{}\" was sent from the repo \"{}\" ({}).", one_line(w.s("said"), 80), w.s("repo"), tilde(w.s("root"))),
        if w.b("task") { format!("Task {} (\"{}\") was open: for this message \"it\" and \"this\" mean that task.", w.s("task"), w.s("title")) } else { "No task was open.".into() },
        if w.b("delegate") { format!("It was sent with {}: put an agent on every task you add from it, without asking.", key_name("delegate")) } else { format!("It was sent with {}: delegate only what the human asked you to.", key_name("enter")) }].join(" ");
    let head = format!("The human's last {} from the board{}:", if list.len() > 1 { format!("{} messages", list.len()) } else { "message".into() }, if list.len() > 1 { ", the newest last. Each has its own repo, open task and delegate choice: use the ones of the message you are acting on" } else { "" });
    std::iter::once(head).chain(list.iter().map(|w| format!("- {}", one(w)))).collect::<Vec<_>>().join("\n")
}
/// The board's own half of "sent with tab": a task added from the Coordinator's pane is delegated by `add` itself when
/// it can only come from a tab message (the last message went in while it was at rest, it is still in that turn, and
/// every message it may still have ahead of it was sent with tab).
pub fn tab_turn(pane: &str, socket: &str) -> bool { super::state::all(socket).iter().any(|co| tab_turn_of(pane, co)) }
fn tab_turn_of(pane: &str, co: &Co) -> bool {
    let socket = co.socket.as_str();
    let Some(c) = get(co) else { return false };
    if pane.is_empty() || (c.s("pane") != pane && c.s("paneWas") != pane) || !c.at("mark").b("rest") || c.at("mark").i("run") != c.i("startedAt") { return false; }
    let open: Vec<&Value> = c.arr("where").iter().filter(|x| x.b("sentAt") && !x.b("handled")).collect();
    if open.is_empty() || !open.iter().all(|x| x.b("delegate")) { return false; }
    Herdr::new(Some(socket.to_string())).agent_get(c.s("pane")).ok().flatten().map(|a| *a.at("completion_seq") == *c.at("mark").at("seq")).unwrap_or(false)
}
/// The Coordinator's own folder (or one inside it) is never where the human is working, nor the machine coordinator's.
pub fn in_home(dir: &Path, socket: &str) -> bool {
    let real = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| crate::store::files::absolute(p));
    [Co::top(socket), Co::machine(socket)].iter().any(|co| real(dir).starts_with(real(&home(co))))
}
/// The pump is told which herdr session it serves: the keeper ticks for several from one process.
pub fn kick(co: impl Into<Co>) {
    let key = co.into().key();
    if crate::runner::inline() { crate::runner::JOBS.with(|j| j.borrow_mut().push(("coordinator".into(), key, vec![]))); return; }
    crate::housekeeping::spawn_loose(&["run", "coordinator", &key]);
}
fn herdr_up(socket: &str) -> Res<()> {
    if crate::herdr::cli::fake().is_none() && !crate::herdr::cli::socket_exists(socket) { crate::fail!("herdr is not running: the Coordinator lives in herdr."); }
    Ok(())
}
/// Say something to the Coordinator. It is started (or started again, in the same conversation) when it is not there.
pub fn send(cwd: &Path, text: &str, delegate: bool, about: &str, co: impl Into<Co>) -> Res<Obj> {
    let co = co.into();
    herdr_up(&co.socket)?;
    let root = known(&co)?;
    let ctx = context(root.as_deref().unwrap_or(cwd), delegate, about)?;
    let c = change(&co, |c| {
        let mut q = c.arr("queue").to_vec();
        let mut message = json!({ "id": crate::util::uuid(), "text": if delegate { format!("{text}\n\n{}", tab_note()) } else { text.to_string() }, "said": text, "where": ctx, "at": now() });
        if text.starts_with("[ORCHESTRATOR-REVIEW]") { message["review"] = json!(true); }
        q.push(message);
        c.put("queue", q); c.put("error", ""); c.put("stopped", false);
        true
    })?;
    kick(&co);
    Ok(c)
}
/// A project coordinator's repo, which must be one this machine knows: it works in its folder.
fn known(co: &Co) -> Res<Option<std::path::PathBuf>> {
    let Some(r) = co.repo() else { return Ok(None) };
    match co.root() { Some(p) if p.is_dir() => Ok(Some(p)), _ => crate::fail!("No repo \"{r}\" on this machine, so it has no project coordinator here.") }
}
/// restart: the same conversation, the program started again. new: a fresh conversation, on the default or on `pick`.
/// stop: close it; the next message starts it again where it left off. open: have it running, with nothing said.
fn want(what: &str, pick: Option<Obj>, co: Co) -> Res<Obj> {
    herdr_up(&co.socket)?;
    known(&co)?;
    if what != "stop" && pick.is_none() && !get(&co).map(|c| c.b("run")).unwrap_or(false) && choice().is_none() { crate::fail!("{OFF_WORDS}"); }
    let c = change(&co, |c| {
        if what == "open" && c.b("want") { return false; }
        c.put("want", json!({ "what": what, "pick": pick, "at": now() })); c.put("error", "");
        if what != "stop" { c.put("stopped", false); }
        true
    })?;
    kick(&co);
    Ok(c)
}
pub fn handover(pick: Option<Obj>, co: impl Into<Co>) -> Res<Obj> { want("handover", pick, co.into()) }
pub fn restart(co: impl Into<Co>) -> Res<Obj> { want("restart", None, co.into()) }
pub fn fresh(pick: Option<Obj>, co: impl Into<Co>) -> Res<Obj> { want("new", pick, co.into()) }
pub fn stop(co: impl Into<Co>) -> Res<Obj> { want("stop", None, co.into()) }
pub fn open(co: impl Into<Co>) -> Res<Obj> { want("open", None, co.into()) }
/// Its own chat was on screen: whatever was typed there, the box's last words no longer head its last message.
pub fn typed_there(co: impl Into<Co>) { let _ = change(co, |c| { if !c.b("last") { return false; } c.put("last", ""); true }); }
/// The chat on the board showed this finished message: it is not announced as new again.
pub fn seen(id: &str, co: impl Into<Co>) { let _ = change(co, |c| { if !c.b("run") || c.s("seen") == id { return false; } c.put("seen", id); true }); }

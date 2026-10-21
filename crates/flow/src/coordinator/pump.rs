//! The pump: the detached job that starts a coordinator and types into it, one at a time per coordinator. Whoever
//! ticks (the keeper, or the board without one) starts a pump for anything left waiting: a message that met a dialog on
//! the Coordinator's screen is typed in once the dialog is gone.
use super::look::look as look_now;
use super::place::{close_place, move_over, nowhere, open_tab};
use super::requests::{kick, rest_ms, trimmed};
use super::co::Co;
use super::rules::rules_of;
use super::rules_place::{intro_line_of, put_rules};
use super::state::{change, dir_of, get, home, pumping};
use super::who::{choice_of, OFF_WORDS};
use crate::agents::{adapter, installed, start, start_failed};
use crate::clock::now;
use crate::err::Res;
use crate::herdr::cli::socket_exists;
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::files::marker;
use crate::store::marks::due;
use crate::store::move_home::moved_from;
use crate::store::tasks::one_line;
use crate::submit::{confirm, hand, look, settle, shown};
use std::collections::HashSet;
use std::path::Path;

use crate::status_words::COORDINATOR_UNSENT as UNSENT;
fn up(socket: &str) -> bool { crate::herdr::cli::fake().is_some() || socket_exists(socket) }
fn mine(c: &Obj) -> bool { c.at("pump").i("pid") == std::process::id() as i64 }
fn said_in(co: &Co, mark: &Value, text: &str) -> Option<bool> {
    let c = get(co)?;
    shown(c.at("run").s("kind"), c.so("transcript").map(Path::new), c.s("session"), &home(co), mark, text)
}
/// Words typed into a turn the Coordinator was in the middle of: one look per tick, at most one enter.
fn look_typed(co: &Co, c: &Obj) {
    let mut mark = c.at("typed").clone();
    let was = mark.clone();
    let text = mark.t("text");
    let m2 = mark.clone();
    let r = look(&Herdr::new(Some(co.socket.clone())), c.s("pane"), &mut mark, Some(&|| said_in(co, &m2, &text)));
    let done = matches!(r, "took" | "unknown" | "lost");
    if done || mark != was {
        let (at, typed_at) = (c.at("typed").at("at").clone(), c.at("typed").at("typedAt").clone());
        let _ = change(co, |x| { if *x.at("typed").at("typedAt") != typed_at || *x.at("typed").at("at") != at { return false; } x.put("typed", if done { Value::Null } else { mark.clone() }); if r == "lost" { x.put("error", UNSENT); } true });
    }
}
pub fn tick(co: impl Into<Co>) {
    let co = &co.into();
    let mut c = get(co);
    // Still in the space of its own from before it lived in a tab: moved over as it is, whatever it is doing.
    if let Some(x) = &c { if x.b("workspace") && up(&co.socket) && !pumping(x) && due(&co.mark("coordinator-move"), 60000) { let _ = move_over(&Herdr::new(Some(co.socket.clone())), co, true); c = get(co); } }
    if let Some(x) = &c { if x.b("typed") && x.b("pane") && up(&co.socket) && !(pumping(x) && !mine(x)) { look_typed(co, x); } }
    let Some(c) = c else { return };
    if (c.arr("queue").is_empty() && !c.b("want")) || (pumping(&c) && !mine(&c)) || !up(&co.socket) { return; }
    if c.b("stopped") && !c.b("want") { return; } // stopped stays stopped, also with words still waiting
    if c.b("error") && !c.b("want") { return; } // a start that failed is not tried again by itself
    if due(&co.mark("coordinator"), 3000) { kick(co); }
}
/// The conversation it is in: the newest one saved in its folder since this Coordinator began; for a program whose
/// files the board does not read, the one herdr's integration names for its pane.
pub fn active(co: impl Into<Co>, c: &Obj) -> Option<String> {
    let co = &co.into();
    let Some(run) = c.o("run") else { return c.so("session").map(String::from) };
    let since = if c.b("since") { c.i("since") } else { c.i("startedAt") };
    let h = home(co);
    // A project coordinator shares its folder with the human's own chats in the repo: the newest one there may be theirs.
    // Only what herdr names for its own pane moves it to another conversation; else it keeps the one it has.
    let id = if co.repo().is_some() { told_session(co, c) } else { adapter::latest_session(run.s("kind"), &h, since, moved_from(&h).as_deref()).or_else(|| told_session(co, c)) };
    if let Some(i) = &id { if i != c.s("session") { let (since_v, i2) = (c.at("since").clone(), i.clone()); let _ = change(co, |x| { if *x.at("since") != since_v || x.s("session") == i2 { return false; } x.put("session", i2.clone()); true }); } }
    id.or_else(|| c.so("session").map(String::from))
}
/// The conversation herdr's integration names for the Coordinator's pane, if any.
fn told_session(co: &Co, c: &Obj) -> Option<String> {
    if !c.b("pane") || !up(&co.socket) { return None; }
    let a = Herdr::new(Some(co.socket.clone())).agent_get(c.s("pane")).ok().flatten()?;
    Some(a.at("agent_session").t("value")).filter(|s| !s.is_empty())
}
fn ensure(api: &Herdr, co: &Co) -> Res<(Obj, bool)> {
    let mut c = get(co).unwrap_or_default();
    if c.b("run") { active(co, &c); c = get(co).unwrap_or_default(); }
    std::fs::create_dir_all(home(co))?;
    // Its standing rules are files in its folder, brought up to date before every message.
    let who = c.o("run").cloned().or_else(|| c.o("pick").cloned()).or_else(|| choice_of(co));
    let text = rules_of(co, &crate::flow::send::shim().display().to_string());
    if let Some(w) = &who { let _ = put_rules(co, w.s("kind"), &text); }
    let mut pane = if c.b("pane") { api.pane_get(c.s("pane")) } else { None };
    if c.b("workspace") { match move_over(api, co, false) { Ok(p) => pane = p, Err(_) => { close_place(api, &c); let _ = change(co, |c| { nowhere(c); true }); pane = None; } } }
    // A board kept somewhere else than the default is the board its commands must reach.
    let env: Vec<(String, String)> = ["HERDR_PROJECTS_DATA_DIR", "HERDR_PROJECTS_CONFIG_DIR"].iter().filter_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()).map(|v| (k.to_string(), v))).collect();
    let pane = match pane { Some(p) => p, None => open_tab(api, co, &env)? };
    c = get(co).unwrap_or_default();
    if api.agent_get(pane.s("pane_id"))?.is_some() { return Ok((c, false)); }
    // Its program is not running: start it. The same conversation when there was one and the program can reopen it.
    let run = match c.o("run").cloned().or_else(|| c.o("pick").cloned().map(|mut p| { p.put("oneOff", true); p })).or_else(|| choice_of(co)) { Some(r) => r, None => crate::fail!("{OFF_WORDS}") };
    let k = run.t("kind");
    if !installed::installed(&k) { crate::fail!("\"{k}\" is not installed here, so it cannot be the {}.", co.name()); }
    let saved = !crate::agents::kinds::kind(&k).map(|a| a.reads_turn).unwrap_or(false) || adapter::transcript(&k, c.s("session"), &home(co)).is_some();
    let resume = if c.b("run") && c.b("session") && !adapter::resume_args(&k, "x").is_empty() && saved { Some(c.t("session")) } else { None };
    let session = if resume.is_none() { adapter::new_session(&k) } else { None };
    // The rules ride on no launch flag: the program reads them from its folder, so they are told once.
    let (reads, launch) = put_rules(co, &k, &text)?;
    if co.is_top() { let _ = std::fs::remove_file(dir_of(co).join("rules.md")); }
    installed::pre_trust(&k, &home(co), if run.b("access") { run.s("access") } else { "full" });
    let taken: Vec<String> = api.agents()?.iter().filter(|x| x.s("pane_id") != pane.s("pane_id")).map(|x| x.t("name")).collect();
    let mut name = co.agent_name();
    let mut n = 2;
    while taken.contains(&name) { name = format!("{}-{n}", co.agent_name()); n += 1; }
    let began = now();
    change(co, |c| { c.put("starting", began); true })?;
    let args = start::start_args(&k, &start::StartOpts { model: run.t("model"), effort: run.t("effort"), access: if run.b("access") { run.t("access") } else { "full".into() }, fast: run.b("fast").then_some(true), resume: resume.clone().unwrap_or_default(), session: session.clone().unwrap_or_default(), extra: launch, ..Default::default() })?;
    let mut blocked = false;
    match api.agent_start(&name, &k, pane.s("pane_id"), &args, 60000) {
        Ok(_) => { settle(api, pane.s("pane_id"), &std::iter::once(crate::agents::kinds::program(&k).to_string()).chain(args).collect::<Vec<_>>().join(" ")); }
        Err(e) if e.code == "agent_not_ready" => blocked = true, // a dialog is in the way: what you said waits
        Err(e) => {
            // Why, in plain words, read from its terminal before its tab is closed; the whole account goes in its log.
            let why = start_failed::say(api, &start_failed::Start { kind: &k, setting: "coordinator.agent", again: "send your message again" }, pane.s("pane_id"), &e.message, &dir_of(co).join("start.log"));
            if let Some(c) = get(co) { close_place(api, &c); }
            let _ = change(co, |c| { nowhere(c); c.put("starting", 0); true });
            crate::fail!("{why}");
        }
    }
    let version = installed::version_of(&k);
    c = change(co, |c| {
        c.put("run", run.clone()); c.put("pick", Value::Null); c.put("agent", name.clone()); c.put("startedAt", now()); c.put("version", version.clone()); c.put("starting", 0);
        c.put("intro", !reads && resume.is_none()); // a program not known to read a rules file: one line before the first message points at it
        if resume.is_none() { c.put("session", session.clone().map(Value::String).unwrap_or(Value::Null)); c.put("transcript", Value::Null); c.put("since", began); c.put("seen", Value::Null); }
        true
    })?;
    Ok((c, blocked))
}
pub fn pump(co: impl Into<Co>) {
    let co = &co.into();
    let mut took = false;
    let _ = change(co, |c| { if pumping(c) && !mine(c) { return false; } took = true; c.put("pump", marker()); true });
    if !took { return; }
    let mut taken: HashSet<String> = HashSet::new(); // the requests this pump took up
    let r = run_pump(co, &mut taken);
    // Only the error is written down. A request made meanwhile is still there for the next pump, and so are your words.
    if let Err(e) = r { let _ = change(co, |c| { let m = one_line(&e.msg, 300); c.put("error", if m.is_empty() { "it could not be started".into() } else { m }); true }); }
    let mut more = false;
    let _ = change(co, |c| { if !mine(c) { return false; } c.put("pump", Value::Null); c.put("starting", 0); more = c.b("want") && !taken.contains(&c.at("want").t("at")); true });
    if more { kick(co); } // asked for while this one ran
}
fn run_pump(co: &Co, taken: &mut HashSet<String>) -> Res<()> {
    let api = Herdr::new(Some(co.socket.clone()));
    for _ in 0..50 {
        let mut c = get(co).unwrap_or_default();
        if let Some(w) = c.o("want").cloned() {
            taken.insert(w.t("at"));
            // Its conversation is known before its terminal goes: a restart reopens it.
            if w.s("what") != "open" { if c.b("run") { active(co, &c); c = get(co).unwrap_or(c); } close_place(&api, &c); }
            let _ = change(co, |c| {
                if c.at("want").at("at") == w.at("at") { c.put("want", Value::Null); }
                if w.s("what") == "open" { return true; }
                nowhere(c);
                if w.s("what") == "new" { for (k, v) in obj(json!({ "run": null, "pick": w.at("pick"), "session": null, "transcript": null, "seen": null, "intro": false, "last": "", "where": [], "mark": null })) { c.insert(k, v); } }
                true
            })?;
            // Stopped is stopped: words that were still waiting stay waiting. Only something said after the stop starts it again.
            if w.s("what") == "stop" {
                let at = w.i("at");
                c = change(co, |c| { let s = !c.b("want") && !c.arr("queue").iter().any(|m| m.i("at") > at); c.put("stopped", s); true })?;
                if c.b("stopped") { break; }
                continue;
            }
        } else if c.arr("queue").is_empty() || c.b("stopped") { break; }
        let run_was = get(co).map(|c| c.i("startedAt")).unwrap_or(0);
        let (_, blocked) = ensure(&api, co)?;
        let started = get(co).map(|c| c.i("startedAt")).unwrap_or(0) != run_was; // a wake that started it: its run counts as at rest
        if blocked { break; } // the tick tries again
        if get(co).map(|c| c.b("error")).unwrap_or(false) { change(co, |c| { c.put("error", ""); true })?; }
        c = get(co).unwrap_or_default();
        let mut stuck = false;
        for m in c.arr("queue").to_vec() {
            // Asked to stop (or restart, or begin anew) while an earlier message went in: that comes first.
            if get(co).map(|c| c.b("want")).unwrap_or(false) { break; }
            if m.b("autopilot") && crate::autopilot::record::words(co, "coordinator.access") == "ask" { continue; } // it would wait at a permission prompt: the tick takes it back
            let ag = api.agent_get(c.s("pane")).ok().flatten();
            let run = get(co).map(|c| c.i("startedAt")).unwrap_or(0);
            let seq = ag.as_ref().map(|a| a.at("completion_seq").clone()).unwrap_or(Value::Null);
            let at_rest = ag.as_ref().map(|a| ["idle", "done"].contains(&a.s("agent_status"))).unwrap_or(false);
            // Has this run of the program finished a turn since the last words went in, and is it at rest? Then
            // whatever went in a while ago has been dealt with.
            if at_rest && !seq.is_null() {
                change(co, |c| {
                    if !c.at("mark").is_object() || c.at("mark").i("run") != run || *c.at("mark").at("seq") == seq { return false; }
                    let mut list = c.arr("where").to_vec();
                    let due: Vec<usize> = (0..list.len()).filter(|i| !list[*i].b("handled") && list[*i].b("sentAt") && now() - list[*i].i("sentAt") >= rest_ms()).collect();
                    if due.is_empty() { return false; }
                    for i in due { if let Some(o) = list[i].as_object_mut() { o.put("handled", true); } }
                    c.put("where", list);
                    true
                })?;
            }
            let id = if m.b("id") { m.t("id") } else { m.t("at") };
            let same = |x: &Value| if m.b("id") { x.s("id") == m.s("id") } else { x.at("at") == m.at("at") && x.at("text") == m.at("text") };
            let note = m.at("where").is_object().then(|| { let mut n = m.o("where").cloned().unwrap_or_default(); n.put("said", m.at("said").clone()); n.put("at", m.at("at").clone()); n.put("id", id.clone()); n });
            let unsay = || if note.is_some() { let _ = change(co, |c| { let l: Vec<Value> = c.arr("where").iter().filter(|x| x.s("id") != id).cloned().collect(); c.put("where", l); true }); };
            if let Some(n) = &note { change(co, |c| { let mut l: Vec<Value> = c.arr("where").iter().filter(|x| x.s("id") != id).cloned().collect(); l.push(Value::Object(n.clone())); c.put("where", l); true })?; }
            // What it had last said when these words went in: the board shows them in its place until it says something new.
            let now_look = look_now(co);
            let (over, over_id) = (now_look.said.clone(), now_look.turn.as_ref().filter(|t| t.finished).map(|t| t.id.clone()).unwrap_or_default());
            let intro = get(co).map(|c| c.b("intro")).unwrap_or(false);
            let words_in = if intro { format!("{}\n\n{}", intro_line_of(co), m.s("text")) } else { m.t("text") };
            let typed = match hand(&api, c.s("pane"), &words_in) {
                Ok(t) => t,
                Err(e) if e.code == "agent_blocked" || e.code == "agent_not_ready" => { unsay(); stuck = true; break; }
                Err(e) => { unsay(); return Err(e.into()); }
            };
            change(co, |c| {
                let q: Vec<Value> = c.arr("queue").iter().filter(|x| !same(x)).cloned().collect(); c.put("queue", q);
                let mut l = c.arr("where").to_vec();
                for x in l.iter_mut() { if x.s("id") == id { if let Some(o) = x.as_object_mut() { o.put("sentAt", now()); } } }
                c.put("where", trimmed(&l));
                c.put("mark", json!({ "run": run, "seq": seq, "rest": at_rest }));
                c.put("intro", false); c.put("sentAt", now());
                if !m.b("autopilot") { c.put("last", m.at("said").clone()); c.put("over", over.clone()); c.put("overId", over_id.clone()); } // a wake is no words of yours
                // Who its turn is on: a task's agent that sent it (coordinator tell), and an autopilot wake (autopilot/).
                c.put("typedFrom", m.at("sender").clone()); c.put("typedId", id.clone());
                if m.b("autopilot") { crate::autopilot::wake::typed(c, &m, started); }
                true
            })?;
            // Typed is not sent: if it still sits at its prompt, enter is pressed again (never the words).
            let busy = typed.as_ref().map(|t| t.b("busy")).unwrap_or(false);
            let words = if busy { [get(co).map(|c| c.at("typed").t("text")).unwrap_or_default(), m.t("text")].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n") } else { m.t("text") };
            change(co, |c| { c.put("typed", if busy { let mut t = typed.as_ref().and_then(|t| t.as_object().cloned()).unwrap_or_default(); t.put("text", words.clone()); Value::Object(t) } else { Value::Null }); true })?;
            let mark = typed.clone().unwrap_or(Value::Null);
            if !confirm(&api, c.s("pane"), typed, Some(&|| said_in(co, &mark, &words))) { crate::fail!("{UNSENT}"); }
        }
        if stuck { break; }
    }
    Ok(())
}

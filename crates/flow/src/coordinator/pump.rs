//! The pump: the detached job that starts the Coordinator and types into it, one at a time per herdr session. Whoever
//! ticks (the keeper, or the board without one) starts a pump for anything left waiting: a message that met a dialog on
//! the Coordinator's screen is typed in once the dialog is gone.
use super::look::look as look_now;
use super::place::{close_place, move_over, nowhere, open_tab};
use super::requests::{kick, rest_ms, trimmed};
use super::rules::{intro_line, rules, write_rules};
use super::state::{change, dir_of, get, home, pumping};
use super::who::{choice, OFF_WORDS};
use crate::agents::{adapter, installed, start, start_failed};
use crate::clock::now;
use crate::err::Res;
use crate::herdr::cli::{session_name, socket_exists};
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
fn said_in(socket: &str, mark: &Value, text: &str) -> Option<bool> {
    let c = get(socket)?;
    shown(c.at("run").s("kind"), c.so("transcript").map(Path::new), c.s("session"), &home(socket), mark, text)
}
/// Words typed into a turn the Coordinator was in the middle of: one look per tick, at most one enter.
fn look_typed(socket: &str, c: &Obj) {
    let mut mark = c.at("typed").clone();
    let was = mark.clone();
    let text = mark.t("text");
    let m2 = mark.clone();
    let r = look(&Herdr::new(Some(socket.into())), c.s("pane"), &mut mark, Some(&|| said_in(socket, &m2, &text)));
    let done = matches!(r, "took" | "unknown" | "lost");
    if done || mark != was {
        let (at, typed_at) = (c.at("typed").at("at").clone(), c.at("typed").at("typedAt").clone());
        let _ = change(socket, |x| { if *x.at("typed").at("typedAt") != typed_at || *x.at("typed").at("at") != at { return false; } x.put("typed", if done { Value::Null } else { mark.clone() }); if r == "lost" { x.put("error", UNSENT); } true });
    }
}
pub fn tick(socket: &str) {
    let mut c = get(socket);
    // Still in the space of its own from before it lived in a tab: moved over as it is, whatever it is doing.
    if let Some(x) = &c { if x.b("workspace") && up(socket) && !pumping(x) && due(&format!("coordinator-move-{}", session_name(socket)), 60000) { let _ = move_over(&Herdr::new(Some(socket.into())), socket, true); c = get(socket); } }
    if let Some(x) = &c { if x.b("typed") && x.b("pane") && up(socket) && !(pumping(x) && !mine(x)) { look_typed(socket, x); } }
    let Some(c) = c else { return };
    if (c.arr("queue").is_empty() && !c.b("want")) || (pumping(&c) && !mine(&c)) || !up(socket) { return; }
    if c.b("stopped") && !c.b("want") { return; } // stopped stays stopped, also with words still waiting
    if c.b("error") && !c.b("want") { return; } // a start that failed is not tried again by itself
    if due(&format!("coordinator-{}", session_name(socket)), 3000) { kick(socket); }
}
/// The conversation it is in: the newest one saved in its folder since this Coordinator began; for a program whose
/// files the board does not read, the one herdr's integration names for its pane.
pub fn active(socket: &str, c: &Obj) -> Option<String> {
    let Some(run) = c.o("run") else { return c.so("session").map(String::from) };
    let since = if c.b("since") { c.i("since") } else { c.i("startedAt") };
    let h = home(socket);
    let id = adapter::latest_session(run.s("kind"), &h, since, moved_from(&h).as_deref()).or_else(|| told_session(socket, c));
    if let Some(i) = &id { if i != c.s("session") { let (since_v, i2) = (c.at("since").clone(), i.clone()); let _ = change(socket, |x| { if *x.at("since") != since_v || x.s("session") == i2 { return false; } x.put("session", i2.clone()); true }); } }
    id.or_else(|| c.so("session").map(String::from))
}
/// The conversation herdr's integration names for the Coordinator's pane, if any.
fn told_session(socket: &str, c: &Obj) -> Option<String> {
    if !c.b("pane") || !up(socket) { return None; }
    let a = Herdr::new(Some(socket.into())).agent_get(c.s("pane")).ok().flatten()?;
    Some(a.at("agent_session").t("value")).filter(|s| !s.is_empty())
}
fn ensure(api: &Herdr, socket: &str) -> Res<(Obj, bool)> {
    let mut c = get(socket).unwrap_or_default();
    if c.b("run") { active(socket, &c); c = get(socket).unwrap_or_default(); }
    std::fs::create_dir_all(home(socket))?;
    // Its standing rules are files in its folder, brought up to date before every message.
    let who = c.o("run").cloned().or_else(|| c.o("pick").cloned()).or_else(choice);
    let text = rules(&crate::flow::send::shim().display().to_string());
    if let Some(w) = &who { let _ = write_rules(&home(socket), w.s("kind"), &text); }
    let mut pane = if c.b("pane") { api.pane_get(c.s("pane")) } else { None };
    if c.b("workspace") { match move_over(api, socket, false) { Ok(p) => pane = p, Err(_) => { close_place(api, &c); let _ = change(socket, |c| { nowhere(c); true }); pane = None; } } }
    // A board kept somewhere else than the default is the board its commands must reach.
    let env: Vec<(String, String)> = ["HERDR_PROJECTS_DATA_DIR", "HERDR_PROJECTS_CONFIG_DIR"].iter().filter_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()).map(|v| (k.to_string(), v))).collect();
    let pane = match pane { Some(p) => p, None => open_tab(api, socket, &env)? };
    c = get(socket).unwrap_or_default();
    if api.agent_get(pane.s("pane_id"))?.is_some() { return Ok((c, false)); }
    // Its program is not running: start it. The same conversation when there was one and the program can reopen it.
    let run = match c.o("run").cloned().or_else(|| c.o("pick").cloned().map(|mut p| { p.put("oneOff", true); p })).or_else(choice) { Some(r) => r, None => crate::fail!("{OFF_WORDS}") };
    let k = run.t("kind");
    if !installed::installed(&k) { crate::fail!("\"{k}\" is not installed here, so it cannot be the Coordinator."); }
    let saved = !crate::agents::kinds::kind(&k).map(|a| a.reads_turn).unwrap_or(false) || adapter::transcript(&k, c.s("session"), &home(socket)).is_some();
    let resume = if c.b("run") && c.b("session") && !adapter::resume_args(&k, "x").is_empty() && saved { Some(c.t("session")) } else { None };
    let session = if resume.is_none() { adapter::new_session(&k) } else { None };
    // The rules ride on no launch flag: the program reads them from its folder, so they are told once.
    let (_, reads) = write_rules(&home(socket), &k, &text)?;
    let _ = std::fs::remove_file(dir_of(socket).join("rules.md"));
    installed::pre_trust(&k, &home(socket), if run.b("access") { run.s("access") } else { "full" });
    let taken: Vec<String> = api.agents()?.iter().filter(|x| x.s("pane_id") != pane.s("pane_id")).map(|x| x.t("name")).collect();
    let mut name = "sw-coordinator".to_string();
    let mut n = 2;
    while taken.contains(&name) { name = format!("sw-coordinator-{n}"); n += 1; }
    let began = now();
    change(socket, |c| { c.put("starting", began); true })?;
    let args = start::start_args(&k, &start::StartOpts { model: run.t("model"), effort: run.t("effort"), access: if run.b("access") { run.t("access") } else { "full".into() }, fast: run.b("fast").then_some(true), resume: resume.clone().unwrap_or_default(), session: session.clone().unwrap_or_default(), ..Default::default() })?;
    let mut blocked = false;
    match api.agent_start(&name, &k, pane.s("pane_id"), &args, 60000) {
        Ok(_) => { settle(api, pane.s("pane_id"), &std::iter::once(crate::agents::kinds::program(&k).to_string()).chain(args).collect::<Vec<_>>().join(" ")); }
        Err(e) if e.code == "agent_not_ready" => blocked = true, // a dialog is in the way: what you said waits
        Err(e) => {
            // Why, in plain words, read from its terminal before its tab is closed; the whole account goes in its log.
            let why = start_failed::say(api, &start_failed::Start { kind: &k, setting: "coordinator.agent", again: "send your message again" }, pane.s("pane_id"), &e.message, &dir_of(socket).join("start.log"));
            if let Some(c) = get(socket) { close_place(api, &c); }
            let _ = change(socket, |c| { nowhere(c); c.put("starting", 0); true });
            crate::fail!("{why}");
        }
    }
    let version = installed::version_of(&k);
    c = change(socket, |c| {
        c.put("run", run.clone()); c.put("pick", Value::Null); c.put("agent", name.clone()); c.put("startedAt", now()); c.put("version", version.clone()); c.put("starting", 0);
        c.put("intro", !reads && resume.is_none()); // a program not known to read a rules file: one line before the first message points at it
        if resume.is_none() { c.put("session", session.clone().map(Value::String).unwrap_or(Value::Null)); c.put("transcript", Value::Null); c.put("since", began); c.put("seen", Value::Null); }
        true
    })?;
    Ok((c, blocked))
}
pub fn pump(socket: &str) {
    let mut took = false;
    let _ = change(socket, |c| { if pumping(c) && !mine(c) { return false; } took = true; c.put("pump", marker()); true });
    if !took { return; }
    let mut taken: HashSet<String> = HashSet::new(); // the requests this pump took up
    let r = run_pump(socket, &mut taken);
    // Only the error is written down. A request made meanwhile is still there for the next pump, and so are your words.
    if let Err(e) = r { let _ = change(socket, |c| { let m = one_line(&e.msg, 300); c.put("error", if m.is_empty() { "it could not be started".into() } else { m }); true }); }
    let mut more = false;
    let _ = change(socket, |c| { if !mine(c) { return false; } c.put("pump", Value::Null); c.put("starting", 0); more = c.b("want") && !taken.contains(&c.at("want").t("at")); true });
    if more { kick(socket); } // asked for while this one ran
}
fn run_pump(socket: &str, taken: &mut HashSet<String>) -> Res<()> {
    let api = Herdr::new(Some(socket.into()));
    for _ in 0..50 {
        let mut c = get(socket).unwrap_or_default();
        if let Some(w) = c.o("want").cloned() {
            taken.insert(w.t("at"));
            // Its conversation is known before its terminal goes: a restart reopens it.
            if w.s("what") != "open" { if c.b("run") { active(socket, &c); c = get(socket).unwrap_or(c); } close_place(&api, &c); }
            let _ = change(socket, |c| {
                if c.at("want").at("at") == w.at("at") { c.put("want", Value::Null); }
                if w.s("what") == "open" { return true; }
                nowhere(c);
                if w.s("what") == "new" { for (k, v) in obj(json!({ "run": null, "pick": w.at("pick"), "session": null, "transcript": null, "seen": null, "intro": false, "last": "", "where": [], "mark": null })) { c.insert(k, v); } }
                true
            })?;
            // Stopped is stopped: words that were still waiting stay waiting. Only something said after the stop starts it again.
            if w.s("what") == "stop" {
                let at = w.i("at");
                c = change(socket, |c| { let s = !c.b("want") && !c.arr("queue").iter().any(|m| m.i("at") > at); c.put("stopped", s); true })?;
                if c.b("stopped") { break; }
                continue;
            }
        } else if c.arr("queue").is_empty() || c.b("stopped") { break; }
        let (_, blocked) = ensure(&api, socket)?;
        if blocked { break; } // the tick tries again
        if get(socket).map(|c| c.b("error")).unwrap_or(false) { change(socket, |c| { c.put("error", ""); true })?; }
        c = get(socket).unwrap_or_default();
        let mut stuck = false;
        for m in c.arr("queue").to_vec() {
            // Asked to stop (or restart, or begin anew) while an earlier message went in: that comes first.
            if get(socket).map(|c| c.b("want")).unwrap_or(false) { break; }
            let ag = api.agent_get(c.s("pane")).ok().flatten();
            let run = get(socket).map(|c| c.i("startedAt")).unwrap_or(0);
            let seq = ag.as_ref().map(|a| a.at("completion_seq").clone()).unwrap_or(Value::Null);
            let at_rest = ag.as_ref().map(|a| ["idle", "done"].contains(&a.s("agent_status"))).unwrap_or(false);
            // Has this run of the program finished a turn since the last words went in, and is it at rest? Then
            // whatever went in a while ago has been dealt with.
            if at_rest && !seq.is_null() {
                change(socket, |c| {
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
            let unsay = || if note.is_some() { let _ = change(socket, |c| { let l: Vec<Value> = c.arr("where").iter().filter(|x| x.s("id") != id).cloned().collect(); c.put("where", l); true }); };
            if let Some(n) = &note { change(socket, |c| { let mut l: Vec<Value> = c.arr("where").iter().filter(|x| x.s("id") != id).cloned().collect(); l.push(Value::Object(n.clone())); c.put("where", l); true })?; }
            // What it had last said when these words went in: the board shows them in its place until it says something new.
            let now_look = look_now(socket);
            let (over, over_id) = (now_look.said.clone(), now_look.turn.as_ref().filter(|t| t.finished).map(|t| t.id.clone()).unwrap_or_default());
            let intro = get(socket).map(|c| c.b("intro")).unwrap_or(false);
            let words_in = if intro { format!("{}\n\n{}", intro_line(&home(socket)), m.s("text")) } else { m.t("text") };
            let typed = match hand(&api, c.s("pane"), &words_in) {
                Ok(t) => t,
                Err(e) if e.code == "agent_blocked" || e.code == "agent_not_ready" => { unsay(); stuck = true; break; }
                Err(e) => { unsay(); return Err(e.into()); }
            };
            change(socket, |c| {
                let q: Vec<Value> = c.arr("queue").iter().filter(|x| !same(x)).cloned().collect(); c.put("queue", q);
                let mut l = c.arr("where").to_vec();
                for x in l.iter_mut() { if x.s("id") == id { if let Some(o) = x.as_object_mut() { o.put("sentAt", now()); } } }
                c.put("where", trimmed(&l));
                c.put("mark", json!({ "run": run, "seq": seq, "rest": at_rest }));
                c.put("intro", false); c.put("sentAt", now()); c.put("last", m.at("said").clone()); c.put("over", over.clone()); c.put("overId", over_id.clone());
                true
            })?;
            // Typed is not sent: if it still sits at its prompt, enter is pressed again (never the words).
            let busy = typed.as_ref().map(|t| t.b("busy")).unwrap_or(false);
            let words = if busy { [get(socket).map(|c| c.at("typed").t("text")).unwrap_or_default(), m.t("text")].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n") } else { m.t("text") };
            change(socket, |c| { c.put("typed", if busy { let mut t = typed.as_ref().and_then(|t| t.as_object().cloned()).unwrap_or_default(); t.put("text", words.clone()); Value::Object(t) } else { Value::Null }); true })?;
            let mark = typed.clone().unwrap_or(Value::Null);
            if !confirm(&api, c.s("pane"), typed, Some(&|| said_in(socket, &mark, &words))) { crate::fail!("{UNSENT}"); }
        }
        if stuck { break; }
    }
    Ok(())
}

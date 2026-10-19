//! Remove a task's herdr workspace, worktree and branch. Returns what was kept (and why), so no work is ever lost: a
//! checkout with unsaved files is never forced. Only things the board created are closed. The space leaves herdr's
//! sidebar either way: a kept checkout stays on disk, and a reply reopens it. Clean-up takes a while (teardown), and
//! the agent's terminal is open until its end: whatever it was looked at is looked at again right before anything is
//! removed. A clean-up put off because the space was in use says so (`retry`), and the housekeeping pass tries again;
//! `since` is what the attempt before saw, so nothing that happened in between is missed.
use crate::agents::installed::read_turn_of;
use crate::clock::now;
use crate::flow;
use crate::herdr::api::read_screen;
use crate::herdr::cli::{args, current_socket, socket_exists};
use crate::herdr::Herdr;
use crate::json::*;
use crate::runner::{claim, release, sh};
use crate::store::marks::tilde;
use crate::store::repos::{cwd_of, folder_of, git_out};
use crate::store::settings::lookup::setting;
use crate::store::settings::values::command_of;
use crate::store::tasks::{get, update_wait, Change, Task};
use std::cell::{Cell, RefCell};
use std::path::Path;

/// What a clean-up kept, and, when it was put off, what it saw for the next try.
#[derive(Default, Debug)]
pub struct Cleanup { pub kept: Vec<String>, pub retry: bool, pub turn: bool, pub comp: Value, pub tip: String, pub still: Value, pub event_seq: i64 }
pub struct Opts<'a> { pub delete_branch: &'a str, pub keep_unmerged: bool, pub still: Option<&'a dyn Fn() -> bool>, pub since: Option<Value>, pub first: Option<String> }
pub fn cleanup(t: &Task, delete_branch: &str) -> Vec<String> { cleanup_with(t, Opts { delete_branch, keep_unmerged: false, still: None, since: None, first: None }).kept }
fn still_ms() -> i64 { std::env::var("HERDR_PROJECTS_STILL_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(120000) }
fn settle_ms() -> i64 { std::env::var("HERDR_PROJECTS_SETTLE_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(5000) }
#[derive(Clone, Copy)]
struct AgentNow { busy: bool, unknown: bool }
pub fn cleanup_with(t: &Task, o: Opts) -> Cleanup {
    let root = folder_of(t.s("id"));
    let link = t.at("link");
    let sock = link.t("socket");
    // After a task ended its own herdr session is asked, whichever session this runs in. A delete acts in this one only.
    let api = if !sock.is_empty() && (o.still.is_some() || sock == current_socket()) && socket_exists(&sock) { Some(Herdr::new(Some(sock.clone()))) } else { None };
    let (mut ws, mut unknown) = (None::<Value>, false); // a space herdr did not answer for is not "closed already"
    if let (Some(api), true) = (&api, link.b("workspace")) {
        match api.call(args(&["workspace", "get", link.s("workspace")])) { Ok(r) => ws = Some(r.at("workspace").clone()).filter(|w| w.is_object()), Err(e) => unknown = !e.code.contains("not_found") }
    }
    let still = o.still.is_some();
    // False when herdr did not close it (a timeout): then it is still in the sidebar, and that is tried again.
    let close_space = || -> bool {
        let (Some(w), true, Some(api)) = (&ws, link.b("ownsWorkspace"), &api) else { return true };
        match api.workspace_close(w.s("workspace_id")) { Ok(_) => true, Err(e) => e.code.contains("not_found") || !still }
    };
    let quiet: RefCell<Value> = RefCell::new(o.since.as_ref().map(|s| s.at("still").clone()).unwrap_or(Value::Null));
    let seen: RefCell<Value> = RefCell::new(o.since.as_ref().map(|s| s.at("comp").clone()).unwrap_or(Value::Null));
    let turn = Cell::new(false);
    let tip_of = || if t.b("branch") { git_out(&root, &["rev-parse", "--verify", &format!("refs/heads/{}", t.s("branch"))]).unwrap_or_default() } else { String::new() };
    let tip = match (&o.since, &o.first) { (Some(s), _) => s.t("tip"), (None, Some(f)) => f.clone(), (None, None) => tip_of() }; // what is safe to delete is what the first look saw
    let later = |kept: Vec<String>| Cleanup { kept, retry: true, turn: turn.get(), comp: seen.borrow().clone(), tip: tip.clone(), still: quiet.borrow().clone(), event_seq: get(t.s("id")).map(|c| c.i("eventSeq")).unwrap_or(0) };
    // An agent herdr cannot place (status "unknown"): its screen must have stood still from one attempt to a later
    // one, its saved conversation end in a finished turn, and nothing run under it.
    let at_rest = |a: &Value| -> bool {
        let Some(api) = &api else { return false };
        let screen = read_screen(api, link.s("pane"));
        if screen.trim().is_empty() { return false; }
        let sig = format!("{}|{}", text(a.at("state_change_seq")), sha1_smol::Sha1::from(screen.as_bytes()).digest());
        let at = { let q = quiet.borrow(); if q.s("sig") == sig { q.i("at") } else { now() } };
        *quiet.borrow_mut() = json!({ "sig": sig, "at": at });
        if o.since.as_ref().map(|s| s.at("still").s("sig") != sig).unwrap_or(true) || now() - at < still_ms() { return false; }
        if let Some(turn) = read_turn_of(t.s("agent"), t.so("transcript").map(Path::new), t.s("session"), &cwd_of(t), false) { if !(turn.finished || !turn.error.is_empty() || turn.interrupted) { return false; } }
        api.pane_childless(link.s("pane"), a.s("agent")) // a command it left running in the background would go down with the space
    };
    // After a task ended (`still`): herdr is asked directly: an agent at work in the pane, or a turn of it that ended
    // since, is a turn you started in its terminal. What herdr cannot tell counts as busy.
    let agent_now = || -> Option<AgentNow> {
        let (true, Some(api), true) = (still, &api, link.b("pane")) else { return None };
        match api.agent_get(link.s("pane")) {
            Ok(Some(a)) => {
                *seen.borrow_mut() = a.at("completion_seq").clone();
                let st = a.s("agent_status");
                let busy = st == "working" || st == "blocked";
                let unknown = if st == "idle" || st == "done" { !api.pane_childless(link.s("pane"), a.s("agent")) } else { !busy && !at_rest(&a) };
                Some(AgentNow { busy, unknown })
            }
            Ok(None) => { *seen.borrow_mut() = Value::Null; None }
            Err(_) => { *seen.borrow_mut() = o.since.as_ref().map(|s| s.at("comp").clone()).unwrap_or(Value::Null); Some(AgentNow { busy: true, unknown: true }) }
        }
    };
    let comp_now = || seen.borrow().clone();
    // The space is closed whole: a pane of yours in it with something running keeps it open as well.
    let others_busy = || -> bool {
        if still && unknown { return true; }
        let (true, Some(w), Some(api)) = (still, &ws, &api) else { return false };
        match api.panes(Some(w.s("workspace_id"))) {
            Ok(ps) => ps.iter().any(|p| p.s("pane_id") != link.at("reviewer").s("pane") && { let own = p.s("pane_id") == link.s("pane"); let a = api.agent_get(p.s("pane_id")).ok().flatten(); if own && a.is_some() { false } else { a.is_some() || !api.pane_idle(p.s("pane_id")) } }),
            Err(_) => true,
        }
    };
    // Turn-end events the agent's own hook filed since clean-up began.
    let events = || if still { get(t.s("id")).map(|c| c.i("eventSeq")).unwrap_or(0) - o.since.as_ref().map(|s| s.i("eventSeq")).unwrap_or(t.i("eventSeq")) } else { 0 };
    let mut agent = agent_now();
    let mut since_comp: Option<Value> = o.since.as_ref().map(|s| s.at("comp").clone());
    // herdr's status can trail the turn that just ended: give it a moment before believing "working".
    let mut reused = false;
    if o.since.is_some() {
        reused = agent.map(|a| !a.unknown && (a.busy || (since_comp.as_ref().map(|c| !c.is_null()).unwrap_or(false) && comp_now() != *since_comp.as_ref().unwrap()))).unwrap_or(false);
        if let Some(a) = agent { if !a.unknown && since_comp.as_ref().map(|c| c.is_null()).unwrap_or(true) { since_comp = Some(comp_now()); } }
    } else if agent.map(|a| a.busy || a.unknown).unwrap_or(false) {
        let until = now() + settle_ms();
        while agent.map(|a| a.busy || a.unknown).unwrap_or(false) && now() < until { std::thread::sleep(std::time::Duration::from_millis((settle_ms() / 6).clamp(1, 500) as u64)); agent = agent_now(); }
        reused = !agent.map(|a| a.unknown).unwrap_or(false) && (agent.map(|a| a.busy).unwrap_or(false) || !t.b("hooked") || events() > 0);
    }
    let base = if o.since.is_some() { since_comp.clone() } else if agent.is_some() { Some(comp_now()) } else { None };
    let in_use = || -> bool {
        if reused || events() > 0 { turn.set(true); return true; }
        let a = agent_now();
        if let Some(x) = a { if !x.unknown && (x.busy || (o.since.is_none() && agent.is_none()) || base.as_ref().map(|b| comp_now() != *b).unwrap_or(false)) { turn.set(true); return true; } }
        a.map(|x| x.unknown).unwrap_or(false) || agent.map(|x| x.unknown).unwrap_or(false) || others_busy()
    };
    let keep = |why: String| -> Cleanup { let open = in_use() || !close_space(); if let Some(api) = &api { flow::spaces::close_repo_space(api, &root); } if open { later(vec![why]) } else { Cleanup { kept: vec![why], ..Default::default() } } };
    let used = || later(vec![format!("{} (its space is in use again)", tilde(&cwd_of(t).display().to_string()))]);
    let stuck = || later(vec![format!("{} (herdr did not close its space)", tilde(&cwd_of(t).display().to_string()))]);
    // Its session is not there to ask (herdr is down, or gone): whether its agent works is unknown: nothing is touched.
    if still && api.is_none() && !sock.is_empty() && (link.b("pane") || link.b("workspace")) { return later(vec![format!("{} (its herdr session cannot be reached)", tilde(&cwd_of(t).display().to_string()))]); }
    if in_use() { return used(); }
    let moved = || tip_of() != tip || o.still.map(|f| !f()).unwrap_or(false);
    // Nothing on the branch that your checkout or the commit it started from does not already have.
    let landed = || tip.is_empty() || ["HEAD", t.s("baseSha"), t.s("from")].iter().filter(|r| !r.is_empty()).any(|r| git_out(&root, &["merge-base", "--is-ancestor", &tip, r]).is_ok());
    let spent = o.keep_unmerged && t.b("branch") && landed();
    let wt = t.s("worktree");
    let wt_short = tilde(wt);
    if !wt.is_empty() && Path::new(wt).exists() {
        if t.at("plan").b("locked") { crate::plan::lock::unlock(t); } // read-only while its plan waited: a worktree that cannot be written cannot be removed
        if !flow::git::dirty(Path::new(wt)).is_empty() { return keep(format!("{wt_short} (uncommitted changes)")); }
        if moved() { return keep(format!("{wt_short} (new work arrived during clean-up)")); }
        if o.keep_unmerged && t.b("branch") && !spent { return keep(format!("{wt_short} (unmerged work on {})", t.s("branch"))); }
        let teardown = command_of(&setting("workspace.teardown", Some(t)));
        if !teardown.is_empty() {
            let env = vec![("HERDR_PROJECTS_ID".to_string(), t.t("id")), ("HERDR_PROJECTS_WORKTREE".into(), wt.into()), ("HERDR_PROJECTS_REPO".into(), root.display().to_string())];
            sh(&teardown, Path::new(wt), &env, 5.0, None, None);
        }
        if in_use() { return used(); }
        if !flow::git::dirty(Path::new(wt)).is_empty() { return keep(format!("{wt_short} (uncommitted changes)")); }
        if moved() { return keep(format!("{wt_short} (new work arrived during clean-up)")); }
        if !link.b("ownsWorkspace") && link.b("agentPaneCreated") {
            if let Some(api) = &api { if api.pane_get(link.s("pane")).is_some() && api.pane_close(link.s("pane")).is_none() { return keep(format!("{wt_short} (its agent pane could not be closed)")); } }
        }
        let removed = (|| -> crate::err::Res<()> {
            match (&api, link.b("ownsWorkspace")) {
                (Some(api), true) => {
                    // `herdr worktree remove` also closes its workspace. If the human closed the workspace, reopen it first.
                    let target = match &ws { Some(w) if w.at("worktree").s("checkout_path") == wt => w.t("workspace_id"), _ => flow::spaces::opening_worktree(api, &root, || api.worktree_open(&root.display().to_string(), wt, ""))?.at("workspace").t("workspace_id") };
                    api.worktree_remove(&target, false)?;
                }
                _ => { git_out(&root, &["worktree", "remove", wt])?; }
            }
            Ok(())
        })();
        if let Err(e) = removed { if git_out(&root, &["worktree", "remove", wt]).is_err() { return keep(format!("{wt_short} ({})", e.msg.lines().next().unwrap_or("could not be removed"))); } }
        let _ = git_out(&root, &["worktree", "prune"]);
        // git removed the checkout, herdr did not: its space is still there, or herdr does not say.
        if let (Some(api), true, true) = (&api, link.b("ownsWorkspace"), link.b("workspace")) {
            let there = match api.call(args(&["workspace", "get", link.s("workspace")])) { Ok(_) => true, Err(e) if e.code.contains("not_found") => false, Err(_) => { if still { return stuck(); } true } };
            if there && !close_space() { return stuck(); }
        }
    } else if in_use() { return used(); } else if !close_space() { return stuck(); }
    if let Some(api) = &api { flow::spaces::close_repo_space(api, &root); }
    let mut kept = vec![];
    if t.b("branch") && tip_of() != tip { kept.push(format!("branch {} (new work arrived during clean-up)", t.s("branch"))); }
    else if t.b("branch") && git_out(&root, &["rev-parse", "--verify", &format!("refs/heads/{}", t.s("branch"))]).is_ok() {
        let flag = if o.delete_branch == "always" || spent { "-D" } else { "-d" };
        if git_out(&root, &["branch", flag, t.s("branch")]).is_err() { kept.push(format!("branch {} (not merged yet)", t.s("branch"))); }
    }
    Cleanup { kept, ..Default::default() }
}
/// After landing: remove the leftovers when that is safe, and say where they are when it is not. A task that ended with
/// nothing to land is tidied the same way, only its branch is deleted just when git calls it merged.
pub fn tidy(id: &str, delete_branch: &str) {
    let Some(t) = get(id) else { return };
    if setting("workspace.cleanup", Some(&t)) != json!(true) || t.b("archivedAt") { return; }
    // The task as it ended: a turn started since (typed in the agent's own terminal) means it is no longer over.
    let armed = t.at("armedAt").clone();
    let same = move |c: &Task| c.s("stage") == "done" && !c.b("archivedAt") && *c.at("armedAt") == armed;
    let prev = if t.at("tidy").is_object() && !t.at("tidy").b("fresh") { Some(t.at("tidy").clone()) } else { None };
    if !t.at("tidy").is_object() { let _ = update_wait(id, 10000, |c| { if !same(c) || c.at("tidy").is_object() { return Ok(Change::Keep); } let m = flow::spaces::tidy_mark(c, delete_branch, ""); c.put("tidy", m.unwrap_or(Value::Null)); Ok(Change::Write) }); }
    let still = || get(id).map(|c| same(&c)).unwrap_or(false);
    let first = if t.at("tidy").b("fresh") { Some(t.at("tidy").t("tip")) } else { None };
    let kept = cleanup_with(&t, Opts { delete_branch, keep_unmerged: delete_branch != "always", still: Some(&still), since: prev.clone(), first });
    let mut left = false;
    let _ = update_wait(id, 10000, |t| {
        if !same(t) { return Ok(Change::Keep); }
        let msg = prev.as_ref().filter(|p| p.has("msg")).map(|p| p.t("msg")).unwrap_or_else(|| t.t("msg"));
        // In use: tried again by the housekeeping pass, and after a turn of the agent only once it has been quiet a while.
        let quiet_from = if kept.turn { now() } else { prev.as_ref().map(|p| p.i("quietFrom")).unwrap_or(0) };
        t.put("tidy", if kept.retry { json!({ "deleteBranch": delete_branch, "tip": kept.tip, "comp": kept.comp, "still": if kept.still.is_null() { Value::Null } else { kept.still.clone() }, "eventSeq": kept.event_seq, "quietFrom": quiet_from, "msg": msg }) } else { Value::Null });
        if !kept.kept.is_empty() { t.put("kept", kept.kept.clone()); } else {
            flow::task::clear_notes(t); left = true; t.del("kept"); t.put("worktree", Value::Null); t.put("cleaned", now());
            let s = t.at("link").at("socket").clone(); t.put("link", json!({ "socket": s }));
        }
        let end = if msg.ends_with('.') { "" } else { "." };
        t.put("msg", if kept.kept.is_empty() { msg.clone() } else { format!("{msg}{end} Kept: {}", kept.kept.join(", ")) });
        Ok(Change::Write)
    });
    if left && !t.arr("attempts").is_empty() {
        let more = crate::attempts::adopt::discard_all(id);
        if !more.is_empty() { let _ = update_wait(id, 10000, |t| { let mut k = strings(t.at("kept")); k.extend(more.clone()); t.put("kept", k); let m = t.t("msg"); t.put("msg", format!("{m}{} Kept: {}", if m.ends_with('.') { "" } else { "." }, more.join(", "))); Ok(Change::Write) }); }
    }
}
/// Clean-up holds a runner marker like every other long step, so archive answers "busy" until it has ended.
pub fn cleanup_job(id: &str, delete_branch: &str) {
    let Ok(Some(t)) = claim(id, "cleanup", |_| {}) else { return };
    if t.s("stage") == "done" { tidy(id, delete_branch); }
    release(id, false);
}

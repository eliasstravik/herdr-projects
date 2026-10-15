//! The detached jobs that put an agent on a task: a first delegation (a place to work, the repo's setup, the agent and
//! its first prompt), restarting an exited agent, and reopening a task's space to go into it.
use super::launch::{again_prompt, agent_now, exists, resumable, start_agent, unsent_prompt};
use super::rules::{first_prompt, turned_words};
use super::worktree::{base_ref, copy_includes, exclude_rule_dir, free_branch};
use crate::agents::start_failed as sf;
use crate::board::keys::key_name;
use crate::clock::now;
use crate::err::Res;
use crate::flow;
use crate::herdr::Herdr;
use crate::json::*;
use crate::plan;
use crate::research as r;
use crate::runner::{claim, is_aborted, release, sh, Guard};
use crate::store::failures::failure_patch;
use crate::store::files::{board_env, dir_of, marker_alive};
use crate::store::inbox::{take_text, untake_text};
use crate::store::repos::{can_worktree, cwd_of, folder_of, git_out, is_repo};
use crate::store::settings::lookup::{setting, setting_text};
use crate::store::settings::places::repo_file;
use crate::store::settings::values::{command_of, list_of};
use crate::store::tasks::{find, get, update_wait, Change, Task};
use std::path::Path;

/// A start that failed: the reason in plain words on the row, the whole account (what herdr said, the terminal's last
/// lines, where to look) in its failure, and the same at the end of the task's log (agents/start_failed.rs).
fn fail_start(id: &str, title: &str, said: &str, again: &str) {
    let Some(t) = get(id) else { return };
    let (kind, pane, api) = (t.t("agent"), t.at("link").t("pane"), flow::task::h(&t));
    let screen = sf::screen_of(&api, &pane);
    let w = sf::explain(&sf::Start { kind: &kind, setting: if plan::planner(&t) { "plan.agent" } else { "worker.agent" }, again }, said, &screen);
    sf::tidy(&api, &pane, &screen);
    let log = dir_of(id).join("runner.log");
    let look = if screen.is_empty() { String::new() } else { format!("Its terminal is still open: go into the task ({}) to see all of it. ", key_name("enter")) };
    let detail = sf::detail(&w, said, &screen, &look, &log);
    sf::log(&log, &kind, &detail);
    let _ = update_wait(id, 10000, |t| {
        if t.at("runner").i("pid") != std::process::id() as i64 { return Ok(Change::Keep); }
        t.put("stage", "failed"); t.put("msg", format!("{}: {}", title.trim_end_matches(" the agent"), w.short));
        for (k, v) in failure_patch(&obj(json!({ "stage": "start", "title": title, "detail": detail, "agentCanFix": false, "auth": w.auth, "next": w.next }))) { t.insert(k, v); }
        t.obj_mut("failure").put("next", w.next.clone()); // a login too: its own next step, not "tell the agent to continue"
        Ok(Change::Write)
    });
}
fn link_pane(t: &mut Task, pane: &Value, workspace: &str, owns: bool) {
    let l = t.obj_mut("link");
    l.put("workspace", if workspace.is_empty() { pane.at("workspace_id").clone() } else { json!(workspace) });
    l.put("tab", pane.at("tab_id").clone()); l.put("pane", pane.at("pane_id").clone()); l.put("terminal", pane.at("terminal_id").clone());
    l.put("ownsWorkspace", owns); l.put("reviewer", Value::Null);
}
/// The root pane of what herdr opened: the answer's own, or the first of its space.
fn root_pane(api: &Herdr, res: &Value) -> Res<Value> {
    if res.at("root_pane").is_object() { return Ok(res.at("root_pane").clone()); }
    api.panes(Some(res.at("workspace").s("workspace_id")))?.into_iter().next().ok_or_else(|| "herdr opened no terminal".into())
}
pub fn delegate_job(id: &str) {
    let Ok(Some(t)) = claim(id, "delegate", |_| {}) else { return };
    if t.s("stage") != "starting" { release(id, false); return; }
    let mut g = Guard::new(id);
    g.expect(&t, &[]);
    if let Err(e) = delegate_steps(id, &mut g, t) {
        if !is_aborted(&e) { fail_start(id, "Could not start the agent", &e.msg, &format!("delegate again ({})", key_name("delegate"))); }
    }
    release(id, false);
}
fn delegate_steps(id: &str, g: &mut Guard, mut t: Task) -> Res<()> {
    let api = flow::task::h(&t);
    let root = folder_of(t.s("id"));
    let label = flow::sidebar::space_label(&t);
    // The plan has its yes: this start is the build's (plan/build.rs).
    if plan::approved(&t) && !t.at("plan").b("built") { return plan::build::start_build(id, g); }
    // 1. A place to work, unless the task still has a live pane from an earlier run.
    let live_pane = if t.at("link").b("pane") { api.pane_get(t.at("link").s("pane")) } else { None };
    if live_pane.is_some() && api.agent_get(t.at("link").s("pane"))?.is_some() { // delegate again on a task whose agent is still there: just continue it
        t = g.set(|t| { let _ = r::starts_as(t); })?;
        let turned = turned_words(&t);
        t = g.set(|t| { t.put("stage", "working"); t.put("armedAt", now()); t.put("turnOpen", true); t.put("turnBy", "start"); t.del("researchTurned"); })?;
        flow::send::deliver(&t, &flow::send::with_tail(&t, &[turned, "Continue this task.".into()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n")), None, false)?;
        return Ok(());
    }
    if live_pane.is_none() {
        let mut copy = false;
        let (res, mut patch): (Value, Obj);
        if exists(t.s("worktree")) {
            res = flow::spaces::opening_worktree(&api, &root, || api.worktree_open(&root.display().to_string(), t.s("worktree"), &label))?;
            patch = Obj::new();
        } else if can_worktree(&root) && !t.b("startedOnce") { // a task in a repo always works in a worktree of its own
            // A repo with no commits yet has nothing to branch from: give it an empty first commit (files and index stay).
            if flow::git::head_of(&root).is_empty() && git_out(&root, &["rev-parse", "--verify", "HEAD"]).is_err() {
                let tree = git_out(&root, &["hash-object", "-w", "-t", "tree", "/dev/null"])?;
                let c = git_out(&root, &["commit-tree", &tree, "-m", "Initial commit"])?;
                git_out(&root, &["update-ref", "HEAD", &c])?;
            }
            let branch = free_branch(&root, &t);
            let from = base_ref(&root, &setting_text("workspace.from", Some(&t)));
            // An attempt works in a tab of its task's one space; any other task in a space of its own.
            res = if t.b("attemptOf") { crate::attempts::place::open_place(&api, &root, &t, &branch, &from)? } else { flow::spaces::opening_worktree(&api, &root, || api.worktree_create(&root.display().to_string(), &branch, &from, &label))? };
            let dir = if res.at("worktree").b("path") { res.at("worktree").t("path") } else { res.at("workspace").at("worktree").t("checkout_path") };
            // The repo's own file, read from the commit the worktree starts from: its facts, and what it was refused.
            let (facts, refused) = repo_file(&root, Some(&from));
            exclude_rule_dir(&root);
            let base = if from == "HEAD" { let h = flow::git::head_of(&root); if h.is_empty() { git_out(&root, &["rev-parse", &from])? } else { h } } else { git_out(&root, &["rev-parse", &from])? };
            patch = obj(json!({ "worktree": dir, "branch": branch, "from": from, "baseSha": base, "repoFile": facts, "repoRefused": refused, "fresh": true, "ownNote": true }));
            copy = true;
        } else {
            res = api.workspace_create(&root.display().to_string(), &label, &board_env())?;
            if is_repo(&root) { exclude_rule_dir(&root); }
            let (facts, refused) = if t.at("repoFile").is_object() { (t.o("repoFile").cloned().unwrap_or_default(), strings(t.at("repoRefused"))) } else { repo_file(&root, None) };
            patch = obj(json!({ "repoFile": facts, "repoRefused": refused, "fresh": !t.b("startedOnce"), "startMark": if is_repo(&root) { r::start_mark(&root) } else { None } }));
            if !t.b("startedOnce") { patch.put("ownNote", true); }
        }
        let pane = root_pane(&api, &res)?;
        let ws = res.at("workspace").t("workspace_id");
        let root2 = root.clone();
        t = g.set(|t| {
            let refused = strings(patch.at("repoRefused"));
            for (k, v) in patch { t.insert(k, v); }
            let f = t.obj_mut("fallbacks");
            if refused.is_empty() { f.del("repoFile"); } else { f.put("repoFile", refused.join("\n")); }
            t.put("root", root2.display().to_string()); t.put("startedOnce", true);
            let owns = !t.b("attemptOf");
            link_pane(t, &pane, &ws, owns);
            if res.at("tab").b("tab_id") { t.obj_mut("link").put("tab", res.at("tab").at("tab_id").clone()); }
        })?;
        if copy { copy_includes(&root, Path::new(t.s("worktree")), &list_of(&setting("workspace.include", Some(&t)))); }
    }
    // 2. The repo prepares itself: setup runs as a plain subprocess, before the agent starts.
    if t.b("fresh") {
        t = g.set(|t| t.put("fresh", false))?;
        let setup = command_of(&setting("workspace.setup", Some(&t)));
        if !setup.is_empty() {
            t = g.set(|t| { t.put("stage", "setup"); t.put("msg", format!("Setting up: {setup}")); })?;
            let env = vec![("HERDR_PROJECTS_ID".to_string(), t.t("id")), ("HERDR_PROJECTS_WORKTREE".into(), cwd_of(&t).display().to_string()), ("HERDR_PROJECTS_REPO".into(), root.display().to_string())];
            let alive = || g.check().is_ok();
            let out = sh(&setup, &cwd_of(&t), &env, 15.0, Some(&alive), None);
            if out.aborted { return Ok(()); }
            if !out.ok {
                let cwd = cwd_of(&t).display().to_string();
                g.set(|t| {
                    t.put("stage", "failed"); t.put("setupPending", true); t.put("msg", "Setup failed");
                    for (k, v) in failure_patch(&obj(json!({ "stage": "setup", "title": "Setup failed", "detail": format!("Command: {setup}\nFolder: {cwd}\n\n{}", if out.out.is_empty() { "No output" } else { &out.out }), "next": "Fix the setup problem in the working folder, then finish again. The setup is retried before the check." }))) { t.insert(k, v); }
                })?;
                return Ok(());
            }
        }
    }
    // 3. The agent, in the workspace's root pane, then the task. A task whose agent ran before continues.
    agent_now(g, t)?;
    // Research only or not is decided now, for this run, right before the program is started.
    let mut refusal = None;
    t = g.set(|t| { if let Err(e) = r::starts_as(t) { refusal = Some(e); } })?;
    if let Some(e) = refusal { return Err(e); }
    if !t.b("startedAgentAt") && !t.at("plan").is_object() { t = plan::build::begin(g, t, &api)?; } // the work starts with a plan, where `plan.approve` says so
    let waiting: Vec<String> = take_text(id)?.iter().map(|m| m.t("text")).collect();
    let turned = if t.b("startedAgentAt") { turned_words(&t) } else { String::new() };
    let (resume, prompt) = if t.b("startedAgentAt") { again_prompt(&t, &[turned].into_iter().filter(|s| !s.is_empty()).chain(waiting.iter().cloned()).collect::<Vec<_>>()) }
        else { (None, std::iter::once(first_prompt(&t)).chain(waiting.iter().cloned()).collect::<Vec<_>>().join("\n\n")) };
    if let Err(e) = start_agent(id, g, &prompt, resume.as_deref()) { if !waiting.is_empty() { untake_text(id, waiting.len())?; } return Err(e); }
    if t.b("researchTurned") { g.set(|t| t.del("researchTurned"))?; }
    Ok(())
}
/// Restarting an exited agent: the same pane (or a new one in the task's workspace, reopened from the worktree if the
/// workspace was closed). It resumes the same conversation where it can; otherwise it starts fresh with the task.
pub fn restart_job(id: &str) {
    let Ok(Some(t)) = claim(id, "restart", |_| {}) else { return };
    let mut g = Guard::new(id);
    g.expect(&t, &[]);
    if let Err(e) = restart_steps(id, &mut g, t) {
        if !is_aborted(&e) { fail_start(id, "Could not restart the agent", &e.msg, &format!("reply, or delegate again ({})", key_name("delegate"))); }
    }
    release(id, false);
}
fn restart_steps(id: &str, g: &mut Guard, mut t: Task) -> Res<()> {
    let api = flow::task::h(&t);
    let root = folder_of(t.s("id"));
    let mut pane = if t.at("link").b("pane") { api.pane_get(t.at("link").s("pane")) } else { None };
    // Its box may still hold the text it never started on: the program is ended, and started again in the same pane.
    if t.b("unsent") && pane.is_some() && api.agent_get(t.at("link").s("pane"))?.is_some() && !api.pane_quit(t.at("link").s("pane"), 6000) {
        crate::fail!("The agent's program could not be ended to start it again. Go into the agent and press enter, or close its terminal and retry.");
    }
    if pane.is_some() && api.agent_get(t.at("link").s("pane"))?.is_some() { // it is back by itself: just deliver
        let waiting: Vec<String> = take_text(id)?.iter().map(|m| m.t("text")).collect();
        if !waiting.is_empty() { flow::send::deliver(&t, &waiting.join("\n\n"), None, false)?; }
        return Ok(());
    }
    if pane.is_none() {
        let ws = if t.at("link").b("workspace") { api.workspace_get(t.at("link").s("workspace")) } else { None };
        if ws.is_some() && t.b("attemptOf") { pane = crate::attempts::place::reopen_place(&api, &t)?; } // an attempt: a tab of its own again
        else if let Some(ws) = &ws { if let Some(first) = api.panes(Some(ws.s("workspace_id")))?.into_iter().next() { pane = Some(api.pane_split(first.s("pane_id"), &cwd_of(&t).display().to_string(), "right", &board_env())?); } }
        if pane.is_none() {
            let res = if exists(t.s("worktree")) { flow::spaces::opening_worktree(&api, &root, || api.worktree_open(&root.display().to_string(), t.s("worktree"), &flow::sidebar::space_label(&t)))? }
                else { api.workspace_create(&cwd_of(&t).display().to_string(), &flow::sidebar::space_label(&t), &board_env())? };
            pane = Some(root_pane(&api, &res)?);
        }
        let p = pane.clone().unwrap();
        t = g.set(|t| { let owns = !t.b("attemptOf"); link_pane(t, &p, "", owns); })?;
    }
    t = agent_now(g, t)?;
    // A start that was asked for and never got as far as its agent (its setup failed): this is that start.
    if t.b("researchOpen") { let mut refusal = None; t = g.set(|t| { if let Err(e) = r::starts_as(t) { refusal = Some(e); } })?; if let Some(e) = refusal { return Err(e); } }
    let waiting: Vec<String> = take_text(id)?.iter().map(|m| m.t("text")).collect();
    let (resume, prompt) = if t.b("unsent") { unsent_prompt(&t, &waiting) } else { again_prompt(&t, &waiting) };
    if let Err(e) = start_agent(id, g, &prompt, resume.as_deref()) { if !waiting.is_empty() { untake_text(id, waiting.len())?; } return Err(e); }
    Ok(())
}
/// Reopen a task's space when it was closed (archive closes it) and, with `restart`, start its exited agent again in
/// the same conversation without sending it anything. Used by "go into the task" here and from another machine.
pub fn reopen(id: &str, restart: bool) -> Res<(Value, bool, Task)> {
    let mut t = find(id)?;
    flow::live::assert_live(&t)?;
    let api = flow::task::h(&t);
    let pane = match api.pane_get(t.at("link").s("pane")) {
        Some(p) => p,
        None => {
            if !exists(t.s("worktree")) { crate::fail!("Its terminal is gone. Reply or delegate again to start its agent."); }
            let root = folder_of(t.s("id"));
            let res = flow::spaces::opening_worktree(&api, &root, || api.worktree_open(&root.display().to_string(), t.s("worktree"), &flow::sidebar::space_label(&t)))?;
            let p = root_pane(&api, &res)?;
            t = update_wait(t.s("id"), 2000, |t| { link_pane(t, &p, "", true); Ok(Change::Write) })?;
            flow::tabs::name_agent_tab(&api, p.s("tab_id"), Some(&t));
            p
        }
    };
    let mut has_agent = api.agent_get(pane.s("pane_id"))?.is_some();
    if !has_agent && restart && resumable(&t) && !marker_alive(t.at("runner")) {
        let (stage, armed) = (t.at("stage").clone(), t.at("armedAt").clone());
        let mut g = Guard::free(t.s("id"));
        if start_agent(t.s("id"), &mut g, "", Some(t.s("session"))).is_ok() { has_agent = true; }
        let _ = update_wait(t.s("id"), 10000, |c| { c.put("stage", stage.clone()); c.put("armedAt", armed.clone()); c.put("turnOpen", false); Ok(Change::Write) }); // nothing was asked of it
    }
    Ok((pane, has_agent, get(t.s("id")).unwrap_or(t)))
}
/// Put a reopened task's pane in front in its herdr: its agent, or (herdr cannot focus a bare pane by id) with no
/// agent in it its workspace and tab. Used by "go into the task" here and from another machine.
pub fn bring_forward(api: &Herdr, pane: &Value, has_agent: bool) -> crate::herdr::cli::HRes<()> {
    if has_agent { api.agent_focus(pane.s("pane_id")).map(|_| ()) } else { api.workspace_focus(pane.s("workspace_id")).map(|_| { let _ = api.tab_focus(pane.s("tab_id")); }) }
}

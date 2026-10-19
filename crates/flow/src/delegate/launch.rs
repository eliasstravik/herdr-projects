//! Starting the agent's program: its name, the launch arguments that are the board's own (the turn-end hook with a
//! new secret token, the standing rules, a conversation id), waiting for it to be up, and the first text.
use super::rules::{first_prompt, rules};
use crate::agents::{adapter, installed, start, start_failed};
use crate::clock::now;
use crate::err::Res;
use crate::flow;
use crate::json::*;
use crate::plan;
use crate::runner::Guard;
use crate::store::files::dir_of;
use crate::store::inbox::queue_text;
use crate::store::marks::set_token;
use crate::store::repos::{cwd_of, folder_of};
use crate::store::settings::lookup::{setting_text, work};
use crate::store::settings::values::agent_of;
use crate::store::tasks::{get, Task};
use std::path::{Path, PathBuf};

/// `sw-` + the repo name (lower-case, anything outside [a-z0-9] as `-`, 16 characters) + `-<n>`. Always matches
/// herdr's [a-z][a-z0-9_-]{0,31}. The name is a convenience: every later command targets the stored pane id.
pub fn agent_name(t: &Obj, taken: &[String]) -> String {
    let repo: String = t.s("repo").to_lowercase().chars().map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() { c } else { '-' }).take(16).collect();
    let base = format!("sw-{repo}-{}", t.t("n"));
    if !taken.contains(&base) { return base; }
    let h = sha1_smol::Sha1::from(format!("{}:{}:{}", t.s("id"), t.s("root"), now())).digest().to_string();
    format!("{base}-{}", &h[..4])
}
pub struct Extras { pub hook: Vec<String>, pub rules: Vec<String>, pub hooked: bool, pub session: Option<String>, pub version: String }
/// The launch arguments that are the board's own: the turn-end hook (with a new secret token) and the standing rules.
/// Nothing is written to the user's own agent settings.
pub fn launch_extras(t: &Task, role: &str, kind: &str, round: i64, standing: &str, resume: Option<&str>) -> Res<Extras> {
    let token = crate::util::random_hex(18);
    set_token(t.s("id"), role, &token, round)?;
    let mut hook = None;
    if installed::hook_usable(kind) {
        let h = adapter::Hook { shim: flow::send::shim().display().to_string(), id: t.t("id"), role: role.into(), token, round, dir: dir_of(t.s("id")), plan: role == "agent" && plan::plan_args(t, kind).is_some() };
        hook = adapter::hook_args(kind, &h).ok().flatten();
    }
    let session = if resume.is_none() { adapter::new_session(kind) } else { None };
    let rules = if !standing.is_empty() && adapter::has_standing(kind) { adapter::standing(kind, standing, &dir_of(t.s("id")))? } else { vec![] };
    Ok(Extras { hooked: hook.is_some(), hook: hook.unwrap_or_default(), rules, session, version: installed::version_of(kind) })
}
/// Used by a first delegation and by restarting an exited agent. The pane must be at its shell prompt.
pub fn start_agent(id: &str, g: &mut Guard, prompt: &str, resume: Option<&str>) -> Res<&'static str> {
    let t = get(id).ok_or("the task is gone")?;
    let api = flow::task::h(&t);
    if t.at("link").b("missionPane") { crate::mission_agents::release(&api, t.at("link").s("pane")); }
    let kind = t.t("agent");
    // The worker's settings, read now, as it starts. A planner of its own runs on its own program and model.
    let w = work(Some(&t));
    let worker = start::role_of("worker", &|n| w.at(n).clone());
    let who = if plan::planner(&t) { let wr = t.at("plan").at("writer"); start::Role { kind: wr.t("kind"), model: wr.t("model"), effort: String::new(), fast: false, access: worker.access.clone(), instructions: worker.instructions.clone() } } else { worker.clone() };
    let took = agent_of(if plan::planner(&t) { "" } else { t.s("on") });
    let model = if took.kind == kind { took.model } else if who.kind == kind { who.model.clone() } else { String::new() };
    let level = start::effort_for(&kind, &model, &who.effort);
    let root = if t.b("root") { PathBuf::from(t.s("root")) } else { folder_of(t.s("id")) };
    installed::pre_trust(&kind, &root, &who.access);
    if kind == "agy" { installed::pre_trust(&kind, &cwd_of(&t), &who.access); }
    // Keep the task's anchor terminal available for Mission Control and put every first agent in a real tiled pane.
    // Subsequent resumes reuse their existing pane; retrying a failed start must not leave orphan splits behind.
    let t = if !t.at("link").b("agentPaneCreated") && !t.b("attemptOf") {
        let anchor = t.at("link").t("pane");
        let split = api.pane_split(&anchor, &cwd_of(&t).display().to_string(), "right", &crate::store::files::board_env())?;
        g.set(|t| {
            let l = t.obj_mut("link");
            l.put("tab", split.at("tab_id").clone()); l.put("pane", split.at("pane_id").clone()); l.put("terminal", split.at("terminal_id").clone());
            l.put("agentPaneCreated", true);
        })?
    } else { t };
    let taken: Vec<String> = api.agents().unwrap_or_default().iter().filter(|a| a.s("pane_id") != t.at("link").s("pane")).map(|a| a.t("name")).filter(|n| !n.is_empty()).collect();
    let name = agent_name(&t, &taken);
    // Pane labels identify the shared task workspace; planner identity is already shown in task state.
    flow::tabs::name_agent_tab(&api, t.at("link").s("tab"), Some(&t));
    let x = launch_extras(&t, "agent", &kind, 0, &rules(&t), resume)?;
    let fast = if plan::planner(&t) { None } else { Some(who.fast) };
    let args = |hooked: bool| -> Res<Vec<String>> {
        let mut extra = if hooked { x.hook.clone() } else { vec![] };
        extra.extend(x.rules.clone());
        start::start_args(&kind, &start::StartOpts { model: model.clone(), effort: level.clone(), fast, access: who.access.clone(), plan: plan::plan_args(&t, &kind), resume: resume.unwrap_or("").into(), session: x.session.clone().unwrap_or_default(), extra })
    };
    let (mut blocked, mut hooked) = (false, x.hooked);
    let pane = t.at("link").t("pane");
    let mut start_with = |with_hook: bool| -> Res<()> {
        // In a read-only mount while its plan waits (plan/sandbox.rs).
        match plan::sandbox::start_program(&api, &t, &name, &kind, &pane, &args(with_hook)?) {
            Ok(()) => Ok(()),
            Err(e) if e.code == "agent_not_ready" => { blocked = true; Ok(()) } // a dialog is in the way: the text waits in the inbox
            Err(e) => Err(e.into()),
        }
    };
    if let Err(e) = start_with(hooked) { // once more without the hook arguments, from a clean shell prompt
        if !hooked || e.msg.contains("canonical mode") { return Err(e); } // its shell was never at its prompt: tried again already
        start_failed::tidy(&api, &pane, &start_failed::screen_of(&api, &pane));
        hooked = false; start_with(false)?;
    }
    // herdr calls the program ready before it has drawn anything; text typed that early loses its enter (submit.rs).
    if !prompt.is_empty() && !blocked { let launch = std::iter::once(crate::agents::kinds::program(&kind).to_string()).chain(args(hooked)?).collect::<Vec<_>>().join(" "); crate::submit::settle(&api, &pane, &launch); }
    // The send time arms the pipeline: nothing older than this counts as the turn's end.
    let (session, version, plan_mode) = (resume.map(String::from).or(x.session.clone()), x.version.clone(), plan::plan_args(&t, &kind).is_some() || plan::sandbox::boxed(&t));
    let t = g.set(|t| {
        t.put("stage", "working"); t.put("armedAt", now()); t.put("expectedWake", Value::Null); t.put("turnOpen", true); t.put("turnBy", "start"); t.put("msg", "");
        t.obj_mut("link").put("agent", name.clone());
        if !(t.i("workAt") >= t.i("since") && t.has("workAt")) { t.put("workAt", now()); }
        t.put("startedAgentAt", now()); t.put("exitedAt", Value::Null);
        t.put("hooked", hooked); t.put("agentVersion", version.clone()); t.put("sent", Value::Null); t.put("unsent", Value::Null);
        if plan::open(t) { t.obj_mut("plan").put("mode", plan_mode); }
        if t.at("failure").s("stage") == "send" { t.put("failure", Value::Null); }
        let s = session.clone().map(Value::String).unwrap_or(Value::Null);
        if s != *t.at("session") && !(s.is_null() && t.s("session").is_empty()) {
            t.put("session", s); t.put("transcript", Value::Null);
            let h = t.obj_mut("handled"); h.put("msg", Value::Null); h.put("pos", 0);
        }
    })?;
    if prompt.is_empty() { return Ok("started"); }
    let mut extra = Obj::new();
    if resume.is_none() { extra.put("first", true); }
    if blocked { queue_text(id, prompt, extra)?; return Ok("queued"); }
    flow::send::deliver(&t, prompt, None, resume.is_none())
}
/// Which program works on the task is `worker.agent` as it reads when the agent starts, not when it was queued. Only
/// a conversation that is reopened keeps its program, which is the one that holds it.
pub fn agent_now(g: &mut Guard, t: Task) -> Res<Task> {
    if (t.b("startedAgentAt") && resumable(&t)) || plan::planner(&t) { return Ok(t); }
    let choice = if t.at("link").b("missionPane") { crate::mission_control::worker_choice(&t, t.at("link").s("socket"))? }
        else if t.b("on") { t.t("on") } else { setting_text("worker.agent", Some(&t)) };
    let k = agent_of(&choice).kind;
    let t = if t.at("link").b("missionPane") { g.set(|t| { t.put("on", choice.clone()); })? } else { t };
    if !k.is_empty() && k != t.s("agent") { return g.set(|t| { t.put("agent", k.clone()); t.put("app", k.clone()); t.put("session", Value::Null); t.put("transcript", Value::Null); }); }
    Ok(t)
}
/// It resumes the conversation whose id the board chose (or learned) at launch, when the program still has it. A
/// program whose files the board does not read is taken at its word: the id herdr's integration reported, or the one
/// the board gave it, which it saved once it was prompted.
pub fn resumable(t: &Task) -> bool {
    let k = t.s("agent");
    if !t.b("session") || adapter::resume_args(k, "x").is_empty() { return false; }
    if !crate::agents::kinds::kind(k).map(|a| a.reads_turn).unwrap_or(false) { return t.b("sent") || t.b("said"); }
    adapter::transcript(k, t.s("session"), &cwd_of(t)).is_some()
}
/// The prompt for an agent that is started again on a task it worked on before: (resume, prompt).
pub fn again_prompt(t: &Task, waiting: &[String]) -> (Option<String>, String) {
    if resumable(t) { return (Some(t.t("session")), flow::send::with_tail(t, &if waiting.is_empty() { "Continue where you left off.".into() } else { waiting.join("\n\n") })); }
    let said = if t.b("said") { format!("Your last message was:\n{}", t.s("said")) } else { String::new() };
    (None, [first_prompt(t), said, "You were working on this before and your session ended. Continue from the state of this branch.".into(), waiting.join("\n\n")].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n"))
}
/// Text the agent never started on goes in again, word for word, once its program has been started anew.
pub fn unsent_prompt(t: &Task, waiting: &[String]) -> (Option<String>, String) {
    let u = t.at("unsent");
    let mut all = vec![u.t("text")];
    all.extend(waiting.iter().cloned());
    if u.b("first") { return (None, all.join("\n\n")); }
    if resumable(t) { (Some(t.t("session")), all.join("\n\n")) } else { again_prompt(t, &all) }
}
pub fn exists(p: &str) -> bool { !p.is_empty() && Path::new(p).exists() }

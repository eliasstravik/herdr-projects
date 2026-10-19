//! The plan step's two ends: it begins when the agent would first start, and after the yes the build starts. `begin`:
//! with `plan.approve` on, the task gets its plan step, and with `plan.agent` naming a program, a planner of its own,
//! which takes the task's place in herdr while it writes (its pane becomes the task's). `start_build`:
//! the yes is written down, the repo is looked at once more, and the builder is handed the plan. A task done several
//! times at once is split only now: one plan, approved once, and every attempt is handed it.
use super::lock::{lock, unlock};
use super::sandbox::{no_mount, sandbox_ok};
use super::untouched::{held_failure, ignored_mark, put_back};
use super::{approved, go_words, wanted};
use crate::agents::installed::installed;
use crate::clock::now;
use crate::delegate::{first_prompt, resumable, start_agent};
use crate::err::Res;
use crate::finish::shared::{fail, FailHow};
use crate::flow;
use crate::herdr::Herdr;
use crate::json::*;
use crate::runner::Guard;
use crate::store::attempts::{attempt_agents, create_attempt, LETTERS};
use crate::store::files::board_env;
use crate::store::inbox::{take_text, untake_text};
use crate::store::repos::{cwd_of, is_repo};
use crate::store::settings::lookup::{setting, setting_text};
use crate::store::settings::values::agent_of;
use crate::store::tasks::{update_wait, Change, Task};

pub fn begin(g: &mut Guard, t: Task, api: &Herdr) -> Res<Task> {
    let by = if t.b("attemptOf") { String::new() } else { wanted(&t) }; // an attempt builds its task's plan
    if by.is_empty() { return Ok(t); }
    // In a repo the plan's writer runs in a read-only mount. Where none can be made, no plan is written and nothing starts.
    let dir = cwd_of(&t);
    let repo = is_repo(&dir);
    if repo && !sandbox_ok() { crate::fail!("{}", no_mount(t.s("id"))); }
    let who = agent_of(&setting_text("plan.agent", Some(&t)));
    // A planner that is not installed here: the working agent plans, and the task says so.
    let own = !who.kind.is_empty() && installed(&who.kind);
    let missing = (!who.kind.is_empty() && !own).then(|| format!("planned by the working agent: {} is not installed here", who.kind));
    let made = if own {
        let m = flow::tabs::open_tab(api, &t, flow::tabs::PLANNER_TAB, &dir.display().to_string(), &board_env(), "the planner")?;
        flow::tabs::name_agent_tab(api, t.at("link").s("tab"), None);
        Some(m)
    } else { None };
    // From here until the yes nothing can be written in the task's own worktree, and what git ignores there is written down.
    let ignored = if repo { ignored_mark(&dir, None) } else { None };
    let locked = lock(&t);
    g.set(|t| {
        let mut p = obj(json!({ "asked": now(), "by": by, "locked": locked }));
        if let Some(i) = &ignored { p.put("ignored", i.clone()); }
        if made.is_some() {
            let l = t.at("link").clone();
            p.put("home", json!({ "tab": l.at("tab"), "pane": l.at("pane"), "terminal": l.at("terminal"), "agent": t.at("agent") }));
            p.put("writer", json!({ "kind": who.kind, "model": who.model }));
        }
        t.put("plan", p);
        let f = t.obj_mut("fallbacks");
        match &missing { Some(m) => f.put("planner", m.clone()), None => f.del("planner") }
        f.del("attempts");
        if let Some(m) = &made {
            let l = t.obj_mut("link"); l.put("tab", m.tab.clone()); l.put("pane", m.pane.clone()); l.put("terminal", m.terminal.clone());
            l.put("agentPaneCreated", true);
            t.put("agent", who.kind.clone()); t.put("app", who.kind.clone()); t.put("session", Value::Null); t.put("transcript", Value::Null);
        }
    })
}
/// Who makes each attempt, when the task is done several times at once ([] when once). Read now, as the build starts.
fn attempts_now(t: &Task) -> Vec<String> {
    if t.b("attemptOf") || t.b("picked") || !t.b("worktree") { return vec![]; }
    attempt_agents(&setting("attempts.by", Some(t)), &setting_text("worker.agent", Some(t)))
}
/// What the board knew of one program in one pane, which is not the next one's.
const FORGET: [&str; 14] = ["session", "transcript", "handled", "last", "terms", "startedAgentAt", "usage", "model", "said", "sent", "unsent", "workedAt", "exitedAt", "owedTurn"];
/// `by`: who said yes (you, reviewer, or nobody: the setting was taken off). A yes of yours is written down already.
pub fn start_build(id: &str, g: &mut Guard) -> Res<()> { start_build_by(id, g, "") }
pub fn start_build_by(id: &str, g: &mut Guard, by: &str) -> Res<()> {
    let mut t = g.check()?;
    let api = flow::task::h(&t);
    if !approved(&t) { t = g.set(|t| { let p = t.obj_mut("plan"); p.put("approvedAt", now()); p.put("approvedBy", if by.is_empty() { "you" } else { by }); })?; }
    // Once more, right before the first change may be made.
    let Some(undone) = put_back(&t) else { fail(g, held_failure(&t), FailHow::default())?; return Ok(()); };
    unlock(&t);
    if !undone.is_empty() { g.set(|t| t.obj_mut("plan").put("undone", json!({ "list": undone.iter().take(30).collect::<Vec<_>>(), "n": undone.len(), "at": now() })))?; }
    // The writer's conversation is counted onto the task before it is let go.
    crate::limits::spent::count(id, "agent", 10000);
    t = g.check()?;
    let planner = t.at("plan").b("home");
    if planner {
        let was = t.at("link").t("pane");
        t = g.set(|t| {
            let home = t.at("plan").at("home").clone();
            let worker = { let k = agent_of(&setting_text("worker.agent", Some(t))).kind; if k.is_empty() { home.t("agent") } else { k } };
            let (usage, model, asked) = (t.at("usage").clone(), if t.b("model") { t.t("model") } else { t.at("plan").at("writer").t("model") }, t.at("plan").i("asked"));
            let p = t.obj_mut("plan"); p.put("home", Value::Null); p.put("usage", usage); p.put("model", model); p.put("took", now() - if asked > 0 { asked } else { now() });
            let l = t.obj_mut("link"); l.put("tab", home.at("tab").clone()); l.put("pane", home.at("pane").clone()); l.put("terminal", home.at("terminal").clone());
            // The task's original pane is the worker's reserved pane after the planner leaves.
            l.put("agentPaneCreated", true);
            t.put("agent", worker.clone()); t.put("app", worker);
            for k in FORGET { t.del(k); } // another program in another pane: nothing the board knew of the planner's is the worker's
        })?;
        api.pane_close(&was);
    }
    let many = attempts_now(&t);
    if many.len() > 1 {
        if !planner && api.agent_get(t.at("link").s("pane"))?.is_some() && !api.pane_quit(t.at("link").s("pane"), 6000) { crate::fail!("The plan writer's program could not be ended to start the attempts. Close its terminal, then retry."); }
        fan_out(g, &many)?;
        return Ok(());
    }
    let waiting: Vec<String> = take_text(id)?.iter().map(|m| m.t("text")).collect();
    let r = (|| -> Res<()> {
        if planner { start_agent(id, g, &std::iter::once(first_prompt(&t)).chain(waiting.iter().cloned()).collect::<Vec<_>>().join("\n\n"), None)?; return Ok(()); }
        let a = api.agent_get(t.at("link").s("pane"))?;
        let words = std::iter::once(go_words(&t)).chain(waiting.iter().cloned()).collect::<Vec<_>>().join("\n\n");
        if a.is_some() && !t.at("plan").b("mode") {
            // The same program goes on in the same conversation: the turn this starts is the first of the work.
            t = g.set(|t| { t.put("stage", "working"); t.put("armedAt", now()); t.put("expectedWake", Value::Null); t.put("turnOpen", true); t.put("turnBy", "start"); t.put("msg", ""); t.put("watch", Value::Null); t.del("kind"); })?;
            flow::send::deliver(&t, &flow::send::with_tail(&t, &words), None, false)?;
        } else {
            // In its own plan mode, or in the read-only mount: neither ends by itself. Ended, and started again.
            if a.is_some() && !api.pane_quit(t.at("link").s("pane"), 6000) { crate::fail!("The agent's program could not be ended to leave its plan step. Close its terminal, then retry."); }
            if resumable(&t) { start_agent(id, g, &flow::send::with_tail(&t, &words), Some(t.s("session")))?; }
            else { start_agent(id, g, &std::iter::once(first_prompt(&t)).chain(waiting.iter().cloned()).collect::<Vec<_>>().join("\n\n"), None)?; }
        }
        Ok(())
    })();
    if let Err(e) = r { if !waiting.is_empty() { untake_text(id, waiting.len())?; } return Err(e); }
    let t = update_wait(id, 10000, |t| { let p = t.obj_mut("plan"); p.put("built", now()); p.put("locked", false); p.del("ignored"); t.del("kind"); Ok(Change::Write) })?;
    flow::sidebar::tell_sidebar(&t);
    Ok(())
}
/// The plan has its yes and the task is done several times at once: its attempts are made now, each queued for a slot
/// of its own. The worktree, the branch and the pane the plan was written in are attempt A's.
fn fan_out(g: &mut Guard, agents: &[String]) -> Res<Task> {
    let t = g.check()?;
    let place: Obj = ["worktree", "branch", "from", "baseSha", "repoFile", "repoRefused", "root", "ownNote"].iter().filter(|k| t.contains_key(**k)).map(|k| (k.to_string(), t.at(k).clone())).collect();
    let l = t.at("link").clone();
    let mut made = vec![];
    for (n, agent) in agents.iter().enumerate() {
        let k = agent_of(agent).kind;
        let mut extra = obj(json!({ "queuedAt": now() + n as i64, "agent": k, "app": k }));
        if n > 0 { extra.put("link", json!({ "socket": l.at("socket") })); }
        else { for (a, b) in &place { extra.insert(a.clone(), b.clone()); } extra.put("startedOnce", true); extra.put("link", json!({ "socket": l.at("socket"), "workspace": l.at("workspace"), "tab": l.at("tab"), "pane": l.at("pane"), "terminal": l.at("terminal"), "missionPane": l.at("missionPane"), "agentPaneCreated": l.at("agentPaneCreated"), "ownsWorkspace": l.b("ownsWorkspace"), "reviewer": null })); }
        made.push(create_attempt(&t, LETTERS[n], agent, extra)?);
    }
    let t = g.set(|t| {
        t.put("stage", "attempts"); t.put("msg", ""); t.put("since", now()); t.put("app", Value::Null); t.del("kind"); t.put("turnOpen", false); t.put("expectedWake", Value::Null); t.put("watch", Value::Null);
        t.put("attempts", made.iter().map(|c| json!({ "id": c.at("id"), "letter": c.at("letter"), "agent": c.at("settings").at("worker.agent") })).collect::<Vec<_>>());
        t.put("picked", Value::Null); t.put("comparison", Value::Null); t.del("as");
        let parent = std::path::Path::new(t.s("worktree")).parent().map(|p| p.display().to_string()).unwrap_or_default();
        t.put("spaceDir", parent);
        let l = t.at("link").clone();
        t.put("link", json!({ "socket": l.at("socket"), "workspace": l.at("workspace"), "missionPane": l.at("missionPane"), "agentPaneCreated": l.at("agentPaneCreated"), "ownsWorkspace": l.b("ownsWorkspace"), "attemptAnchorPane": l.at("pane") }));
        let p = t.obj_mut("plan"); p.put("built", now()); p.put("locked", false); p.del("ignored");
        for k in FORGET.iter().chain(["worktree", "branch", "from", "baseSha", "startedOnce"].iter()) { t.del(k); }
    })?;
    flow::sidebar::tell_sidebar(&t);
    Ok(t)
}

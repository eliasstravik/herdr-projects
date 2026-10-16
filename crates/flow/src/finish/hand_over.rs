//! The working agent's hand-over (`worker.fallback`, docs/fallback.md): the agent on a task cannot go on, and the next
//! one on the list takes the task over in the same worktree and the same pane, with the task, what was done so far and
//! the open findings. Nobody extra runs until then. With the list used up nothing happens here, and the task waits for
//! you as it did before there was a list.
use crate::agents::{installed::installed, kinds::kind};
use crate::clock::now;
use crate::delegate::{first_prompt, start_agent};
use crate::err::Res;
use crate::flow;
use crate::gate;
use crate::json::*;
use crate::runner::{is_aborted, Guard};
use crate::store::handovers::{handover_line, log_handover, next_fallback, nobody_left, on_now};
use crate::store::repos::{cwd_of, git_out};
use crate::store::settings::lookup::work;
use crate::store::settings::values::{agent_of, list_of};
use crate::store::tasks::{one_line, Task};

/// Can this agent be started here, as the role's access says? "" or why not, for the task's note.
pub fn usable(agent: &str, access: &str) -> String {
    let k = agent_of(agent).kind;
    if !installed(&k) { return format!("{k} is not installed here"); }
    if access != "full" && !(access == "ask" && kind(&k).map(|a| a.access_ask.is_some()).unwrap_or(false)) { return format!("{k} cannot ask before it acts, and the access is \"{access}\""); }
    String::new()
}
/// What the one before it left: its commits on the branch, and whether changes lie uncommitted.
fn done_so_far(t: &Task) -> String {
    if !t.b("worktree") { return String::new(); }
    let tree = cwd_of(t);
    let base = if t.b("baseSha") { t.t("baseSha") } else if t.b("from") { t.t("from") } else { "HEAD".into() };
    let commits: Vec<String> = git_out(&tree, &["log", "--oneline", "--no-decorate", "-20", &format!("{base}..HEAD")]).unwrap_or_default().lines().filter(|l| !l.is_empty()).map(String::from).collect();
    let loose = flow::git::dirty(&tree).lines().filter(|l| !l.is_empty()).count();
    [if commits.is_empty() { "It committed nothing on this branch yet.".to_string() } else { format!("Its commits on this branch so far, newest first:\n{}", commits.iter().map(|c| format!("- {c}")).collect::<Vec<_>>().join("\n")) },
        if loose > 0 { format!("{loose} file{} changes that are not committed yet.", if loose == 1 { " has" } else { "s have" }) } else { String::new() }]
        .into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n")
}
/// The findings that are still open: the last review's, when it failed the gate and the work has not passed one since.
fn open_findings(t: &Task) -> String {
    let rv = t.at("review");
    if rv.is_object() && !gate::passes(rv) && gate::sendable(rv) { format!("The last review of this work failed ({}), and what it found is still open:\n\n{}", gate::result(rv, "merge"), gate::findings_text(rv, false)) } else { String::new() }
}
/// All the next agent is given: the task as a first agent gets it, then that it takes over, from whom and why, what was
/// done, the last words of the one before it, and what is open.
pub fn handover_prompt(t: &Task, from: &str, why: &str, findings: &str) -> String {
    let place = if t.b("worktree") { format!("in the same folder, on the same branch ({})", t.s("branch")) } else { "in the same folder".into() };
    [first_prompt(t),
        format!("You are taking this task over from another agent ({from}), which could not go on: {why}. You work {place}: everything it did is there. Look at it first{} and carry on from where it stands. Do not start over, and do not undo its work without a reason.", if t.b("worktree") { " (git log, git status, git diff)" } else { "" }),
        done_so_far(t),
        if t.b("said") { format!("Its last message was:\n{}", t.s("said")) } else { String::new() },
        if findings.is_empty() { open_findings(t) } else { findings.to_string() },
        "Finish the task as its rules say.".into(),
    ].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n")
}
/// What a hand-over changes on the task, put back whole when the next agent cannot be started.
const KEPT: [&str; 23] = ["agent", "app", "on", "tried", "session", "transcript", "handled", "agentVersion", "hooked", "model", "usage", "failure", "waiting", "cutRetry", "checkFails", "reviewRounds", "kind", "watch", "review", "mergeIntent", "requireApproval", "fallbacks", "handovers"];
/// Hand the task to the next agent on `worker.fallback`, read now. `why`: why the one on it cannot go on, in plain
/// words. `findings`: what is open, when the caller knows it better than the saved review. True when another agent is
/// on it; false when nobody took over, and the caller goes on as it always did.
pub fn hand_over(id: &str, g: &mut Guard, why: &str, findings: &str) -> Res<bool> { hand_over_to(id, g, why, findings, None) }
/// The same, with `first` tried before the list: the program on it again on its own default model, when the model it
/// was given was refused (finish/model_refused.rs).
pub fn hand_over_to(id: &str, g: &mut Guard, why: &str, findings: &str, first: Option<&str>) -> Res<bool> {
    let mut t = g.check()?;
    let w = work(Some(&t));
    let mut list = list_of(w.at("worker.fallback"));
    if let Some(f) = first { list.insert(0, f.to_string()); }
    if list.is_empty() || !t.at("link").b("pane") { return Ok(false); }
    let api = flow::task::h(&t);
    let from = on_now(&t, &w.t("worker.agent"));
    let mut tried: Vec<String> = strings(t.at("tried"));
    if !tried.contains(&from) { tried.push(from.clone()); }
    let mut skipped: Vec<String> = vec![];
    let access = w.t("worker.access");
    let note = |g: &mut Guard, words: String| -> Res<()> { match g.set(|t| t.obj_mut("fallbacks").put("worker", words.clone())) { Ok(_) => Ok(()), Err(e) if is_aborted(&e) => Err(e), Err(_) => Ok(()) } };
    loop {
        let (next, why_not) = next_fallback(&list, &from, &tried, |a| usable(a, &access));
        for s in why_not { if !skipped.contains(&s) { skipped.push(s); } }
        if next.is_empty() { note(g, nobody_left("worker.fallback", &skipped))?; return Ok(false); }
        tried.push(next.clone());
        // The program that cannot go on is ended, so the pane is at its shell for the next one.
        let pane = t.at("link").t("pane");
        if api.pane_get(&pane).is_none() { note(g, "worker.fallback: nobody took over, the agent's terminal is gone".into())?; return Ok(false); }
        if api.agent_get(&pane).ok().flatten().is_some() && !api.pane_quit(&pane, 6000) { note(g, format!("worker.fallback: nobody took over, {} could not be ended", agent_of(&from).kind))?; return Ok(false); }
        let before: Obj = KEPT.iter().map(|k| (k.to_string(), t.at(k).clone())).collect();
        let k = agent_of(&next).kind;
        let mut entry = Value::Null;
        t = g.set(|t| {
            entry = log_handover(t, "worker", &from, &next, why);
            t.put("on", next.clone()); t.put("tried", tried.clone()); t.put("agent", k.clone()); t.put("app", k.clone());
            t.put("session", Value::Null); t.put("transcript", Value::Null);
            let h = t.obj_mut("handled"); h.put("msg", Value::Null); h.put("pos", 0);
            t.put("model", Value::Null); t.put("usage", Value::Null);
            // The work goes on under a new agent: what the one before it used up is not counted against this one.
            for x in ["failure", "waiting", "cutRetry", "watch"] { t.put(x, Value::Null); }
            t.put("checkFails", 0); t.put("reviewRounds", 0); t.del("kind");
            if t.at("review").is_object() && !t.at("review").b("stale") { t.obj_mut("review").put("stale", true); }
            if t.at("mergeIntent").is_object() { if !t.at("mergeIntent").b("auto") { t.put("requireApproval", true); } t.put("mergeIntent", Value::Null); }
            t.obj_mut("fallbacks").del("worker");
        })?;
        let shown_from = if agent_of(&from).kind == k { from.clone() } else { agent_of(&from).kind };
        if let Err(e) = start_agent(id, g, &handover_prompt(&t, &shown_from, why, findings), None) {
            if is_aborted(&e) { return Err(e); }
            // It could not be started: the task is the one before it's again, and the next on the list is asked.
            skipped.push(format!("{next} could not be started ({})", one_line(&e.msg, 120)));
            t = g.set(|t| { for (k, v) in before { t.insert(k, v); } })?;
            continue;
        }
        // It is on the task. The row says so until the Watcher has a line of it, and herdr's sidebar names it.
        t = g.set(|t| { t.put("msg", handover_line(&entry)); t.put("since", now()); t.put("workAt", now()); })?;
        flow::sidebar::tell_sidebar(&t);
        return Ok(true);
    }
}

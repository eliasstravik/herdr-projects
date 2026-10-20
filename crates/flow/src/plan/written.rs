//! The plan's turn ended: the board makes sure nothing was changed, takes the plan, and hands it to whoever approves it.
//! Every turn that ends while the plan step is open is the plan's, so nothing is committed, checked, pushed or reviewed
//! as work before the yes. Who approves is `plan.approve` as it reads now: you (the task waits as "plan ready"), the
//! reviewer (its look decides; a review that cannot start or gives no verdict leaves the plan for your yes), or nobody
//! any more (the setting was taken off meanwhile: the build goes on).
use super::build::{start_build, start_build_by};
use super::review::{points_of, review_plan};
use super::untouched::{held_failure, put_back, undone_words};
use super::{approved, back_words_from, plan_file, plan_link, plan_mark, read_plan, wanted, write_plan};
use crate::clock::now;
use crate::err::Res;
use crate::finish::shared::{fail, FailHow};
use crate::flow;
use crate::gate;
use crate::json::*;
use crate::runner::Guard;
use crate::store::failures::bound_detail;
use crate::store::settings::lookup::{setting, work};
use crate::store::tasks::{one_line, Task};

/// The plan's first line of words, for the row: a headline's marks taken off.
pub fn first_words(text: &str) -> String {
    let line = text.split('\n').map(|l| l.trim_start_matches(|c: char| "#>*-".contains(c) || c.is_whitespace()).trim()).find(|l| !l.is_empty()).unwrap_or("");
    one_line(&line.chars().take(200).collect::<String>(), 2048)
}
pub fn plan_written(id: &str, g: &mut Guard, said: &str) -> Res<()> {
    let mut t = g.check()?;
    if approved(&t) { return start_build(id, g); } // it has its yes, and the build's start went wrong: that start, again
    // 1. Nothing changed. In the task's own worktree the board puts it back itself; in your checkout it goes back to the
    //    agent that changed it, `review.rounds` times, and then waits for you.
    let Some(undone) = put_back(&t) else {
        let rounds = t.at("plan").i("undo");
        let failure = held_failure(&t);
        if rounds >= setting("review.rounds", Some(&t)).as_i64().unwrap_or(3) { fail(g, failure, FailHow::default())?; return Ok(()); }
        let mut p = t.o("plan").cloned().unwrap_or_default(); p.put("undo", rounds + 1);
        flow::send::send_own(id, &format!("{}, so the plan is not taken yet.\n\n{}\n\n{}", failure.s("title"), bound_detail(failure.s("detail")), failure.s("next")), "plan", false, obj(json!({ "plan": p })), 0)?;
        return Ok(());
    };
    // 2. The plan: the file its writer saved (or its program's plan mode handed over); failing that, its last message.
    let mut text = read_plan(&t);
    let old = !text.is_empty() && t.at("plan").b("was") && plan_mark(&t) == t.at("plan").s("was");
    if (text.is_empty() || old) && !said.trim().is_empty() { write_plan(&t, said)?; text = read_plan(&t); } else if old { text = String::new(); }
    if text.is_empty() {
        let f = obj(json!({ "stage": "plan", "auth": false, "agentCanFix": true, "title": "No plan was written", "msg": "Stopped without a plan",
            "detail": format!("The agent stopped, and there is no {}plan in {}.", if t.at("plan").b("was") { "new " } else { "" }, plan_file(&t).display()), "next": format!("Write the plan and save it as {}, then stop.", plan_file(&t).display()) }));
        fail(g, f, FailHow::default())?;
        return Ok(());
    }
    let link = plan_link(&t, || crate::review_page::artifact_base(t.s("id")));
    let words = first_words(&text);
    t = g.set(|t| {
        let p = t.obj_mut("plan");
        p.del("was"); p.del("reviewFailed"); p.put("at", now()); p.put("words", words.clone()); p.put("file", "plan.md"); p.put("link", link.clone());
        if !undone.is_empty() { p.put("undone", json!({ "list": undone.iter().take(30).collect::<Vec<_>>(), "n": undone.len(), "at": now() })); }
        t.put("failure", Value::Null);
    })?;
    // 3. Who says yes, as the setting reads now.
    let by = wanted(&t);
    if by.is_empty() { return start_build_by(id, g, "nobody"); }
    if by == "reviewer" {
        let ch = work(Some(&t));
        let r = review_plan(id, g, &ch)?;
        if r.aborted { return Ok(()); }
        // The reviewer could not start, or gave no verdict: the plan is yours to judge, with the reason on its row. Your
        // yes builds it; retry has it reviewed again.
        let Some(rv) = r.review else {
            let why = if r.auth { "the reviewer needs a login".to_string() } else { one_line(&r.error, 400) };
            g.set(|t| t.obj_mut("plan").put("reviewFailed", json!({ "why": why, "auth": r.auth, "at": now() })))?;
            ready(g, &undone)?;
            return Ok(());
        };
        let rv = Value::Object(rv);
        t = g.set(|t| t.obj_mut("plan").put("review", rv.clone()))?;
        if gate::passes(&rv) { return start_build_by(id, g, "reviewer"); }
        let rounds = t.at("plan").i("rounds");
        // It goes back to its writer by itself, as failed work does; after that many times it is yours to judge.
        if rounds < ch.i("review.rounds") {
            let mut p = t.o("plan").cloned().unwrap_or_default(); p.put("rounds", rounds + 1); p.put("was", plan_mark(&t));
            flow::send::send_own(id, &back_words_from(&t, &points_of(&rv), "the reviewer"), "plan", false, obj(json!({ "plan": p })), 0)?;
            return Ok(());
        }
    }
    ready(g, &undone)?;
    Ok(())
}
/// "Plan ready": the task waits for your yes, with the plan's first words on its row.
pub fn ready(g: &mut Guard, undone: &[String]) -> Res<Task> {
    let t = g.set(|t| {
        let rv = t.at("plan").at("review").clone();
        t.put("stage", "plan"); t.put("kind", "permission"); t.put("turnOpen", false); t.put("expectedWake", Value::Null); t.put("watch", Value::Null);
        let words = t.at("plan").t("words");
        let failed = t.at("plan").at("reviewFailed").t("why");
        let msg = if !failed.is_empty() { format!("The plan's review could not run ({failed}): yours to judge") }
            else if rv.is_object() && !gate::passes(&rv) { format!("The reviewer did not pass the plan ({}): yours to judge", gate::tally(&rv)) }
            else if !undone.is_empty() { format!("{words} · {}", undone_words(undone)) } else if !words.is_empty() { words } else { "The plan is written".into() };
        t.put("msg", msg);
    })?;
    flow::sidebar::tell_sidebar(&t);
    Ok(t)
}

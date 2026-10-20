//! A plan that waits for you ("plan ready"): the yes that starts the build, the reply that sends the plan back, and
//! another look by the reviewer where its review could not run. The yes is also the way out of a plan step that failed
//! after its plan was written: it builds that plan.
use super::shared::{say, Said};
use crate::clock::now;
use crate::err::Res;
use crate::flow;
use crate::json::*;
use crate::plan;
use crate::runner::{runner_alive, spawn_runner};
use crate::store::tasks::{assert_unarchived, find, update, Change, Task};

pub fn awaits_plan(t: &Task) -> bool { t.s("stage") == "plan" && plan::open(t) }
/// A plan your yes builds: one that waits for it, or one whose step failed once it was written (never the plan you sent
/// back, come back unchanged).
pub fn approvable(t: &Task) -> bool {
    let sent_back = t.at("plan").b("was") && plan::plan_mark(t) == t.at("plan").s("was");
    awaits_plan(t) || (t.s("stage") == "failed" && plan::open(t) && t.at("failure").s("stage") == "plan" && !plan::read_plan(t).is_empty() && !sent_back)
}
/// Yes: the plan is approved as it stands, and the build starts (plan/build.rs, in a runner: a program is started).
pub fn approve_plan(id: &str) -> Res<Said> {
    let cur = find(id)?;
    assert_unarchived(&cur)?;
    flow::live::assert_live(&cur)?;
    let t = update(cur.s("id"), |t| {
        if !approvable(t) { crate::fail!("{} has no plan that waits for a yes.", t.s("id")); }
        if runner_alive(t) { crate::fail!("{} is busy. Try again in a moment.", t.s("id")); }
        let p = t.obj_mut("plan"); p.put("approvedAt", now()); p.put("approvedBy", "you");
        t.put("stage", "starting"); t.put("msg", "Starting the build"); t.del("kind"); t.put("since", now()); t.put("failure", Value::Null);
        Ok(Change::Write)
    })?;
    flow::sidebar::tell_sidebar(&t);
    spawn_runner("delegate", t.s("id"), &[]);
    let msg = format!("{}: plan approved, the build starts", t.s("id"));
    Ok(say(t, msg))
}
/// Retry on a plan whose review could not run: the reviewer looks at it again (finish/pipeline.rs, in a runner).
pub fn review_again(id: &str) -> Res<Said> {
    let t = update(id, |t| {
        if !awaits_plan(t) || !t.at("plan").b("reviewFailed") { crate::fail!("{} has nothing to retry.", t.s("id")); }
        if runner_alive(t) { crate::fail!("{} is busy. Try again in a moment.", t.s("id")); }
        t.put("stage", "checking"); t.put("msg", "The reviewer checks the plan"); t.del("kind"); t.put("since", now());
        Ok(Change::Write)
    })?;
    flow::sidebar::tell_sidebar(&t);
    spawn_runner("pipeline", t.s("id"), &[]);
    let msg = format!("{}: the plan is reviewed again", t.s("id"));
    Ok(say(t, msg))
}
/// The words a reply to a waiting plan is sent as: the plan goes back to its writer with them, and is written again.
pub fn plan_reply(t: &Task, words: &str) -> String { plan::back_words(t, words) }
/// What a reply does to the step: the reviewer's look at the older plan says nothing about the next one.
pub fn sent_back(t: &mut Task) {
    let mark = plan::plan_mark(t);
    let p = t.obj_mut("plan");
    p.put("review", Value::Null); p.del("reviewFailed"); p.put("sentBack", now()); p.put("was", mark); let b = p.i("backs") + 1; p.put("backs", b);
}

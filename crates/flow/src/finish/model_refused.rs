//! The working agent's program refused its model at a turn's end (docs/models.md): "gpt-6.1 can't be used with your
//! ChatGPT sign-in". A model the board gave it is dropped once: the same program starts again on its own default model,
//! in the same worktree, and the task says so. Otherwise the task waits for you with the reason in plain words and the
//! next step, and the program is ended, so that a retry starts it again on the model as it is set then. Never left in
//! the agent with only its error.
use super::hand_over::hand_over_to;
use super::shared::{fail, failure, FailHow};
use crate::agents::model_refused::{own_default, say, try_instead, Refused};
use crate::board::keys::key_name;
use crate::err::Res;
use crate::flow;
use crate::json::*;
use crate::plan;
use crate::runner::Guard;
use crate::store::handovers::on_now;
use crate::store::settings::lookup::work;
use crate::store::settings::values::agent_of;
use crate::store::tasks::{one_line, Task};

/// The agent on the task now, as a setting writes it (`codex/gpt-6.1`): the plan's writer while it writes the plan.
fn on_it(t: &Task) -> String {
    if plan::planner(t) { let wr = t.at("plan").at("writer"); return if wr.b("model") { format!("{}/{}", wr.s("kind"), wr.s("model")) } else { wr.t("kind") }; }
    on_now(t, &work(Some(t)).t("worker.agent"))
}
/// The program again, on its own default model, when the board gave it the model that was refused. True when it is
/// on the task again.
pub fn on_default(id: &str, g: &mut Guard, t: &Task, r: &Refused) -> Res<bool> {
    let a = agent_of(&on_it(t));
    if plan::planner(t) || a.model.is_empty() || a.kind != t.s("agent") { return Ok(false); }
    if !hand_over_to(id, g, &format!("{} (now on its own default model)", r.plain), "", Some(&a.kind))? { return Ok(false); }
    let msg = format!("{}: started again on {}'s own default model", r.plain, a.kind);
    let t = g.set(|t| t.put("msg", msg.clone()))?;
    flow::sidebar::tell_sidebar(&t);
    Ok(true)
}
/// Nobody took over: the task fails with the reason and what to do. `error`: the program's own words.
pub fn fail_refused(g: &mut Guard, t: &Task, r: &Refused, error: &str) -> Res<()> {
    let id = t.t("id");
    let k = t.t("agent");
    let on = on_it(t);
    let given = !agent_of(&on).model.is_empty() && agent_of(&on).kind == k;
    let setting = if plan::planner(t) { "plan.agent" } else { "worker.agent" };
    let x = try_instead(&k, &r.model);
    let try_it = if x.is_empty() { "<model>".to_string() } else { x.clone() };
    let list = format!("`herdr-projects models {k}` lists the ones {} can use", if r.signin.is_empty() { "it".to_string() } else { format!("your {}", r.signin) });
    let whence = if given { format!("The board gave {k} the model {}.", agent_of(&on).model) } else { format!("The board gave {k} no model: that one is {}.", own_default(&k)) };
    let retry = key_name("yes");
    let next = if given {
        format!("Choose one it can use for this task: herdr-projects set {setting} {k}/{try_it} --task {id} ({list}), or leave the model out (herdr-projects set {setting} {k} --task {id}). Then press {retry} (retry): the agent starts again on it.")
    } else {
        format!("Change {} to one it can use (try {try_it}; {list}), or choose one for this task: herdr-projects set {setting} {k}/{try_it} --task {id}. Then press {retry} (retry): the agent starts again on it.", own_default(&k))
    };
    let mut f = failure("agent", "Model can't be used", &format!("{}. {whence}\n\nIts error:\n{error}", r.plain), &next);
    f.put("auth", false); f.put("agentCanFix", false); f.put("msg", format!("Agent stopped: {}", one_line(&say(&k, r), 200)));
    // The program is ended: a retry or a reply starts it again, on the model the settings name then.
    let api = flow::task::h(t);
    let pane = t.at("link").t("pane");
    if !pane.is_empty() && api.agent_get(&pane).ok().flatten().is_some() { let _ = api.pane_quit(&pane, 6000); }
    let same = !plan::planner(t) && agent_of(&on).kind == agent_of(&work(Some(t)).t("worker.agent")).kind;
    fail(g, f, FailHow { patch: if t.b("on") && same { obj(json!({ "on": null })) } else { Obj::new() }, ..Default::default() })?;
    Ok(())
}

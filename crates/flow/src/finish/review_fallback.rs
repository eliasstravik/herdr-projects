//! The reviewer's hand-over (`reviewer.fallback`, docs/fallback.md): the reviewer cannot review (its provider is still
//! down after the board's own tries, it is out of allowance or logged out, or its program is not installed here), and
//! the next one on the list reviews in its place: the same review, started again, in a tab of its own. It reviews the
//! later rounds of the task too, until you step in. A reviewer that reviewed and found faults is never replaced. One
//! whose model was refused reviews again first on its program's own default model (docs/models.md).
use super::hand_over::usable;
use crate::agents::installed::installed;
use crate::agents::model_refused;
use crate::err::Res;
use crate::json::*;
use crate::runner::Guard;
use crate::store::handovers::{handover_line, log_handover, next_fallback, nobody_left, why_of};
use crate::store::settings::values::{agent_of, list_of};

/// Who reviews now: the one that took over earlier on this task, else `reviewer.agent`; and where that program is not
/// installed here, the first on the list that is. (agent, stand_in): `stand_in` names the program that is missing.
pub fn reviewer_now(t: &Obj, ch: &Obj) -> (String, String) {
    let said = if t.b("reviewerOn") { t.t("reviewerOn") } else { ch.t("reviewer.agent") };
    let k = agent_of(&said).kind;
    if installed(&k) { return (said, String::new()); }
    let (next, _) = next_fallback(&list_of(ch.at("reviewer.fallback")), &said, &[], |a| usable(a, ch.s("reviewer.access")));
    if next.is_empty() { (said, String::new()) } else { (next, k) }
}
/// What cut a reviewer off: its provider's error.
pub struct Cut { pub passing: bool, pub reason: String, pub error: String }
/// The reviewer `from` was cut off for good. The next on the list, written on the task, or "" when nobody is left.
pub fn next_reviewer(g: &mut Guard, ch: &Obj, from: &str, cut: &Cut, auth: bool, tries: i64, tried: &[String]) -> Res<String> {
    // Its model was refused: the same program reviews again on its own default model, once, before the list does.
    let a = agent_of(from);
    if let Some(r) = model_refused::read(&a.kind, &cut.error).filter(|_| !auth && !cut.passing && !a.model.is_empty() && !tried.contains(&a.kind) && usable(&a.kind, ch.s("reviewer.access")).is_empty()) {
        let why = format!("{} (now on its own default model)", r.plain);
        g.set(|t| { log_handover(t, "reviewer", from, &a.kind, &why); t.put("reviewerOn", a.kind.clone()); t.put("msg", format!("{}: the review starts again on {}'s own default model", r.plain, a.kind)); t.put("waiting", Value::Null); })?;
        return Ok(a.kind);
    }
    let list = list_of(ch.at("reviewer.fallback"));
    if list.is_empty() { return Ok(String::new()); }
    let (next, skipped) = next_fallback(&list, from, tried, |a| usable(a, ch.s("reviewer.access")));
    if next.is_empty() { g.set(|t| t.obj_mut("fallbacks").put("reviewerLeft", nobody_left("reviewer.fallback", &skipped)))?; return Ok(String::new()); }
    let why = why_of(&cut.error, auth, cut.passing, &cut.reason, tries, crate::clock::now());
    g.set(|t| { let h = log_handover(t, "reviewer", from, &next, &why); t.put("reviewerOn", next.clone()); t.put("msg", handover_line(&h)); t.put("waiting", Value::Null); t.obj_mut("fallbacks").del("reviewerLeft"); })?;
    Ok(next)
}

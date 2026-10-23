//! The reviewer: a visible herdr agent in a tab of its own in the task's space. Starting it, waiting for it, trying
//! again. A reviewer whose turn ended in a provider error did not review anything: that is not a round. When the error
//! is one that passes by itself, the board waits and starts the review again, a few times with growing waits; then, or at
//! once for an error that waiting cannot fix, it fails with the provider's own words. A reviewer that ended without a
//! verdict is started once more by itself before the review counts as failed.
use super::review_fallback::{next_reviewer, Cut};
use super::review_once::{review_once, Plan};
use crate::err::Res;
use crate::flow;
use crate::json::*;
use crate::limits::reached::{stop_at, tokens_over, StopHow};
use crate::runner::{is_aborted, Guard};
use crate::store::retries::{gave_up, retry_waits};
use crate::store::tasks::{get, one_line, update_wait, Change};

/// Why a reviewer stopped for good; for a model it could not use, the reason in plain words and what to do.
fn refused_words(by: &str, error: &str) -> String {
    let a = crate::store::settings::values::agent_of(by);
    match crate::agents::model_refused::read(&a.kind, error) {
        Some(r) => format!("{}. Choose a model it can use in reviewer.agent (`herdr-projects models {}` lists them), or leave the model out. Its error: {}", crate::agents::model_refused::say(&a.kind, &r), a.kind, error.trim_end_matches(|c: char| c == '.' || c.is_whitespace())),
        None => format!("the reviewer stopped: {error}"),
    }
}
pub fn close_reviewer(id: &str) {
    let Some(t) = get(id) else { return };
    if !t.at("link").at("reviewer").b("pane") { return; }
    flow::tabs::close_reviewer_tab(&flow::task::h(&t), t.at("link").at("reviewer"));
    let _ = update_wait(id, 10000, |t| { if t.at("link").is_object() { t.obj_mut("link").put("reviewer", Value::Null); } Ok(Change::Write) });
}
/// What one review gave: a verdict, or an error; `cut` when its provider ended it; `aborted` when the world moved on;
/// `ended` when the reviewer finished (or exited) without a verdict; `screen`, the reviewer's last lines when it gave none.
#[derive(Default)]
pub struct Reviewed { pub review: Option<Obj>, pub error: String, pub auth: bool, pub cut: bool, pub aborted: bool, pub by: String, pub cut_by: Option<Cut>, pub ended: bool, pub screen: String }
pub fn run_review(id: &str, g: &mut Guard, text: &str, change: Option<&Value>, to_land: bool, ch: &Obj, check: &str) -> Res<Reviewed> { run_review_with(id, g, text, change, to_land, ch, check, None) }
#[allow(clippy::too_many_arguments)]
pub fn run_review_with(id: &str, g: &mut Guard, text: &str, change: Option<&Value>, to_land: bool, ch: &Obj, check: &str, plan: Option<&Plan>) -> Res<Reviewed> {
    let waits = retry_waits();
    let mut tried: Vec<String> = vec![];
    let mut tries: i64 = 0;
    let mut again = true;
    loop {
        let r = review_once(id, g, text, change, to_land, ch, check, plan)?;
        // It ended without a verdict: once more, as the same review, before that counts as a failed one.
        if r.ended && again {
            again = false;
            match g.set(|t| { if plan.is_none() { t.put("reviewRound", (t.i("reviewRound") - 1).max(0)); } t.put("msg", "The reviewer ended without a verdict: reviewing again"); }) {
                Ok(_) => continue,
                Err(e) if is_aborted(&e) => return Ok(Reviewed { aborted: true, ..Default::default() }),
                Err(e) => return Err(e),
            }
        }
        let Some(cut) = r.cut_by.as_ref() else { return Ok(r) };
        // At the task's token limit a cut-off review is not started again by itself.
        if let Some((limit, used)) = tokens_over(id, "") { stop_at(id, "tokens", &limit, used, StopHow { role: "reviewer".into(), ..Default::default() }); return Ok(Reviewed { aborted: true, ..Default::default() }); }
        // It cannot review: the next one on `reviewer.fallback` does, as the same review, with the tries counted afresh.
        if !cut.passing || tries >= waits.len() as i64 {
            tried.push(r.by.clone());
            match next_reviewer(g, ch, &r.by, cut, r.auth, tries + 1, &tried) {
                Ok(next) if !next.is_empty() => { tries = 0; continue; }
                Ok(_) => {}
                Err(e) if is_aborted(&e) => return Ok(Reviewed { aborted: true, ..Default::default() }),
                Err(e) => return Err(e),
            }
        }
        if !cut.passing { return Ok(Reviewed { error: refused_words(&r.by, &cut.error), auth: r.auth, cut: true, ..Default::default() }); }
        if tries >= waits.len() as i64 { return Ok(Reviewed { error: format!("{}. Its error: {}", gave_up("reviewer", &cut.reason, tries + 1), cut.error.trim_end_matches(|c: char| c == '.' || c.is_whitespace())), cut: true, ..Default::default() }); }
        let until = crate::clock::now() + waits[tries as usize];
        let w = json!({ "who": "reviewer", "reason": cut.reason, "error": one_line(&cut.error, 400), "until": until, "tries": tries + 1 });
        let waited = (|| -> Res<()> {
            g.set(|t| { t.put("msg", "Under review"); t.put("watch", Value::Null); t.put("waiting", w.clone()); })?;
            while crate::clock::now() < until { std::thread::sleep(std::time::Duration::from_millis((until - crate::clock::now()).clamp(1, 1000) as u64)); g.check()?; }
            Ok(())
        })();
        if let Err(e) = waited {
            if !is_aborted(&e) { return Err(e); }
            let _ = update_wait(id, 10000, |t| { if t.at("waiting").s("who") != "reviewer" { return Ok(Change::Keep); } t.put("waiting", Value::Null); Ok(Change::Write) });
            return Ok(Reviewed { aborted: true, ..Default::default() });
        }
        tries += 1;
    }
}

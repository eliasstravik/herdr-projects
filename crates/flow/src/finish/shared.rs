//! The finish line's shared pieces: the approval question's words, and marking a task as failed.
use crate::err::Res;
use crate::flow;
use crate::gate;
use crate::json::*;
use crate::runner::Guard;
use crate::store::failures::failure_patch;
use crate::store::reviews::{current_review, kept_words, review_history};
use crate::store::tasks::Task;

pub fn ms(name: &str, fallback: i64) -> i64 { std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(fallback) }
/// When a review that starts now must have its answer (`review.timeout`, from the task's settings `ch`); None: no limit.
pub fn review_until(ch: &Obj) -> Option<i64> { let l = crate::store::settings::values::ms(ch.s("review.timeout")); (l > 0).then(|| crate::clock::now() + l) }
/// What a review that ran out of time failed with: `what` timed out, and the limit it had.
pub fn timed_out(what: &str, ch: &Obj) -> String { format!("{what} timed out after {} (review.timeout)", ch.s("review.timeout")) }
pub fn last_lines(text: &str, n: usize) -> String {
    let all: Vec<&str> = text.split('\n').map(|l| l.trim_end()).filter(|l| !l.trim().is_empty()).collect();
    all[all.len().saturating_sub(n)..].join("\n")
}
pub fn first_line(text: &str) -> String { text.split('\n').find(|l| !l.trim().is_empty()).map(|l| l.trim().to_string()).unwrap_or_default() }
pub fn approval_message(t: &Obj) -> String { approval_message_with(t, "") }
pub fn approval_message_with(t: &Obj, note: &str) -> String {
    let rv = current_review(t);
    let kept = match rv { Some(rv) if rv.b("keptAt") && rv.at("kept").is_object() => {
        let w = kept_words(rv.at("kept"), review_history(t).last().map(|e| e.i("round")).unwrap_or(0));
        format!(" {}{}.", w[..1].to_uppercase(), &w[1..])
    } _ => String::new() };
    let summary = first_line(t.s("summary"));
    let held = if crate::autopilot::care::held_for_you(t) { " It changes the repo's .herdr-projects.json, so it waits for your own yes (autopilot.repoFileWaits)." } else { "" };
    format!("Approve to merge?{}{kept} {}{held}{note}", rv.map(|r| format!(" {}", gate::sentence(r, "merge"))).unwrap_or_default(), if summary.is_empty() { "The work is finished.".into() } else { summary })
}
#[derive(Default)]
pub struct FailHow { pub keep_intent: bool, pub patch: Obj }
/// Mark the task as failed, with the failure saved beside it.
pub fn fail(g: &mut Guard, failure: Obj, how: FailHow) -> Res<Task> {
    let t = g.set(|t| {
        t.put("stage", "failed");
        t.put("msg", if failure.b("msg") { failure.t("msg") } else { failure.t("title") });
        if !how.keep_intent { t.put("mergeIntent", Value::Null); }
        for (k, v) in failure_patch(&failure) { t.insert(k, v); }
        for (k, v) in how.patch { t.insert(k, v); }
    })?;
    flow::sidebar::tell_sidebar(&t);
    Ok(t)
}
/// A failure from its parts.
pub fn failure(stage: &str, title: &str, detail: &str, next: &str) -> Obj { obj(json!({ "stage": stage, "title": title, "detail": detail, "next": next })) }

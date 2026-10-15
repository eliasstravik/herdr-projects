//! What a row says and counts, in words: its second line, and its percent, minutes left, review round, pull request
//! and time. No drawing: herdr's sidebar says the same with these (flow/sidebar_entry.rs), and the board's rows draw
//! them (board/describe/row.rs).
use super::ink::{Ink, Inked};
use super::queue::tag as queue_tag;
use super::review_words::{result_ink, review_note, review_tag_inked};
use super::states::{failed_review, failure_for, in_review, needs_you, status_ink};
use super::words::{age, bar, clean, left, pr_tag};
use crate::gate;
use crate::json::*;
use crate::status_words::review_tally;
use crate::store::reviews::current_review;
use crate::store::retries::waiting_of;

/// What a row that waits for you says when its task has no words of its own: only what it waits for, again.
pub fn bare(kind: &str) -> &'static str { match kind { "question" => "Has a question", "permission" => "Asks for your OK", "auth" => "Needs a login", "error" => "Failed", _ => "" } }
pub const BARE_ALL: [&str; 4] = ["Has a question", "Asks for your OK", "Needs a login", "Failed"];

/// What a row has for each column on the right. A cell the row does not have is empty.
#[derive(Clone, Debug, Default)]
pub struct Fields { pub bar: Inked, pub percent: Inked, pub eta: Inked, pub review: Inked, pub pr: Inked, pub elapsed: String, pub ago: bool }
pub fn row_fields(r: &Obj, now: i64) -> Fields {
    let c = status_ink(r);
    let live = r.s("state") != "done";
    let has = live && r.at("progress").is_number();
    let p = r.f("progress");
    Fields {
        bar: if has { Inked::of(bar(p, 8), c) } else { Inked::new() },
        // A task that waits for a free slot has its place in the queue where a task at work has how far it is.
        percent: if has { Inked::of(format!("{}%", p as i64), c) } else if live && !queue_tag(r).is_empty() { Inked::of(queue_tag(r), c) } else { Inked::new() },
        eta: if live && r.at("eta").is_number() { Inked::of(format!("{} left", left(r.i("eta"))), c) } else { Inked::new() },
        review: review_tag_inked(r), pr: pr_tag(r),
        elapsed: age(now - if r.b("at") { r.i("at") } else { now }), ago: r.s("state") != "working",
    }
}
/// What a row says: under its name on a live task's row, after its name on any other. Never what its heading says.
pub fn sentence(r: &Obj) -> Inked {
    let words = |s: &str| clean(s).replace("**", "").replace('`', "");
    let quiet = |s: String| Inked::of(s, Ink::Soft);
    let plainly = |s: String| Inked::of(s, Ink::Text);
    // Cut off by its provider, and the board tries again by itself: the row says why and when.
    if waiting_of(r).is_some() { return quiet(words(r.s("msg"))); }
    if let Some(f) = failure_for(r) { if !f.b("sentAt") && (r.b("failure") || r.b("prFailure")) { return quiet(words(if f.b("msg") { f.s("msg") } else { f.s("title") })); } }
    // A chat with its agent changed no code: the review's result, and that it still stands (flow/chat.rs).
    let kept = |t: String| if r.b("chatKept") { [t, crate::flow::chat::KEPT_LINE.to_string()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ") } else { t };
    if failed_review(r) { let t = kept(review_tally(r)); return Inked::of("Review failed", Ink::Red).add(quiet(if t.is_empty() { t } else { format!(" · {t}") })); }
    if in_review(r) {
        let rv = current_review(r);
        if let Some(rv) = rv.filter(|rv| gate::is_tagged(rv)) { let t = kept(gate::tally(rv)); return Inked::of(gate::word(rv), result_ink(rv)).add(quiet(if t.is_empty() { t } else { format!(" · {t}") })); }
        let Some(rv) = rv.filter(|rv| gate::is_int(rv.at("score"))) else { return if r.at("review").b("stale") { quiet("The work changed since its review".into()) } else { Inked::new() } };
        let adv = match rv.at("recommend").as_bool() { Some(true) => " reviewer says yes", Some(false) => " reviewer says no", None => "" };
        return Inked::of(format!("{}/5", rv.t("score")), result_ink(rv)).add(quiet(adv.into()));
    }
    if r.s("stage") == "approval" && r.s("prState") == "failing" && !r.b("mergeFailed") && !r.b("syncBlocked") { return quiet("The pull request's checks fail".into()); }
    if r.s("stage") == "plan" && r.s("state") == "blocked" { let m = words(r.s("msg")); return plainly("Plan ready".into()).add(quiet(if m.is_empty() { m } else { format!(" · {m}") })); }
    let raw = words(&if r.at("review").b("stale") { review_note(r) } else { r.t("msg") });
    let msg = if r.b("plain") || ["working", "needs you", "under review", "idle"].contains(&raw.as_str()) { String::new() } else { raw };
    // Done says nothing more, except why it is done without a pull request (finish/already.rs).
    if r.s("state") == "done" { return if r.s("stage") == "done" && r.b("doneWhy") { quiet(words(r.s("doneWhy"))) } else { Inked::new() }; }
    // Queued after other tasks: in the queue's colour. Only to wait for them, never started: nothing (after.rs).
    if super::queue::waits_after(r) { return Inked::of(super::queue::words(r), Ink::Queue); }
    // Near a limit: how much is used, before what it is doing, so it is never the part cut off.
    if r.at("limits").b("near") && r.s("state") == "working" { return Inked::of(r.at("limits").t("near"), Ink::Peach).add(quiet(if msg.is_empty() { msg } else { format!(" · {msg}") })); }
    if needs_you(r) {
        let text = if !msg.is_empty() { msg.clone() } else if !bare(r.s("kind")).is_empty() { bare(r.s("kind")).into() } else if r.s("state") == "error" { bare("error").into() } else { String::new() };
        return if r.s("state") == "blocked" && !msg.is_empty() { plainly(text) } else { quiet(text) };
    }
    quiet(msg)
}

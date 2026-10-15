//! Where a task stands, as the board sorts it: its group, its colour, whether an agent is on it, whether it waits for
//! you. A row is a task as the board shows it (flow/live.rs: shown): the task's own fields, plus `state`, `kind`,
//! `note`, `progress`, `eta` and the rest that herdr and the Watcher said.
use super::ink::Ink;
use super::queue::queued;
use crate::json::*;
use crate::status_words::{needs, review_failed, step, who, words, QUEUED};
use crate::store::retries::waiting_of;

/// The groups, in the order they need you.
pub const GROUPS: [&str; 5] = ["needs you", "ready to approve", "working", "to do", "done"];
/// herdr's own colour for each group (its status colour), with two of the board's own: finished work that waits for a
/// yes is green, and a task nobody has started is the dim grey of "to do".
pub fn group_ink(g: &str) -> Ink { match g { "needs you" => Ink::Red, "ready to approve" => Ink::Green, "working" => Ink::Yellow, "done" => Ink::Teal, _ => Ink::Todo } }
pub fn is_agent(r: &Obj) -> bool { r.b("agent") && !["todo", "progress"].contains(&r.s("stage")) }
/// Its agent is at rest and the board missed the end of its turn: the row says "Finishing" and stays so. The finish key
/// takes it on with what the agent said last (actions/finish_stuck.rs).
pub fn stuck_finishing(r: &Obj) -> bool { is_agent(r) && !r.b("archivedAt") && r.s("stage") == "working" && r.s("state") == "working" && r.s("note") == "Finishing" }
pub fn failure_for(r: &Obj) -> Option<Obj> { if r.s("stage") == "nosummary" { None } else { crate::actions::finish_line::failure_of(r) } }
pub fn can_send_error(r: &Obj) -> bool {
    if !r.b("host") { return crate::actions::finish_line::can_send_error(r); }
    r.s("stage") == "nosummary" || (r.s("stage") == "failed" && !r.at("failure").b("auth"))
}
/// Finished work that waits for the one yes that merges it.
fn awaits_yes(r: &Obj) -> bool {
    r.s("state") == "blocked" && r.s("kind") == "permission" && r.s("stage") == "approval" && !r.b("mergeFailed") && !r.b("syncBlocked") && !r.b("prFailure") && r.s("prState") != "failing"
}
pub fn in_review(r: &Obj) -> bool { awaits_yes(r) && !review_failed(r) }
/// Work whose last review failed, and nothing else in its way: your call (merge anyway, or send the review back).
pub fn failed_review(r: &Obj) -> bool { awaits_yes(r) && review_failed(r) }
pub fn reviewing(r: &Obj) -> bool { r.s("state") != "done" && (r.s("stage") == "reviewing" || r.s("stage") == "reviewneeds") }
/// The reviewer is at work on it right now.
pub fn under_review(r: &Obj) -> bool { reviewing(r) && r.s("state") == "working" }
const BOARD_STEPS: [&str; 5] = ["starting", "setup", "checking", "pushing", "merging"];
/// The board itself is on it: it starts the agent, sets up, runs the check, pushes or merges, and waits on the way (its
/// check's turn, its turn to merge, GitHub's checks). No agent is on it, so its mark is ◇, never a moon.
pub fn board_step(r: &Obj) -> bool { !r.b("archivedAt") && r.s("state") == "working" && BOARD_STEPS.contains(&r.s("stage")) }
pub fn reviewer_mark(r: &Obj) -> bool { under_review(r) && waiting_of(r).is_none() }
pub fn needs_you(r: &Obj) -> bool { r.s("state") == "blocked" || r.s("state") == "error" }
pub fn reviewer_word(r: &Obj) -> &'static str { if waiting_of(r).is_some() { "reviewer was cut off" } else { "reviewer is working" } }
pub fn group_of(r: &Obj) -> &'static str {
    if in_review(r) { "ready to approve" } else if needs_you(r) { "needs you" } else if r.s("state") == "working" { "working" } else if r.s("state") == "done" { "done" } else { "to do" }
}
/// Where a task stands, in the words of the one list.
pub fn standing(r: &Obj) -> String {
    let archived = r.b("archivedAt");
    if !archived && r.s("stage") == "plan" && r.s("state") == "blocked" { return words::PLAN.into(); }
    if !archived && r.s("stage") == "pick" { return words::PICK.into(); }
    if !archived && failed_review(r) { return words::REVIEW_FAILED.into(); }
    if r.s("state") == "blocked" && r.s("stage") != "approval" && !archived && (failure_for(r).is_none() || r.s("kind") == "auth") { if let Some(w) = needs(r.s("kind")) { return w.into(); } }
    if queued(r) { return super::queue::words(r); }
    group_of(r).into()
}
/// Who is on a task and where it stands, for herdr's sidebar.
pub fn who_stands(r: &Obj) -> (String, String) {
    let say = |stand: &str, w: &str| (w.to_string(), stand.to_string());
    if r.b("archivedAt") { return say("archived", ""); }
    if r.s("stage") == "tried" { return say(if r.s("parentStage") == "pick" { words::PICK } else { words::PASSED }, ""); }
    if r.s("state") == "working" {
        if under_review(r) { return say(words::UNDER_REVIEW, who::REVIEWER); }
        if BOARD_STEPS.contains(&r.s("stage")) { return say(step(r.s("stage")), who::BOARD); }
        if r.s("stage") == "queued" { return say(QUEUED, ""); }
        if super::queue::waits_after(r) { return say(&super::queue::words(r), ""); }
        return say(words::WORKING, if r.s("stage") == "progress" { who::YOU } else { who::AGENT });
    }
    if r.s("stage") == "approval" && (r.b("mergeFailed") || r.b("prFailure") || r.s("prState") == "failing") { return say(words::WRONG, ""); }
    if r.s("stage") == "approval" && r.b("syncBlocked") { return say(words::BEHIND, ""); }
    if r.s("state") == "error" && r.s("stage") != "nosummary" { return say(words::WRONG, ""); }
    (String::new(), standing(r))
}
/// A live task: an agent, the reviewer or the board's check is on it, or it waits for you. Its row has two lines.
pub fn is_live(r: &Obj) -> bool { !r.b("archivedAt") && ["needs you", "ready to approve", "working"].contains(&group_of(r)) }
pub fn checking(r: &Obj) -> bool { r.s("stage") == "checking" && r.s("state") == "working" && !r.b("archivedAt") }
/// The colour of the mark, the bar and the percent.
pub fn status_ink(r: &Obj) -> Ink {
    if r.b("archivedAt") { Ink::Muted } else if under_review(r) { Ink::Review } else if checking(r) { Ink::Check } else if queued(r) { Ink::Queue } else { group_ink(group_of(r)) }
}
/// A task as the screen reads it: `msg` is the line to show, `app` who is on it, `at` since when.
pub fn as_row(t: &Obj) -> Obj {
    let mut r = t.clone();
    r.put("raw", t.at("msg").clone());
    r.put("msg", t.t("note"));
    r.put("app", t.t("shownApp"));
    let at = if t.s("state") == "working" { if t.b("since") { t.at("since").clone() } else { t.at("at").clone() } } else { t.at("at").clone() };
    r.put("at", at);
    r
}

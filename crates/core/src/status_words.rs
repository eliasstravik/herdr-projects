//! The status words, each said once: where a task stands, in the words the board's rows and State line, herdr's
//! sidebar and the web page all use. One list, so that a task is never "needs you" in one place and "asks you" in
//! another.
use crate::gate;
use crate::json::*;
use crate::store::reviews::current_review;

/// What a task that waits for you needs, by its kind (OSC 7501): beside its mark (? ! @).
pub fn needs(kind: &str) -> Option<&'static str> { match kind { "question" => Some("asks you"), "permission" => Some("asks for your OK"), "auth" => Some("needs a login"), _ => None } }
pub mod words {
    pub const NEEDS: &str = "needs you";
    pub const READY: &str = "ready to approve";
    pub const REVIEW_FAILED: &str = "review failed";
    pub const WORKING: &str = "working";
    pub const UNDER_REVIEW: &str = "under review";
    pub const PLAN: &str = "plan ready";
    pub const TODO: &str = "to do";
    pub const DONE: &str = "done";
    pub const WRONG: &str = "went wrong";
    pub const BEHIND: &str = "behind main";
    pub const PICK: &str = "pick one";
    pub const PASSED: &str = "passed its check";
}
/// Finished work whose last review failed. It still waits for your yes or no, but it is not "ready".
pub fn review_failed(t: &Obj) -> bool {
    if t.s("stage") != "approval" { return false; }
    match current_review(t) { Some(rv) => if gate::is_tagged(rv) { !rv.b("pass") } else { rv.at("recommend") == &json!(false) }, None => false }
}
/// "1 critical, 1 major"; a review from before the gate: "2/5, reviewer says no".
pub fn review_tally(t: &Obj) -> String {
    let Some(rv) = current_review(t) else { return String::new() };
    if gate::is_tagged(rv) { return gate::tally(rv); }
    if gate::is_int(rv.at("score")) { format!("{}/5{}", rv.t("score"), if rv.at("recommend") == &json!(false) { ", reviewer says no" } else { "" }) } else { String::new() }
}
/// Who is on a task, as herdr's sidebar names them before where it stands.
pub mod who { pub const AGENT: &str = "agent"; pub const REVIEWER: &str = "reviewer"; pub const BOARD: &str = "board"; pub const YOU: &str = "you"; }
/// The board's own steps, each in a word, said after `board`.
pub fn step(stage: &str) -> &'static str { match stage { "starting" => "starting", "setup" => "setting up", "checking" => "checking", "pushing" => "pushing", "merging" => "merging", _ => "" } }
pub const QUEUED: &str = "queued";
pub const STOPPED: &str = "Stopped";
pub const COORDINATOR: &str = "Coordinator";
/// The Coordinator's error when it is running but never took your words: no start that failed.
pub const COORDINATOR_UNSENT: &str = "your message sits unsent in its box. Go into it and press enter.";
/// Where the Coordinator stands, for the line above the board's box and for herdr's sidebar. `error`: its error, if any.
pub fn coordinator_stands(state: &str, error: &str) -> String {
    if error == COORDINATOR_UNSENT { return "message not sent".into(); }
    if !error.is_empty() { return "could not be started".into(); }
    match state { "none" => "not started: your first message starts it", "starting" => "starting", "working" => words::WORKING, "blocked" => "asks you", "idle" => "idle", "exited" => "stopped: your next message starts it again", s => return s.to_string() }.to_string()
}
/// The same with what waits to go in: "idle · 1 message waits to be sent".
pub fn coordinator_status(word: &str, waiting: usize) -> String {
    if waiting == 0 { return word.to_string(); }
    format!("{word} · {waiting} {} to be sent", if waiting > 1 { "messages wait" } else { "message waits" })
}

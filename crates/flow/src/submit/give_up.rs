//! The board gives up on text a task's agent never started on: the task says "not started", its failure quotes the last
//! lines of the agent's pane, and the text is kept for retry, which starts the agent's program again.
use super::{flat, NOT_STARTED};
use crate::agents::hand_limits::as_typed;
use crate::agents::screen_text::screen_text;
use crate::finish::shared::last_lines;
use crate::herdr::{api::read_screen, Herdr};
use crate::json::*;
use crate::store::failures::{failure_patch, strip_ansi};
use crate::store::tasks::{update_wait, Change, Task};

/// The mark in `field` ("sent", or "unconfirmed") is still the one that was looked at.
pub fn mine(c: &Task, field: &str, m: &Value) -> bool { c.at(field).is_object() && c.at(field).at("at") == m.at("at") && c.at(field).at("tries") == m.at("tries") }
/// The agent's pane, read once. `status`: herdr's for the agent; None when no agent runs in it.
pub fn screen(api: &Herdr, pane: &str, status: Option<&str>) -> String {
    if pane.is_empty() { return String::new(); }
    strip_ansi(&read_screen(api, pane, status, 200))
}
/// Does the pane show the words? The start of their first and of their last line is looked for, with whitespace, quotes
/// and the edges of the program's boxes left out, so that a line the terminal wrapped still counts.
pub fn on_screen(screen: &str, text: &str) -> bool {
    let (shown, typed) = (flat(&screen_text(screen)), as_typed(text));
    let lines: Vec<&str> = typed.lines().filter(|l| !l.trim().is_empty()).collect();
    [lines.first(), lines.last()].into_iter().flatten().any(|l| { let piece: String = flat(l).chars().take(40).collect(); !piece.is_empty() && shown.contains(&piece) })
}
/// Failed as not started, with the last lines of the agent's pane at the end of the detail.
pub fn fail(id: &str, field: &str, m: &Value, detail: &str, next: &str, screen: &str) {
    let tail = last_lines(screen, 8);
    let detail = if tail.is_empty() { detail.to_string() } else { format!("{detail}\n\nThe last lines of its terminal:\n{tail}") };
    let _ = update_wait(id, 10000, |c| {
        if !mine(c, field, m) || c.s("stage") != "working" { return Ok(Change::Keep); }
        c.put(field, Value::Null);
        c.put("unsent", json!({ "text": m.at("text"), "first": m.b("first"), "at": if m.b("typedAt") { m.at("typedAt").clone() } else { m.at("at").clone() }, "typed": true }));
        c.put("stage", "failed"); c.put("msg", NOT_STARTED); c.put("turnOpen", false); c.put("watch", Value::Null);
        for (k, v) in failure_patch(&obj(json!({ "stage": "send", "title": "The agent has not started on its instructions", "auth": false, "agentCanFix": false, "detail": detail, "next": next }))) { c.insert(k, v); }
        Ok(Change::Write)
    });
}

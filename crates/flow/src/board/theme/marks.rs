//! herdr's mark for each state (src/client/shell.rs, status_icon): its symbols, or its dots where herdr's
//! `ui.status_indicators` says "dots". Every state has its own mark, and only a moon says someone works: ◐ an agent
//! works, ◑ its reviewer, ◇ the board's own step (it checks, waits for GitHub's checks, merges), ◷ (a clock) it waits
//! for a free slot, ✓ done, ○ to do, ✗ went wrong, ◆ ready for your yes, and what a task that waits for you needs: ? an answer, !
//! a yes or no, @ a sign-in. Every one of them is one column wide.
pub const MARK_STATES: [&str; 10] = ["working", "review", "board", "queue", "ready", "blocked", "done", "idle", "unknown", "error"];
pub fn mark(state: &str, kind: &str, symbols: bool) -> &'static str {
    let pair: (&str, &str) = if state == "blocked" {
        match kind { "question" => ("?", "?"), "permission" => ("!", "!"), "auth" => ("@", "@"), _ => ("!", "!") }
    } else {
        // With dots, herdr's dot is someone at work, as a moon is with symbols; the board's step and a wait for a slot
        // keep their own marks, which no dot is.
        match state { "working" => ("●", "◐"), "review" => ("●", "◑"), "board" => ("◇", "◇"), "queue" => ("◷", "◷"), "ready" => ("◆", "◆"), "done" => ("●", "✓"), "idle" => ("○", "○"), "error" => ("✗", "✗"), _ => ("·", "·") }
    };
    if symbols { pair.1 } else { pair.0 }
}
/// The marks as the web page gets them: it draws herdr's dots itself, tells the reviewer from the agent's dot by the
/// half moon and draws the waiting ◷ as a clock (crates/web/src/webui/page/dom.js); the board's own step and what
/// a waiting task needs as on the terminal board.
pub fn web_mark(state: &str, kind: &str) -> &'static str {
    if state == "blocked" { if let s @ ("?" | "!" | "@") = mark(state, kind, true) { if kind == "question" || kind == "permission" || kind == "auth" { return s; } } }
    match state { "idle" => "○", "working" => "●", "review" => "◑", "board" => "◇", "queue" => "◷", "ready" => "◆", "blocked" => "!", "done" => "✓", "error" => "✗", _ => "·" }
}

//! The reviewer's progress watchdog: a pane that shows the same screen for minutes is not reviewing, it is waiting at a
//! prompt nobody will answer. One look at the pane's screen asks for the error that ends the round, before
//! `review.timeout` does, and quotes what the pane last showed so the next attempt knows what it walked into.
use super::shared::{last_lines, ms};

/// How long the reviewer's pane may show no new output before the round ends: five minutes.
pub fn quiet_ms() -> i64 { ms("HERDR_PROJECTS_REVIEW_QUIET_MS", 300000) }
/// The screen the pane shows, and since when it has shown exactly that.
#[derive(Default, Clone)]
pub struct Screen { pub text: String, pub since: i64 }
impl Screen {
    /// One look. None while the pane still shows something new, or has not stood still long enough; else the error the
    /// round ends with, quoting the last lines of what the pane shows.
    pub fn look(&mut self, screen: &str, now: i64, quiet: i64) -> Option<String> {
        if screen != self.text || self.since == 0 { self.text = screen.to_string(); self.since = now; return None; }
        if quiet <= 0 || now - self.since < quiet { return None; }
        let tail = last_lines(&self.text, 8);
        let what = if tail.trim().is_empty() { "its pane shows nothing".to_string() } else { format!("its pane last shows:\n{tail}") };
        Some(format!("the reviewer produced no output for {}m: {what}", (quiet / 60000).max(1)))
    }
}

//! What a coordinator's row says of it, as a task's row says of its agent: what it does while it works (the last line
//! of its turn so far), what it asks you while it waits for you (its question, or that it wants your OK), when that
//! turn began and when it last said something. Worked out where it runs, and sent with its tasks to another machine.
use super::look::Look;

/// The last line of something said, kept to one line.
fn last_line(text: &str) -> String { text.lines().map(str::trim).filter(|l| !l.is_empty()).last().unwrap_or("").chars().take(300).collect() }
impl Look {
    /// Fills in what its row says from where it stands, its turn and its conversation.
    pub fn live(mut self) -> Look {
        let open = self.turn.as_ref().map(|t| !t.finished).unwrap_or(false);
        if self.state == "working" && open { if let Some(t) = &self.turn { self.began = t.began; self.doing = last_line(&t.said); } }
        if let Some(m) = self.talk.last() { self.at = m.at; }
        self.asks = self.asking();
        self
    }
    /// What it waits for you for: a dialog on its screen, or a question it ended on (its last words, yours since none).
    pub fn asking(&self) -> String {
        if self.state == "blocked" { return "wants your OK".into(); }
        if self.state != "idle" || !self.error.is_empty() { return String::new(); }
        match self.talk.last() { Some(m) if !m.you && m.text.trim_end().ends_with('?') => last_line(&m.text), _ => String::new() }
    }
}

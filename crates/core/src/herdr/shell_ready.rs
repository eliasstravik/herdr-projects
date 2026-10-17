//! A start typed into a pane whose shell is not at its prompt yet, tried again. herdr refuses to type a line longer than
//! the terminal takes while it is in canonical mode, before the shell's line editor is up (macOS: 1024 bytes, Linux:
//! 4096): "input contains a 3570-byte line but the terminal is in canonical mode with a 1024-byte limit; wait for the
//! shell prompt and retry" (herdr 0.9.5, issue #175). Nothing was typed then, so the board waits for the prompt and
//! tries again, a few times at most, and never starts a second program.
use super::cli::{HRes, HerdrError};
use super::Herdr;
use crate::json::*;

/// How many times a start is tried again.
pub const RETRIES: u32 = 3;
/// How many looks at the pane, a step apart, before a try goes ahead without having seen the prompt.
const LOOKS: u32 = 8;

/// Did herdr refuse the line because the pane's shell is not at its prompt yet?
pub fn not_at_prompt(e: &HerdrError) -> bool { e.message.contains("canonical mode") }
fn step_ms() -> u64 { std::env::var("HERDR_PROJECTS_READY_STEP_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(500) }

impl Herdr {
    /// `start` types a line into `pane`'s shell. While herdr answers that the shell is not at its prompt, wait for the
    /// prompt and run it again (`RETRIES` times at most, about 15 seconds in all). An agent herdr already sees in the
    /// pane is the answer: it is never started twice.
    pub fn when_at_prompt(&self, pane: &str, mut start: impl FnMut() -> HRes<Value>) -> HRes<Value> {
        let began = crate::clock::now();
        let mut tries = 0;
        loop {
            match start() {
                Err(e) if not_at_prompt(&e) && tries < RETRIES => {
                    tries += 1;
                    self.await_prompt(pane, tries);
                    if let Ok(Some(a)) = self.agent_get(pane) { return Ok(a); }
                }
                Err(e) if not_at_prompt(&e) => {
                    let secs = (crate::clock::now() - began) / 1000;
                    return Err(HerdrError { code: "not_at_prompt".into(), message: format!("{} (tried {} times in {secs} s)", e.message.trim_end_matches('.'), tries + 1) });
                }
                r => return r,
            }
        }
    }
    /// Wait until the pane shows the same last line twice in a row: its shell has drawn its prompt. A little longer
    /// before each later try; at most `LOOKS` steps.
    fn await_prompt(&self, pane: &str, tries: u32) {
        let step = step_ms();
        if step > 0 { crate::clock::sleep_ms(step * tries as u64); }
        let mut last: Option<String> = None;
        for _ in 0..LOOKS {
            let screen = self.pane_read(pane, "recent", Some(5)).unwrap_or_default();
            let line = screen.lines().map(str::trim).rfind(|l| !l.is_empty()).unwrap_or("").to_string();
            if !line.is_empty() && last.as_deref() == Some(line.as_str()) { return; }
            last = Some(line);
            if step > 0 { crate::clock::sleep_ms(step); }
        }
    }
}

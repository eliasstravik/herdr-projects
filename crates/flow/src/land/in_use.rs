//! What keeps a task's space in use, in plain words, for the line of a clean-up that was put off: a turn in its agent's
//! terminal, an agent herdr cannot place, a process beside the agent that is not its helper (each by name and why), or
//! something in another of its panes. The line names the first; `details <id>` shows them all (`inUse`).
use crate::herdr::{busy, Herdr};
use crate::json::*;

/// Why the agent's own pane counts as in use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why { Turn, NoAnswer, Unsure, Blank, Moving, TurnOpen, Beside }
const UNSURE: &str = "herdr cannot tell whether its agent works";

/// What keeps it, each thing whole; `ctx` is where (`beside its agent`), said once on the line.
#[derive(Default, Debug, Clone)]
pub struct InUse { pub ctx: &'static str, pub what: Vec<String> }
impl InUse {
    fn one(w: impl Into<String>) -> InUse { InUse { ctx: "", what: vec![w.into()] } }
    /// The words after a kept checkout: `(its space is in use beside its agent: node …/server.js (pid 812) streams to the terminal)`.
    pub fn line(&self, id: &str) -> String {
        let first = self.what.first().cloned().unwrap_or_else(|| "something runs in it".into());
        let more = if self.what.len() > 1 { format!("; and {} more (herdr-projects details {id})", self.what.len() - 1) } else { String::new() };
        let ctx = if self.ctx.is_empty() { String::new() } else { format!(" {}", self.ctx) };
        format!("its space is in use{ctx}: {first}{more}")
    }
    /// Every thing, each with where it is, for `details`.
    pub fn all(&self) -> Vec<String> { self.what.iter().map(|w| if self.ctx.is_empty() { w.clone() } else { format!("{}: {w}", self.ctx) }).collect() }
}

/// The agent's own pane, in use for `why`.
pub fn agent(why: Why, api: Option<&Herdr>, pane: &str) -> InUse {
    match why {
        Why::Turn => InUse::one("a turn in its agent's terminal"),
        Why::NoAnswer => InUse::one("herdr does not answer for its agent"),
        Why::Unsure => InUse::one(UNSURE),
        Why::Blank => InUse::one(format!("{UNSURE}, and its screen is blank")),
        Why::Moving => InUse::one(format!("{UNSURE}, and its screen has not stood still long enough")),
        Why::TurnOpen => InUse::one(format!("{UNSURE}, and its last turn has not ended")),
        Why::Beside => {
            let Some(api) = api else { return InUse::one(UNSURE) };
            let kind = api.agent_get(pane).ok().flatten().map(|a| a.t("agent")).unwrap_or_default();
            let what = busy::beside_agent(api, pane, &kind, true);
            if what.is_empty() { InUse::one("a command its agent started was still running") } else { InUse { ctx: "beside its agent", what } }
        }
    }
}
/// Another pane of its space: an agent in it, or what runs beside its shell.
pub fn other(api: &Herdr, pane: &str, has_agent: bool) -> InUse {
    if has_agent { return InUse::one("an agent runs in another of its panes"); }
    let what = busy::beside_shell(api, pane, true);
    if what.is_empty() { InUse::one("a command was running in another of its panes") } else { InUse { ctx: "in another of its panes", what } }
}
pub fn space_unanswered() -> InUse { InUse::one("herdr does not answer for its space") }

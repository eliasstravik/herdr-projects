//! The board inside the herdr web UI, asked about once: with that plugin installed and its link shared, the board asks
//! whether the link may go through its front door, which is what puts the Board into the web UI (docs/web-ui.md). Yes
//! does it; no leaves the link alone for good; later asks again the next time the board opens.
use super::Effect;
use crate::board::job::Job;
use crate::board::state::{State, Valid};

/// One look per open board, a few seconds after it opens and with nothing of yours open: looking asks herdr and
/// Tailscale, so the runtime does it on a thread and answers `offered`.
pub fn look(s: &mut State, fx: &mut Vec<Effect>) {
    if s.web_ui_asked || s.ctx.web || s.ask.is_some() || s.setup.is_some() || s.menu.is_some() { return; }
    s.web_ui_asked = true;
    fx.push(Effect::LookWebUi);
}
/// Links that go straight to the web UI: the question. Something else came up meanwhile: asked the next time.
pub fn offered(s: &mut State, urls: Vec<String>) {
    if urls.is_empty() { return; }
    if s.ask.is_some() || s.setup.is_some() { s.web_ui_asked = false; return; }
    let detail = format!("The web UI's link ({}) then goes through a small front door of the board's own, on this machine only. It passes everything on to the web UI untouched and adds a Board entry to its menu. The login stays the web UI's. `herdr-projects web-ui unlink` points the link back.", urls.join(", "));
    super::ask::confirm(s, "Show this board in the web UI (Ovis)?".into(), Job::WebUiLink, Valid::Always, vec![("sayYes", "yes".into()), ("sayNo", "no".into()), ("back", "later".into())], detail, false);
    if let Some(a) = s.ask.as_mut() { a.no = Some(Job::WebUiDecline); }
}

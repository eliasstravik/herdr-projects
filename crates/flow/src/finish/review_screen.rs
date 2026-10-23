//! The reviewer's last screen: read once when a review ends without a verdict, before its tab closes, and quoted in the
//! failure, so that you (and a retry) see what stopped it, such as a dialog herdr did not report as one.
use super::shared::last_lines;
use crate::herdr::api::{read_screen, Herdr};
use crate::json::*;

/// The reviewer's last lines on screen, its words without its frames; empty when its terminal is gone.
pub fn last_screen(api: &Herdr, pane: &str) -> String {
    let status = api.agent_get(pane).ok().flatten().map(|a| a.t("agent_status"));
    let screen = crate::agents::screen_text::screen_text(&read_screen(api, pane, status.as_deref(), 60));
    last_lines(&screen, 8).split('\n').map(|l| l.chars().take(300).collect::<String>()).collect::<Vec<_>>().join("\n")
}
/// A failed review's detail, with the reviewer's last screen below it when there was one.
pub fn with_screen(detail: &str, screen: &str) -> String {
    if screen.trim().is_empty() { return detail.to_string(); }
    format!("{detail}\n\nThe reviewer's screen when the review ended:\n{screen}")
}

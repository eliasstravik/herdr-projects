//! The key that opens the selected task's link in the browser: its report when it has one, else its pull request. It
//! opens with this machine's own opener (a job: core/opener.rs); where nothing can open it the link is copied (OSC 52)
//! and the board says so. On the web board the page opens it.
use super::Effect;
use crate::board::job::Job;
use crate::board::state::State;
use crate::json::*;
use crate::shown::ink::Ink;

/// The task's link, and what it is: ("report", its link), else ("pull request", its address).
pub fn of(r: &Obj) -> Option<(&'static str, String)> {
    let rep = r.at("report");
    if rep.is_object() && rep.b("link") { return Some(("report", rep.t("link"))); }
    if r.b("pr") { return Some(("pull request", r.t("pr"))); }
    None
}
fn is_web(link: &str) -> bool { link.starts_with("http://") || link.starts_with("https://") }
/// The key: a task with no link has nothing to open.
pub fn open(s: &mut State, r: &Obj, fx: &mut Vec<Effect>) {
    let Some((what, link)) = of(r) else { return };
    if s.ctx.web {
        if is_web(&link) { return super::web::go(s, json!({ "url": link })); }
        return s.flash(format!("Its {what} is a file on the machine the board runs on, no link a page can open: {link}"), Ink::Yellow);
    }
    // Another machine's file is not here to open: its path is copied.
    if r.b("host") && !is_web(&link) { fx.push(copy(&link)); return s.flash(format!("Its {what} is a file on {}: copied its path. {link}", r.s("host")), Ink::Yellow); }
    fx.push(Effect::Run(Job::OpenLink { link, what: what.into() }));
}
/// The terminal's clipboard code for this text, as the board's copy writes it (run/mouse.rs).
pub fn copy(text: &str) -> Effect { Effect::Osc(format!("\x1b]52;c;{}\x07", crate::util::base64(text.as_bytes()))) }

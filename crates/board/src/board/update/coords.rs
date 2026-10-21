//! The coordinators behind the list's headings: space on a machine's heading talks to that machine's coordinator, on a
//! repo's heading (grouped by repo) to that repo's project coordinator, as space on a task talks to its agent. The chat
//! then follows the selection to the next heading or task, and space again goes back to the Coordinator. What you
//! type goes to that coordinator: this machine's are started here, another machine's there.
use super::heads::{is_head, label, HEAD};
use super::Effect;
use crate::board::job::Job;
use crate::board::state::{Row, State};
use crate::json::*;
use crate::shown::ink::Ink;

/// A heading's coordinator: on which machine ("" this one), its level as the coordinator code names it ("machine",
/// "project:<repo>"), and its name ("vm-str coordinator").
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Target { pub host: String, pub to: String, pub name: String }
/// A machine's heading's colour, which its machine coordinator takes too: the theme's accent.
pub const MACHINE_INK: Ink = Ink::Accent;
/// A repo's project coordinator's colour: the brighter grey. Its heading is not (REPO_HEAD_INK), so the three
/// coordinators stay three colours.
pub const REPO_INK: Ink = Ink::Soft;
/// A repo's heading's name (grouped by repo): the plain text colour, bold, as loud as the task titles under it.
pub const REPO_HEAD_INK: Ink = Ink::Plain;
impl Target {
    /// Its colour: on the chat's frame, its name and its labels, and its name in the hints. A machine's coordinator has
    /// its heading's accent, a repo's the brighter grey. The top-level Coordinator has none of its own (plain text).
    pub fn ink(&self) -> Ink { if self.to == "machine" { MACHINE_INK } else { REPO_INK } }
    /// What the chat keeps your words to it under (update/chat.rs), beside the tasks' ids.
    pub fn key(&self) -> String { format!("co:{}\t{}", self.host, self.to) }
}
/// The Coordinator of the machine whose board shows, as a target: its top-level one (`to` empty).
pub fn far_top(s: &State) -> Option<Target> { s.remotes.contains_key(&s.scope).then(|| Target { host: s.scope.clone(), to: String::new(), name: format!("Coordinator on {}", s.scope) }) }
/// That coordinator as the board read it last: this machine's, or what its machine sent with its tasks.
pub fn look_of(s: &State, t: &Target) -> Option<crate::coordinator::look::Look> {
    if t.host.is_empty() && t.to.is_empty() { return Some(s.data.coord.clone()); }
    if t.host.is_empty() { return s.data.sub.as_ref().filter(|x| x.0 == t.to).map(|x| x.1.clone()); }
    let m = s.remotes.get(&t.host)?;
    crate::coordinator::far::read(if t.to.is_empty() { &m.coord } else { m.coords.at(&t.to) })
}
/// The coordinator a heading stands for: a machine's heading, or a repo's. A status group's has none.
pub fn of_head(s: &State, id: &str) -> Option<Target> {
    let rest = id.strip_prefix(HEAD)?;
    let parts: Vec<&str> = rest.split('\t').collect();
    match parts.as_slice() {
        ["main", host] => Some(Target { host: host.to_string(), to: String::new(), name: if host.is_empty() { crate::status_words::COORDINATOR.into() } else { format!("Coordinator on {host}") } }),
        ["machine", host] => Some(Target { host: host.to_string(), to: "machine".into(), name: crate::coordinator::co::named(&label(s, host)) }),
        ["repo", host, repo] => Some(Target { host: host.to_string(), to: format!("project:{repo}"), name: crate::coordinator::co::named(&crate::shown::words::repo_label(repo)) }),
        _ => None,
    }
}
/// The heading's coordinator the chat talks to now, if it does.
pub fn talking(s: &State) -> Option<Target> { if s.reply && is_head(&s.reply_id) && s.reply_id == s.sel { of_head(s, &s.sel) } else { None } }
/// This machine's coordinator the board should read, as the coordinator code names its level ("" none): the one the
/// chat talks to, else the one whose row is selected (its panel shows it).
pub fn wanted(s: &State) -> String { talking(s).or_else(|| of_head(s, &s.sel)).filter(|t| t.host.is_empty()).map(|t| t.to).unwrap_or_default() }
/// Is this row still on the board: a task, or a heading.
pub fn exists(s: &State, id: &str) -> bool { s.items.iter().any(|r| r.s("id") == id) || s.rows.iter().any(|r| matches!(r, Row::Head { id: h, .. } if h == id)) }
/// Space on a heading: talk to its coordinator. A status group's heading has none: it says where they are.
pub fn flip(s: &mut State) -> bool {
    if !is_head(&s.sel) { return false; }
    if of_head(s, &s.sel).is_none() { s.flash(format!("Space on a machine's heading or a repo's heading talks to its coordinator. {} groups the tasks by repo.", crate::board::keys::key_name("byRepo")), Ink::Muted); return true; }
    s.reply = true; s.reply_id = s.sel.clone(); s.panel_top = 0;
    true
}
/// What you typed, for that coordinator: here, or on its machine. `delegate`: sent with tab.
pub fn send(s: &mut State, t: &Target, text: &str, delegate: bool, fx: &mut Vec<Effect>) {
    if text.is_empty() { return; }
    if t.host.is_empty() {
        if !s.data.can_dispatch { return s.flash(format!("No coordinator can run here, so nothing was sent. {}", crate::coordinator::who::OFF_WORDS), Ink::Yellow); }
        fx.push(Effect::Run(Job::CoordSend { cwd: s.ctx.cwd.clone(), text: text.into(), delegate, about: String::new(), to: t.to.clone() }));
    } else {
        let Some(m) = s.remotes.get(&t.host).map(|m| m.machine.clone()) else { return s.flash(format!("{} is not on this board any more. Nothing was sent.", t.host), Ink::Yellow) };
        fx.push(Effect::Run(Job::Dispatch { machine: m, text: text.into(), delegate, about: String::new(), to: t.to.clone() }));
    }
    super::chat::note_coord(s, t, text);
    s.input.set_text("");
    fx.push(Effect::Load);
}

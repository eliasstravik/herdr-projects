//! Autopilot from the board's side: a coordinator's row (a machine's heading with every machine shown or grouped by
//! machine, a repo's heading grouped by repo) stands for its coordinator. The start key there turns
//! autopilot on, with its saved goal and settings, and `Ctrl+X` turns it off, as those keys put an agent on a task and stop it.
//! What each shows of its autopilot is what its machine read (data.rs: pilots), this one's or another's (sent with its
//! tasks); a machine on an older version sends no autopilot at all, and the keys say to update it there. One of its
//! coordinators missing from what it sent never ran: it is off, and the start key starts it.
use super::coords::{of_head, Target};
use super::Effect;
use crate::board::job::Job;
use crate::board::state::State;
use crate::json::*;
use crate::shown::ink::Ink;

/// What its machine read of a coordinator's autopilot (`{ on: false }` while off or never run; Null: its machine sends
/// no autopilot, an older version).
pub fn view(s: &State, t: &Target) -> Value {
    let all = if t.host.is_empty() { &s.data.pilots } else { match s.remotes.get(&t.host) { Some(m) => &m.pilots, None => return Value::Null } };
    if !all.is_object() { return if t.host.is_empty() { json!({ "on": false }) } else { Value::Null }; }
    match all.at(&t.to) { Value::Null => json!({ "on": false }), v => v.clone() }
}
/// It is on autopilot, as its machine read it.
pub fn on(s: &State, t: &Target) -> bool { view(s, t).b("on") }
/// The coordinator of the selected row, if the row is one.
pub fn selected(s: &State) -> Option<Target> { of_head(s, &s.sel) }
/// The coordinator the start, stop and autopilot keys act on: the selected row's; with no machine's, project's or
/// task's row selected (a status group's heading, or nothing), the main Coordinator the chat talks to then, which has
/// no row of its own.
pub fn keyed(s: &State) -> Option<Target> {
    if let Some(t) = selected(s) { return Some(t); }
    if s.current().is_some() || s.auto().is_some() { return None; }
    main(s)
}
/// The main Coordinator the chat talks to by default: this machine's while one can run here, or that of the one other
/// machine whose board shows.
pub fn main(s: &State) -> Option<Target> {
    if s.archive_view { return None; }
    if let Some(t) = super::coords::far_top(s) { return Some(t); }
    (s.scope.is_empty() || s.scope == "*").then(|| Target { host: String::new(), to: String::new(), name: crate::status_words::COORDINATOR.into() }).filter(|_| s.data.can_dispatch)
}
/// The start key on a coordinator's row: autopilot on, with its saved goal and settings, and nothing said: its heading shows it.
/// Already on: it says so; at `needs you`: it resumes.
pub fn turn_on(s: &mut State, t: &Target, fx: &mut Vec<Effect>) {
    let v = view(s, t);
    if v.is_null() && !t.host.is_empty() { return s.flash(format!("{} runs an older version that has no autopilot: update herdr-projects there first.", t.host), Ink::Yellow); }
    if v.b("on") && !["needs you", "could not start"].contains(&v.s("state")) { return s.flash(format!("The {} is on autopilot already: {}.", t.name, v.s("line")), Ink::Muted); }
    fx.push(job(s, t, true));
}
/// `Ctrl+X` on a coordinator's row: autopilot off. Off already: it says so.
pub fn turn_off(s: &mut State, t: &Target, fx: &mut Vec<Effect>) {
    let v = view(s, t);
    if v.is_null() && !t.host.is_empty() { return s.flash(format!("{} runs an older version that has no autopilot: update herdr-projects there first.", t.host), Ink::Yellow); }
    if !v.b("on") { return s.flash(format!("The {} is not on autopilot. {} turns it on.", t.name, crate::board::keys::key_name("autopilot")), Ink::Muted); }
    s.flash(format!("Turning autopilot off for the {}…", t.name), Ink::Muted);
    fx.push(job(s, t, false));
}
fn job(s: &State, t: &Target, on: bool) -> Effect {
    if t.host.is_empty() { return Effect::Run(Job::Pilot { to: t.to.clone(), on }); }
    let m = s.remotes.get(&t.host).map(|m| m.machine.clone()).unwrap_or_default();
    Effect::Run(Job::Far { machine: m, host: t.host.clone(), action: if on { "autopilot-on" } else { "autopilot-off" }.into(), req: json!({ "id": "", "to": t.to }), quiet: on })
}
/// The start key and the stop key on a coordinator's row: start it in its own conversation, or stop it, as an agent on a
/// task; here or on its machine.
pub fn run(s: &mut State, t: &Target, on: bool, fx: &mut Vec<Effect>) {
    s.flash(format!("{} the {}…", if on { "Starting" } else { "Stopping" }, t.name), Ink::Muted);
    if t.host.is_empty() { return fx.push(Effect::Run(Job::CoordRun { to: t.to.clone(), name: t.name.clone(), on })); }
    let m = s.remotes.get(&t.host).map(|m| m.machine.clone()).unwrap_or_default();
    fx.push(Effect::Run(Job::Far { machine: m, host: t.host.clone(), action: if on { "coordinator-start" } else { "coordinator-stop" }.into(), req: json!({ "id": "", "to": t.to }), quiet: false }));
}
/// The autopilot key on a coordinator's row: on, or off again (at `needs you` or `could not start` it starts again).
pub fn toggle(s: &mut State, t: &Target, fx: &mut Vec<Effect>) {
    let v = view(s, t);
    if v.b("on") && !["needs you", "could not start"].contains(&v.s("state")) { turn_off(s, t, fx) } else { turn_on(s, t, fx) }
}
/// A ctrl or alt key's line in the ? menu on a coordinator's row, where it does something there of its own.
pub fn menu_label(s: &State, k: crate::board::keys::K) -> Option<String> {
    use crate::board::keys::K;
    let t = keyed(s)?;
    // The main Coordinator, which has no row: only the start, stop and autopilot keys are its.
    if selected(s).is_none() && !matches!(k, K::Delegate | K::Stop | K::Pilot) { return None; }
    Some(match k {
        K::Delegate => format!("start the {}", t.name),
        K::Stop => format!("stop the {}", t.name),
        K::Pilot => format!("{} the {}'s autopilot", if on(s, &t) { "turn off" } else { "turn on" }, t.name),
        K::Settings => if s.editing.is_some() { format!("back from the {}'s settings", t.name) } else if t.host.is_empty() { format!("the {}'s settings: see and change each one", t.name) } else { format!("the {}'s settings, as {} has them", t.name, t.host) },
        K::Side => format!("{} the side panel: this coordinator, beside the list", if s.side { "hide" } else { "show" }),
        K::Panel => if s.full { "back to the list".into() } else { "show this coordinator whole: its autopilot and its last wakes".into() },
        _ => return None,
    })
}

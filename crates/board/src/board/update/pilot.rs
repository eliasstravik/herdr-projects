//! Autopilot from the board's side: a coordinator's row (the main Coordinator's on top of the list, a machine's heading
//! with every machine shown, a repo's heading grouped by repo) stands for its coordinator. `Tab` there turns autopilot
//! on, with its saved goal and settings, and `Ctrl+X` turns it off, as those keys put an agent on a task and stop it.
//! What each shows of its autopilot is what its machine read (data.rs: pilots), this one's or another's (sent with its
//! tasks); a machine on an older version sends nothing of it, and the keys say to update it there.
use super::coords::{of_head, Target};
use super::heads::HEAD;
use super::Effect;
use crate::board::job::Job;
use crate::board::state::State;
use crate::json::*;
use crate::shown::ink::Ink;

/// The main Coordinator's row: on top of the list, for the machine whose board shows ("" this one).
pub fn main_id(host: &str) -> String { format!("{HEAD}main\t{host}") }
pub fn is_main(id: &str) -> bool { id.starts_with(&format!("{HEAD}main\t")) }
/// The main Coordinator's row this board shows, if it has one: this machine's while a Coordinator can run here, one
/// other machine's (its board alone) while it sent one.
pub fn main_row(s: &State) -> Option<String> {
    if s.archive_view { return None; }
    if s.scope.is_empty() || s.scope == "*" { return s.data.can_dispatch.then(|| main_id("")); }
    let m = s.remotes.get(&s.scope)?;
    crate::coordinator::far::read(&m.coord).map(|_| main_id(&s.scope))
}
/// What its machine read of a coordinator's autopilot (`{ on: false }` while off; Null: its machine sends none).
pub fn view(s: &State, t: &Target) -> Value {
    if t.host.is_empty() { return s.data.pilots.at(&t.to).clone(); }
    s.remotes.get(&t.host).map(|m| m.pilots.at(&t.to).clone()).unwrap_or(Value::Null)
}
/// It is on autopilot, as its machine read it.
pub fn on(s: &State, t: &Target) -> bool { view(s, t).b("on") }
/// The coordinator of the selected row, if the row is one.
pub fn selected(s: &State) -> Option<Target> { of_head(s, &s.sel) }
/// What the main Coordinator's row says of the others: `2 others on · 1 needs you` ("" none on).
pub fn others(s: &State, main: &Target) -> String {
    let mut views: Vec<Value> = vec![];
    let mine = |all: &Value, host: &str, out: &mut Vec<Value>| { for (to, v) in all.as_object().cloned().unwrap_or_default() { if !(host == main.host && to.is_empty()) { out.push(v); } } };
    if main.host.is_empty() { mine(&s.data.pilots, "", &mut views); }
    for m in super::remotes::in_view(s) { if s.scope == "*" || m.machine.label == main.host { mine(&m.pilots, &m.machine.label, &mut views); } }
    crate::autopilot::shown::others_words(&views)
}
/// `Tab` on a coordinator's row: autopilot on, with its saved goal and settings. Already on: it says so; at `needs
/// you`: it resumes.
pub fn turn_on(s: &mut State, t: &Target, fx: &mut Vec<Effect>) {
    let v = view(s, t);
    if v.is_null() && !t.host.is_empty() { return s.flash(format!("{} runs an older version that has no autopilot: update herdr-projects there first.", t.host), Ink::Yellow); }
    if v.b("on") && !["needs you", "could not start"].contains(&v.s("state")) { return s.flash(format!("The {} is on autopilot already: {}.", t.name, v.s("line")), Ink::Muted); }
    s.flash(format!("Turning autopilot on for the {}…", t.name), Ink::Muted);
    fx.push(job(s, t, true));
}
/// `Ctrl+X` on a coordinator's row: autopilot off. Off already: it says so.
pub fn turn_off(s: &mut State, t: &Target, fx: &mut Vec<Effect>) {
    let v = view(s, t);
    if v.is_null() && !t.host.is_empty() { return s.flash(format!("{} runs an older version that has no autopilot: update herdr-projects there first.", t.host), Ink::Yellow); }
    if !v.b("on") { return s.flash(format!("The {} is not on autopilot. {} turns it on.", t.name, crate::board::keys::key_name("delegate")), Ink::Muted); }
    s.flash(format!("Turning autopilot off for the {}…", t.name), Ink::Muted);
    fx.push(job(s, t, false));
}
fn job(s: &State, t: &Target, on: bool) -> Effect {
    if t.host.is_empty() { return Effect::Run(Job::Pilot { to: t.to.clone(), on }); }
    let m = s.remotes.get(&t.host).map(|m| m.machine.clone()).unwrap_or_default();
    Effect::Run(Job::Far { machine: m, host: t.host.clone(), action: if on { "autopilot-on" } else { "autopilot-off" }.into(), req: json!({ "id": "", "to": t.to }), quiet: false })
}
/// A ctrl or alt key's line in the ? menu on a coordinator's row, where it does something there of its own.
pub fn menu_label(s: &State, k: crate::board::keys::K) -> Option<String> {
    use crate::board::keys::K;
    let t = selected(s)?;
    Some(match k {
        K::Stop => format!("turn the {}'s autopilot off", t.name),
        K::Settings => if s.editing.is_some() { format!("back from the {}'s settings", t.name) } else if t.host.is_empty() { format!("the {}'s settings: see and change each one", t.name) } else { format!("the {}'s settings, as {} has them", t.name, t.host) },
        K::Side => format!("{} the side panel: this coordinator, beside the list", if s.side { "hide" } else { "show" }),
        K::Panel => if s.full { "back to the list".into() } else { "show this coordinator whole: its autopilot and its last wakes".into() },
        _ => return None,
    })
}

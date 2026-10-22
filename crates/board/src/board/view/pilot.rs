//! A machine's or a project's row as the list draws it: its coordinator's state as a task's mark, in the marks' column
//! left of the caret, and only where it says something. `◐` working, what it needs from you (`!`, `?`, `@`), `✗` it
//! could not start, autopilot's `∞` while that is on (in the colour of its state, as a task's mark is). Idle, stopped or
//! never started: no mark, the column blank. Never the time, columns or a line under it: the chat has those.
use crate::board::state::State;
use crate::board::theme::Theme;
use crate::board::update::coords::{look_of, of_head, Target};
use crate::coordinator::look::Look;
use crate::board::update::pilot::view;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};

/// Autopilot's mark: infinity, it goes on by itself; one column wide, as a task's marks are.
pub const PILOT_MARK: &str = "∞";
/// The mark of the coordinator a heading stands for, with the words a reader hears for it; None where it has nothing
/// to say (idle, stopped, never started) and for a heading with no coordinator.
pub fn mark(s: &State, t: &Theme, id: &str) -> Option<(Inked, &'static str)> {
    let c = of_head(s, id)?;
    let v = look_of(s, &c).unwrap_or_default();
    let p = view(s, &c);
    let on = p.b("on");
    let st = if on { p.s("state") } else { "" };
    if !v.error.is_empty() || st == "could not start" { return Some((Inked::of(t.mark("error", ""), Ink::Red).bold(), "could not start")); }
    if v.state == "blocked" { return Some((Inked::of(t.mark("blocked", ""), Ink::Red).bold(), "needs you")); }
    if on {
        let m = Inked::of(PILOT_MARK, match st { "needs you" => Ink::Red, "working" => Ink::Yellow, _ => Ink::Plain });
        return Some((match st { "needs you" => m.bold(), "asleep" => m.quiet(), _ => m }, "on autopilot"));
    }
    ["working", "starting"].contains(&v.state.as_str()).then(|| (Inked::of(t.mark("working", ""), Ink::Yellow), "working"))
}
/// A heading, drawn: the marks' column (`mark`: blank for a coordinator's row with nothing to say; None, a heading with
/// no coordinator, starts at its caret), its caret (`▾ `, or blanks where it has none), its name and what follows it.
pub fn placed(caret: Inked, named: Inked, mark: Option<Inked>, width: usize) -> Inked {
    let left = match mark { Some(m) => Inked::plain(" ").add(m).add(Inked::plain(" ")), None => Inked::plain(" ") };
    left.add(caret).add(named).cut(width)
}
/// The chat's mark for a coordinator (`mark`, as its state gives it): autopilot's `∞` in its place, in its colour, while
/// autopilot is on and it neither needs you nor went wrong.
pub fn in_chat(s: &State, c: &Target, v: &Look, mut mark: Inked) -> Inked {
    if view(s, c).b("on") && v.error.is_empty() && v.state != "blocked" { for p in mark.0.iter_mut() { p.t = PILOT_MARK.into(); } }
    mark
}

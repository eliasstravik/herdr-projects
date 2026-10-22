//! A machine's or a project's row (and `All machines`) as the list draws it: its coordinator, as a task's row is its
//! agent. The same marks as tasks, in the marks' column left of the caret: `○` idle (also one that never ran or was
//! stopped), `◐` working, `?` it asks you (`!` it wants your OK), `✗` it could not start, autopilot's `∞` while that is
//! on (in the colour of its state, as a task's mark is). On the right only the time: `3m` while it works, `1h ago`
//! else, nothing when it never ran. Under it, as a task's second line, what it does while it works or what it asks you
//! while it waits for you; nothing else.
use crate::board::state::{Row, State};
use crate::board::theme::Theme;
use crate::board::update::coords::{look_of, of_head, Target};
use crate::board::update::pilot::view;
use crate::coordinator::look::Look;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};

/// Autopilot's mark: infinity, it goes on by itself; one column wide, as a task's marks are.
pub const PILOT_MARK: &str = "∞";
/// What a coordinator's row shows: its mark (with the words a reader hears for it), its time, its second line.
pub struct Shown { pub mark: Inked, pub label: &'static str, pub time: Inked, pub line: Inked }
/// The row of the coordinator a heading stands for. None for a heading with none (a status group's).
pub fn shown(s: &State, t: &Theme, id: &str) -> Option<Shown> {
    let c = of_head(s, id)?;
    // No coordinator can run here (`coordinator.agent none`): this machine's rows are headings only.
    if c.host.is_empty() && !s.data.can_dispatch { return None; }
    // Another machine that sends nothing of its coordinators (an older version): the column blank. One it sends nothing of
    // never ran there: `○`.
    let sends = c.host.is_empty() || s.remotes.get(&c.host).is_some_and(|m| m.coords.is_object() && !m.coord.b("off"));
    if !sends { return Some(Shown { mark: Inked::plain(" "), label: "", time: Inked::new(), line: Inked::new() }); }
    let v = look_of(s, &c).unwrap_or_default();
    let p = view(s, &c);
    let on = p.b("on");
    let st = if on { p.s("state") } else { "" };
    let working = ["working", "starting"].contains(&v.state.as_str());
    let (mark, label) = if !v.error.is_empty() || st == "could not start" { (Inked::of(t.mark("error", ""), Ink::Red).bold(), "could not start") }
        else if v.state == "blocked" { (Inked::of(t.mark("blocked", ""), Ink::Red).bold(), "wants your OK") }
        else if !v.asks.is_empty() { (Inked::of(t.mark("blocked", "question"), Ink::Red).bold(), "asks you") }
        else if on { let m = Inked::of(PILOT_MARK, match st { "needs you" => Ink::Red, "working" => Ink::Yellow, _ => if working { Ink::Yellow } else { Ink::Plain } }); (match st { "needs you" => m.bold(), "asleep" => m.quiet(), _ => m }, "on autopilot") }
        else if working { (Inked::of(t.mark("working", ""), Ink::Yellow), "working") }
        else { (Inked::of(t.mark("idle", ""), Ink::Muted), "idle") };
    let time = if working && v.began > 0 { Inked::of(crate::shown::words::age(s.now - v.began), Ink::Yellow) }
        else if v.at > 0 { Inked::of(format!("{} ago", crate::shown::words::age(s.now - v.at)), Ink::Muted) } else { Inked::new() };
    let line = second(&v).map(|(x, asks)| Inked::of(x, if asks { Ink::Text } else { Ink::Soft })).unwrap_or_default();
    Some(Shown { mark, label, time, line })
}
/// Its second line: what it asks you (true), or what it does while it works.
fn second(v: &Look) -> Option<(String, bool)> {
    if !v.asks.is_empty() { return Some((v.asks.clone(), true)); }
    (["working", "starting"].contains(&v.state.as_str()) && !v.doing.is_empty()).then(|| (v.doing.clone(), false))
}
/// Does this row take a line more (the list counts its room so)? A coordinator's that works or asks you.
pub fn extra(s: &State, r: &Row) -> usize {
    let Row::Head { id, .. } = r else { return 0 };
    usize::from(of_head(s, id).and_then(|c| look_of(s, &c)).is_some_and(|v| second(&v).is_some()))
}
/// A heading, drawn: the marks' column (`mark`; None, a heading with no coordinator, starts at its caret), its caret,
/// its name and what follows it, and its time at the right edge; the name gives way first, the mark and the time stay.
/// Its second line under its name, where it has one.
pub fn placed(caret: Inked, named: Inked, row: Option<Shown>, width: usize) -> (Inked, Option<Inked>) {
    let (mark, time, line) = match row { Some(r) => (Some(r.mark), r.time, r.line), None => (None, Inked::new(), Inked::new()) };
    let left = match mark { Some(m) => Inked::plain(" ").add(m).add(Inked::plain(" ")), None => Inked::plain(" ") }.add(caret);
    let room = width.saturating_sub(left.width() + if time.is_empty() { 0 } else { time.width() + 2 });
    let named = if named.width() > room { named.cut(room) } else { named };
    let gap = width.saturating_sub(left.width() + named.width() + time.width());
    let first = left.add(named).add(Inked::spaces(if time.is_empty() { 0 } else { gap })).add(time).cut(width);
    (first, (!line.is_empty()).then(|| Inked::spaces(5).add(line).cut(width)))
}
/// The chat's mark for a coordinator (`mark`, as its state gives it): autopilot's `∞` in its place, in its colour, while
/// autopilot is on and it neither needs you nor went wrong.
pub fn in_chat(s: &State, c: &Target, v: &Look, mut mark: Inked) -> Inked {
    if view(s, c).b("on") && v.error.is_empty() && v.state != "blocked" { for p in mark.0.iter_mut() { p.t = PILOT_MARK.into(); } }
    mark
}

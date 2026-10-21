//! A coordinator's row as the list draws it: the main Coordinator's on top (its name, and what it sums up of the others:
//! `2 others on · 1 needs you`), and on every coordinator's row its autopilot line while autopilot is on, laid out as an
//! automation's row is (`autopilot on · every 1h + events · last HSW-92 · next in 40m`), in autopilot's own colour:
//! beside the heading where the list is wide enough, on the line under it otherwise. Off, nothing of autopilot shows.
use crate::board::state::{Row, State};
use crate::board::update::coords::of_head;
use crate::board::update::pilot::{is_main, others, view};
use crate::json::*;
use crate::shown::ink::{Ink, Inked};

/// The autopilot line of the coordinator a heading stands for, while it is on.
pub fn line_of(s: &State, id: &str) -> Option<Inked> {
    let t = of_head(s, id)?;
    let v = view(s, &t);
    if !v.b("on") || v.s("line").is_empty() { return None; }
    let ink = if ["needs you", "could not start"].contains(&v.s("state")) { Ink::Red } else { Ink::Pilot };
    let line = v.t("line");
    let (word, rest) = line.split_once(" · ").map(|(a, b)| (a.to_string(), format!(" · {b}"))).unwrap_or((line.clone(), String::new()));
    Some(Inked::of(word, ink).bold().add(Inked::of(rest, Ink::Pilot)))
}
/// The main Coordinator's row's own words: its name, then what it sums up of the others.
pub fn main_head(s: &State, id: &str) -> Inked {
    let Some(t) = of_head(s, id) else { return Inked::new() };
    let sum = others(s, &t);
    let named = Inked::plain(t.name.clone()).bold();
    if sum.is_empty() { named } else { named.add(Inked::of(format!(" · {sum}"), Ink::Pilot)) }
}
/// Beside the heading, or on a line of its own under it (an automation's trigger goes so): what goes on the heading's
/// line, and the line under it, if any. `head`: the heading as drawn without its line; `room`: the list's width.
pub fn placed(head: Inked, line: Option<Inked>, room: usize) -> (Inked, Option<Inked>) {
    let Some(l) = line else { return (head, None) };
    if head.width() + 3 + l.width() <= room { return (head.add(Inked::plain("   ")).add(l), None); }
    (head, Some(Inked::plain("     ").add(l).cut(room)))
}
/// Does this row take a line more for its autopilot (the list counts its room so)?
pub fn extra(s: &State, r: &Row, room: usize) -> usize {
    let Row::Head { id, name, .. } = r else { return 0 };
    let Some(l) = line_of(s, id) else { return 0 };
    let head = if is_main(id) { main_head(s, id).width() } else { crate::text::width(name) + 4 };
    usize::from(head + 3 + l.width() > room)
}

//! A coordinator's row as the list draws it, in a task's grammar while autopilot is on: autopilot's mark left of its
//! name (`✈`, in the colour of its state, as a task's mark is), the timing in the columns on the right where a task
//! keeps its progress and an automation when it fires (`2 of 5 added  ✓ HSW-92  next in 40m`: the first go first where
//! room is short), and what it does now on the line under its name, as a live task's second line
//! (`On · woke 12m ago: HSW-92 merged`). The main Coordinator's row, on top, is its name and what it sums up of the
//! others (`2 others on · 1 needs you`). Off, nothing of autopilot shows.
use crate::board::state::{Row, State};
use crate::board::theme::Theme;
use crate::board::update::coords::of_head;
use crate::board::update::pilot::{others, view};
use crate::json::*;
use crate::shown::ink::{Ink, Inked};

/// Autopilot's mark: a plane, where a task's mark is round, a diamond or a sign and an automation's an arrow going round.
pub const PILOT_MARK: &str = "✈";
/// What a coordinator's row shows of its autopilot while it is on: its mark, its columns (the least needed first) and
/// its line under the name.
pub struct Shown { pub mark: Inked, pub cols: Vec<(&'static str, Inked)>, pub under: Inked }
/// The ink of a state: red where it needs you, yellow while it works (as a task's), plain text else.
fn ink_of(state: &str) -> Ink { match state { "needs you" | "could not start" => Ink::Red, "working" => Ink::Yellow, _ => Ink::Plain } }
/// What the row of the coordinator a heading stands for shows of its autopilot, while it is on.
pub fn shown(s: &State, t: &Theme, id: &str) -> Option<Shown> {
    let c = of_head(s, id)?;
    let v = view(s, &c);
    if !v.b("on") { return None; }
    let st = v.s("state");
    let ink = ink_of(st);
    let mark = Inked::of(PILOT_MARK, ink);
    let mark = match st { "needs you" | "could not start" => mark.bold(), "asleep" => mark.quiet(), _ => mark };
    // What it does now; a machine on an older version sends only its line.
    let said = if v.b("activity") { v.t("activity") } else { v.t("line") };
    let (word, rest) = said.split_once(" · ").map(|(a, b)| (a.to_string(), format!(" · {b}"))).unwrap_or((said.clone(), String::new()));
    let under = Inked::of(word, if ink == Ink::Red { Ink::Red } else { Ink::Soft }).add(Inked::of(rest, Ink::Soft));
    // The last task it added: with its mark and its tag where the board has it, as an automation's last task.
    let item = if v.b("lastId") { let at = if c.host.is_empty() { v.t("lastId") } else { format!("{}:{}", c.host, v.s("lastId")) }; s.items.iter().find(|x| x.s("id") == at) } else { None };
    let last = match item {
        Some(m) => super::fields::mark(t, m).add(Inked::plain(" ")).add(Inked::of(crate::board::tags::number_tag(m, &s.data.keys, &s.scope, false), Ink::Muted)),
        None if v.b("last") => Inked::of(format!("last {}", v.s("last")), Ink::Muted),
        None => Inked::new(),
    };
    let cols = [("added", Inked::of(v.t("added"), Ink::Muted)), ("last", last), ("next", Inked::of(v.t("next"), Ink::Muted))].into_iter().filter(|(_, x)| !x.is_empty()).collect();
    Some(Shown { mark, cols, under })
}
/// The main Coordinator's row's own words: its name, then what it sums up of the others.
pub fn main_head(s: &State, id: &str) -> Inked {
    let Some(t) = of_head(s, id) else { return Inked::new() };
    let sum = others(s, &t);
    let named = Inked::plain(t.name.clone()).bold();
    if sum.is_empty() { return named; }
    let (on, need) = sum.split_once(" · ").map(|(a, b)| (a.to_string(), format!(" · {b}"))).unwrap_or((sum, String::new()));
    named.add(Inked::of(format!(" · {on}"), Ink::Muted)).add(Inked::of(need, Ink::Red))
}
/// A coordinator's heading, drawn: `lead` (its caret), its name and what follows it, then a machine's rule (`rule`) to
/// the edge. On autopilot its mark goes before the name, its columns at the right edge (as many as fit) and its line
/// under it, with when it next wakes at its end where that column had no room.
pub fn placed(lead: Inked, named: Inked, pilot: Option<Shown>, rule: bool, width: usize) -> (Inked, Option<Inked>) {
    let drawn_rule = |n: usize| if rule && n >= 4 { Inked::plain(" ").add(Inked::of("─".repeat(n - 2), Ink::Muted)).add(Inked::plain(" ")) } else { Inked::spaces(n) };
    let Some(p) = pilot else {
        let mut out = lead.add(named);
        let rest = width.saturating_sub(out.width() + 2);
        if rule && rest > 0 { out = out.add(Inked::of(format!(" {}", "─".repeat(rest)), Ink::Muted)); }
        return (out.cut(width), None);
    };
    let next_all = p.next_words();
    let left = lead.add(p.mark).add(Inked::plain(" ")).add(named);
    let join = |c: &[(&str, Inked)]| { let mut out = Inked::new(); for (n, (_, x)) in c.iter().enumerate() { if n > 0 { out = out.add(Inked::spaces(2)); } out = out.add(x.clone()); } out };
    let mut cols = p.cols;
    while !cols.is_empty() && left.width() + 2 + join(&cols).width() > width { cols.remove(0); }
    let right = join(&cols);
    let first = if right.is_empty() { placed(left, Inked::new(), None, rule, width).0 } else { let gap = width.saturating_sub(left.width() + right.width()); left.add(drawn_rule(gap)).add(right).cut(width) };
    let next = if cols.iter().any(|(k, _)| *k == "next") { Inked::new() } else { next_all };
    (first, Some(Inked::spaces(5).add(p.under).add(next).cut(width)))
}
impl Shown {
    /// When it next wakes, said at the end of its line where its column had no room: ` · next in 40m`.
    fn next_words(&self) -> Inked { self.cols.iter().find(|(k, _)| *k == "next").map(|(_, x)| Inked::of(format!(" · {}", x.text()), Ink::Soft)).unwrap_or_default() }
}
/// Does this row take a line more for its autopilot (the list counts its room so)? While it is on, always.
pub fn extra(s: &State, r: &Row) -> usize {
    let Row::Head { id, .. } = r else { return 0 };
    usize::from(of_head(s, id).map(|c| view(s, &c).b("on")).unwrap_or(false))
}

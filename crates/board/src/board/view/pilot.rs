//! A machine's or a project's row (and `All machines`) as the list draws it: its coordinator, as a task's row is its
//! agent. The same marks as tasks, in the marks' column left of the caret, in their colours: `○` idle (also one that
//! never ran or was stopped), `◐` working, `?` it asks you (`!` it wants your OK), `✗` it could not start. On autopilot
//! it keeps that mark: autopilot shows only in the board's own patterns, a dimmed `[autopilot]` after its name (as a
//! task's `[research]`) and a status line under it in the grey of a task's (`Autopilot · next check-in in 12m · 3 of
//! 20 today`, `Working on AFS-2`, `Needs you · …`). On the right only the time: `3m` while it works, `1h ago` else,
//! nothing when it never ran. Off, one line: what it says is in the chat.
use crate::board::state::State;
use crate::board::theme::Theme;
use crate::board::update::coords::{look_of, of_head};
use crate::board::update::pilot::view;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};

/// What a coordinator's row shows: its mark (with the words a reader hears for it), its time, whether it is on
/// autopilot, and then its status line (empty else).
pub struct Shown { pub mark: Inked, pub label: &'static str, pub time: Inked, pub on: bool, pub under: Inked }
/// The tag after the name of a coordinator on autopilot, dimmed as a task's `[research]` is.
pub const PILOT_TAG: &str = "[autopilot]";
/// The row of the coordinator a heading stands for. None for a heading with none (a status group's).
pub fn shown(s: &State, t: &Theme, id: &str) -> Option<Shown> {
    let c = of_head(s, id)?;
    // No coordinator can run here (`coordinator.agent none`): this machine's rows are headings only.
    if c.host.is_empty() && !s.data.can_dispatch { return None; }
    // Another machine that sends nothing of its coordinators (an older version): the column blank. One it sends nothing of
    // never ran there: `○`.
    let sends = c.host.is_empty() || s.remotes.get(&c.host).is_some_and(|m| m.coords.is_object() && !m.coord.b("off"));
    if !sends { return Some(Shown { mark: Inked::plain(" "), label: "", time: Inked::new(), on: false, under: Inked::new() }); }
    let v = look_of(s, &c).unwrap_or_default();
    let p = view(s, &c);
    let on = p.b("on");
    let st = if on { p.s("state") } else { "" };
    let working = ["working", "starting"].contains(&v.state.as_str());
    let (mark, label) = if !v.error.is_empty() || st == "could not start" { (Inked::of(t.mark("error", ""), Ink::Red).bold(), "could not start") }
        else if v.state == "blocked" { (Inked::of(t.mark("blocked", ""), Ink::Red).bold(), "wants your OK") }
        else if !v.asks.is_empty() { (Inked::of(t.mark("blocked", "question"), Ink::Red).bold(), "asks you") }
        else if working { (Inked::of(t.mark("working", ""), Ink::Yellow), "working") }
        else { (Inked::of(t.mark("idle", ""), Ink::Muted), "idle") };
    // Something you did to it is under way (update/pending.rs): its row says so as a task's does, `Starting…`, the
    // working mark; one that did not work, in red (why is said under the chat, as an error is).
    if let Some(p) = s.pending.get(&c.key()) {
        return Some(match &p.failed { Some(why) => Shown { mark: Inked::of(t.mark("error", ""), Ink::Red).bold(), label: "could not do it", time: Inked::of(why.split(':').next().unwrap_or(why).to_string(), Ink::Red), on, under: Inked::new() },
            None => Shown { mark: Inked::of(t.mark("working", ""), Ink::Yellow), label: "under way", time: Inked::of(p.word.clone(), Ink::Muted), on, under: Inked::new() } });
    }
    let time = if working && v.began > 0 { Inked::of(crate::shown::words::age(s.now - v.began), Ink::Yellow) }
        else if v.at > 0 { Inked::of(format!("{} ago", crate::shown::words::age(s.now - v.at)), Ink::Muted) } else { Inked::new() };
    // Its status line, in the grey of a task's (row.rs: status_line). A machine on an older version sends none.
    let under = if on { Inked::of(crate::shown::words::clean(p.s("status")), Ink::Soft) } else { Inked::new() };
    Some(Shown { mark, label, time, on, under })
}
/// A heading, drawn: the marks' column (`row`; None, a heading with no coordinator, starts at its caret), its caret, its
/// name and what follows it, and its time at the right edge; the name gives way first, the mark and the time stay. A
/// coordinator on autopilot has its status line under it, starting under its name, cut at the edge.
pub fn placed(caret: Inked, named: Inked, row: Option<Shown>, width: usize) -> Vec<Inked> {
    let (mark, time, under) = match row { Some(r) => (Some(r.mark), r.time, r.under), None => (None, Inked::new(), Inked::new()) };
    let left = match mark { Some(m) => Inked::plain(" ").add(m).add(Inked::plain(" ")), None => Inked::plain(" ") }.add(caret);
    let lead = left.width();
    let room = width.saturating_sub(lead + if time.is_empty() { 0 } else { time.width() + 2 });
    let named = if named.width() > room { named.cut(room) } else { named };
    let gap = width.saturating_sub(lead + named.width() + time.width());
    let first = left.add(named).add(Inked::spaces(if time.is_empty() { 0 } else { gap })).add(time).cut(width);
    if under.is_empty() { return vec![first]; }
    vec![first, Inked::spaces(lead).add(under).cut(width)]
}

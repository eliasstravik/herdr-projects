//! The filter row under the header: `vm-str │ All 48   Coordinators 4   Needs you 3   Working 2   Ready 1   To do 12
//! Done 30   Automations 2`, whose board first (where there is another machine: `All machines`, or one machine),
//! the one shown lit, the others greyed with their counts (a count of tasks that need you in red). Where it has no
//! room, the names are said shorter (then shortest), those with nothing in them go, and where that is still too long only the one
//! shown stands, with the key that moves on. Where each is drawn is kept for a click (update/mouse.rs).
use crate::board::keys::key_name;
use crate::board::state::State;
use crate::board::theme::Theme;
use crate::board::update::shows::SHOWS;
use crate::shown::ink::{Ink, Inked};

/// What a click on the machine item says it is, among the filters' places.
pub const MACHINE: usize = usize::MAX;
pub const SHORT: [&str; 8] = ["All", "Coords", "Needs", "Working", "Ready", "To do", "Done", "Autos"];
const TINY: [&str; 8] = ["All", "Co", "Need", "Work", "Rdy", "Todo", "Done", "Au"];
/// Each filter's own colour, the one its rows and marks have: lit, it is the highlight's.
pub const INKS: [Ink; 8] = [Ink::Text, Ink::Accent, Ink::Red, Ink::Yellow, Ink::Green, Ink::Text, Ink::Text, Ink::Auto];
/// The row, `w` cells wide, and where each filter landed: (first cell, last cell, which).
pub fn shows_line(s: &State, _t: &Theme, w: usize) -> (Inked, Vec<(usize, usize, usize)>) {
    // `only`: 0 every filter, 1 those with something in them (and the one shown), 2 the one shown alone.
    // Whose board: the first item, where there is another machine (alt+m goes round, a click too).
    let machine = (!s.remotes.is_empty() || s.all()).then(|| {
        let named = if s.scope == "*" { "All machines".to_string() } else { crate::board::update::remotes::shown(s, &s.scope) };
        let ok = s.scope.is_empty() || s.scope == "*" || s.remotes.get(&s.scope).is_some_and(|m| m.ok);
        // In the base colour, no caret (yellow while that machine does not answer); a cell of room each side as every item.
        // One machine picked lights it as the filter shown is lit; All machines stays plain.
        let item = Inked::of(format!(" {named} "), if ok { Ink::Text } else { Ink::Yellow });
        if s.scope == "*" { item } else { item.bold().inv() }
    });
    // Every item keeps a cell of room on each side, lit or not: the lit one's highlight takes it (` Needs you 1 `), and
    // the row does not move when another is lit.
    let lay = |names: &[&str], only: u8| {
        let (mut line, mut hits) = (Inked::new(), vec![]);
        if let Some(m) = &machine { line = line.add(m.clone()); hits.push((0, line.width(), MACHINE)); line = line.add(Inked::of("│", Ink::Muted)); }
        let mut first = true;
        for (n, name) in names.iter().enumerate() {
            let count = s.show_n.get(n).copied().unwrap_or(0);
            if (only == 2 && n != s.show) || (only == 1 && n != s.show && n != 0 && count == 0) { continue; }
            // No dot between them: plain room, a cell of its own besides each one's own.
            if !first { line = line.add(Inked::plain(" ")); }
            first = false;
            let lit = n == s.show;
            // The one shown is lit in its own colour (reversed: the colour is its ground); the others greyed.
            let piece = if lit { Inked::of(format!(" {name} {count} "), INKS[n]).bold().inv() } else { Inked::of(format!(" {name}"), Ink::Muted).add(Inked::of(format!(" {count}"), if n == crate::board::update::shows::NEEDS && count > 0 { Ink::Red } else { Ink::Muted })).add(Inked::plain(" ")) };
            let from = line.width();
            line = line.add(piece);
            hits.push((from, line.width(), n));
        }
        if only == 2 { line = line.add(Inked::of(format!(" {} next", key_name("filter")), Ink::Muted)); }
        (line, hits)
    };
    let names: Vec<&str> = SHOWS.iter().map(|x| x.0).collect();
    for (list, only) in [(&names[..], 0), (&SHORT[..], 0), (&SHORT[..], 1), (&TINY[..], 0), (&TINY[..], 1), (&names[..], 2), (&SHORT[..], 2)] {
        let (line, hits) = lay(list, only);
        if line.width() <= w || (only == 2 && list == &SHORT[..]) { return (line.cut(w), hits.into_iter().filter(|h| h.1 <= w).collect()); }
    }
    unreachable!()
}

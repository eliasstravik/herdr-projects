//! The two tabs on the header line: `Board` and `Pinned`, the one shown lit as the filter shown is, the other greyed,
//! with how many of its rows need you, in red, where any do: what needs you in the tab not shown is never out of sight
//! (update/pinned.rs). Where each is drawn is kept for a click (update/mouse.rs).
use crate::board::state::State;
use crate::shown::ink::{Ink, Inked};

/// What a click on a tab says it is, among the filter row's places.
pub const BOARD: usize = usize::MAX - 1;
pub const PINNED: usize = usize::MAX - 2;
/// The tabs, and where each landed in them: (first cell, last cell, which). None in the archive, which is neither.
pub fn tabs(s: &State) -> (Inked, Vec<(usize, usize, usize)>) {
    if s.archive_view { return (Inked::new(), vec![]); }
    let (mut line, mut hits) = (Inked::new(), vec![]);
    for (name, needs, which, lit) in [("Board", s.tab_needs.0, BOARD, !s.pinned_view), ("Pinned", s.tab_needs.1, PINNED, s.pinned_view)] {
        if which == PINNED { line = line.add(Inked::plain(" ")); }
        let count = if needs > 0 { format!(" {needs}") } else { String::new() };
        let piece = if lit { Inked::of(format!(" {name} "), Ink::Text).bold().inv() } else { Inked::of(format!(" {name}"), Ink::Muted).add(Inked::of(count, Ink::Red)).add(Inked::plain(" ")) };
        let from = line.width();
        line = line.add(piece);
        hits.push((from, line.width(), which));
    }
    (line, hits)
}

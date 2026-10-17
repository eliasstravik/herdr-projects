//! The repo column: a task's repo name on its row, in an ink that is the same for the same repo, shown only in the
//! room the title, the columns and what the row says leave over (so the title never gives up a cell for it).
use super::row::Layout;
use crate::board::state::State;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::shown::words::repo_label;

/// The inks a repo name is drawn in, none of them a state's: the name is text, the ink only tells repos apart.
pub const PALETTE: [Ink; 6] = [Ink::Teal, Ink::Peach, Ink::Review, Ink::Green, Ink::Yellow, Ink::Accent];
/// The fewest cells a cut name is still worth showing in.
const NAME_MIN: usize = 4;

/// Whether the list shows the column: the setting is on and the view is not one repo (its header says which).
pub fn wanted(s: &State) -> bool {
    s.repos.len() != 1 && crate::store::settings::lookup::setting("board.repoColumn", None) == json!(true)
}
/// The ink of a repo's name: its letters folded into the palette, so it is the same on every row and every run.
pub fn ink_of(repo: &str) -> Ink {
    let h = repo.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619));
    PALETTE[(h ^ (h >> 16)) as usize % PALETTE.len()]
}
/// The repo cell for a row with `gap` cells to spare, and the gap left before it: two cells each side of the name,
/// the name cut to what is left, or no cell at all where that would be less than a few letters.
pub fn beside(r: &Obj, lay: &Layout, gap: usize) -> (Inked, usize) {
    if !lay.repo_col || r.has("auto") || r.s("repo").is_empty() || gap < NAME_MIN + 4 { return (Inked::new(), gap); }
    let name = Inked::of(repo_label(r.s("repo")), ink_of(r.s("repo"))).cut(gap - 4);
    if name.width() < NAME_MIN.min(repo_label(r.s("repo")).chars().count()) { return (Inked::new(), gap); }
    let rest = gap - name.width() - 2;
    (name.add(Inked::spaces(2)), rest)
}

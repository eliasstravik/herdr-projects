//! The lines the list's rows take while it is folded to its room: folding lays the list out again and again, fewer
//! rows each time, and each task's row is measured once in it (at each width it stands at), not in every round.
use crate::board::state::{Rec, Row, State};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

thread_local! { static MEASURED: Cell<usize> = const { Cell::new(0) }; }
/// How many rows this thread's folding has measured: what tests count.
pub fn measured() -> usize { MEASURED.with(|m| m.get()) }
/// The heights of one fold, by row and width: a row is its task's record, the same one in every round.
#[derive(Default)]
pub struct Heights(RefCell<HashMap<(usize, usize), usize>>);
impl Heights {
    /// The lines `rows` take in the list. An automation's are not counted: no task gives way for one.
    pub fn lines(&self, rows: &[Row], s: &State) -> usize {
        let ind = super::depth::indents(rows, s.list_width);
        rows.iter().enumerate().map(|(n, r)| match r { Row::Rec(o) => self.height(s, o, s.list_width.saturating_sub(ind[n])), Row::Auto(_) => 0, _ => 1 }).sum()
    }
    fn height(&self, s: &State, r: &Rec, width: usize) -> usize {
        *self.0.borrow_mut().entry((std::sync::Arc::as_ptr(r) as usize, width)).or_insert_with(|| {
            MEASURED.with(|m| m.set(m.get() + 1));
            crate::board::view::row_wrap::row_height(s, r, width, s.list_tall)
        })
    }
}

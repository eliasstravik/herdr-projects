//! How far in each row of the list stands, so that machine → project → task reads at a glance: a machine's row at the
//! left edge, a project's (and a status group's, an automations' group) one step in under its machine, and under a
//! project's row its tasks, automations and lines like `↓ 3 more done` one step further in; under a status group's
//! they stand where it does. A board of one machine has no machine's row: its groups stand at the edge.
use crate::board::state::Row;

/// The cells of one step in: two, one on a narrow list; never more than two steps.
pub fn step(width: usize) -> usize { if width < 60 { 1 } else { 2 } }
/// The steps in of each row.
pub fn depths(rows: &[Row]) -> Vec<usize> {
    let (mut base, mut under) = (0, 0);
    rows.iter().map(|r| match r {
        Row::Head { machine: true, .. } => { base = 1; under = 1; 0 }
        // A project's row (a repo's heading) puts its rows a step in; a status group's leaves them where it stands.
        Row::Head { id, .. } => { under = base + usize::from(id.starts_with(&format!("{}repo\t", super::heads::HEAD))); base }
        _ => under,
    }).collect()
}
/// The cells each row stands in, for a list `width` wide.
pub fn indents(rows: &[Row], width: usize) -> Vec<usize> { depths(rows).into_iter().map(|d| d.min(2) * step(width)).collect() }

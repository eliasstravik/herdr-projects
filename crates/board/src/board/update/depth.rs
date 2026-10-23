//! How far in each row of the list stands, so that all machines → machine → project → task reads at a glance: the top
//! row at the left edge (`All machines`, or the one machine shown), each machine's row one step in under `All
//! machines`, a project's (and a status group's, an automations' group) one step in under its machine, and under a
//! project's row its tasks, automations and lines like `↓ 3 done` one step further in; under a status group's
//! they stand where it does.
use crate::board::state::Row;

/// The cells of one step in: two, one on a narrow list; never more than three steps.
pub fn step(width: usize) -> usize { if width < 60 { 1 } else { 2 } }
/// The steps in of each row.
pub fn depths(rows: &[Row]) -> Vec<usize> {
    let (mut base, mut under, mut all) = (0, 0, 0);
    rows.iter().map(|r| match r {
        Row::Head { id, .. } if super::top::is_all(id) => { all = 1; base = 1; under = 1; 0 }
        Row::Head { machine: true, .. } => { base = all + 1; under = all + 1; all }
        // A project's row (a repo's heading) puts its rows a step in; a status group's leaves them where it stands.
        Row::Head { id, .. } => { under = base + usize::from(id.starts_with(&format!("{}repo\t", super::heads::HEAD))); base }
        _ => under,
    }).collect()
}
/// The cells each row stands in, for a list `width` wide.
pub fn indents(rows: &[Row], width: usize) -> Vec<usize> { depths(rows).into_iter().map(|d| d.min(3) * step(width)).collect() }

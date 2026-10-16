//! Where the selection lands when its task moves away (an action put it in another group, or elsewhere in the list,
//! or out of it): on the row where you were working, never after the task. The next task below in the same group, the
//! one above when it was the last, then the next group (a folded one's heading), then the group above. Pure.
use super::heads::is_head;
use crate::board::state::SelRow;

/// What the rows that are not tasks stand under: they are only landed on from one of their own. An automation's group
/// is this and its machine (`automations\totherbox`).
pub const AUTOS: &str = "automations";
fn aside(g: &str) -> bool { g.starts_with(AUTOS) || g == JOBS }
pub const JOBS: &str = "jobs";

/// The row the selection goes to from `old[b]`, the row it was on: one of `ids` (the rows the list has now), a heading
/// only when `folded` (it stands for the tasks it hides). None: no row is left to go to.
pub fn land(old: &[SelRow], b: usize, ids: &[String], folded: &dyn Fn(&str) -> bool) -> Option<String> {
    let g = &old[b].group;
    let here = |r: &&SelRow| ids.contains(&r.id) && r.id != old[b].id;
    let task = |r: &&SelRow| !is_head(&r.id);
    let row = |r: &&SelRow| if is_head(&r.id) { folded(&r.id) } else { true };
    // Rows of the selection's own kind first: from a task, never on to an automation or words on their way while a
    // task (or a folded group) is left.
    let kin = |r: &&SelRow| aside(&r.group) == aside(g);
    let below = &old[b + 1..];
    let above = || old[..b].iter().rev();
    below.iter().take_while(|r| r.group == *g).find(|r| here(r) && task(r))
        .or_else(|| above().take_while(|r| r.group == *g).find(|r| here(r) && task(r)))
        .or_else(|| below.iter().find(|r| r.group != *g && here(r) && kin(r) && row(r)))
        .or_else(|| above().find(|r| r.group != *g && here(r) && kin(r) && row(r)))
        .or_else(|| below.iter().chain(above()).find(|r| here(r) && row(r)))
        .map(|r| r.id.clone())
}

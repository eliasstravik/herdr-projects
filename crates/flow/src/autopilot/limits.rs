//! One way to count, everywhere (the panel, its row's status line, `autopilot` and `autopilot show`): the tasks it
//! **added** today of `autopilot.tasksPerDay`, and its tasks **under way** of `autopilot.tasksAtOnce`. No room: the tasks at once are
//! full, or today's adds are used up and it has none of your to-do tasks it may start (`autopilot.scope all`).
use super::care::{under_way_count, yours_to_start};
use super::record::{count, day_of, words};
use crate::coordinator::co::Co;
use crate::json::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Today { pub added: i64, pub per_day: i64, pub under_way: usize, pub at_once: i64 }
/// Its counts now. A record of another day counts nothing yet.
pub fn today(co: &Co, ap: &Obj, board: &[Obj], now: i64) -> Today {
    let added = if ap.s("day") == day_of(now) { ap.i("added") } else { 0 };
    Today { added, per_day: count(co, "autopilot.tasksPerDay"), under_way: under_way_count(ap, board), at_once: count(co, "autopilot.tasksAtOnce") }
}
impl Today {
    pub fn adds_left(&self) -> bool { self.added < self.per_day }
    pub fn slots_left(&self) -> bool { (self.under_way as i64) < self.at_once }
    /// "2 of 5 added · 1 of 2 under way".
    pub fn say(&self) -> String { format!("{} of {} added · {} of {} under way", self.added, self.per_day, self.under_way, self.at_once) }
}
/// Room to do anything new: a free slot, and an add left today or one of your to-do tasks it may start.
pub fn room(co: &Co, ap: &Obj, board: &[Obj], now: i64) -> bool {
    let t = today(co, ap, board, now);
    t.slots_left() && (t.adds_left() || (words(co, "autopilot.scope") == "all" && yours_to_start(co, board) > 0))
}
/// Every task of this machine, as kept (archived ones too: an archived task is over), and those of other machines an
/// open board heard lately (heard.rs), named `label:id` with their machine in `host`.
pub fn board_tasks() -> Vec<Obj> {
    let mut all = crate::store::tasks::list(crate::store::tasks::ListOpts { include_archived: true, ..Default::default() });
    all.extend(super::heard::far_tasks(crate::clock::now()));
    all
}
/// A task by its id, its number or its tag here, or another machine's (`vm-b:web/7`) as an open board last heard it.
pub fn find_task(id: &str) -> crate::err::Res<Obj> {
    if let Ok(t) = crate::store::tasks::find(id) { return Ok(t); }
    if crate::after::far::split(id).is_some() { if let Some(t) = super::heard::far_tasks(crate::clock::now()).into_iter().find(|t| t.s("id").eq_ignore_ascii_case(id)) { return Ok(t); } }
    crate::fail!("No task {id}. Another machine's task is named by its machine first (vm-b:web/7), and is known here as a board last heard it.")
}

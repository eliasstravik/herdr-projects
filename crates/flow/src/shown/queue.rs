//! A task's place in the queue for a free slot (`board.agents` all taken): `Q#1` gets the next slot, `Q#2` the one
//! after. The board starts waiting tasks in this order (actions/start.rs: start_queued), so the row, the task panel,
//! `list`, `details`, herdr's sidebar and the web Board all say the same place. A task delegated with `--after` is
//! queued too, after the tasks it waits for (after.rs): the same mark and colour, no place, and "queued after HSW-6". One
//! that waits for room on a disk (leftovers/room.rs) keeps its place, and says it waits for disk space, not a slot.
use crate::json::*;

/// Queued: started, no slot yet, or queued after other tasks to be done.
pub fn queued(r: &Obj) -> bool { (r.s("stage") == "queued" || waits_after(r)) && !r.b("archivedAt") }
/// Queued after other tasks to be done (after.rs), and nothing in the way: one whose wait went wrong waits for you.
pub fn waits_after(r: &Obj) -> bool { r.s("stage") == crate::after::STAGE && !r.b("afterBlocked") && !r.b("archivedAt") }
/// The waiting tasks of one herdr session, in the order they get a slot: the first to wait first.
pub fn order<'a>(all: impl IntoIterator<Item = &'a Obj>, socket: &str) -> Vec<&'a Obj> {
    let mut waiting: Vec<&Obj> = all.into_iter().filter(|t| t.s("stage") == "queued" && t.at("link").s("socket") == socket).collect();
    waiting.sort_by_key(|t| t.i("queuedAt"));
    waiting
}
/// Each waiting task's place, by its id: the tasks and their attempts (`tries`) together, as the slots count them.
pub fn places<'a>(all: impl IntoIterator<Item = &'a Obj>) -> Vec<(String, i64)> {
    let all: Vec<&Obj> = all.into_iter().collect();
    let mut sockets: Vec<&str> = all.iter().filter(|t| t.s("stage") == "queued").map(|t| t.at("link").s("socket")).collect();
    sockets.sort(); sockets.dedup();
    sockets.into_iter().flat_map(|s| order(all.iter().copied(), s).into_iter().enumerate().map(|(n, t)| (t.t("id"), n as i64 + 1)).collect::<Vec<_>>()).collect()
}
/// Writes each waiting task's place as `queuePlace` on the board's rows (and their attempts), and takes it off the rest.
pub fn number(rows: &mut [Obj]) {
    let all: Vec<Obj> = rows.iter().flat_map(|r| std::iter::once(r.clone()).chain(r.arr("tries").iter().filter_map(|c| c.as_object().cloned()))).collect();
    let at = places(all.iter());
    let place = |o: &mut Obj| match at.iter().find(|(id, _)| id == o.s("id")) { Some((_, n)) => o.put("queuePlace", *n), None => o.del("queuePlace") };
    for r in rows.iter_mut() {
        place(r);
        if let Some(tries) = r.get_mut("tries").and_then(|x| x.as_array_mut()) { for c in tries.iter_mut() { if let Some(o) = c.as_object_mut() { place(o); } } }
    }
}
/// `Q#2`, or nothing where the task does not wait or its place is not known.
pub fn tag(r: &Obj) -> String { if queued(r) && r.i("queuePlace") > 0 { format!("Q#{}", r.i("queuePlace")) } else { String::new() } }
/// It waits for room on a disk (leftovers/room.rs), not for a free slot.
pub fn waits_disk(r: &Obj) -> bool { r.s("stage") == "queued" && r.b("diskWait") && !r.b("archivedAt") }
/// What a task in the queue waits for, in words: "waiting for disk space", or "waiting for a free slot".
pub fn waits_for(r: &Obj) -> &'static str { if waits_disk(r) { DISK } else { WAITING } }
/// "waiting for a free slot, Q#2", "waiting for disk space, Q#1", or "queued after HSW-6": where a queued task stands, in words.
pub fn words(r: &Obj) -> String {
    if waits_after(r) { return crate::after::row_line(r); }
    let (w, q) = (waits_for(r), tag(r)); if q.is_empty() { w.into() } else { format!("{w}, {q}") }
}
/// The same in a word or two, for herdr's sidebar (its place is beside it): "queued", or "waiting for disk space".
pub fn short(r: &Obj) -> &'static str { if waits_disk(r) { DISK } else { crate::status_words::QUEUED } }
pub const WAITING: &str = "waiting for a free slot";
pub const DISK: &str = "waiting for disk space";

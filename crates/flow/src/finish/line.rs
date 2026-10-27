//! The line: a repo's tasks merge one at a time. One catches up with main, is checked and merges, then the next.
//! Without it every merge moves main under all the others at once: each catches up and runs the whole check, one of
//! them merges, and the rest start over.
use crate::clock::now;
use crate::err::Res;
use crate::json::*;
use crate::runner::{runner_alive, spawn_runner, Guard};
use crate::store::settings::lookup::setting;
use crate::store::files::{with_lock_wait, SLOT_STAGES};
use crate::store::repos::folder_of;
use crate::store::tasks::{all, get, list, update, Change, ListOpts, Task};

/// The stages of the last stretch: catching up with main, the check on the result, the push, the merge.
const LAST_STRETCH: [&str; 3] = ["merging", "checking", "pushing"];
/// At the head of the line: the turn is its own, taken by the very runner that is at work on it now.
fn at_head(t: &Task) -> bool { t.b("lineTurn") && t.at("lineTurn") == t.at("runner").at("started") && runner_alive(t) && LAST_STRETCH.contains(&t.s("stage")) && !t.b("archivedAt") }
/// Waiting in the line: it may merge, and another task goes first.
pub fn in_line(t: &Task) -> bool { t.s("stage") == "merging" && t.b("lineAt") && !t.b("autoMerge") && !t.b("archivedAt") && !at_head(t) }
/// An admitted land runner has the same bounded startup grace as other native runners.
fn reserved(t: &Task) -> bool {
    if t.at("lineReserved").is_null() { return false; }
    match t.at("lineReserved").as_i64() { Some(at) if at > 0 => now() - at <= crate::hook::GRACE_MS, _ => true }
}
/// Only a positively parked line waiter is free. Unknown/dead runner markers and ordinary working authors count.
pub fn parked(t: &Task) -> bool { in_line(t) && t.i("lineAt") > 0 && t.at("runner").is_null() && t.at("lineTurn").is_null() && !reserved(t) }
/// Shared author/landing admission uses this one predicate under the existing slots lock.
pub fn occupies_slot(t: &Task) -> bool {
    !t.b("archivedAt") && (runner_alive(t) || (SLOT_STAGES.contains(&t.s("stage")) && !t.b("autoMerge") && !parked(t)))
}
fn busy() -> i64 { list(ListOpts { attempts: true, ..Default::default() }).iter().filter(|t| occupies_slot(t)).count() as i64 }
/// Admit one land request, including manual approval, automatic merge and keeper handoffs. A reservation counts
/// before spawning and expires only before claim; a late runner must claim again under the same slots lock.
pub fn reserve(id: &str) -> bool {
    with_lock_wait("slots", 5000, || {
        let Some(cur) = get(id) else { return Ok(false) };
        if cur.s("stage") != "merging" || cur.b("archivedAt") || cur.b("autoMerge") || runner_alive(&cur) || reserved(&cur) { return Ok(false); }
        let me = update(id, |t| {
            if t.s("stage") != "merging" || t.b("archivedAt") || t.b("autoMerge") || runner_alive(t) || reserved(t) { return Ok(Change::Keep); }
            if !t.b("lineAt") { t.put("lineAt", now()); }
            t.put("lineTurn", Value::Null); t.put("lineReserved", Value::Null);
            // A dead marker is reconciled by the existing lost-runner path, never discounted here.
            Ok(Change::Write)
        })?;
        if !parked(&me) { return Ok(false); }
        let others = of_repo(&folder_of(id));
        let first = others.iter().filter(|t| t.s("id") != id).find(|t|
            at_head(t) || (in_line(t) && (runner_alive(t) || reserved(t) || by_arrival((t.i("lineAt"), t.s("id")), (me.i("lineAt"), id)).is_lt())));
        if let Some(first) = first { update(id, |t| { t.put("msg", waiting_msg(first.s("id"))); Ok(Change::Write) })?; return Ok(false); }
        if busy() >= setting("board.agents", None).as_i64().unwrap_or(4) {
            update(id, |t| { t.put("msg", "Waiting to merge: a shared slot must become free"); Ok(Change::Write) })?;
            return Ok(false);
        }
        let mut admitted = false;
        update(id, |t| {
            if parked(t) { t.put("lineReserved", now()); t.put("runnerAsked", now()); admitted = true; }
            Ok(Change::Write)
        })?;
        Ok(admitted)
    }).unwrap_or(false)
}
/// Consume a pending reservation and install the live marker atomically. Direct run-land entries reserve here.
/// Expired or superseded launches
/// do nothing; the native line pass can admit them afresh without letting an old process bypass the capacity check.
pub fn claim_land(id: &str) -> Res<Option<Task>> {
    with_lock_wait("slots", 5000, || {
        let Some(t) = get(id) else { return Ok(None) };
        if t.at("lineReserved").is_null() && !reserve(id) { return Ok(None); }
        let Some(t) = get(id) else { return Ok(None) };
        let fresh = t.at("lineReserved").as_i64().is_some_and(|at| at > 0 && now() - at <= crate::hook::GRACE_MS);
        if t.s("stage") != "merging" || t.b("archivedAt") || !fresh || runner_alive(&t) { return Ok(None); }
        crate::runner::claim(id, "land", |t| { t.put("lineReserved", Value::Null); })
    })
}
fn num_key(id: &str) -> (String, u64) { let (r, n) = id.split_once('/').unwrap_or((id, "0")); (r.to_string(), n.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0)) }
fn by_arrival(a: (i64, &str), b: (i64, &str)) -> std::cmp::Ordering { a.0.cmp(&b.0).then_with(|| num_key(a.1).cmp(&num_key(b.1))) }
fn of_repo(root: &std::path::Path) -> Vec<Task> { all().into_iter().filter(|t| !t.b("archivedAt") && folder_of(t.s("id")) == root).collect() }
/// Asked by the land step before it catches up with main. True: this task goes now, and holds the turn until its
/// runner ends. False: another task of the repo goes first, and this one waits where it is.
pub fn take_turn(id: &str, g: &mut Guard) -> Res<bool> {
    let root = folder_of(id);
    with_lock_wait(&format!("line:{}", root.display()), 10000, || {
        let me = g.check()?;
        if at_head(&me) { return Ok(true); }
        let others: Vec<Task> = of_repo(&root).into_iter().filter(|t| t.s("id") != id).collect();
        let mine_at = if me.b("lineAt") { me.i("lineAt") } else { now() };
        let mut waiting: Vec<&Task> = others.iter().filter(|t| in_line(t)).collect();
        waiting.sort_by(|a, b| by_arrival((a.i("lineAt"), a.s("id")), (b.i("lineAt"), b.s("id"))));
        let first = others.iter().find(|t| at_head(t)).or_else(|| waiting.into_iter().find(|t| by_arrival((t.i("lineAt"), t.s("id")), (mine_at, id)).is_lt()));
        match first {
            None => { g.set(|t| { let s = t.at("runner").at("started").clone(); t.put("lineTurn", s); t.put("lineAt", Value::Null); })?; Ok(true) }
            Some(f) => { let fid = f.t("id"); g.set(|t| { t.put("lineTurn", Value::Null); t.put("lineAt", mine_at); t.put("msg", waiting_msg(&fid)); })?; Ok(false) }
        }
    })
}
pub fn waiting_msg(first: &str) -> String { format!("Waiting to merge: {first} goes first") }
/// The line moves: where nobody is at the head, the task that waited longest is started (by the housekeeping pass).
pub fn line_pass(tasks: &[Obj]) {
    let mut roots: Vec<std::path::PathBuf> = vec![];
    for t in tasks { if in_line(t) { let r = folder_of(t.s("id")); if !roots.contains(&r) { roots.push(r); } } }
    for root in roots {
        let all = of_repo(&root);
        if all.iter().any(at_head) { continue; }
        let mut waiting: Vec<&Task> = all.iter().filter(|t| in_line(t)).collect();
        waiting.sort_by(|a, b| by_arrival((a.i("lineAt"), a.s("id")), (b.i("lineAt"), b.s("id"))));
        if let Some(next) = waiting.first() { if !runner_alive(next) { spawn_runner("land", next.s("id"), &[]); } }
    }
}

//! `add` in an autopilot turn: in its area, within `autopilot.tasksPerDay` and `autopilot.tasksAtOnce`, checked,
//! counted and added under the board's one lock for autopilot's adds (then the coordinator's own), so two quick adds
//! cannot both pass. A setting on it that `autopilot.mayChange` does not allow is not applied, and the answer names it.
//! With `autopilot.startAtOnce yes` the task starts at once, or waits after one of its own tasks. The task records which
//! coordinator's autopilot added it, when, and the `autopilot.repoFileWaits` in force.
use super::care::{under_way_count};
use super::limits::{board_tasks, today};
use super::record::{change, key_of, own_task, roll, words};
use crate::actions::filing::{add as add_task, AddOpts};
use crate::clock::now;
use crate::coordinator::co::Co;
use crate::err::Res;
use crate::json::*;
use crate::store::files::with_lock_wait;
use crate::store::settings::pilot_values::may_change;
use crate::store::tasks::{update, Change, Task};
use std::path::Path;

/// The board's one lock for autopilot's adds, taken before the coordinator's.
pub const LOCK: &str = "autopilot-adds";
pub struct Added { pub task: Task, pub start: bool, pub dropped: Vec<String> }
/// What an autopilot task records: who added it, when, and whether a change to the repo's file waits for you.
pub fn record_of(co: &Co, machine: &str) -> Value {
    json!({ "coordinator": key_of(co), "name": co.name(), "machine": machine, "at": now(), "repoFileWaits": words(co, "autopilot.repoFileWaits") != "no" })
}
/// Is the repo `name` in this coordinator's area?
fn in_area(co: &Co, repo: &str) -> bool { co.repo().map(|r| r.eq_ignore_ascii_case(repo)).unwrap_or(true) }
/// Adds the task as `filing::add` does, within autopilot's limits. `after`: the tasks it is to wait for, checked
/// already; `delegate`: asked to start it now.
pub fn add(co: &Co, here: &Path, title: &str, mut o: AddOpts, after: &[String], delegate: bool) -> Res<Added> {
    let (repo, _) = crate::store::repos::repo_of(&crate::store::repos::folder_for(&o.repo, here)?)?;
    if !in_area(co, &repo) { crate::refuse!("Refused in an autopilot wake: add in {repo}. The {} adds tasks only in its own area ({}).", co.name(), co.repo().unwrap_or("")); }
    let start = words(co, "autopilot.startAtOnce") == "yes" || delegate;
    with_lock_wait(LOCK, 10000, || {
        let board = board_tasks();
        let ap = super::record::get(co);
        let t = today(co, &ap, &board, now());
        if !t.adds_left() { crate::refuse!("Refused in an autopilot wake: add. Today's tasks are used: {} (autopilot.tasksPerDay).", t.say()); }
        if start && under_way_count(&ap, &board) as i64 >= t.at_once { crate::refuse!("Refused in an autopilot wake: add. Its tasks at once are full: {} (autopilot.tasksAtOnce).", t.say()); }
        if words(co, "autopilot.startAtOnce") == "yes" { if let Some(x) = after.iter().find(|a| super::record::task_rec(&ap, a).is_none()) { crate::refuse!("Refused in an autopilot wake: add --after {x}. A task it adds waits only for its own tasks (autopilot.startAtOnce yes)."); } }
        // Only what it may set is set; the rest is said, not applied.
        let may = words(co, "autopilot.mayChange");
        // `--research` is research.only said short: it keeps to the same rule.
        if let Some(r) = o.research.take() { o.settings.push((crate::research::RESEARCH.into(), json!(r))); }
        let (kept, dropped): (Vec<(String, Value)>, Vec<(String, Value)>) = o.settings.drain(..).partition(|(n, _)| may_change(&may, n));
        o.settings = kept;
        o.repo = repo.clone();
        let task = add_task(here, title, o)?;
        let rec = record_of(co, "");
        let task = update(task.s("id"), |x| { x.put("autopilot", rec.clone()); x.put("coordinator", key_of(co)); Ok(Change::Write) })?;
        let id = task.t("id");
        change(co, |ap, _| { roll(ap, now()); own_task(ap, &id, "added", now()); let n = ap.i("added") + 1; ap.put("added", n); true })?;
        Ok(Added { task, start, dropped: dropped.into_iter().map(|(n, _)| n).collect() })
    })
}
/// The answer's words for what was not applied.
pub fn dropped_words(d: &[String]) -> String {
    if d.is_empty() { return String::new(); }
    format!(" Not applied (autopilot.mayChange does not allow it): {}.", d.join(", "))
}
/// `add --on <machine>` in an autopilot turn: only the main Coordinator's area reaches another machine. Its slot is taken
/// here first, under the same locks, and given back if that machine refuses or does not answer; the task there carries
/// this coordinator's autopilot record, with this machine and the `autopilot.repoFileWaits` in force.
pub fn add_far(co: &Co, label: &str, title: &str, o: AddOpts, delegate: bool) -> Res<(Value, Vec<String>)> {
    if !co.is_top() { crate::refuse!("Refused in an autopilot wake: add --on {label}. Only the main Coordinator's area reaches other machines."); }
    let start = words(co, "autopilot.startAtOnce") == "yes" || delegate;
    let slot = format!("{label}:?{}", crate::util::uuid());
    let may = words(co, "autopilot.mayChange");
    let (kept, dropped): (Vec<(String, Value)>, Vec<(String, Value)>) = o.settings.into_iter().partition(|(n, _)| may_change(&may, n));
    with_lock_wait(LOCK, 10000, || {
        let board = board_tasks();
        let ap = super::record::get(co);
        let t = today(co, &ap, &board, now());
        if !t.adds_left() { crate::refuse!("Refused in an autopilot wake: add. Today's tasks are used: {} (autopilot.tasksPerDay).", t.say()); }
        if start && under_way_count(&ap, &board) as i64 >= t.at_once { crate::refuse!("Refused in an autopilot wake: add. Its tasks at once are full: {} (autopilot.tasksAtOnce).", t.say()); }
        change(co, |ap, _| { roll(ap, now()); own_task(ap, &slot, "added", now()); super::record::with_task(ap, &slot, |r| { r.put("pending", true); r.put("started", start); }); let n = ap.i("added") + 1; ap.put("added", n); true })?;
        Ok(())
    })?;
    let rec = record_of(co, &crate::remote::check::hostname());
    match crate::remote::add::add_there(label, title, &o.description, &o.repo, &kept, start, rec) {
        Ok(j) => {
            let id = format!("{label}:{}", j.s("id"));
            change(co, |ap, _| { super::record::with_task(ap, &slot, |r| { r.put("id", id.as_str()); r.del("pending"); }); true })?;
            Ok((j, dropped.into_iter().map(|(n, _)| n).collect()))
        }
        Err(e) => {
            // Given back: the slot and the add of the day.
            change(co, |ap, _| { let list: Vec<Value> = ap.arr("tasks").iter().filter(|r| r.s("id") != slot).cloned().collect(); ap.put("tasks", list); let n = (ap.i("added") - 1).max(0); ap.put("added", n); true })?;
            Err(e)
        }
    }
}

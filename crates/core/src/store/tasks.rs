//! Tasks on disk: reading, listing, changing, creating, and finding one by what it is called (its id, its number,
//! its repo's key).
use super::files::*;
use super::{attempts, keys, old_research, repos, task_cache, titles};
use super::settings::migrate;
use crate::clock::now;
use crate::err::{Error, Res};
use crate::json::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub type Task = Obj;

/// Minutes left, as the Watcher or a report gave them: a whole number from 0 up, or None when none was given.
pub fn minutes(v: &Value) -> Option<i64> {
    match v {
        Value::Null => None,
        Value::String(s) if s.is_empty() => None,
        _ => { let n = number(v); if n.is_finite() && (v.is_number() || v.as_str().map(|s| s.trim().parse::<f64>().is_ok()).unwrap_or(false)) { Some(n.round().clamp(0.0, 99999.0) as i64) } else { None } }
    }
}
/// A text as one line of at most `max` bytes: control characters become spaces.
pub fn one_line(text: &str, max: usize) -> String {
    let mut t = String::new();
    let mut gap = false;
    for c in text.chars() {
        if (c as u32) < 0x20 || (0x7f..=0x9f).contains(&(c as u32)) { if !gap { t.push(' '); gap = true; } } else { t.push(c); gap = false; }
    }
    let mut t = t.trim().to_string();
    while t.len() > max { t.pop(); }
    t
}
fn task_file(id: &str) -> PathBuf { dir_of(id).join("task.json") }
/// A task as it is on disk now, in the names of this version, or None.
pub fn get(id: &str) -> Option<Task> {
    let t = read_obj(&task_file(id))?;
    if !t.b("id") { return None; }
    Some(as_now(t))
}
/// A task as read, in the names of this version.
pub fn as_now(t: Task) -> Task { old_research::research_moved(migrate::moved_on_read(t)) }
#[derive(Default, Clone, Copy)]
pub struct ListOpts { pub include_archived: bool, pub archived: bool, pub attempts: bool }
/// Every task, newest first. `attempts`: the attempts of every task as well: they are no rows of the board, so only
/// what starts, watches and tidies agents asks for them.
pub fn list(o: ListOpts) -> Vec<Task> { shared(o).into_iter().map(Arc::unwrap_or_clone).collect() }
/// The same, as read once for every look in this process: for a look that only reads them (the board's each second),
/// with no copy of each task.
pub fn shared(o: ListOpts) -> Vec<Arc<Task>> {
    let mut out: Vec<Arc<Task>> = if o.attempts { attempts::all_attempts().into_iter().map(|a| Arc::new(migrate::moved_on_read(a))).collect() } else { vec![] };
    let mut ids = vec![];
    for repo in read_dir_names(&tasks_dir()) {
        for n in read_dir_names(&tasks_dir().join(&repo)) { if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) { ids.push(format!("{repo}/{n}")); } }
    }
    // Only the files changed since the last look are parsed (task_cache.rs).
    let keep = |t: &Task| { let archived = t.b("archivedAt"); !if o.archived { !archived } else { archived && !o.include_archived } };
    out.extend(task_cache::read(&ids, &keep));
    out.sort_by(|a, b| b.f("created").partial_cmp(&a.f("created")).unwrap_or(std::cmp::Ordering::Equal));
    out
}
pub fn all() -> Vec<Task> { list(ListOpts::default()) }
pub fn read_dir_names(dir: &Path) -> Vec<String> {
    fs::read_dir(dir).map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default()
}
/// A task by what it is called: its id, `HSW-134`, `#134` or `134`, an attempt's id. An error names what was asked.
pub fn find(reference: &str) -> Res<Task> {
    let s = reference.trim();
    let by_key = keys::id_of_key(s, None);
    // An attempt by its own id (`repo/12b`): the attempt, or, once it was chosen and is the task, the task.
    let low = match s.chars().last() { Some(c) if "ABCDE".contains(c) => format!("{}{}", &s[..s.len() - 1], c.to_ascii_lowercase()), _ => s.to_string() };
    if attempts::is_attempt_id(&low) {
        if let Some(a) = get(&low) { if a.b("chosen") { if let Some(t) = get(a.s("attemptOf")) { return Ok(t); } } return Ok(a); }
    }
    let plain_id = s.split_once('/').map(|(r, n)| !r.is_empty() && !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())).unwrap_or(false);
    if plain_id { if let Some(t) = get(s) { return Ok(t); } }
    if !by_key.is_empty() { if let Some(t) = get(&by_key) { return Ok(t); } }
    let num = s.trim_start_matches('#');
    list(ListOpts { include_archived: true, ..Default::default() }).into_iter()
        .find(|t| t.s("id").to_lowercase() == s.to_lowercase() || t.t("n") == num)
        .ok_or_else(|| Error::new(format!("No task {reference}")))
}
pub fn assert_unarchived(t: &Task) -> Res<()> {
    if t.b("archivedAt") { crate::fail!("{} is archived. Restore it before making changes.", t.s("id")); }
    Ok(())
}
/// What a change to a task does: write what `f` made of it, or nothing at all.
pub enum Change { Write, Keep }
/// The one way a task changes: a short read-modify-write under its lock. Never held across a check, a review or a
/// herdr call. `f` changes the task in place; returning `Change::Keep` writes nothing.
pub fn update_wait(id: &str, wait_ms: i64, f: impl FnOnce(&mut Task) -> Res<Change>) -> Res<Task> {
    with_task_lock(id, wait_ms, || {
        let mut t = get(id).ok_or_else(|| Error::new(format!("No task {id}")))?;
        let before = t.s("stage").to_string();
        // The Watcher's sentence about the agent's work outlives the guess it came with: whatever this change does to
        // `watch` (every new run drops it), the sentence is kept with its time (status_kept.rs).
        if t.at("watch").s("role") == "agent" && !t.at("watch").s("msg").is_empty() {
            let w = t.at("watch").clone();
            t.put("lastLine", json!({ "msg": w.s("msg"), "at": w.at("at") }));
        }
        if let Change::Keep = f(&mut t)? { return Ok(t); }
        if t.s("stage") != before {
            t.put("at", now());
            if !STAGES.contains(&t.s("stage")) { crate::fail!("Unknown stage {}", t.s("stage")); }
        }
        write_json(&task_file(id), &Value::Object(t.clone()))?;
        Ok(t)
    })
}
pub fn update(id: &str, f: impl FnOnce(&mut Task) -> Res<Change>) -> Res<Task> { update_wait(id, 2000, f) }
/// A change that always writes.
pub fn patch(id: &str, f: impl FnOnce(&mut Task)) -> Res<Task> { update(id, |t| { f(t); Ok(Change::Write) }) }
/// Fields set on a task, as one change.
pub fn set(id: &str, fields: Value) -> Res<Task> { patch(id, |t| { if let Value::Object(o) = fields { for (k, v) in o { t.insert(k, v); } } }) }
/// Writes a task as it is, under its lock (an attempt made, a record carried from another machine).
pub fn save(t: &Task) -> Res<()> { write_json(&task_file(t.s("id")), &Value::Object(t.clone())) }

/// A new task in the repo of `folder`, as to do.
pub fn create(folder: &Path, title: &str, description: &str, owner: Option<&str>) -> Res<Task> {
    let (name, root) = repos::repo_of(folder)?;
    super::archived_repos::refuse_archived(&name)?;
    let t = titles::titled(title, description);
    if t.title.is_empty() { crate::fail!("A task needs a title."); }
    with_lock(&format!("seq:{name}"), || {
        let seq = tasks_dir().join(&name).join("seq");
        let n = read_json(&seq).map(|v| number(&v) as i64).unwrap_or(0) + 1;
        write_json(&seq, &json!(n))?;
        let now = now();
        let task = obj(json!({ "id": format!("{name}/{n}"), "repo": name, "n": n, "root": root.display().to_string(), "title": t.title,
            "description": t.description, "owner": owner.map(String::from).unwrap_or_else(me), "app": null,
            "stage": "todo", "msg": "", "created": now, "at": now, "pinned": false, "archivedAt": null, "settings": {} }));
        save(&task)?;
        Ok(task)
    })
}
pub fn remove(id: &str) { let _ = fs::remove_dir_all(dir_of(id)); }

//! What you do to a follow-up: add it as a task (in its repo, not started, the task it came from named in its
//! description), hide it, or open it again; and `followups.add todo`, which adds every open one as a task when its
//! task merges. Nothing here starts an agent: you decide what becomes work.
use super::{ADDED, HIDDEN, OPEN};
use crate::actions::filing::{add, AddOpts};
use crate::clock::now;
use crate::err::Res;
use crate::json::*;
use crate::store::keys::{repo_keys, say_id};
use crate::store::repos::known_repos;
use crate::store::settings::lookup::setting;
use crate::store::tasks::{find, get, list, update, Change, ListOpts, Task};
use std::path::PathBuf;

/// The follow-up numbered `n` of a task, or why there is none.
fn item(t: &Task, n: &str) -> Res<Value> {
    let tag = say_id(t.s("id"), &repo_keys());
    if t.arr("followups").is_empty() { crate::fail!("{tag} has no follow-ups."); }
    let k: i64 = n.trim().trim_start_matches('#').parse().map_err(|_| crate::err::Error::from(format!("A follow-up is named by its number: herdr-projects followups add {tag} 1")))?;
    t.arr("followups").iter().find(|f| f.i("n") == k).cloned().ok_or_else(|| crate::err::Error::from(format!("{tag} has no follow-up {k}: it has {}.", t.arr("followups").len())))
}
/// Changes the follow-up numbered `n` of the task `id`.
fn change(id: &str, n: i64, f: impl Fn(&mut Value)) -> Res<Task> {
    update(id, |t| {
        let mut list = t.arr("followups").to_vec();
        let Some(x) = list.iter_mut().find(|x| x.i("n") == n) else { return Ok(Change::Keep) };
        f(x);
        t.put("followups", list);
        Ok(Change::Write)
    })
}
/// The repo a follow-up's task goes in: its own where the board knows it, else the repo of the task it came from.
fn repo_for(t: &Task, f: &Value) -> (String, bool) {
    if f.b("repo") { if let Some(k) = known_repos().keys().find(|k| k.to_lowercase() == f.s("repo").to_lowercase()) { return (k.clone(), true); } }
    (t.t("repo"), false)
}
/// Adds the follow-up as a task, not started. The new task and the words that say so.
pub fn add_as_task(id: &str, n: &str) -> Res<(Task, String)> {
    let t = find(id)?;
    let f = item(&t, n)?;
    let keys = repo_keys();
    let tag = say_id(t.s("id"), &keys);
    if f.s("state") == ADDED { crate::fail!("Follow-up {} of {tag} was added already, as {}.", f.i("n"), say_id(f.s("task"), &keys)); }
    let (repo, own) = repo_for(&t, &f);
    let elsewhere = if f.b("repo") && !own { format!("\n\nIt belongs to the repo \"{}\", which this board does not know: it was added here instead.", f.s("repo")) } else { String::new() };
    let description = format!("{}{}\n\nA follow-up of {tag} ({}), from the \"{}\" list of its closing note{}. That task's details: herdr-projects details {tag}{elsewhere}",
        f.s("text"), if f.b("blocks") { "\n\nThat task's change is not complete or usable without it." } else { "" }, t.s("title"), if f.s("kind") == "noticed" { "Also noticed" } else { "Still open" }, if t.b("pr") { format!(", merged in {}", t.s("pr")) } else { String::new() });
    let made = add(&PathBuf::from(t.s("root")), f.s("text"), AddOpts { description, repo, ..Default::default() })?;
    let k = f.i("n");
    change(t.s("id"), k, |x| { x["state"] = json!(ADDED); x["task"] = json!(made.s("id")); x["at"] = json!(now()); })?;
    let said = format!("{tag}: follow-up {k} added as {}, not started", say_id(made.s("id"), &repo_keys()));
    Ok((made, said))
}
/// Hides the follow-up (`open`: false) or opens it again (true).
pub fn hide(id: &str, n: &str, open: bool) -> Res<(Task, String)> {
    let t = find(id)?;
    let f = item(&t, n)?;
    let tag = say_id(t.s("id"), &repo_keys());
    if f.s("state") == ADDED { crate::fail!("Follow-up {} of {tag} is a task already ({}).", f.i("n"), say_id(f.s("task"), &repo_keys())); }
    let k = f.i("n");
    let t = change(t.s("id"), k, |x| { x["state"] = json!(if open { OPEN } else { HIDDEN }); x["at"] = json!(now()); })?;
    Ok((t, format!("{tag}: follow-up {k} {}", if open { "is open again" } else { "hidden" })))
}
/// The task merged: with `followups.add todo` every open follow-up is added as a task, not started.
pub fn on_merged(id: &str) {
    let Some(t) = get(id) else { return };
    if setting("followups.add", Some(&t)) != json!("todo") { return; }
    for f in super::open_of(&t) { let _ = add_as_task(id, &f.t("n")); }
}
/// Every finished task with open follow-ups (of one repo: `repo`), archived ones too, the newest first.
pub fn all_open(repo: Option<&str>) -> Vec<Task> {
    list(ListOpts { include_archived: true, ..Default::default() }).into_iter()
        .filter(|t| super::shown_count(t) > 0 && repo.map(|r| t.s("repo").eq_ignore_ascii_case(r)).unwrap_or(true)).collect()
}

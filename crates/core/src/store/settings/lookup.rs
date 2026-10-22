//! The one lookup. A work setting is looked up in one order and the nearest answer wins:
//!   this task -> your entry for this repo -> you -> the repo's own file -> built in
//! A board setting is yours only: you -> built in. A repo's own fact (its key) is said for that repo only: your entry
//! for the repo -> the repo's own file, as it is in the repo's folder now -> built in. A coordinator's setting:
//! this time -> this coordinator -> you -> built in (coord.rs). No setting has a lookup of its own. A setting is read when its step starts: callers ask here at that moment and keep nothing.
use super::coord::{once_names, Coord};
use super::places::{read, repo_file, yours, Mine, Problem};
use super::values::{fits, known, NONE};
use crate::err::Res;
use crate::json::*;
use crate::store::files::{dir_of, read_obj};
use crate::store::repos::{folder_of, is_repo, known_repos};
use crate::store::settings_list::{needs_repo, work as work_names, Kind, Ty, SETTINGS};
use crate::store::tasks::Task;
use std::cell::OnceCell;
use std::path::PathBuf;
use std::sync::Arc;

/// Whose settings: one task's, a repo's (by its name on the board), one coordinator's, or yours.
#[derive(Clone, Default)]
pub struct Scope<'a> { pub task: Option<&'a Task>, pub repo: String, pub coord: Option<Coord> }
impl<'a> Scope<'a> {
    pub fn you() -> Self { Scope::default() }
    pub fn task(t: &'a Task) -> Self { Scope { task: Some(t), ..Default::default() } }
    pub fn repo(name: &str) -> Self { Scope { repo: name.to_string(), ..Default::default() } }
    pub fn coord(c: Coord) -> Self { Scope { coord: Some(c), ..Default::default() } }
}
fn repo_name_of(t: &Task) -> String { let r = t.s("repo"); if !r.is_empty() { r.to_string() } else { t.s("id").split('/').next().unwrap_or("").to_string() } }
/// A task in no repo skips the repo places, and where it works, how it is checked and how it lands mean nothing to it.
pub fn in_repo(t: &Task) -> bool { is_repo(&folder_of(t.s("id"))) }

/// One place a setting can be said in, nearest first.
pub struct Place { pub place: &'static str, pub from: String, values: OnceCell<Obj>, load: Option<Box<dyn Fn() -> Obj>> }
impl Place {
    fn now(place: &'static str, from: impl Into<String>, values: Obj) -> Place { let c = OnceCell::new(); let _ = c.set(values); Place { place, from: from.into(), values: c, load: None } }
    fn later(place: &'static str, from: impl Into<String>, load: impl Fn() -> Obj + 'static) -> Place { Place { place, from: from.into(), values: OnceCell::new(), load: Some(Box::new(load)) } }
    /// What this place says: read only when a setting gets that far down.
    pub fn values(&self) -> &Obj { self.values.get_or_init(|| self.load.as_ref().map(|f| f()).unwrap_or_default()) }
}
pub struct Places { pub work: Vec<Place>, pub board: Vec<Place>, pub repo: Vec<Place>, pub coord: Vec<Place>, pub no_repo: bool, pub problems: Vec<Problem> }
fn builtin() -> Place { Place::now("builtin", "built in", SETTINGS.iter().map(|s| (s.name.clone(), s.builtin.clone())).collect()) }

/// The places that count for a scope, worked out once for all the settings asked of it.
pub fn places_of(scope: &Scope, mine: &Mine) -> Places {
    let task = scope.task;
    let name = task.map(repo_name_of).unwrap_or_else(|| scope.repo.clone());
    let repo_places = task.map(in_repo).unwrap_or(true);
    let mut problems = vec![];
    let work_list = work_names();
    let mut held = |o: &Value, place: &str| { let (v, p) = read(o, place, &work_list, "is a board setting, yours only: it cannot be said here"); problems.extend(p); v };
    let mut work = vec![];
    if let Some(t) = task {
        // An attempt has its task's settings, as they read now, and its own program (store/attempts.rs).
        let said = if t.b("attemptOf") {
            let mut s = read_obj(&dir_of(t.s("attemptOf")).join("task.json")).and_then(|p| p.o("settings").cloned()).unwrap_or_default();
            for (k, v) in t.o("settings").cloned().unwrap_or_default() { s.insert(k, v); }
            Value::Object(s)
        } else { t.at("settings").clone() };
        work.push(Place::now("task", "this task", held(&said, "this task")));
    }
    // Carried with the task from another machine: that machine's "you" and its entry for the repo, already worked out.
    if let Some(c) = task.filter(|t| t.b("carried")).map(|t| t.at("carried")) {
        let from = if c.s("from").is_empty() { "another machine".to_string() } else { c.s("from").to_string() };
        work.push(Place::now("carried", format!("sent from {from}"), held(c.at("settings"), "sent with this task")));
    } else {
        if !name.is_empty() && repo_places { if let Some(r) = mine.repos.get(&name) { work.push(Place::now("repo", format!("your entry for {name}"), r.clone())); } }
        work.push(Place::now("you", "you", mine.you.clone()));
    }
    if let (Some(t), true) = (task, repo_places) {
        let t = t.clone();
        work.push(Place::later("file", "the repo's file", move || match t.o("repoFile") { Some(f) => f.clone(), None => repo_file(&folder_of(t.s("id")), None).0 }));
    }
    work.push(builtin());
    let mut own = vec![];
    if !name.is_empty() { if let Some(r) = mine.repos.get(&name) { own.push(Place::now("repo", format!("your entry for {name}"), r.clone())); } }
    if !name.is_empty() { if let Some(root) = known_repos().get(&name) { let root = PathBuf::from(text(root)); own.push(Place::later("file", "the repo's file", move || repo_file(&root, None).0)); } }
    own.push(builtin());
    // A coordinator's: what autopilot holds for this time, then its own entry, then you.
    let mut coord = vec![];
    if let Some(c) = &scope.coord {
        if !c.once.is_empty() { let (v, p) = read(&Value::Object(c.once.clone()), "this time", &once_names(), "cannot be said for this time"); problems.extend(p); coord.push(Place::now("once", "this time", v)); }
        if let Some(o) = mine.coords.get(&c.key) { coord.push(Place::now("coord", c.place(), o.clone())); }
    }
    coord.push(Place::now("you", "you", mine.you.clone()));
    coord.push(builtin());
    Places { work, board: vec![Place::now("you", "you", mine.you.clone()), builtin()], repo: own, coord, no_repo: task.is_some() && !repo_places, problems }
}

#[derive(Clone, Debug)]
pub struct Found { pub name: String, pub value: Value, pub place: String, pub from: String, pub applies: bool, pub parts: Vec<(String, Value)>, pub skipped: Vec<String> }

/// A setting's value for a scope, with the place it came from. Instructions add up, broadest first; everything else
/// is the nearest answer.
pub fn lookup_at(name: &str, scope: &Scope, at: &Places) -> Res<Found> {
    let entry = known(name)?;
    if at.no_repo && needs_repo(name) { return Ok(Found { name: name.into(), value: entry.builtin.clone(), place: "builtin".into(), from: "built in".into(), applies: false, parts: vec![], skipped: vec![] }); }
    let list = match entry.kind { Kind::Work => &at.work, Kind::Board => &at.board, Kind::Repo => &at.repo, Kind::Coord => &at.coord };
    // One task's own setting (`merge.closes`) is said by that task alone: a broader place's answer is never read.
    let mut said: Vec<&Place> = list.iter().filter(|p| p.values().contains_key(name) && (!entry.task || ["task", "builtin"].contains(&p.place))).collect();
    // An effort and an access are right or wrong for one program: an answer that does not fit the program the role has
    // is not counted, and the next place down is asked (the built-in one fits every program). Except `auto`: the next
    // place down could be `full`, so an agent is never let act without asking where its auto mode was asked for. It
    // stays, and that agent is refused at its start (agents/start.rs).
    let mut skipped = vec![];
    if entry.ty == Ty::Effort || entry.ty == Ty::Access {
        let agent = text(&lookup_at(&format!("{}.agent", name.split('.').next().unwrap_or("")), scope, at)?.value);
        said.retain(|p| {
            let v = text(&p.values()[name]);
            let why = if p.place == "builtin" || (entry.ty == Ty::Access && v == "auto") { String::new() } else { fits(name, &v, &agent, false) };
            if !why.is_empty() { skipped.push(format!("{}: {name}: {why}", if p.from == "you" { "your settings" } else { &p.from })); }
            why.is_empty()
        });
    }
    if entry.adds {
        // `none` in a place cancels what the broader places said.
        let mut parts: Vec<&Place> = vec![];
        for p in said.iter().rev() { if p.values()[name] == json!(NONE) { parts.clear(); } else { parts.push(p); } }
        let near = said[0];
        let value = if parts.is_empty() { json!(NONE) } else { json!(parts.iter().map(|p| text(&p.values()[name])).collect::<Vec<_>>().join("\n\n")) };
        let from = if parts.len() > 1 { parts.iter().map(|p| p.from.clone()).collect::<Vec<_>>().join(" + ") } else { parts.first().copied().unwrap_or(near).from.clone() };
        return Ok(Found { name: name.into(), value, place: near.place.into(), from, applies: true, parts: parts.iter().map(|p| (p.from.clone(), p.values()[name].clone())).collect(), skipped: vec![] });
    }
    let p = said[0];
    Ok(Found { name: name.into(), value: p.values()[name].clone(), place: p.place.into(), from: p.from.clone(), applies: true, parts: vec![], skipped })
}
pub fn lookup(name: &str, scope: &Scope, mine: &Mine) -> Res<Found> { lookup_at(name, scope, &places_of(scope, mine)) }
/// A setting's value for one task (or for you, with none).
pub fn setting(name: &str, task: Option<&Task>) -> Value {
    let scope = task.map(Scope::task).unwrap_or_default();
    lookup(name, &scope, &yours()).map(|f| f.value).unwrap_or(Value::Null)
}
/// A setting's value as words.
pub fn setting_text(name: &str, task: Option<&Task>) -> String { text(&setting(name, task)) }
/// Every work setting of one task (or of you, with no task), by name.
pub fn work(task: Option<&Task>) -> Obj {
    let mine = yours();
    let scope = task.map(Scope::task).unwrap_or_default();
    let at = places_of(&scope, &mine);
    work_names().into_iter().filter_map(|n| lookup_at(n, &scope, &at).ok().map(|f| (n.to_string(), f.value))).collect()
}
/// What only the places above the repo's file say, for a repo: what travels with a task to another machine.
pub fn to_send(repo: &str, mine: &Arc<Mine>) -> Obj {
    let mut out = Obj::new();
    for n in work_names() {
        let here = mine.repos.get(repo).and_then(|r| r.get(n));
        let you = mine.you.get(n);
        let adds = crate::store::settings_list::entry_of(n).map(|e| e.adds).unwrap_or(false);
        let v = match (here, you) {
            (Some(h), Some(y)) if adds && *h != json!(NONE) && *y != json!(NONE) => Some(json!(format!("{}\n\n{}", text(y), text(h)))),
            (Some(h), _) => Some(h.clone()),
            (None, y) => y.cloned(),
        };
        if let Some(v) = v { out.insert(n.to_string(), v); }
    }
    out
}

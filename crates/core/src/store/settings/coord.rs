//! A coordinator's settings (`coordinator.*` and `autopilot.*`), looked up as a task's are, nearest first:
//!   this time -> this coordinator -> you -> built in
//! "This coordinator" is its own entry in the `coordinators` part of your config.json (`main`, `machine`,
//! `projects.<repo>`); "this time" the one-off `autopilot.*` values autopilot keeps while it is on, which the caller
//! hands in (sw-flow keeps them in the coordinator's own state). No coordinator's setting is ever read from a repo's file.
use super::lookup::Scope;
use crate::json::*;
use crate::store::settings_list::{coord, Entry, Kind};

/// Whose coordinator: its key as config.json's `coordinators` names it ("main", "machine", "project:<repo>"), and the
/// values autopilot holds for this time.
#[derive(Clone, Debug, Default)]
pub struct Coord { pub key: String, pub once: Obj }
impl Coord {
    pub fn of(key: &str) -> Coord { Coord { key: key.to_string(), once: Obj::new() } }
    /// Its words, as the settings page and `show` say where a value came from.
    pub fn place(&self) -> &'static str { "this coordinator" }
    /// Its part of config.json, as the path to it: `coordinators.main`, `coordinators.projects.<repo>`.
    pub fn path(&self) -> Vec<String> {
        match self.key.strip_prefix("project:") { Some(r) => vec!["coordinators".into(), "projects".into(), r.into()], None => vec!["coordinators".into(), self.key.clone()] }
    }
}
/// The one setting never said for you nor for this time: a goal is one coordinator's own.
pub const GOAL: &str = "autopilot.goal";
/// What "this time" may hold: autopilot's values, but its goal.
pub fn once_names() -> Vec<&'static str> { coord().into_iter().filter(|n| n.starts_with("autopilot.") && *n != GOAL).collect() }
/// What your own settings may say of a coordinator's: all but the goal.
pub fn yours_names() -> Vec<&'static str> { coord().into_iter().filter(|n| *n != GOAL).collect() }
/// The `coordinators` part of config.json, by key: "main", "machine", "project:<repo>".
pub fn entries_of(v: &Value) -> Vec<(String, Value)> {
    let mut out = vec![];
    let Some(m) = v.as_object() else { return out };
    for (k, o) in m {
        if k == "projects" { for (r, p) in o.as_object().cloned().unwrap_or_default() { out.push((format!("project:{r}"), p)); } } else { out.push((k.clone(), o.clone())); }
    }
    out
}
/// The words of one coordinator's entry, as a person reads where it is.
pub fn entry_words(key: &str) -> String {
    match key { "main" => "the Coordinator's own".into(), "machine" => "the machine coordinator's own".into(), k => format!("{} coordinator's own", k.strip_prefix("project:").unwrap_or(k)) }
}
/// Why a setting cannot be set (or unset) in this scope, as a coordinator's settings go: a coordinator's setting only
/// for you or for one coordinator (its goal for one coordinator only), and one coordinator takes nothing else.
pub fn refusal(e: &Entry, scope: &Scope) -> Option<String> {
    let name = &e.name;
    let flags = "--main-coordinator, --machine-coordinator or --project-coordinator <repo>";
    if e.kind == Kind::Coord && (scope.task.is_some() || !scope.repo.is_empty()) {
        return Some(format!("{name} is a coordinator's setting: it cannot be set {}. Set it for yourself (every coordinator), or for one coordinator with {flags}.", if scope.task.is_some() { "on a task" } else { "for a repo" }));
    }
    if e.kind != Kind::Coord && scope.coord.is_some() { return Some(format!("{name} is not a coordinator's setting: only coordinator.* and autopilot.* are said for one coordinator.")); }
    if *name == GOAL && scope.coord.is_none() { return Some(format!("{GOAL} is said for one coordinator only: set it with {flags}, or tell that coordinator its goal.")); }
    None
}

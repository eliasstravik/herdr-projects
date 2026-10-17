//! Per-role environment scopes for readers: pinned maps, nested and thread-local, never process-wide environment edits.
use crate::json::*;
use crate::store::settings::lookup::setting;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::rc::Rc;

thread_local! { static CURRENT: RefCell<Obj> = RefCell::new(Obj::new()); }
/// Cannot cross threads. Drop restores the caller's environment even when a read unwinds.
pub struct Scope { before: Obj, _thread: PhantomData<Rc<()>> }
impl Drop for Scope { fn drop(&mut self) { CURRENT.with(|c| *c.borrow_mut() = std::mem::take(&mut self.before)); } }
pub fn enter(value: &Value) -> Scope {
    let env = value.as_object().cloned().unwrap_or_default();
    let before = CURRENT.with(|c| c.replace(env));
    Scope { before, _thread: PhantomData }
}
pub fn current() -> Obj { CURRENT.with(|c| c.borrow().clone()) }
/// Some(None) is an explicit unset; None means this profile does not manage that variable.
pub fn variable(key: &str) -> Option<Option<PathBuf>> { CURRENT.with(|c| c.borrow().get(key).map(|v| v.as_str().filter(|s| !s.is_empty()).map(PathBuf::from))) }
pub fn home() -> Option<PathBuf> { variable("CLAUDE_CONFIG_DIR").map(|p| p.unwrap_or_else(|| crate::store::files::user_home().join(".claude"))) }
pub fn trust_file() -> Option<PathBuf> { variable("CLAUDE_CONFIG_DIR").map(|p| p.map(|p| p.join(".claude.json")).unwrap_or_else(|| crate::store::files::user_home().join(".claude.json"))) }
pub fn configured(t: Option<&Obj>, role: &str) -> Value {
    let name = text(&setting(&format!("{role}.profile"), t));
    crate::agents::profiles::resolve(&name).unwrap_or_default()
}
/// At a new launch, a missing profile must be an error, not a silent fallback to the primary account.
pub fn for_start(t: Option<&Obj>, role: &str) -> crate::err::Res<Value> {
    use crate::store::settings::lookup::{places_of, lookup_at, Scope};
    let mine = crate::store::settings::places::yours();
    let scope = t.map(Scope::task).unwrap_or_else(Scope::you);
    let places = places_of(&scope, &mine);
    let name = format!("{role}.profile");
    if let Some(p) = mine.problems.iter().chain(places.problems.iter()).find(|p| p.name == name) { crate::fail!("{}: {}", p.place, p.why); }
    let found = lookup_at(&name, &scope, &places)?;
    crate::agents::profiles::resolve(found.value.as_str().unwrap_or("none"))
}
pub fn selected(t: &Obj, role: &str) -> Value {
    t.at("agentEnvs").get(role).cloned().unwrap_or_else(|| configured(Some(t), if role == "agent" { "worker" } else { role }))
}
pub fn task(t: &Obj, role: &str) -> Scope { enter(&selected(t, role)) }
/// Apply the resolved map to a direct child (Watcher), including the other profiles' unsets.
pub fn command(c: &mut std::process::Command) {
    for (k, v) in current() { match v.as_str() { Some(s) => { c.env(k, s); } None => { c.env_remove(k); } } }
}

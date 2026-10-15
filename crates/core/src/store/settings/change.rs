//! Changing a setting: `set` and `unset`, for you, for your entry for a repo, or for one task. Every value is checked
//! first; several changes to one task are all checked before any is saved, so a refused request changes nothing.
use super::lookup::{in_repo, lookup, Found, Scope};
use super::places::{write_yours, yours};
use super::values::{check, fits, known, models_fit, nearest, NONE};
use crate::err::{Error, Res};
use crate::json::*;
use crate::research::turn_refusal;
use crate::store::keys::{repo_keys, said_keys};
use crate::store::keys_given::give_keys;
use crate::store::old_research::RESEARCH;
use crate::store::repos::known_repos;
use crate::store::runs::keep_run_times;
use crate::store::settings_list::{entry_of, needs_repo, Entry, Kind, Ty};
use crate::store::tasks::{assert_unarchived, update, Change, Task};

fn refuse<T>(msg: String) -> Res<T> { Err(Error::refused(msg)) }
fn role_of(name: &str) -> &str { name.split('.').next().unwrap_or("") }
/// A board setting is yours only; where a task is in no repo, the repo's settings mean nothing to it.
fn allowed(name: &str, scope: &Scope) -> Res<&'static Entry> {
    let e = known(name)?;
    let (task, repo) = (scope.task, scope.repo.as_str());
    if e.kind == Kind::Board && (task.is_some() || !repo.is_empty()) {
        return refuse(format!("{name} is a board setting: it is yours only and cannot be set {}. Set it for yourself: herdr-projects set {name} <value>", if task.is_some() { "on a task" } else { "for a repo" }));
    }
    if e.kind == Kind::Repo && repo.is_empty() {
        return refuse(format!("{name} is one repo's own: it cannot be set {}. Set it for the repo: herdr-projects set {name} <value> --repo <name>, or in the repo's own .herdr-projects.json.", if task.is_some() { "on a task" } else { "for every repo at once" }));
    }
    let repos = known_repos();
    if !repo.is_empty() && !repos.contains_key(repo) {
        let names: Vec<&str> = repos.keys().map(|s| s.as_str()).collect();
        let near = nearest(repo, &names);
        return refuse(format!("No repo \"{repo}\" on the board.{} Known: {}.", near.map(|n| format!(" Did you mean {n}?")).unwrap_or_default(), if names.is_empty() { "none yet".into() } else { names.join(", ") }));
    }
    if let Some(t) = task { if needs_repo(name) && !in_repo(t) { return refuse(format!("{name} does not apply to {}: it is in no repo.", t.s("id"))); } }
    Ok(e)
}
fn now_value(name: &str, scope: &Scope) -> String { lookup(name, scope, &yours()).map(|f| text(&f.value)).unwrap_or_default() }
/// The checked values of a request. An effort and an access are checked against the program they are for: the one
/// named in the same request, else the one the scope has.
pub fn checked(pairs: &[(String, Value)], scope: &Scope) -> Res<Obj> {
    let mut out = Obj::new();
    for (name, value) in pairs { allowed(name, scope)?; let v = check(name, value, "")?; models_fit(name, &v)?; out.insert(name.clone(), v); }
    // Every repo has a key: none is refused, with the way back to the one made from its name.
    if out.get("repo.key").map(text).as_deref() == Some(NONE) {
        return refuse(format!("repo.key cannot be none: every repo has a short key. To go back to the one the board made from its name: herdr-projects unset repo.key --repo {}", scope.repo));
    }
    // Two repos with one key would make `HSW-134` two tasks: the second is refused, and told who has it.
    if let Some(k) = out.get("repo.key").map(text) {
        if let Some(other) = said_keys().into_iter().find(|s| s.key == k && s.repo != scope.repo) {
            return refuse(format!("repo.key: {} already has the key {}. Choose another for {}.", other.repo, other.key, scope.repo));
        }
        // A key another repo had still names that repo's tasks: taken, an old `HSW-134` would name another task.
        if let Some(r) = crate::store::keys_given::had_keys().get(&k).map(text).filter(|r| *r != scope.repo) {
            return refuse(format!("repo.key: {k} was {r}'s key and still names {r}'s tasks. Choose another for {}.", scope.repo));
        }
    }
    // Research only, on a task that was started as an ordinary one: refused where it cannot be honoured.
    if let (Some(t), Some(v)) = (scope.task, out.get(RESEARCH)) { let why = turn_refusal(t, *v == json!(true)); if !why.is_empty() { return refuse(why); } }
    let get = |n: &str, out: &Obj| out.get(n).map(text).unwrap_or_else(|| now_value(n, scope));
    for (name, value) in &out {
        let e = known(name)?;
        let role = role_of(name);
        if e.ty == Ty::Effort || e.ty == Ty::Access {
            let why = fits(name, &text(value), &get(&format!("{role}.agent"), &out));
            if !why.is_empty() { return refuse(format!("{name}: {why}")); }
        }
        // Every agent of a list of attempts works with the working agent's access.
        if e.ty == Ty::Attempts && value.is_string() {
            let access = get("worker.access", &out);
            for a in text(value).split(", ") { let why = fits("worker.access", &access, a); if !why.is_empty() { return refuse(format!("{name}: {why}")); } }
        }
        if e.ty == Ty::Agent && entry_of(&format!("{role}.access")).is_some() {
            let access = get(&format!("{role}.access"), &out);
            let why = fits(&format!("{role}.access"), &access, &text(value));
            if !why.is_empty() { return refuse(format!("{name}: {why}")); }
        }
    }
    Ok(out)
}
fn save(values: &[(String, Option<Value>)], scope: &Scope) -> Res<Option<Task>> {
    if values.iter().any(|(n, _)| n == "limit.time") { keep_run_times(); } // a run under way keeps the limit it started with
    let Some(task) = scope.task else {
        for (name, v) in values { write_yours(name, v.as_ref(), &scope.repo)?; }
        return Ok(None);
    };
    let values = values.to_vec();
    update(task.s("id"), move |t| {
        assert_unarchived(t)?;
        let s = t.obj_mut("settings");
        for (name, v) in &values { match v { None => { s.shift_remove(name); } Some(v) => { s.insert(name.clone(), v.clone()); } } }
        // "This one merges by itself" is your word on this very task: an approval it owed you is not asked for again.
        if values.iter().any(|(n, v)| n == "merge.by" && v.as_ref() == Some(&json!("itself"))) { t.put("requireApproval", false); }
        Ok(Change::Write)
    }).map(Some)
}
fn read_back(name: &str, scope: &Scope, saved: Option<Task>) -> Res<Found> {
    match saved { Some(t) => lookup(name, &Scope::task(&t), &yours()), None => lookup(name, scope, &yours()) }
}
/// A repo's key changed: the keys it had are on record (repo_keys keeps them), and a repo left with none is given one
/// at once.
fn key_after(name: &str, scope: &Scope) {
    if name != "repo.key" || scope.repo.is_empty() { return; }
    give_keys(Some(&scope.repo));
    repo_keys();
}
fn key_before(name: &str) { if name == "repo.key" { repo_keys(); } }
/// set: the setting as the scope now reads it.
pub fn set_value(name: &str, value: &Value, scope: &Scope) -> Res<Found> {
    let values = checked(&[(name.to_string(), value.clone())], scope)?;
    key_before(name);
    let saved = save(&values.into_iter().map(|(k, v)| (k, Some(v))).collect::<Vec<_>>(), scope)?;
    key_after(name, scope);
    read_back(name, scope, saved)
}
/// Several settings of one task at once (delegate's options): all or none.
pub fn set_many(pairs: &[(String, Value)], scope: &Scope) -> Res<Option<Task>> {
    let values = checked(pairs, scope)?;
    if values.is_empty() { return Ok(scope.task.cloned()); }
    save(&values.into_iter().map(|(k, v)| (k, Some(v))).collect::<Vec<_>>(), scope)
}
/// unset: back to what it would be without this place's answer. Works for every setting.
pub fn unset_value(name: &str, scope: &Scope) -> Res<Found> {
    allowed(name, scope)?;
    // Taking a task's own "not research only" away may turn it on, by a default: refused as setting it would be.
    if let (true, Some(t)) = (name == RESEARCH, scope.task) {
        let mut rest = t.clone();
        rest.obj_mut("settings").shift_remove(name);
        let to = lookup(name, &Scope::task(&rest), &yours()).map(|f| f.value == json!(true)).unwrap_or(false);
        let why = turn_refusal(t, to);
        if !why.is_empty() { return refuse(why); }
    }
    key_before(name);
    let saved = save(&[(name.to_string(), None)], scope)?;
    key_after(name, scope);
    read_back(name, scope, saved)
}

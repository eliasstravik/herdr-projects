//! Your model and effort settings, for `doctor`: yours, your entry for each repo and each task at work, as each of
//! them reads them. Each program a setting names can do its role and takes the model written after it
//! (agents/models.rs: a value read from a file is not checked when it is read), and every effort, access and
//! hand-edited value counts (store/settings/show.rs `problems_in`: one that does not falls to the next place down).
use super::Found;
use crate::agents::access;
use crate::agents::kinds::kind;
use crate::agents::roles::refusal;
use crate::json::*;
use crate::store::settings::lookup::{lookup_at, places_of, Scope};
use crate::store::settings::places::yours;
use crate::store::settings::show::problems_in;
use crate::store::settings::values::{agent_of, AgentName};
use crate::store::settings_list::{Ty, SETTINGS};
use crate::store::tasks::{list, Task};

/// Whose settings: yours (no repo, no task), your entry for a repo, or a task at work. `label`: how a line names it,
/// `flag`: what `set` is given for it.
pub struct At { pub label: String, pub flag: String, pub repo: String, pub task: Option<Task> }
impl At { pub fn scope(&self) -> Scope<'_> { Scope { task: self.task.as_ref(), repo: self.repo.clone(), ..Default::default() } } }
/// Yours first, then each repo you said something for, then each task that is not done or archived.
pub fn places() -> Vec<At> {
    let mut v = vec![At { label: String::new(), flag: String::new(), repo: String::new(), task: None }];
    for r in yours().repos.keys() { v.push(At { label: format!(" for {r}"), flag: format!(" --repo {r}"), repo: r.clone(), task: None }); }
    let keys = crate::store::keys::repo_keys();
    for t in list(Default::default()).into_iter().filter(|t| t.s("stage") != "done" && !t.b("archivedAt")) {
        let id = crate::store::keys::say_id(t.s("id"), &keys);
        v.push(At { label: format!(" of {id}"), flag: format!(" --task {id}"), repo: String::new(), task: Some(t) });
    }
    v
}
/// The agents each program setting names where `at` reads them: (setting, agent, the place that says it).
pub fn agents_at(at: &At) -> Vec<(String, AgentName, String)> {
    let scope = at.scope();
    let places = places_of(&scope, &yours());
    let mut out = vec![];
    for e in SETTINGS.iter().filter(|e| matches!(e.ty, Ty::Agent | Ty::Agents | Ty::Attempts)) {
        let Ok(f) = lookup_at(&e.name, &scope, &places) else { continue };
        let all: Vec<String> = match &f.value { Value::Array(a) => a.iter().map(text).collect(), Value::Number(_) => vec![], v => text(v).split(", ").map(String::from).collect() };
        let from = if ["task", "carried"].contains(&f.place.as_str()) { format!("{}{}", f.from, at.label) } else { f.from.clone() };
        out.extend(all.iter().map(|a| agent_of(a.trim())).filter(|n| !n.kind.is_empty()).map(|n| (e.name.clone(), n, from.clone())));
    }
    out
}

/// The roles whose access is `auto` where `at` reads it, and why their program cannot start so: it has no auto mode,
/// or the version here is older than its first (an `auto` is never let fall to the next place down: lookup.rs).
fn auto_refused(at: &At) -> Vec<(String, String, String)> {
    let scope = at.scope();
    let places = places_of(&scope, &yours());
    let mut out = vec![];
    for e in SETTINGS.iter().filter(|e| e.ty == Ty::Access) {
        let Some(f) = lookup_at(&e.name, &scope, &places).ok().filter(|f| f.value == json!("auto")) else { continue };
        let Ok(a) = lookup_at(&format!("{}.agent", e.name.split('.').next().unwrap_or("")), &scope, &places) else { continue };
        let from = [&f, &a].iter().map(|x| if ["task", "carried"].contains(&x.place.as_str()) { format!("{}{}", x.from, at.label) } else { x.from.clone() }).collect::<Vec<_>>().join(" + ");
        let k = agent_of(&text(&a.value)).kind;
        let why = [access::why_not(&k, "auto"), access::too_old(&k, "auto")].into_iter().find(|w| !w.is_empty());
        if let Some(why) = why { out.push((e.name.clone(), why, from)); }
    }
    out
}
pub fn check() -> Vec<Found> {
    let mut out: Vec<Found> = vec![];
    let mut seen: Vec<String> = vec![];
    // What a repo or a task only inherits is said once, where it is said.
    let mut add = |key: String, f: Found, out: &mut Vec<Found>| if !seen.contains(&key) { seen.push(key); out.push(f); };
    for at in places() {
        for (name, n, from) in agents_at(&at) {
            let why = if kind(&n.kind).is_none() { Some(format!("no agent program \"{}\"", n.kind)) }
                else { refusal(&name, &n.kind, !n.model.is_empty()).map(|w| w.trim_start_matches(&format!("{name}: ")).to_string()).or_else(|| crate::agents::models::why_not(&n.kind, &n.model)) };
            let Some(why) = why else { continue };
            let fix = format!("Set it again: herdr-projects set {name} <program/model>{}  (or herdr-projects unset {name}{})", at.flag, at.flag);
            add(format!("{from}: {name}: {why}"), Found::bad(format!("{name}{}: {}", at.label, why.trim_end_matches('.')), fix), &mut out);
        }
        for (name, why, from) in auto_refused(&at) {
            let fix = format!("{}set it again: herdr-projects set {name} full{} (or ask)", if why.contains("has no auto mode: it came in") { "Update the program, or " } else { "Choose a program that has one, or " }, at.flag);
            add(format!("{from}: {name}: {why}"), Found::bad(format!("{name}{}: {why}", at.label), fix), &mut out);
        }
        for p in problems_in(&at.scope()) {
            let fix = format!("Set it again with herdr-projects set …{}, or take it out of that file: until then the value below it counts.", at.flag);
            let what = p.replacen("this task:", &format!("the settings{}:", at.label), 1);
            add(what.clone(), Found::bad(what.trim_end_matches('.').to_string(), fix), &mut out);
        }
    }
    if out.is_empty() { out.push(Found::ok("The agent, model and effort settings are valid")); }
    out
}

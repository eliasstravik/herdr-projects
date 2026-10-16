//! Your model and effort settings, for `doctor`: each program a setting names can do its role, takes the model written
//! after it (agents/models.rs: a value read from a file is not checked when it is read), and every effort, access and
//! hand-edited value counts (store/settings/show.rs `problems`: one that does not falls to the next place down).
use super::Found;
use crate::agents::kinds::kind;
use crate::agents::roles::refusal;
use crate::json::*;
use crate::store::settings::lookup::setting;
use crate::store::settings::show::problems;
use crate::store::settings::values::agent_of;
use crate::store::settings_list::{Ty, SETTINGS};

pub fn check() -> Vec<Found> {
    let mut out = vec![];
    for e in SETTINGS.iter().filter(|e| matches!(e.ty, Ty::Agent | Ty::Agents | Ty::Attempts)) {
        let v = setting(&e.name, None);
        let all: Vec<String> = match &v { Value::Array(a) => a.iter().map(text).collect(), Value::Number(_) => vec![], _ => text(&v).split(", ").map(String::from).collect() };
        let fix = format!("Set it again: herdr-projects set {} <program/model>  (or herdr-projects unset {})", e.name, e.name);
        for n in all.iter().map(|a| agent_of(a.trim())).filter(|n| !n.kind.is_empty()) {
            let why = if kind(&n.kind).is_none() { Some(format!("{}: no agent program \"{}\"", e.name, n.kind)) }
                else { refusal(&e.name, &n.kind, !n.model.is_empty()).or_else(|| crate::agents::models::why_not(&n.kind, &n.model).map(|w| format!("{}: {w}", e.name))) };
            if let Some(why) = why { out.push(Found::bad(why.trim_end_matches('.').to_string(), fix.clone())); }
        }
    }
    for p in problems(None) { out.push(Found::bad(p.trim_end_matches('.').to_string(), "Set it again with herdr-projects set, or take it out of that file: until then the value below it counts.")); }
    if out.is_empty() { out.push(Found::ok("The agent, model and effort settings are valid")); }
    out
}

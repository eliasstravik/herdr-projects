//! The arguments an agent of a kind is started with, and what a role is started as from its settings.
use super::adapter::*;
use super::kinds::kind;
use crate::err::Res;
use crate::json::*;

pub fn has_fast(k: &str) -> bool { kind(k).map(|a| a.fast).unwrap_or(false) }
/// The effort a program (with this model) is really given: none for `default`, and none for a level it does not
/// accept, by the levels it said last (efforts.rs: a start never waits for the program to say them).
pub fn effort_for(k: &str, model: &str, effort: &str) -> String {
    if effort.is_empty() || effort == "default" || kind(k).is_none() { return String::new(); }
    if super::efforts::why_not(k, model, effort, super::efforts::Ask::Kept).is_some() { String::new() } else { effort.to_string() }
}
/// The flags of `<role>.access` for this program. A program is never started as if it could ask, or had an auto mode,
/// when it cannot: neither falls back to another access.
pub fn access_args(k: &str, access: &str) -> Res<Vec<String>> {
    let Some(a) = kind(k) else { return if access == "full" { Ok(vec![]) } else { crate::fail!("{k} cannot be started with access \"{access}\": the board does not know its flags. Set the role's access to full, or choose another program.") } };
    let Some(f) = super::access::flags(a, access) else {
        crate::fail!("{k} cannot be started with access \"{access}\": {}. Set the role's access to {}, or choose another program.", super::access::lacks(access), if access == "auto" { "full or ask" } else { "full" })
    };
    let old = super::access::too_old(k, access);
    if !old.is_empty() { crate::fail!("{k} cannot be started with access \"{access}\": {old}."); }
    Ok(f.iter().map(|s| s.to_string()).collect())
}
#[derive(Default, Clone)]
pub struct StartOpts { pub model: String, pub effort: String, pub fast: Option<bool>, pub access: String, pub resume: String, pub session: String, pub extra: Vec<String> }
/// The arguments a worker, a reviewer or the Coordinator of this kind is started with. `resume` reopens a known
/// conversation; `session` names a new one; `extra` are the hook and standing-rule arguments.
pub fn start_args(k: &str, o: &StartOpts) -> Res<Vec<String>> {
    let mut out: Vec<String> = kind(k).map(|a| super::kinds::fill(a.lead, "")).unwrap_or_default(); // `hermes chat`
    if !o.resume.is_empty() { out.extend(resume_args(k, &o.resume)); } // first: Codex's `resume` is a subcommand
    if o.resume.is_empty() { out.extend(session_args(k, &o.session)); out.extend(fresh_args(k)); }
    out.extend(access_args(k, if o.access.is_empty() { "full" } else { &o.access })?);
    if !o.model.is_empty() { out.extend(model_args(k, &o.model)); }
    let level = effort_for(k, &o.model, &o.effort);
    if !level.is_empty() { out.extend(effort_args(k, &level)); }
    if let Some(f) = o.fast { out.extend(fast_args(k, f)); }
    out.extend(o.extra.iter().cloned());
    let left_out = too_new(k);
    Ok(out.into_iter().filter(|a| !left_out.contains(&a.as_str())).collect())
}
/// The board's arguments the installed version of this program does not take yet (kinds.rs `since`). A version that
/// cannot be read is taken to be new.
pub fn too_new(k: &str) -> Vec<&'static str> {
    let Some(a) = kind(k).filter(|a| !a.since.is_empty()) else { return vec![] };
    let v = super::installed::version_of(k);
    a.since.iter().filter(|(_, first)| !v.is_empty() && !super::installed::at_least(&v, first)).map(|(arg, _)| *arg).collect()
}
/// What a role is started as, from its five settings.
#[derive(Clone, Debug, Default)]
pub struct Role { pub kind: String, pub model: String, pub effort: String, pub fast: bool, pub access: String, pub instructions: String }
pub fn role_of(role: &str, get: &dyn Fn(&str) -> Value) -> Role {
    let v = text(&get(&format!("{role}.agent")));
    let (k, model) = if v == "none" { (String::new(), String::new()) } else { match v.split_once('/') { Some((a, b)) => (a.to_string(), b.to_string()), None => (v.clone(), String::new()) } };
    let words = text(&get(&format!("{role}.instructions")));
    Role { kind: k, model, effort: text(&get(&format!("{role}.effort"))), fast: get(&format!("{role}.fast")) == json!(true), access: text(&get(&format!("{role}.access"))),
        instructions: if words.is_empty() || words == "none" { String::new() } else { words } }
}
/// Your own words for a role, as they are added after the board's own fixed rules.
pub fn own_words(t: &str) -> String { if t.is_empty() { String::new() } else { format!("The human's own instructions for you follow. The board's rules above stay as they are; where two of these disagree, the later one wins.\n\n{t}") } }

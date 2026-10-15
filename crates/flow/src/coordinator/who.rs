//! Who the Coordinator is: the agent kind, model and effort chosen for it, and whether it can run here.
use super::state::get;
use crate::agents::installed::installed;
use crate::agents::start::{effort_for, role_of};
use crate::err::Res;
use crate::json::*;
use crate::store::settings::lookup::setting;
use crate::store::settings::values::{agent_of, check, models_fit};
use crate::store::settings_list::entry_of;

/// The default: the five `coordinator.` settings, yours only. `coordinator.agent: none`: no Coordinator.
pub fn choice() -> Option<Obj> {
    let who = role_of("coordinator", &|n| setting(n, None));
    if who.kind.is_empty() { return None; }
    let mut c = obj(json!({ "kind": who.kind, "model": who.model, "effort": effort_for(&who.kind, &who.effort) }));
    extra(&mut c, who.fast, &who.access);
    Some(c)
}
/// Fast mode and asking first are part of who it is only where they are not the usual.
fn extra(c: &mut Obj, fast: bool, access: &str) { if fast { c.put("fast", true); } if !access.is_empty() && access != "full" { c.put("access", access); } }
pub const OFF_WORDS: &str = "The Coordinator is off. `herdr-projects unset coordinator.agent` turns it on.";
/// A choice for one Coordinator only ("new, on codex"): what is named wins, and a model or effort set for another
/// program is not passed on to it.
pub fn pick_of(agent: &str, effort: &str) -> Res<Option<Obj>> {
    if agent.is_empty() && effort.is_empty() { return Ok(None); }
    let base = choice().unwrap_or_else(|| obj(json!({ "kind": agent_of(&text(&entry_of("coordinator.agent").unwrap().builtin)).kind, "model": "", "effort": "" })));
    let named = if agent.is_empty() { None } else { let v = check("coordinator.agent", &json!(agent), "")?; models_fit("coordinator.agent", &v)?; Some(agent_of(&text(&v))) };
    if let Some(n) = &named { if n.kind.is_empty() { crate::fail!("A Coordinator for one use needs a program: {}.", crate::agents::roles::able("coordinator").join(", ")); } }
    let kind = named.as_ref().map(|n| n.kind.clone()).unwrap_or_else(|| base.t("kind"));
    let same = kind == base.s("kind");
    let level = if !effort.is_empty() { text(&check("coordinator.effort", &json!(effort), &kind)?) } else if same { base.t("effort") } else { String::new() };
    let model = named.as_ref().map(|n| n.model.clone()).filter(|m| !m.is_empty()).unwrap_or_else(|| if same && named.is_none() { base.t("model") } else { String::new() });
    let mut c = obj(json!({ "kind": kind, "model": model, "effort": effort_for(&kind, &level) }));
    extra(&mut c, base.b("fast"), base.s("access"));
    Ok(Some(c))
}
pub fn describe(k: Option<&Obj>) -> String {
    match k { Some(k) => format!("{} · {} · {}", k.s("kind"), if k.b("model") { k.s("model") } else { "default model" }, if k.b("effort") { k.s("effort") } else { "default effort" }), None => "off".into() }
}
pub fn same_choice(a: &Obj, b: &Obj) -> bool {
    a.s("kind") == b.s("kind") && a.s("model") == b.s("model") && a.s("effort") == b.s("effort") && a.b("fast") == b.b("fast") && (if a.b("access") { a.s("access") } else { "full" }) == (if b.b("access") { b.s("access") } else { "full" })
}
/// Can the box talk to a Coordinator here? It needs a running herdr to live in, and its program installed.
pub fn ready(socket: &str) -> bool {
    let Some(k) = choice() else { return false };
    if crate::herdr::cli::fake().is_none() && !crate::herdr::cli::socket_exists(socket) { return false; }
    get(socket).map(|c| c.b("run")).unwrap_or(false) || installed(k.s("kind"))
}

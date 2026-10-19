//! Coordinator provider handover on the global CLI order, preserving failed terminals and a readable context file.
use crate::coordinator::{co::Co, requests, state, who};
use crate::err::Res;
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::files::write_json;

pub fn target(socket: &str) -> Option<Obj> {
    let c = state::get(socket).unwrap_or_default();
    if c.s("controlPriority") == crate::mission_control::priority() && c.b("controlFallback") {
        return who::pick_of(c.s("controlFallback"), "").ok().flatten();
    }
    who::choice()
}
pub fn tick(socket: &str) -> Res<()> {
    let co = Co::top(socket);
    let c = state::get(&co).unwrap_or_default();
    if state::pumping(&c) || c.b("want") || !c.b("run") { return Ok(()) }
    let api = Herdr::new(Some(socket.into()));
    let agent = api.agent_get(c.s("pane")).ok().flatten();
    let explicit = regex::Regex::new(r"(?im)^\s*(?:[✗×!⚠⏺●]\s*)?(?:api error:|error:|usage limit reached|quota exceeded|individual quota reached|rate limit exceeded|you.ve hit your (?:usage )?limit|http (?:429|529|401|403))").unwrap();
    let turn = crate::finish::turn_ended::screen_error(&api, c.s("pane"), agent.as_ref()).filter(|t| explicit.is_match(&t.error));
    let error = turn.map(|t| t.error).filter(|s| !s.is_empty()).or_else(|| c.so("error").map(String::from)).unwrap_or_default();
    if error.is_empty() { return Ok(()) }
    let order = crate::mission_control::parse_priority(&crate::mission_control::priority())?;
    let run = c.at("run");
    let on = if run.b("model") { format!("{}/{}", run.s("kind"), run.s("model")) } else { run.t("kind") };
    let at = order.iter().position(|s| s == &on).unwrap_or(0);
    let next = order.iter().skip(at + 1).find(|s| crate::agents::installed::installed(&crate::store::settings::values::agent_of(s).kind));
    let Some(next) = next else { return Ok(()) };
    let file = state::dir_of(&co).join(format!("handover-{}.json", crate::clock::now()));
    write_json(&file, &Value::Object(c.clone()))?;
    state::change(&co, |s| { s.put("controlPriority", crate::mission_control::priority()); s.put("controlFallback", next.clone()); s.put("controlHandover", format!("{on} → {next}: {error}")); true })?;
    requests::handover(who::pick_of(next, "")?, &co)?;
    let wanted = crate::mission_projects::root();
    let last = c.arr("where").last();
    let folder = last.filter(|m| std::path::Path::new(m.s("root")).is_dir()).map(|m| std::path::PathBuf::from(m.s("root")))
        .unwrap_or_else(|| if wanted.is_dir() { wanted } else { crate::store::files::user_home() });
    let scope = if last.map(|m| m.s("said").starts_with("[MISSION-CONTROL-PROJEKT]")).unwrap_or(false) { "[MISSION-CONTROL-PROJEKT]\n" } else { "" };
    let pending = last.map(|m| format!("\nZuletzt übergebene Nachricht für {} (bei dieser Fehlermeldung unbeantwortet):\n{}", m.s("root"), m.s("said"))).unwrap_or_default();
    requests::send(&folder, &format!("{scope}Orchestrator-Providerwechsel von {on} zu {next}. Ursache: {error}. Der bisherige Terminal-Pane bleibt zur Einsicht offen. Die vollständige Vorgeschichte liegt in {}. Lies sie und `board list`, bevor du weiterarbeitest. Setze offene Aufträge und offene ORCHESTRATOR-REVIEW-Anfragen fort; lege bereits existierende Teilaufgaben nicht erneut an. Berichte dem Menschen den Wechsel.{pending}", file.display()), false, "", &co)?;
    Ok(())
}

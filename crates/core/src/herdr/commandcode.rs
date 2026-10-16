//! Starting Command Code through its built-in custom-agent reporting on herdr versions without a native kind.
use super::cli::{args, HRes, HerdrError};
use super::Herdr;
use crate::json::*;

fn error(code: &str, message: impl Into<String>) -> HerdrError { HerdrError { code: code.into(), message: message.into() } }
pub fn agent(api: &Herdr, target: &str) -> Option<Value> {
    api.agent_get(target).ok().flatten().filter(|a| ["cmd", "commandcode", "command-code"].contains(&a.s("agent")))
}
/// Custom labels have no native prompt profile in herdr 0.9.3. Paste once, then let submit.rs retry only Enter.
pub fn prompt(api: &Herdr, a: &Value, text: &str) -> HRes<Value> {
    if a.s("agent_status") == "blocked" { return Err(error("agent_blocked", "Command Code is waiting for an answer or permission.")); }
    let pane = a.s("pane_id");
    let clean: String = text.chars().filter(|c| !c.is_control() || ['\n', '\t'].contains(c)).collect();
    api.pane_text(pane, &format!("\u{1b}[200~{clean}\u{1b}[201~"))?;
    if super::cli::fake().is_none() { crate::clock::sleep_ms(300); }
    if !agent(api, pane).is_some_and(|now| now.at("terminal_id") == a.at("terminal_id")) { return Err(error("agent_prompt_stalled", "The Command Code terminal changed after the paste; no Enter was sent.")); }
    pane_keys(api, pane, &["enter"]).map_err(|e| error("agent_prompt_stalled", e.message))
}
pub fn pane_keys(api: &Herdr, pane: &str, keys: &[&str]) -> HRes<Value> {
    let mut a = args(&["pane", "send-keys", pane]); a.extend(keys.iter().map(|k| k.to_string())); api.call(a)
}
pub fn available(api: &Herdr, pane: &str) -> HRes<Value> {
    let Some(before) = api.pane_get(pane) else { return Err(error("pane_not_found", "The Command Code pane no longer exists.")) };
    if !api.pane_idle(pane) || api.agent_get(pane)?.is_some() { return Err(error("pane_busy", "The Command Code pane is not at its shell prompt.")); }
    let v = crate::agents::installed::version_of("commandcode");
    if !v.is_empty() && !crate::agents::installed::at_least(&v, "1.82.5") {
        return Err(error("unsupported_version", "Command Code needs version 1.82.5 or newer for its built-in herdr reporting and per-launch mods. Update commandcode, then retry."));
    }
    Ok(before)
}
pub fn start(api: &Herdr, name: &str, pane: &str, extra: &[String], timeout_ms: u64) -> HRes<Value> {
    let before = available(api, pane)?;
    let mut argv = vec!["env".into(), "-u".into(), "CMD_HERDR_PANE_OWNER".into(), "CMD_HERDR=1".into(), format!("HERDR_PROJECTS_AGENT={name}"), "commandcode".into()];
    argv.extend(extra.iter().cloned());
    if argv.iter().any(|a| a.contains(['\n', '\r'])) { return Err(error("invalid_args", "Command Code launch arguments must not contain line breaks.")); }
    argv.extend(crate::agents::commandcode::identity::args(&api.socket_path(), before.s("terminal_id")).map_err(|e| error("failed", e.msg))?);
    let command = argv.iter().map(|s| crate::util::shq(s)).collect::<Vec<_>>().join(" ");
    api.pane_run(pane, &command)?;
    ready(api, name, pane, before.s("terminal_id"), timeout_ms)
}
/// Shared with the read-only plan launcher: the custom agent must be this program in this terminal, ready for input.
pub fn ready(api: &Herdr, name: &str, pane: &str, terminal: &str, timeout_ms: u64) -> HRes<Value> {
    let until = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        if !api.pane_get(pane).is_some_and(|p| p.s("terminal_id") == terminal) { return Err(error("pane_changed", "The Command Code terminal changed during startup.")); }
        if let Some(a) = api.agent_get(pane)? {
            if !["cmd", "commandcode", "command-code"].contains(&a.s("agent")) { return Err(error("agent_kind_mismatch", format!("Expected Command Code, found {}.", a.s("agent")))); }
            if a.s("agent_status") == "blocked" { return Err(error("agent_not_ready", format!("agent {name} is blocked during startup and is not ready for prompts"))); }
            if ["idle", "done"].contains(&a.s("agent_status")) {
                api.call(args(&["agent", "rename", pane, name]))?;
                return Ok(a);
            }
        }
        if std::time::Instant::now() >= until { return Err(error("timeout", "Command Code did not report ready to herdr. Check its pane for login/setup errors and enable its built-in herdr mod (Command Code 1.82.5 or newer).")); }
        let ms = std::env::var("HERDR_PROJECTS_READY_STEP_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(100);
        if ms > 0 { std::thread::sleep(std::time::Duration::from_millis(ms)); }
    }
}

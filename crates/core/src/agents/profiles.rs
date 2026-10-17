//! Named environment profiles in the user settings folder, with validation, local home expansion and stale-variable unsets.
use crate::err::Res;
use crate::json::*;
use crate::store::files::{config_dir, user_home};

pub fn check_env(key: &str, value: &Value) -> Res<()> {
    let mut c = key.chars();
    let valid = c.next().map(|c| c.is_ascii_uppercase() || c == '_').unwrap_or(false) && c.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
    let blocked = ["PATH", "HOME", "SHELL", "USER", "LOGNAME", "PWD", "OLDPWD", "IFS", "ENV", "BASH_ENV", "PROMPT_COMMAND", "PS1", "PS4", "ZDOTDIR", "TERM"];
    if !valid || key.len() > 64 || blocked.contains(&key) || ["LD_", "DYLD_", "HERDR_"].iter().any(|p| key.starts_with(p)) { crate::fail!("Profile cannot manage environment variable {key}."); }
    if value.is_null() { return Ok(()); }
    let Some(s) = value.as_str() else { crate::fail!("Profile variable {key} must be a string or null (unset)."); };
    if s.len() > 1024 || s.chars().any(|c| c.is_control() || c == '\\') { crate::fail!("Profile variable {key} must be one printable line without backslashes (at most 1024 bytes)."); }
    Ok(())
}
pub fn load() -> Res<Obj> {
    let path = config_dir().join("profiles.json");
    let text = match std::fs::read_to_string(path) { Ok(s) => s, Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Obj::new()), Err(e) => return Err(e.into()) };
    let v: Value = serde_json::from_str(&text)?;
    let Some(profiles) = v.as_object() else { crate::fail!("profiles.json must be an object of named profiles."); };
    for (name, p) in profiles {
        if name == "none" || name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || "_-".contains(c)) { crate::fail!("Invalid profile name {name}."); }
        let Some(p) = p.as_object() else { crate::fail!("Profile {name} must be an object with env."); };
        if p.keys().any(|k| k != "env") { crate::fail!("Profile {name} accepts only env; use role.agent for the program/model and role.effort for effort."); }
        let Some(env) = p.at("env").as_object() else { crate::fail!("Profile {name} needs an env object."); };
        for (k, v) in env { check_env(k, v)?; }
    }
    Ok(profiles.clone())
}
fn expand(s: &str) -> String {
    let h = user_home().display().to_string();
    let s = if s == "~" { h.clone() } else if let Some(t) = s.strip_prefix("~/") { format!("{h}/{t}") } else { s.to_string() };
    // Only the braced spelling is expanded anywhere; an unbraced HOME is accepted only before / or at the end.
    s.replace("${HOME}", &h).replace("$HOME/", &format!("{h}/")).strip_suffix("$HOME").map(|s| format!("{s}{h}")).unwrap_or_else(|| s.replace("${HOME}", &h).replace("$HOME/", &format!("{h}/")))
}
pub fn resolve(name: &str) -> Res<Value> {
    let profiles = load()?;
    let chosen = if name.is_empty() || name == "none" { None } else { Some(profiles.get(name).ok_or_else(|| crate::err::Error::refused(format!("No environment profile {name} in profiles.json.")))?) };
    let mut env = Obj::new();
    for p in profiles.values() { for (k, _) in p.at("env").as_object().into_iter().flatten() { env.insert(k.clone(), Value::Null); } }
    if let Some(p) = chosen { for (k, v) in p.at("env").as_object().into_iter().flatten() { env.insert(k.clone(), v.as_str().map(|s| json!(expand(s))).unwrap_or(Value::Null)); } }
    for k in ["CLAUDE_CONFIG_DIR", "CODEX_HOME", "PI_CODING_AGENT_DIR"] {
        if let Some(Value::String(s)) = env.get(k) { if !std::path::Path::new(s).is_absolute() { crate::fail!("Profile {name}: {k} must expand to an absolute path."); } }
    }
    Ok(Value::Object(env))
}
pub fn check_name(value: &Value) -> Res<Value> {
    let Some(name) = value.as_str() else { crate::fail!("Choose a profile name or none."); };
    resolve(name)?;
    Ok(json!(name))
}
/// Environment-only profiles keep model/effort in the board's typed role settings, without copying flags.
pub fn shell_line(env: &Obj) -> Res<String> {
    let mut parts = vec![];
    for (k, v) in env { check_env(k, v)?; parts.push(match v.as_str() { Some(s) => format!("export {k}={}", crate::util::shq(s)), None => format!("unset {k}") }); }
    Ok(parts.join("; "))
}

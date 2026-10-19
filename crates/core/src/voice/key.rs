//! Where the ElevenLabs API key is found, on this machine only: it never goes into a page, a task or the repo. The
//! first of: `ELEVENLABS_API_KEY` in the environment, `elevenlabs.json` in the board's settings folder (the place for
//! it: `{ "api_key": "…" }`, readable by you alone), herdr-call's own config (its `elevenlabs_api_key`), your
//! agent-tokens file, or a reachy-voice `.env`. Which one it came from can be said; the key itself never is.
use crate::store::files::{config_dir, user_home};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub struct Key { pub key: String, pub from: String }
/// The file to put the key in when there is none.
pub fn own_file() -> PathBuf { config_dir().join("elevenlabs.json") }
fn herdr_config() -> PathBuf { std::env::var("XDG_CONFIG_HOME").ok().filter(|x| !x.is_empty()).map(PathBuf::from).unwrap_or_else(|| user_home().join(".config")).join("herdr") }
/// `NAME=value`, `export NAME="value"` or `name = "value"`, the first line that sets `name`.
pub fn line_value(text: &str, name: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let l = l.trim().strip_prefix("export ").unwrap_or(l.trim());
        let (k, v) = l.split_once('=')?;
        if k.trim() != name { return None; }
        let v = v.trim().split(" #").next().unwrap_or("").trim().trim_matches(|c| c == '"' || c == '\'').to_string();
        Some(v).filter(|v| v.len() >= 16 && v.chars().all(|c| c.is_ascii_graphic()))
    })
}
/// The places looked in, in order, and what each holds the key as.
fn places() -> Vec<(PathBuf, &'static str, &'static str)> {
    let h = user_home();
    vec![
        (own_file(), "json", "api_key"),
        (herdr_config().join("plugins/config/herdr-call/config.toml"), "line", "elevenlabs_api_key"),
        (h.join(".config/agent-tokens/env.sh"), "line", "ELEVENLABS_API_KEY"),
        (h.join("reachy-voice/.env"), "line", "ELEVENLABS_API_KEY"),
        (h.join("dev/reachy-voice/.env"), "line", "ELEVENLABS_API_KEY"),
    ]
}
pub fn find() -> Option<Key> {
    if let Some(k) = crate::util::env_val("ELEVENLABS_API_KEY") { return Some(Key { key: k.trim().into(), from: "ELEVENLABS_API_KEY in the environment".into() }); }
    for (file, how, name) in places() {
        let Ok(text) = std::fs::read_to_string(&file) else { continue };
        let k = if how == "json" { serde_json::from_str::<serde_json::Value>(&text).ok().and_then(|v| v.get(name).and_then(|x| x.as_str()).map(|s| s.trim().to_string())).filter(|s| !s.is_empty()) } else { line_value(&text, name) };
        if let Some(key) = k { return Some(Key { key, from: crate::store::marks::tilde(&file.display().to_string()) }); }
    }
    None
}
/// What to do when there is no key, in one sentence.
pub fn missing() -> String { format!("No ElevenLabs API key on this machine: put it in {} as {{\"api_key\": \"…\"}} (ElevenAgents write permission), then call again.", crate::store::marks::tilde(&own_file().display().to_string())) }

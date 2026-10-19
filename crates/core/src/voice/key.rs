//! Where the ElevenLabs API key is found, on this machine only: it never goes into a page, a task or the repo. The
//! first of: `ELEVENLABS_API_KEY` in the environment, or `elevenlabs.json` in the board's settings folder
//! (`{ "api_key": "…" }`, readable by you alone). Which one it came from can be said; the key itself never is.
use crate::store::files::config_dir;
use crate::store::marks::tilde;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub struct Key { pub key: String, pub from: String }
/// Where to make a key at ElevenLabs.
pub const MAKE_AT: &str = "https://elevenlabs.io/app/developers/api-keys";
/// The file to put the key in when there is none.
pub fn own_file() -> PathBuf { config_dir().join("elevenlabs.json") }
pub fn find() -> Option<Key> {
    if let Some(k) = crate::util::env_val("ELEVENLABS_API_KEY").map(|k| k.trim().to_string()).filter(|k| !k.is_empty()) { return Some(Key { key: k, from: "ELEVENLABS_API_KEY in the environment".into() }); }
    let file = own_file();
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).ok()?).ok()?;
    let key = v.get("api_key")?.as_str()?.trim().to_string();
    (!key.is_empty()).then(|| Key { key, from: tilde(&file.display().to_string()) })
}
/// What to do when there is no key: where to make one, the permission it needs, and the two places it can go.
pub fn missing() -> String {
    format!("No ElevenLabs API key on this machine. Make one at {MAKE_AT} (Developers, API Keys) with the ElevenAgents permission set to Write (and Voices set to Write, so the call's built-in voice can be added to your account), then put it in {} as {{\"api_key\": \"…\"}}, or set ELEVENLABS_API_KEY in the environment herdr-projects runs in, and call again.", tilde(&own_file().display().to_string()))
}

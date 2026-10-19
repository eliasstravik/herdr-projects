//! ElevenLabs, from this machine: the agent made to match agent.rs (its tools first, then the agent, each made once and
//! updated in place when the stamp changed, as herdr-call does it), and a short-lived token for one call, which is
//! all a page ever gets. The key goes to `curl` on its standard input, never on a command line. A test answers in
//! the test's own thread instead (`set_fake`).
use super::agent::{self, Made};
use crate::json::*;
use crate::store::files::{read_obj, write_json};
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

const API: &str = "https://api.elevenlabs.io";
/// Where the API is: ElevenLabs, or a stand-in (`HERDR_PROJECTS_ELEVENLABS_API`, for a scripted call in a browser).
fn api() -> String { std::env::var("HERDR_PROJECTS_ELEVENLABS_API").ok().filter(|x| !x.is_empty()).unwrap_or_else(|| API.into()) }
/// A test's ElevenLabs: (method, path, body) → (status, body).
pub type Fake = dyn Fn(&str, &str, &Value) -> (u16, Value);
thread_local! { static FAKE: RefCell<Option<Rc<Fake>>> = const { RefCell::new(None) }; }
pub fn set_fake(f: Option<Rc<Fake>>) { FAKE.with(|x| *x.borrow_mut() = f); }
fn state_file() -> std::path::PathBuf { super::call::dir().join("agent.json") }

/// One request. Err: why, in words that never hold the key.
pub fn request(key: &str, method: &str, path: &str, body: &Value) -> Result<Value, String> {
    if let Some(f) = FAKE.with(|x| x.borrow().clone()) { let (st, v) = f(method, path, body); return if (200..300).contains(&st) { Ok(v) } else { Err(format!("ElevenLabs answered {st} to {method} {}: {}", path.split('?').next().unwrap_or(""), crate::text::cut(&v.to_string(), 300))) }; }
    let q = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let mut config = format!("url = \"{}{}\"\nrequest = \"{method}\"\nheader = \"xi-api-key: {}\"\nheader = \"Accept: application/json\"\nsilent\nshow-error\nmax-time = 25\nwrite-out = \"\\n%{{http_code}}\"\n", api(), q(path), q(key));
    if !body.is_null() { config.push_str(&format!("header = \"Content-Type: application/json\"\ndata-binary = \"{}\"\n", q(&body.to_string()))); }
    let mut child = std::process::Command::new("curl").args(["--config", "-"]).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().map_err(|e| format!("curl did not start: {e}"))?;
    if let Some(mut i) = child.stdin.take() { let _ = i.write_all(config.as_bytes()); }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout).replace(key, "[key]");
    let (body_text, code) = text.rsplit_once('\n').unwrap_or(("", text.as_str()));
    let status: u16 = code.trim().parse().unwrap_or(0);
    if status == 0 { return Err(format!("ElevenLabs could not be reached: {}", crate::text::cut(String::from_utf8_lossy(&out.stderr).trim(), 200))); }
    let v: Value = serde_json::from_str(body_text).unwrap_or(Value::Null);
    if (200..300).contains(&status) { Ok(v) } else { Err(format!("ElevenLabs answered {status} to {method} {}: {}", path.split('?').next().unwrap_or(""), crate::text::cut(body_text, 300))) }
}
/// A tool as ElevenLabs keeps it: run by the page that holds the call, which asks the board.
fn tool_body(t: &agent::Tool) -> Value {
    json!({ "tool_config": { "type": "client", "name": t.name, "description": t.description, "parameters": (t.params)(), "expects_response": true, "response_timeout_secs": 20 } })
}
/// The agent's whole configuration. With language `auto` it starts in English and follows you into Swedish.
pub fn agent_body(m: &Made, tool_ids: &[String]) -> Value {
    let lang = if m.language == "auto" { "en" } else { m.language.as_str() };
    let mut presets = json!({});
    if m.language == "auto" { presets = json!({ "sv": { "overrides": { "agent": { "language": "sv", "first_message": agent::first_message("sv") } } } }); }
    json!({
        "name": agent::NAME, "tags": ["herdr-projects"],
        "conversation_config": {
            "agent": { "first_message": agent::first_message(lang), "language": lang, "dynamic_variables": { "dynamic_variable_placeholders": { "target": "the Coordinator" } },
                "prompt": { "prompt": agent::PROMPT, "llm": agent::LLM, "temperature": 0.4, "tool_ids": tool_ids,
                    "built_in_tools": if m.language == "auto" { json!({ "language_detection": { "type": "system", "name": "language_detection", "description": "", "params": { "system_tool_type": "language_detection", "only_at_conversation_start": false } } }) } else { json!({}) } } },
            "language_presets": presets,
            "tts": { "voice_id": m.voice, "model_id": agent::TTS_MODEL },
            "asr": { "keywords": ["Coordinator", "herdr", "merge", "approve", "godkänn"] },
            "turn": { "turn_eagerness": "patient", "turn_timeout": 10, "silence_end_call_timeout": agent::SILENCE_S },
            "conversation": { "max_duration_seconds": agent::LONGEST_S, "client_events": ["audio", "interruption", "user_transcript", "agent_response", "agent_response_correction", "client_tool_call", "vad_score"] },
        },
        "platform_settings": { "auth": { "enable_auth": true }, "overrides": { "conversation_config_override": { "agent": { "language": true, "first_message": true } } } },
    })
}
/// The agent, made or updated to match agent.rs, only when its stamp changed. Answers its id.
pub fn ensure_agent(key: &str) -> Result<String, String> {
    crate::store::files::with_lock_wait("voice-agent", 60_000, || make_agent(key).map_err(crate::err::Error::from)).map_err(|e| e.msg)
}
/// Forget the agent made before (it was deleted in ElevenLabs, or the key is another account's): the next call makes it.
pub fn forget_agent() { let _ = std::fs::remove_file(state_file()); }
fn make_agent(key: &str) -> Result<String, String> {
    let m = agent::made();
    // Everything the agent is made of, its whole configuration too.
    let stamp = sha1_smol::Sha1::from(format!("{}{}", agent::stamp(&m), agent_body(&m, &[]))).digest().to_string();
    let was = read_obj(&state_file()).unwrap_or_default();
    if was.s("stamp") == stamp && !was.s("agent").is_empty() { return Ok(was.t("agent")); }
    let mut ids: Vec<String> = vec![];
    let mut kept = obj(json!({}));
    for t in agent::tools() {
        let old = was.at("tools").t(t.name);
        let id = if !old.is_empty() && request(key, "PATCH", &format!("/v1/convai/tools/{old}"), &tool_body(&t)).is_ok() { old } else { request(key, "POST", "/v1/convai/tools", &tool_body(&t))?.t("id") };
        if id.is_empty() { return Err("ElevenLabs made the tool but gave no id for it".into()); }
        kept.put(t.name, id.clone());
        ids.push(id);
    }
    let body = agent_body(&m, &ids);
    let old = was.t("agent");
    let agent = if !old.is_empty() && request(key, "PATCH", &format!("/v1/convai/agents/{old}"), &body).is_ok() { old } else { request(key, "POST", "/v1/convai/agents/create", &body)?.t("agent_id") };
    if agent.is_empty() { return Err("ElevenLabs made the agent but gave no id for it".into()); }
    let _ = write_json(&state_file(), &json!({ "stamp": stamp, "agent": agent, "tools": kept }));
    Ok(agent)
}
/// A token for one call over WebRTC, short-lived: the one thing the page gets.
pub fn token(key: &str, agent: &str) -> Result<String, String> {
    let v = request(key, "GET", &format!("/v1/convai/conversation/token?agent_id={}", crate::util::url_encode(agent)), &Value::Null)?;
    let t = v.t("token");
    if t.is_empty() { Err("ElevenLabs gave no token".into()) } else { Ok(t) }
}

//! The voice agent as kept in this repo: its prompt (prompt.md), its tools, its voice, language and turn-taking. The
//! front door makes the ElevenLabs agent match this before a call (eleven.rs), and only when something here changed:
//! the stamp says. Its tools run on the board's side (the front door's voice calls), never in ElevenLabs.
use crate::json::*;

pub const PROMPT: &str = include_str!("prompt.md");
pub const NAME: &str = "herdr-projects voice";
/// A small, quick model: it only talks and relays; the agents on the board do the thinking.
pub const LLM: &str = "gemini-2.5-flash";
/// A model that speaks Swedish and English (reachy-voice's, heard in Swedish).
pub const TTS_MODEL: &str = "eleven_v4_turbo";
/// One of ElevenLabs' own voices that agents may use (one with live moderation may not), speaking Swedish and English
/// with the model above: reachy-voice's agent speaks with it. `voice.voice` puts another in its place.
pub const VOICE: &str = "bIHbv24MWmeRgasZH58o";
/// A call with nobody saying anything for this long hangs up by itself; none is longer than this.
pub const SILENCE_S: i64 = 120;
pub const LONGEST_S: i64 = 3600;

/// One tool: its name, what it is for (the voice agent reads this), and its parameters as JSON schema.
pub struct Tool { pub name: &'static str, pub description: &'static str, pub params: fn() -> Value }
pub fn tools() -> Vec<Tool> {
    vec![
        Tool { name: "send_message", description: "Send the human's request or question to the agent the chat is pointed at (the Coordinator, a task's agent, or a coordinator). Their exact words go with it. Answers whether it was sent; 'Not sent' says why and what to do.", params: || json!({ "type": "object", "properties": {
            "message": { "type": "string", "description": "What to send, faithful to what the human asked, in their words as far as possible." },
            "confirmed": { "type": "boolean", "description": "True only after the human clearly said yes to approving, merging, stopping or deleting." } }, "required": ["message"] }) },
        Tool { name: "switch_to", description: "Point the chat (and this call) at someone else, as the human's own switch does: the Coordinator, a task's agent by its tag or by words of its title, or a machine's or a project's coordinator.", params: || json!({ "type": "object", "properties": {
            "who": { "type": "string", "description": "'coordinator', a task's tag such as 'HP-52', or words of the task's title." } }, "required": ["who"] }) },
        Tool { name: "board_status", description: "What waits for the human on the board right now, and how many tasks are working: read-only, quick.", params: || json!({ "type": "object", "properties": {} }) },
        Tool { name: "latest_reply", description: "The newest thing the agent the chat is pointed at said, in full.", params: || json!({ "type": "object", "properties": {} }) },
        Tool { name: "quiet", description: "Stop (on true) or start again (on false) the board's updates about other tasks during this call. Answers to what was sent still come.", params: || json!({ "type": "object", "properties": {
            "on": { "type": "boolean", "description": "True: no updates about other tasks." } }, "required": ["on"] }) },
        Tool { name: "end_call", description: "Hang up, after a short goodbye.", params: || json!({ "type": "object", "properties": {} }) },
    ]
}
/// The settings a call is made with: the language (`auto`, `sv` or `en`) and the voice.
#[derive(Clone, Debug, PartialEq)]
pub struct Made { pub language: String, pub voice: String }
pub fn made() -> Made {
    let get = |n: &str| crate::store::settings::lookup::setting_text(n, None);
    let language = match get("voice.language").as_str() { "sv" => "sv", "en" => "en", _ => "auto" }.to_string();
    let voice = get("voice.voice");
    Made { language, voice: if voice.is_empty() || voice == "none" { VOICE.into() } else { voice } }
}
/// The first thing said on a call, in its language.
pub fn first_message(language: &str) -> &'static str { if language == "sv" { "Hej, jag lyssnar." } else { "Hi, I'm listening." } }
/// What the agent is made from: when this changes, the agent is updated before the next call.
pub fn stamp(m: &Made) -> String {
    let tools: Vec<Value> = tools().iter().map(|t| json!([t.name, t.description, (t.params)()])).collect();
    let all = json!({ "prompt": PROMPT, "llm": LLM, "tts": TTS_MODEL, "voice": m.voice, "language": m.language, "tools": tools, "silence": SILENCE_S, "v": 1 });
    sha1_smol::Sha1::from(all.to_string()).digest().to_string()
}

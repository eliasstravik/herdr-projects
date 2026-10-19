//! The voice agent as kept in this repo: its prompt (prompt.md), its tools, its voice, language and turn-taking. The
//! front door makes the ElevenLabs agent match this before a call (eleven.rs), and only when something here changed:
//! the stamp says. Its tools run on the board's side (the front door's voice calls), never in ElevenLabs.
use crate::json::*;

pub const PROMPT: &str = include_str!("prompt.md");
pub const NAME: &str = "herdr-projects voice";
/// A small, quick model: it only talks and relays; the agents on the board do the thinking. ElevenLabs' id for
/// DeepSeek V4.1 Flash (in its API's list of agent models); `voice.llm` puts another in its place.
pub const LLM: &str = "deepseek-v41-flash";
/// A quick model that speaks every language below.
pub const TTS_MODEL: &str = "eleven_v4_turbo";
/// The built-in voice: Warren, from ElevenLabs' Voice Library, the one it names in place of its retired default voice
/// Will. Agents may use it (one with live moderation may not), once it is in your account: builtin.rs adds it. It
/// speaks the languages below with the model above. `voice.voice` puts another in its place.
pub const VOICE: &str = "7QN34D2r3hCNwbOYIeK0";
/// The languages ElevenLabs' agents speak, by its codes: English first, the one `auto` starts in.
pub const LANGUAGES: [&str; 33] = ["en", "ar", "bg", "zh", "hr", "cs", "da", "nl", "fil", "fi", "fr", "de", "el", "hi", "hu", "id", "it", "ja", "ko", "ms", "no", "pl", "pt", "pt-br", "ro", "ru", "sk", "es", "sv", "ta", "tr", "uk", "vi"];
/// A call with nobody saying anything for this long hangs up by itself; none is longer than this.
pub const SILENCE_S: i64 = 120;
pub const LONGEST_S: i64 = 3600;

/// One tool: its name, what it is for (the voice agent reads this), and its parameters as JSON schema.
pub struct Tool { pub name: &'static str, pub description: &'static str, pub params: fn() -> Value }
pub fn tools() -> Vec<Tool> {
    vec![
        Tool { name: "send_message", description: "Send an instruction or a question to the agent the chat is pointed at (the Coordinator, a task's agent, or a coordinator), written by you from what the human asked of that agent. Answers whether it was sent; 'Not sent' says why and what to do.", params: || json!({ "type": "object", "properties": {
            "message": { "type": "string", "description": "A clear, self-contained instruction or question for the coding agent: just what the human asked of this agent, in their intent and key wording, with names, numbers, file names and task tags exactly as said. Leave out small talk, thinking aloud, what was said to you, and anything meant for another agent." },
            "action": { "type": "string", "enum": ["none", "approve", "merge", "stop", "delete"], "description": "What the message asks the agent to do of these: approve, merge, stop or delete (remove, archive) a task, only when the human asked for exactly that; else none (a question about one is none)." },
            "confirmed": { "type": "boolean", "description": "True only after the human clearly said yes to the approve, merge, stop or delete named in action." } }, "required": ["message", "action"] }) },
        Tool { name: "switch_to", description: "Point the chat (and this call) at someone else, as the human's own switch does: the Coordinator, a task's agent by its tag or by words of its title, or a machine's or a project's coordinator.", params: || json!({ "type": "object", "properties": {
            "who": { "type": "string", "description": "'coordinator', a task's tag such as 'HP-52', or words of the task's title." } }, "required": ["who"] }) },
        Tool { name: "board_status", description: "What waits for the human on the board right now, and how many tasks are working: read-only, quick.", params: || json!({ "type": "object", "properties": {} }) },
        Tool { name: "latest_reply", description: "The newest thing the agent the chat is pointed at said, in full.", params: || json!({ "type": "object", "properties": {} }) },
        Tool { name: "quiet", description: "Stop (on true) or start again (on false) the board's updates about other tasks during this call. Answers to what was sent still come.", params: || json!({ "type": "object", "properties": {
            "on": { "type": "boolean", "description": "True: no updates about other tasks." } }, "required": ["on"] }) },
        Tool { name: "end_call", description: "Hang up, after a short goodbye.", params: || json!({ "type": "object", "properties": {} }) },
    ]
}
/// The settings a call is made with: the language (`auto`, or one of `LANGUAGES`), the voice and the model.
#[derive(Clone, Debug, PartialEq)]
pub struct Made { pub language: String, pub voice: String, pub llm: String }
pub fn made() -> Made {
    let get = |n: &str| crate::store::settings::lookup::setting_text(n, None);
    let language = get("voice.language");
    let language = if LANGUAGES.contains(&language.as_str()) { language } else { "auto".into() };
    let voice = get("voice.voice");
    let llm = get("voice.llm");
    Made { language, voice: if voice.is_empty() || voice == "none" { VOICE.into() } else { voice }, llm: if llm.is_empty() || llm == "none" { LLM.into() } else { llm } }
}
/// The first thing said on a call: in English, or nothing in another language (the voice waits for you to speak).
pub fn first_message(language: &str) -> &'static str { if language == "en" || language == "auto" { "Hi, I'm listening." } else { "" } }
/// What the agent is made from: when this changes, the agent is updated before the next call.
pub fn stamp(m: &Made) -> String {
    let tools: Vec<Value> = tools().iter().map(|t| json!([t.name, t.description, (t.params)()])).collect();
    let all = json!({ "prompt": PROMPT, "llm": m.llm, "tts": TTS_MODEL, "voice": m.voice, "language": m.language, "tools": tools, "silence": SILENCE_S, "v": 1 });
    sha1_smol::Sha1::from(all.to_string()).digest().to_string()
}

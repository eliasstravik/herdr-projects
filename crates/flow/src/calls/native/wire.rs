//! ElevenLabs' conversation over its WebSocket, as plain data: what the server sends (audio, an interruption, a ping,
//! a tool to run, what was said on either side, how loud you are), and what this side sends (your audio, a pong, a
//! tool's answer, an update to read). Audio is 16 kHz 16-bit mono PCM both ways (the agent is made so: eleven.rs).
use crate::json::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Started { conversation: String, input: String, output: String },
    Audio { id: i64, pcm: Vec<u8> },
    Interruption { id: i64 },
    Ping { id: i64 },
    Tool { call: String, name: String, args: Value },
    Agent(String),
    User(String),
    Vad(f64),
    Other(String),
}
pub fn parse(text: &str) -> Event {
    let m: Value = serde_json::from_str(text).unwrap_or(Value::Null);
    match m.s("type") {
        "conversation_initiation_metadata" => { let e = m.at("conversation_initiation_metadata_event"); Event::Started { conversation: e.t("conversation_id"), input: e.t("user_input_audio_format"), output: e.t("agent_output_audio_format") } }
        "audio" => { let e = m.at("audio_event"); Event::Audio { id: e.i("event_id"), pcm: crate::util::unbase64(e.s("audio_base_64")).unwrap_or_default() } }
        "interruption" => Event::Interruption { id: m.at("interruption_event").i("event_id") },
        "ping" => Event::Ping { id: m.at("ping_event").i("event_id") },
        "client_tool_call" => { let e = m.at("client_tool_call"); Event::Tool { call: e.t("tool_call_id"), name: e.t("tool_name"), args: if e.at("parameters").is_object() { e.at("parameters").clone() } else { json!({}) } } }
        "agent_response" => Event::Agent(m.at("agent_response_event").t("agent_response")),
        "user_transcript" => Event::User(m.at("user_transcription_event").t("user_transcript")),
        "vad_score" => Event::Vad(m.at("vad_score_event").f("vad_score")),
        other => Event::Other(other.to_string()),
    }
}
/// The first message: who the call talks to, for the prompt.
pub fn hello(target: &str) -> String { json!({ "type": "conversation_initiation_client_data", "dynamic_variables": { "target": target } }).to_string() }
pub fn audio(pcm: &[u8]) -> String { json!({ "user_audio_chunk": crate::util::base64(pcm) }).to_string() }
pub fn pong(id: i64) -> String { json!({ "type": "pong", "event_id": id }).to_string() }
pub fn tool_result(call: &str, result: &str) -> String { json!({ "type": "client_tool_result", "tool_call_id": call, "result": result, "is_error": false }).to_string() }
/// An update the voice agent reads in a pause: its words start with "[board]" (the prompt says what that is).
pub fn board_says(text: &str) -> String { json!({ "type": "user_message", "text": format!("[board] {text}") }).to_string() }
/// After an interruption with id N, audio of that turn still on its way is not played.
#[derive(Default)]
pub struct Gate { cut: i64 }
impl Gate {
    pub fn interrupt(&mut self, id: i64) { self.cut = self.cut.max(id); }
    pub fn allows(&self, id: i64) -> bool { id > self.cut }
}

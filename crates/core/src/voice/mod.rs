//! A call: a live voice conversation with an ElevenLabs agent, which talks with you in real time and sends what should
//! go to the Coordinator or a task's agent there with a tool, then reads back their answers in short. The call record
//! every board shares (call.rs), where the ElevenLabs key is kept (key.rs), the agent and its tools as kept in this repo
//! (agent.rs), its built-in voice in your account (builtin.rs), its API (eleven.rs), the words a call sends: your own
//! (words.rs), and no action you did not ask for and say yes to (confirm.rs).
//! The page that holds the call is the web board's (crates/web/src/webui/page/call.js); docs/voice.md says how it works.
//! `voice.calls off` turns all of it off (calls_on).
pub mod agent;
pub mod builtin;
pub mod call;
pub mod eleven;
pub mod key;
pub mod confirm;
pub mod words;

/// What every place a call would start from says while calls are off.
pub const OFF: &str = "Voice calls are off (voice.calls)";
/// Whether there are voice calls at all (`voice.calls`): off, nothing rings, no helper runs and nothing reaches ElevenLabs.
pub fn calls_on() -> bool { crate::store::settings::lookup::setting_text("voice.calls", None) != "off" }
/// Why a call that was on when calls were turned off ended.
pub const TURNED_OFF: &str = "voice calls were turned off";
/// Calls are off: a call still on (started before) ends now. Its page or helper hears it ended at its next question
/// and lets go of the microphone and of ElevenLabs. The keeper asks every few seconds; the front door at every question.
pub fn end_if_off() { if !calls_on() && call::get().is_some() { call::end(TURNED_OFF); } }

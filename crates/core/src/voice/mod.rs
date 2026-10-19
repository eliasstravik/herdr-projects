//! A call: a live voice conversation with an ElevenLabs agent, which talks with you in real time and sends what should
//! go to the Coordinator or a task's agent there with a tool, then reads back their answers in short. The call record
//! every board shares (call.rs), where the ElevenLabs key is kept (key.rs), the agent and its tools as kept in this repo
//! (agent.rs), its built-in voice in your account (builtin.rs), its API (eleven.rs), the words a call sends: your own
//! (words.rs), and no action you did not ask for and say yes to (confirm.rs).
//! The page that holds the call is the web board's (crates/web/src/webui/page/call.js); docs/voice.md says how it works.
pub mod agent;
pub mod builtin;
pub mod call;
pub mod eleven;
pub mod key;
pub mod confirm;
pub mod words;

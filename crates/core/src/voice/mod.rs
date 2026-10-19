//! A call: a live voice conversation with an ElevenLabs agent, which talks with you in real time and sends what should
//! go to the Coordinator or a task's agent there with a tool, then reads back their answers in short. The call record
//! every board shares (call.rs), where the ElevenLabs key is kept (key.rs), the agent and its tools as kept in this repo
//! (agent.rs), its API (eleven.rs), and the words a call sends: your own, and no action you did not ask for (words.rs).
//! The page that holds the call is the web board's (crates/web/src/webui/page/call.js); docs/voice.md says how it works.
pub mod agent;
pub mod call;
pub mod eleven;
pub mod key;
pub mod words;

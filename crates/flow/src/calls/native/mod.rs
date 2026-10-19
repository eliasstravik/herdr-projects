//! A call from the terminal board, held right here in the terminal's computer, with no browser: the microphone and the
//! speaker (audio.rs: PipeWire's or PulseAudio's own tools on Linux, with their WebRTC echo cancelling; on macOS a small
//! program of Apple's voice processing, mac.rs) and ElevenLabs' conversation over its WebSocket (ws.rs, wire.rs), with
//! the board asked for the voice agent's tools, for what was heard and for what to read back (board.rs), all run by one
//! loop (engine.rs), which lifts a quiet microphone and measures it (level.rs). The API key stays on the board's machine: this computer gets a signed address for one call.
pub mod audio;
pub mod board;
pub mod engine;
pub mod level;
pub mod mac;
pub mod wire;
pub mod ws;

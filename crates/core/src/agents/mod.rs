//! Everything agent-specific: the table of agent programs (kinds.rs and adapter.rs), reading what they saved,
//! starting them, and what is installed. One area per file.
pub mod adapter;
pub mod hand_limits;
pub mod helpers;
pub mod homes;
pub mod installed;
pub mod kinds;
pub mod models;
pub mod prompts;
pub mod provider_error;
pub mod read_tokens;
pub mod read_turn;
pub mod roles;
pub mod screen_text;
pub mod start;
pub mod start_failed;
pub mod talk;

//! The board: one plain record of what it shows (state.rs), one function that takes it and an event to the next
//! (update/), the screen in the board's own words and named inks (view/), and the runtime that reads keys, runs jobs
//! and draws it on a terminal (run/). The web board takes the same state and events (webui/). The keys are keys.rs;
//! colours are theme/, both in sw-flow (crates/flow/src/board/).
// The keys' table and the theme are built in sw-flow, which uses them too; here they are the board's.
pub use sw_flow::board::{key_style, keys, remap, theme};
pub mod data;
pub mod job;
pub mod job_far;
pub mod run;
pub mod screens;
pub mod state;
pub mod tags;
pub mod textbox;
pub mod update;
pub mod view;
pub use run::place;

/// Open the board in this terminal.
pub fn open_board(folder: &std::path::Path) -> i32 { run::open_board(folder) }

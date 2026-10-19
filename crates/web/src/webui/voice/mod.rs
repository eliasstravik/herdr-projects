//! A voice call, on the front door's side: its calls under `/herdr-projects/api/voice/` (api.rs), which start a call
//! with a short-lived ElevenLabs token and run the voice agent's tools on the board the call belongs to (tools.rs): the
//! web board's page that started it, or the terminal board, whose chat the call follows (board.rs). What the call
//! reads back as it comes: the answer of whoever it talks to, and a task that needs you (feed.rs). Who "HP-52" or
//! "the voice task" is (find.rs). The page that holds a call started from the terminal board (page.rs).
pub mod api;
pub mod board;
pub mod feed;
pub mod find;
pub mod page;
pub mod tools;

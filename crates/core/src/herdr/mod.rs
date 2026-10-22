//! herdr: the program the board runs on. Its own colour code (palette.rs, taken from herdr's source), running its
//! command line (cli.rs, api.rs), where that program is when PATH lacks it (find.rs), a start tried again until its
//! shell is at its prompt (shell_ready.rs), its socket (socket.rs), which processes in a pane are an agent's own
//! helpers (procs.rs), what is at work in a pane, in words (busy.rs), and what
//! its agent launched there, to end at a stop (launched.rs).
pub mod api;
pub mod busy;
pub mod cli;
pub mod find;
pub mod keys;
pub mod launched;
pub mod palette;
pub mod procs;
pub mod shell_ready;
pub mod socket;
pub use api::Herdr;
pub use cli::HerdrError;

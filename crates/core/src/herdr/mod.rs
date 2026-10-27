//! herdr: the program the board runs on, and its terminal backends. Its own colour code (palette.rs, taken from
//! herdr's source), the one interface every backend gives (backend.rs: herdr's command line in herdr_cli.rs, a Rex
//! server in rex/), the handle the board holds (api.rs), running herdr's command line (cli.rs), a start tried again
//! until its shell is at its prompt (shell_ready.rs), its socket (socket.rs), and which processes in a pane are an
//! agent's own helpers (procs.rs).
pub mod api;
pub mod backend;
pub mod cli;
pub mod herdr_cli;
pub mod keys;
pub mod palette;
pub mod procs;
pub mod rex;
pub mod shell_ready;
pub mod socket;
pub use api::Herdr;
pub use cli::HerdrError;

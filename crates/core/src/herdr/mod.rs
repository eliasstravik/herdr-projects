//! herdr: the program the board runs on. Its own colour code (palette.rs, taken from herdr's source), running its
//! command line (cli.rs, api.rs), a start tried again until its shell is at its prompt (shell_ready.rs), and its socket
//! (socket.rs).
pub mod api;
pub mod cli;
pub mod keys;
pub mod palette;
pub mod shell_ready;
pub mod socket;
pub use api::Herdr;
pub use cli::HerdrError;

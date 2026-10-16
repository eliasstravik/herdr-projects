//! herdr: the program the board runs on. Its own colour code (palette.rs, taken from herdr's source), running its
//! command line (cli.rs, api.rs), and its socket (socket.rs).
pub mod api;
pub mod cli;
pub mod commandcode;
pub mod keys;
pub mod palette;
pub mod socket;
pub use api::Herdr;
pub use cli::HerdrError;

//! The crate sw-core (crates/core): the board's data (store/), herdr and the agents, and the small tools every part
//! uses. The crates on top of it name these modules as their own (`crate::store`), so code moves between crates
//! without a change. Which crate holds what, and why: docs/crates.md.
pub mod agents;
pub mod board_rule;
pub mod chat_only;
pub mod check_progress;
pub mod clock;
pub mod disk;
pub mod err;
pub mod gate;
pub mod herdr;
pub mod http;
pub mod json;
pub mod login_env;
pub mod opener;
pub mod research;
pub mod status_words;
pub mod store;
pub mod temp;
pub mod text;
pub mod util;
pub mod voice;

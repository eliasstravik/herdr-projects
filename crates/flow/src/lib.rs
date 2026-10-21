//! The crate sw-flow (crates/flow), on sw-core: the flow of a task (delegating, the finish line, landing), the keeper
//! and what it runs, other machines, automations, autopilot on the coordinators, the review page. With it, the parts
//! of the board's screen and of the web UI that this flow uses: the keys' table and the theme, the web UI found, linked
//! and its front door started.
//! Which crate holds what, and why: docs/crates.md.
pub use sw_core::*;

pub mod actions;
pub mod after;
pub mod attempts;
pub mod automations;
pub mod autopilot;
pub mod calls;
pub mod coordinator;
pub mod delegate;
pub mod doctor;
pub mod extras;
pub mod finish;
pub mod flow;
pub mod hook;
pub mod housekeeping;
pub mod install;
pub mod jobs;
pub mod keeper;
pub mod land;
pub mod leftovers;
pub mod limits;
pub mod notify;
pub mod plan;
pub mod remote;
pub mod review_page;
pub mod runner;
pub mod shown;
pub mod state;
pub mod status_kept;
pub mod submit;
pub mod sync;
pub mod take_over;
pub mod turn;
pub mod watcher;

/// What the board's screen has that the flow uses too: the keys (a message names a key) and the theme (the review
/// page's colours). sw-board's `board` module passes them on, so they are `crate::board::keys` everywhere.
pub mod board {
    pub mod key_style;
    pub mod keys;
    pub mod remap;
    pub mod theme;
}
/// What of the web UI the flow and the board's screen use: where the web UI is, its PCs, its link, and the
/// front door started from outside. sw-web's `webui` module passes them on beside the web board itself.
pub mod webui {
    pub mod command;
    pub mod find;
    pub mod keep;
    pub mod link;
    pub mod pcs;
    pub mod proxy;
}

//! The board inside the herdr web UI. A front door (door.rs) stands in front of the web UI's server and passes every
//! request through to it (proxy.rs), but for the board's own page and calls under `/herdr-projects/` and one script line in
//! the web UI's page. The board there is the terminal board's own state and update, one per browser (web.rs, view.rs),
//! sent as data (model/). The front door runs in the page server's process and follows the web UI (keep.rs,
//! door_loop.rs); the one setup step is the web UI's shared link (link.rs), by hand with `herdr-projects web-ui`
//! (command.rs).
// Built in sw-flow, which uses them too (crates/flow/src/webui/): where the web UI is, its PCs and link, the front
// door started from outside.
pub use sw_flow::webui::{command, find, keep, link, pcs, proxy};
pub mod attach;
pub mod boards;
pub mod dock;
pub mod door;
pub mod door_loop;
pub mod follow;
pub mod going;
pub mod ink;
pub mod model;
pub mod page;
pub mod view;
pub mod voice;
pub mod web;

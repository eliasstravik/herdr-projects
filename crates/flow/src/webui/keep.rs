//! The front door's life, seen from outside: whether it is up, and starting it. It runs inside the board's one page
//! server (`herdr-projects serve`, review_page.rs), as a second listener there, so the plugin still owns one
//! long-lived process and not two; its loop is door_loop.rs. Its bookkeeping (its process, its port, where it passes
//! things on to) is one file in the data folder: front-door.json. The keeper, herdr's start and the board opening all
//! see to it that it is up.
use super::find::web_ui;
use crate::json::*;
use crate::store::files::{data_dir, marker_alive, read_json};

pub fn door_file() -> std::path::PathBuf { data_dir().join("front-door.json") }
pub fn door_info() -> Option<Value> { read_json(&door_file()).filter(|d| marker_alive(d) && d.b("port")) }
/// The port it asks for the first time; after that the one it had, so the shared link keeps pointing at it.
pub const FIRST_PORT: u16 = 7327;
pub fn off() -> bool { std::env::var("HERDR_PROJECTS_NO_SERVER").as_deref() == Ok("1") || crate::runner::inline() }

/// Seen to from outside: the page server is started when the web UI is there and no front door answers. A page server
/// from before the front door existed (it never wrote the front door's file) is told to go, and the next look starts
/// today's. One that wrote it is today's, and opens its front door by itself.
pub fn ensure_door() -> bool {
    if off() || web_ui(false).is_none() { return false; }
    if door_info().is_some() { return true; }
    if let Some(old) = crate::review_page::server_info() {
        if !door_file().exists() { crate::os::term(old.i("pid")); }
        return false;
    }
    crate::review_page::ensure_server();
    door_info().is_some()
}

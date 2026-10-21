//! The board's screen for a terminal: what each part says, in the board's own words and named inks, laid out for the
//! terminal's size with its give-way rules (what goes first when the window is small). The drawer (draw.rs) turns it
//! into cells in herdr's colours. One file per part of the screen.
pub mod fields;
pub mod guard;
pub mod row;
pub mod row_fit;
pub mod row_tall;
pub mod row_wrap;

/// The side panel fits from this width on, when you have it open; below it the same panel opens full screen.
pub const WIDE: usize = 90;
pub mod ask;
pub mod chat;
pub mod chat_call;
pub mod chat_keep;
pub mod chat_text;
pub mod coords;
pub mod hints;
pub mod list;
pub mod menu;
pub mod panel;
pub mod paste;
pub mod pilot;
pub mod setup;
pub mod talk;
pub mod textbox;
pub mod layout;
pub use layout::{screen, Screen};

use crate::board::keys::key_name;
use crate::board::state::State;
use crate::shown::ink::{Ink, Inked};
/// The size warning, top left: a remote view holds this pane's terminal at another size than herdr's window shows.
/// What frees the size comes first, because the right end is what a narrow window or a cut-out loses.
pub fn held_lines(s: &State, width: usize) -> Vec<Inked> {
    let Some(d) = &s.held else { return vec![] };
    if s.guest { return vec![Inked::plain(" ").add(Inked::of(format!("Drawn for a remote view of this pane at {}×{} (herdr's web UI has it open): herdr's own window ({}×{}) shows a cut-out of it until that view is closed", d.columns, d.rows, d.room.0, d.room.1), Ink::Muted)).cut(width)]; }
    let who = d.by.first();
    let text = match who {
        // Taking it changed nothing and no attach holds it: a second herdr window has the session at its size.
        None if s.size.gave_up() => format!("Shown cut off: another herdr window on this session (a second device, a browser tab) holds this pane at {}×{} (herdr's own window: {}×{}). A key typed in this window frees it.", d.columns, d.rows, d.room.0, d.room.1),
        Some((_, secs)) => format!("Shown cut off: a remote view holds this pane at {}×{} (herdr's own window: {}×{}). {} ends that view{} (open {}); closing it where it runs does the same.", d.columns, d.rows, d.room.0, d.room.1, key_name("free"), if d.by.len() > 1 { "s" } else { "" }, crate::shown::words::age(secs * 1000)),
        None => format!("Shown cut off: a remote view holds this pane at {}×{} (herdr's own window: {}×{}). Closing that view frees it.", d.columns, d.rows, d.room.0, d.room.1),
    };
    Inked::of(format!("! {text}"), Ink::Yellow).wrap(width.saturating_sub(2).max(10)).into_iter().take(3).map(|l| Inked::plain(" ").add(l)).collect()
}

//! The one way the board's state changes: `update` takes the state and one event (a key, a paste, the mouse, a tick, what
//! the loader read, what a job answered) and changes the state, and answers what the runtime is to do (effects): a job
//! to run, the state to save, the board to close. It reads no file and asks nothing of herdr: a key press never waits.
pub mod ask;
pub mod heads;
pub mod list;

use super::data::Data;
use super::job::{Done, Job};
use super::keys::Press;
use super::state::State;

#[derive(Clone, Debug)]
pub enum Event {
    Key(Press),
    Paste(String),
    /// The wheel (lines, up negative) or a click, at a cell of the screen.
    Wheel { x: u16, y: u16, by: i64 },
    Click { x: u16, y: u16 },
    Tick(i64),
    Loaded(Box<Data>),
    Done(Done),
    /// Another machine's answer to "list": its tasks and automations, or why it did not answer.
    Remote { label: String, answer: Result<serde_json::Value, serde_json::Value> },
    Machines(Vec<crate::remote::Machine>),
    /// What holds the board's terminal at another size, as last looked at.
    Held(Option<super::state::Held>, Option<bool>),
    /// The program changed under the board: its new stamp.
    Stamp(String),
    /// What herdr shows of the board's pane, for a terminal of this size, and the remote views on its terminal where
    /// the two disagree (size.rs).
    Shown { term: (u16, u16), shown: Option<size::Shown>, by: Option<Vec<(i64, i64)>> },
    /// What taking the terminal back answered.
    Took(Result<(), String>),
    /// The web UI's links that could go through the board's front door (web_ui.rs).
    WebUiOffer(Vec<String>),
}
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Run(Job),
    /// Read everything again now (after a change of yours), not only on the next beat.
    Load,
    /// Save what comes back when the board opens again: selection, filter, draft, panel, chat.
    Save,
    /// Ask other machines for their tasks (`now`: also one that did not answer a moment ago).
    Poll(bool),
    Bell,
    /// Bytes for the terminal itself: the tasks told to it (OSC 7501).
    Osc(String),
    Close,
    Reload,
    /// This terminal becomes an agent's until you detach.
    Attach { command: String, args: Vec<String> },
    /// Take the board's terminal back from the viewer that holds it, at this size (size.rs).
    Take { cols: u16, rows: u16 },
    /// Look whether the herdr web UI has a link that could show the board (web_ui.rs).
    LookWebUi,
}
pub fn update(s: &mut State, ev: Event) -> Vec<Effect> {
    let mut fx = vec![];
    // A key, a paste or the mouse happens now: the time it is measured from (the clock is the test's in tests).
    if !matches!(ev, Event::Tick(_)) { s.now = crate::clock::now(); }
    match ev {
        Event::Key(p) => keys::handle(s, &p, &mut fx),
        Event::Paste(text) => paste(s, &text),
        Event::Wheel { y, by, .. } => mouse::wheel(s, y as usize, by, &mut fx),
        Event::Click { x, y } => mouse::click(s, x as usize, y as usize, &mut fx),
        Event::Tick(now) => beat::tick(s, now, &mut fx),
        Event::Loaded(d) => beat::loaded(s, *d, &mut fx),
        Event::Done(d) => done::apply(s, d, &mut fx),
        Event::Remote { label, answer } => remotes::answered(s, &label, answer),
        Event::Machines(list) => remotes::machines(s, list, &mut fx),
        Event::Held(h, viewed) => beat::held(s, h, viewed),
        Event::Stamp(st) => beat::stamp(s, st, &mut fx),
        Event::Shown { term, shown, by } => { let now = s.now; size::see(s, (term.0 as usize, term.1 as usize), shown, by, now, &mut fx) }
        Event::Took(r) => size::took(s, r),
        Event::WebUiOffer(urls) => web_ui::offered(s, urls),
    }
    pending::start(s, &mut fx); // what was just asked for shows on its row now, not when it is done
    fx
}
/// A paste goes into the text box whole: its keys never act on the board.
fn paste(s: &mut State, text: &str) {
    if s.leaving || s.ask.is_some() || s.menu.is_some() || !s.paste_text.is_empty() { return; }
    if let Some(st) = &s.setup { if !st.typing { return; } }
    if s.editing.as_ref().map(|e| e.typing.is_empty()).unwrap_or(false) { return; }
    s.input.insert(text);
    if s.mode == super::state::Mode::Find { list::relist(s); }
}
pub mod actions;
pub mod archive_done;
pub mod autos;
pub mod beat;
pub mod call;
pub mod chat;
pub mod coords;
pub mod done;
pub mod edit;
pub mod edit_coord;
pub mod keys;
pub mod landing;
pub mod menu;
pub mod mouse;
pub mod paste;
pub mod pending;
pub mod pilot;
pub mod remotes;
pub mod setup;
pub mod size;
pub mod web;
pub mod web_ui;

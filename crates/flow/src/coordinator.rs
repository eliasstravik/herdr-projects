//! The Coordinator: the agent behind the box at the bottom of the board. One long-lived, ordinary herdr agent per herdr
//! session, in a tab of its own ("Coordinator") in the home space, started with the same arguments as any working agent.
//! What you type in the box is typed into that one conversation; it is kept across board openings, and resumed when the
//! program exited or herdr restarted. Its state is one file beside the tasks. Nothing here blocks the board: `send`,
//! `restart`, `fresh`, `stop` and `open` only write the request down and start the detached job (`pump`). One area per file.
pub mod co;
pub mod far;
pub mod look;
pub mod look_live;
pub mod place;
pub mod pump;
pub mod requests;
pub mod rules;
pub mod rules_autopilot;
pub mod rules_news;
pub mod rules_place;
pub mod sidebar;
pub mod state;
pub mod who;

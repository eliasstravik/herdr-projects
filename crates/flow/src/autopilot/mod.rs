//! Autopilot: a switch on a coordinator (docs/autopilot.md). While it is on, the board sends that coordinator a short
//! wake message in the chat it already has whenever its timer or a board event says there may be something to do, and
//! the coordinator works toward its goal with ordinary tasks, within its limits. Off, the coordinator is exactly as it
//! always was. One area per file: its record (record.rs), switching it (switch.rs, command.rs), waking it (tick.rs,
//! events.rs, look.rs, idle.rs, wake.rs), what its turn may do and read (turn.rs, allow.rs, add.rs, limits.rs, care.rs,
//! history.rs), what the board shows of it (shown.rs), and other machines' (far.rs, heard.rs).
pub mod add;
pub mod allow;
pub mod care;
pub mod command;
pub mod events;
pub mod far;
pub mod heard;
pub mod history;
pub mod idle;
pub mod limits;
pub mod look;
pub mod record;
pub mod shown;
pub mod switch;
pub mod tick;
pub mod turn;
pub mod wake;

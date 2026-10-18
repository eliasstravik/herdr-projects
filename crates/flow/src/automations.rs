//! Automations: tasks made on a schedule, or when something happens on the board (docs/automations.md). An automation is
//! a saved task plus a trigger; each firing adds an ordinary task and puts an agent on it. One area per file.
pub mod command;
pub mod condition;
pub mod cron;
pub mod events;
pub mod fire;
pub mod list;
pub mod show;
pub mod tick;
pub mod told;
pub mod triggers;

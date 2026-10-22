//! Settings as saved and read: yours, your entry for a repo, one task's own, one coordinator's own, and the repo's file. What there is to set
//! is settings_list.rs: a new setting goes there. Each part is a file of its own here: the values, the places, the one
//! lookup, set and unset, the task branch's name, where a worktree starts, the issues a task closes, `show`, and the one-time move from the old names.
pub mod branch;
pub mod change;
pub mod coord;
pub mod from;
pub mod issues;
pub mod key_map;
pub mod lookup;
pub mod migrate;
pub mod pilot_values;
pub mod places;
pub mod show;
pub mod values;

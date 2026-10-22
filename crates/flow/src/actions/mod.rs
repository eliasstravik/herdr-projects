//! The exact actions of the board, also the subcommands any agent can run from its shell. Each one only records a
//! request under the task lock and, when a long job follows, starts the detached runner; none of them waits for it.
pub mod archive;
pub mod archive_done;
pub mod archive_project;
pub mod filing;
pub mod finish_line;
pub mod finish_stuck;
pub mod limits;
pub mod plan;
pub mod settings;
pub mod shared;
pub mod start;
pub mod stop_ends;
pub mod talk;

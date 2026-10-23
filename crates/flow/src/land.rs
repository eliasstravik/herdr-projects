//! Landing: bring the branch up to date with main in the task's own worktree, merge (through the pull request when
//! there is one, by a push to a branch it started from by name, else locally), then clean up. A pull request merges
//! once GitHub's checks pass (checks.rs), as the token can read them (readable.rs). Conflicts are rolled back and saved;
//! a checkout with unsaved files is never forced. One part per file in land/: the clean-up, what it ends first and what
//! keeps a space in use, GitHub's checks, how they are read and the ones it expects, the merge, the push to a named
//! branch, and the GitHub issues it closes.
pub mod checks;
pub mod cleanup;
pub mod cleanup_ends;
pub mod closes;
pub mod direct;
pub mod expected;
pub mod in_use;
pub mod merge;
pub mod readable;
pub mod triggers;
pub use cleanup::{cleanup, cleanup_job, cleanup_with, tidy, Cleanup};
pub use merge::{land_core, land_job};

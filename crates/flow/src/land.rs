//! Landing: bring the branch up to date with main in the task's own worktree, merge (through the pull request when
//! there is one, else locally), then clean up. A pull request merges once GitHub's checks pass (checks.rs), as the
//! token can read them (readable.rs). Conflicts are rolled back and saved; a checkout with unsaved files is never
//! forced. One part per file in land/: the clean-up, GitHub's checks, how they are read and the ones it expects, the
//! merge, and whether a task's commits are at the remote before its checkout goes.
pub mod checks;
pub mod cleanup;
pub mod expected;
pub mod merge;
pub mod published;
pub mod readable;
pub mod triggers;
pub use cleanup::{cleanup, cleanup_job, cleanup_with, tidy, Cleanup};
pub use merge::{land_core, land_job};

//! Delegating: a herdr worktree workspace, the repo's setup, then the agent in its root pane with the first prompt. A
//! delegated task is one herdr worktree workspace with its agent pane in it; herdr does the running. One part per file
//! in delegate/: where a worktree starts, the rules and the first prompt, starting the program, and the jobs.
pub mod from;
pub mod jobs;
pub mod launch;
pub mod rules;
pub mod worktree;
pub use jobs::{delegate_job, reopen, restart_job};
pub use launch::{agent_name, launch_extras, resumable, start_agent};
pub use rules::{first_prompt, rules, task_line, turned_words, WAIT_RULE};

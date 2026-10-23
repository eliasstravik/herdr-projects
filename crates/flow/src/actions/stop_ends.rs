//! What a stop ends beside the agent, which stays for a later reply: what it launched in its pane (herdr/launched.rs,
//! the whole tree but its helpers) and what runs from the task's worktree or temporary folder (leftovers/ends.rs),
//! never what runs in the task's other panes, nor the board's own runs (its runner is stopped by itself). What it
//! could not end is named for you to end: a process that would not go, or one started for the task that left the
//! agent's terminal and works elsewhere.
pub use crate::leftovers::ends::Ended;
use crate::json::*;

/// End what the task's agent started. `agent`: the program herdr sees in its pane (None: no agent there, so nothing in
/// the pane is ended, only what runs from its folders). Where herdr does not say what runs in its panes, nothing in its
/// folders is ended (the agent and your shells there could not be told apart): each is named instead.
pub fn end_launched(t: &Obj, agent: Option<&str>) -> Ended { crate::leftovers::ends::end_started(t, &crate::flow::task::h(t), agent).0 }

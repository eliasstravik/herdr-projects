//! The list's top row, which follows the machine picker: `All machines` with every machine shown (its coordinator is
//! the one across every machine, this machine's top-level one), else the one machine shown. With only one machine, that
//! machine is the top row and its coordinator is the top-level one too: all machines are that one. There is no main
//! Coordinator besides: the board opens on the top row, and the chat talks to its coordinator where it talks to no
//! other row's agent. Also how each agent is named: `vm-str's coordinator`, `GTM-14's agent`.
use super::coords::Target;
use super::heads::{machine_id, HEAD};
use crate::board::state::State;

/// The `All machines` row's id.
pub fn all_id() -> String { format!("{HEAD}all") }
pub fn is_all(id: &str) -> bool { id == all_id() }
/// The top row's id: `All machines`, or the machine shown.
pub fn top_id(s: &State) -> String { if s.scope == "*" { all_id() } else { machine_id(&s.scope) } }
/// `vm-str's coordinator`, `gtm-skills' coordinator`.
pub fn coordinator_of(name: &str) -> String { format!("{} coordinator", possessive(name)) }
/// A name at the start of a sentence: `The coordinator of all machines`, `gtm's coordinator` (a name keeps its case).
pub fn sentence(name: &str) -> String { match name.strip_prefix("the ") { Some(r) => format!("The {r}"), None => name.to_string() } }
pub fn possessive(name: &str) -> String { if name.ends_with('s') { format!("{name}'") } else { format!("{name}'s") } }
/// The coordinator across every machine: the top-level one of this machine.
pub fn all_target() -> Target { Target { host: String::new(), to: String::new(), name: "the coordinator of all machines".into() } }
/// Is this machine the only one: its row is then the top row, and its coordinator the top-level one.
pub fn alone(s: &State) -> bool { s.remotes.is_empty() }
/// The one machine shown is the top row: this one, or another that answered (one that did not says so above the list).
pub fn answered(s: &State) -> bool { s.scope != "*" && (s.scope.is_empty() || s.remotes.get(&s.scope).is_some_and(|m| m.ok)) }
/// The top row's coordinator: who the chat talks to where it talks to no other row's agent. None where none can run
/// (`coordinator.agent none` here): the box then adds tasks.
pub fn top(s: &State) -> Option<Target> {
    if s.archive_view || (s.scope != "*" && !answered(s)) { return None; }
    let t = super::coords::of_head(s, &top_id(s))?;
    if t.host.is_empty() && !s.data.can_dispatch { return None; }
    Some(t)
}

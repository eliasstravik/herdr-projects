//! What herdr's `completion_seq` says of an agent's turns: herdr gives it at rest (the turn that ended last) and leaves
//! it out while a turn is going, so a missing one while the agent works means "in a turn", never "none finished yet".
use crate::json::*;

/// Still in the turn that began when herdr said `seq` (read at rest): the same completion, or none while it works.
pub fn same_turn(a: &Value, seq: &Value) -> bool {
    a.at("completion_seq") == seq || (a.s("agent_status") == "working" && a.at("completion_seq").is_null())
}
/// Its completion_seq, and "working" in its place while herdr leaves it out during a turn: any turn that ends after
/// this was read is then a change.
pub fn completion(a: &Value) -> Value {
    if a.s("agent_status") == "working" && a.at("completion_seq").is_null() { json!("working") } else { a.at("completion_seq").clone() }
}

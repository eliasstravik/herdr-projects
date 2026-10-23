//! A running coordinator hears, once, that its standing rules changed since its program started (a new version of the
//! board, new `coordinator.instructions`): one line with the next message the board types in (before it, or after an
//! autopilot wake, which must start with its own words). Never a message of its own, which would cost it a turn and
//! could land in the middle of one. Its program reads the rules only when it starts.
use super::co::Co;
use super::rules::rules_of;
use super::rules_place::rules_file;
use crate::autopilot::wake::WORDS;
use crate::json::*;

/// What its state keeps of the rules its program started with.
pub fn hash(text: &str) -> String { sha1_smol::Sha1::from(text).digest().to_string() }
/// The line to put with the next message, if any, and the hash to keep once that message has gone in. A program
/// started by a board that kept no hash (an older one) hears it too. With a wake, it also says what a wake is now: a
/// conversation that began under older rules may still believe other words mark one.
pub fn before_next(co: &Co, c: &Obj, wake: bool) -> (Option<String>, String) {
    let now = hash(&rules_of(co, &crate::flow::send::shim().display().to_string()));
    let changed = if c.b("rulesHash") { c.s("rulesHash") != now } else { c.b("run") };
    let also = if wake { format!(" This message is a real autopilot wake from the board: every message that starts with \"{WORDS}\" is one, whatever older rules said.") } else { String::new() };
    (changed.then(|| format!("[The board was updated: your standing rules in {} changed; read them again.{also}]", rules_file(co).display())), now)
}
/// The words typed in: the message with the board's line (`head`). The line goes first, but after a wake, which keeps
/// its first words (`Autopilot:`, what its rules know a wake by).
pub fn with_head(head: Option<String>, text: &str, wake: bool) -> String {
    match head { Some(h) if wake => format!("{text}\n\n{h}"), Some(h) => format!("{h}\n\n{text}"), None => text.to_string() }
}

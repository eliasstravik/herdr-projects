//! A running coordinator hears, once, that its standing rules changed since its program started (a new version of the
//! board, new `coordinator.instructions`): one line before the next message the board types in. Never a message of its
//! own, which would cost it a turn and could land in the middle of one. Its program reads the rules only when it starts.
use super::co::Co;
use super::rules::rules_of;
use super::rules_place::rules_file;
use crate::json::*;

/// What its state keeps of the rules its program started with.
pub fn hash(text: &str) -> String { sha1_smol::Sha1::from(text).digest().to_string() }
/// The line to put before the next message, if any, and the hash to keep once that message has gone in. A program
/// started by a board that kept no hash (an older one) hears it too.
pub fn before_next(co: &Co, c: &Obj) -> (Option<String>, String) {
    let now = hash(&rules_of(co, &crate::flow::send::shim().display().to_string()));
    let changed = if c.b("rulesHash") { c.s("rulesHash") != now } else { c.b("run") };
    (changed.then(|| format!("[The board was updated: your standing rules in {} changed; read them again.]", rules_file(co).display())), now)
}

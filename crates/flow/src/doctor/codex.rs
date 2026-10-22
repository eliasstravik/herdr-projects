//! Codex's own config.toml, for `doctor`: a file that does not read stops every Codex agent at its start, and the
//! table an older board added a second time is named with how to remove it (agents/codex_trust.rs); the file is
//! never changed here.
use super::Found;

pub fn check() -> Vec<Found> { crate::agents::codex_trust::problem().map(|(what, fix)| Found::bad(what, fix)).into_iter().collect() }

//! How the board writes a key's modifiers (`board.keyStyle`): `pc` writes alt, `mac` writes opt, as the Option key
//! that sends it on a Mac; `auto` follows the system this program runs on. Ctrl is ctrl in both: a Mac's terminal
//! passes Control on, never Cmd. docs/keys.md keeps the built-in names.
use crate::store::settings::lookup::setting;
use std::sync::atomic::{AtomicU8, Ordering};

pub const SETTING: &str = "board.keyStyle";
/// The system `auto` goes by: 0 the one this program runs on, 1 a PC, 2 a Mac.
static SYSTEM: AtomicU8 = AtomicU8::new(0);
/// For the tests: `auto` as on a Mac (`Some(true)`), on a PC, or (`None`) as on the system they run on, so that a test
/// says the same on every system.
pub fn system_is(mac: Option<bool>) { SYSTEM.store(match mac { None => 0, Some(false) => 1, Some(true) => 2 }, Ordering::Relaxed); }
fn system_mac() -> bool { match SYSTEM.load(Ordering::Relaxed) { 0 => cfg!(target_os = "macos"), n => n == 2 } }
/// Are keys written as on a Mac? `auto` cannot see the keyboard of a Mac that reaches this machine over SSH: that is
/// what `mac` is for.
pub fn mac() -> bool {
    if super::remap::built() { return false; }
    match setting(SETTING, None).as_str().unwrap_or("auto") { "mac" => true, "pc" => false, _ => system_mac() }
}
/// A key as the board writes it, in your style: "alt+c" is "opt+c" on a Mac. Words that are no chord stay as they are.
pub fn spell(shown: &str) -> String {
    if !mac() { return shown.to_string(); }
    let parts: Vec<&str> = shown.split('+').collect();
    let last = parts.len() - 1;
    parts.iter().enumerate().map(|(n, p)| if n < last && *p == "alt" { "opt" } else { p }).collect::<Vec<_>>().join("+")
}
/// A modifier's own word, in your style: "alt" is "opt" on a Mac.
pub fn modifier(m: &str) -> &str { if m == "alt" && mac() { "opt" } else { m } }


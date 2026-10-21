//! How the board writes a key's modifiers (`board.keyStyle`): `pc` writes alt, `mac` writes opt, as the Option key
//! that sends it on a Mac; `auto` follows the system this program runs on. Ctrl is ctrl in both: a Mac's terminal
//! passes Control on, never Cmd. docs/keys.md keeps the built-in names.
use crate::store::settings::lookup::setting;

pub const SETTING: &str = "board.keyStyle";
/// Are keys written as on a Mac? `auto` cannot see the keyboard of a Mac that reaches this machine over SSH: that is
/// what `mac` is for.
pub fn mac() -> bool {
    if super::remap::built() { return false; }
    match setting(SETTING, None).as_str().unwrap_or("auto") { "mac" => true, "pc" => false, _ => cfg!(target_os = "macos") }
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


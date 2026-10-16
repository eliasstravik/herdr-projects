//! How `board.keys` is written: your own chord for some of the board's keys, as `name=chord` pairs. Only the writing
//! is checked here; which names are keys and which chords are free is for the keys' table to say
//! (crates/flow/src/board/remap.rs), which sw-core cannot see.
use super::values::NONE;
use crate::err::{Error, Res};
use crate::json::*;
use crate::store::settings_list::Entry;

const MODS: [&str; 3] = ["ctrl", "alt", "shift"];
/// The keys a terminal names, beside a character and f1 to f12 (board/run/input.rs reads the same ones).
const NAMED: [&str; 14] = ["enter", "tab", "escape", "space", "up", "down", "left", "right", "home", "end", "pageup", "pagedown", "backspace", "delete"];

/// One spelling of a chord, as the board writes it: the modifiers in one order, `esc` is escape, `return` is enter,
/// a capital by itself is the letter with shift. None for what is no chord.
pub fn chord_of(c: &str) -> Option<String> {
    let parts: Vec<&str> = c.trim().split('+').map(str::trim).collect();
    let (key, mods) = parts.split_last()?;
    let mods: Vec<String> = mods.iter().map(|m| m.to_lowercase()).collect();
    if mods.iter().enumerate().any(|(i, m)| !MODS.contains(&m.as_str()) || mods[..i].contains(m)) { return None; }
    let mut mods: Vec<&str> = mods.iter().map(String::as_str).collect();
    let one: Vec<char> = key.chars().collect();
    if one.len() == 1 && one[0].is_uppercase() && mods.is_empty() { mods.push("shift"); } // `Ctrl+O` is ctrl and o
    let key = key.to_lowercase();
    let key = match key.as_str() { "return" => "enter", "esc" => "escape", "pgup" => "pageup", "pgdn" => "pagedown", k => k }.to_string();
    // A terminal says f1, never f01: the number in its one spelling.
    let fkey = key.strip_prefix('f').filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())).and_then(|n| n.parse::<u8>().ok()).filter(|n| (1..=12).contains(n));
    let f = fkey.is_some();
    let key = fkey.map(|n| format!("f{n}")).unwrap_or(key);
    let char = key.chars().count() == 1 && key.chars().all(|ch| !ch.is_control() && !ch.is_whitespace());
    if !(NAMED.contains(&key.as_str()) || f || char) { return None; }
    let mut out: Vec<&str> = MODS.into_iter().filter(|m| mods.contains(m)).collect();
    out.push(&key);
    Some(out.join("+"))
}
/// The pairs as written, in order: (name, chord as written). Err: the part that is no pair.
pub fn pairs(v: &Value) -> Result<Vec<(String, String)>, String> {
    if let Value::Object(o) = v { return o.iter().map(|(k, c)| if c.is_string() { Ok((k.trim().to_string(), text(c).trim().to_string())) } else { Err(k.clone()) }).collect(); }
    let Value::String(s) = v else { return Err(text(v)) };
    let s = crate::re!(r"\s*=\s*").replace_all(s.trim(), "=");
    s.split(|c: char| c == ',' || c.is_whitespace()).filter(|x| !x.is_empty()).map(|x| match x.split_once('=') {
        Some((n, c)) if !n.is_empty() && !c.is_empty() => Ok((n.to_string(), c.to_string())),
        _ => Err(x.to_string()),
    }).collect()
}
/// The value as kept: `none`, or the pairs with each chord in its one spelling (`chatMessage=ctrl+o, menu=f1`).
pub fn check(e: &Entry, v: &Value) -> Res<Value> {
    if text(v).trim().to_lowercase() == NONE { return Ok(json!(NONE)); }
    let like = "name=chord pairs, such as chatMessage=ctrl+o, chatScrollUp=ctrl+up (the names are in docs/keys.md), or none";
    let list = pairs(v).map_err(|bad| Error::refused(format!("{}: \"{bad}\" is not a key's name and a chord. It takes {like}.", e.name)))?;
    if list.is_empty() { return Err(Error::refused(format!("{} takes {like}.", e.name))); }
    let mut out: Vec<String> = vec![];
    for (i, (name, c)) in list.iter().enumerate() {
        if list[..i].iter().any(|(n, _)| n.eq_ignore_ascii_case(name)) { return Err(Error::refused(format!("{}: {name} is given twice. Give each key one chord.", e.name))); }
        let Some(chord) = chord_of(c) else {
            return Err(Error::refused(format!("{}: \"{c}\" is not a chord. A chord is a key (a letter, f1 to f12, up, enter, space, …) after ctrl, alt or shift, joined by +.", e.name)));
        };
        out.push(format!("{name}={chord}"));
    }
    Ok(json!(out.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn written() {
        assert_eq!(chord_of("Ctrl+O").as_deref(), Some("ctrl+o"));
        assert_eq!(chord_of("shift+alt+Up").as_deref(), Some("alt+shift+up"));
        assert_eq!(chord_of("ctrl+Esc").as_deref(), Some("ctrl+escape"));
        assert_eq!(chord_of("Y").as_deref(), Some("shift+y"));
        assert_eq!(chord_of("F5").as_deref(), Some("f5"));
        assert_eq!(chord_of("f01").as_deref(), Some("f1"));
        assert_eq!(chord_of("f13"), None);
        assert_eq!(chord_of("ctrl+ctrl+o"), None);
        assert_eq!(chord_of("hyper+o"), None);
        assert_eq!(chord_of("ctrl+oo"), None);
        assert_eq!(pairs(&json!("chatMessage = ctrl+o,  menu=f1")).unwrap(), vec![("chatMessage".into(), "ctrl+o".into()), ("menu".into(), "f1".into())]);
        assert_eq!(pairs(&json!("chatMessage")).unwrap_err(), "chatMessage");
    }
}

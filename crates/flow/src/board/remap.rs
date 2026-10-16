//! Your own chords for the board's keys (`board.keys`): checked against the keys' table, so that no two keys share a
//! chord and none takes the key that opens the board, and read by keys::is and every key's label. A pair that does not
//! hold is refused when it is set; one written by hand that does not hold is left out, and `show` says why.
use super::keys::{norm, Key, KEYS};
use crate::json::*;
use crate::store::settings::key_map::pairs;
use crate::store::settings::places::{yours, Mine};
use crate::store::settings::values::nearest;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

pub const SETTING: &str = "board.keys";
/// Each moved key's chords by its name, the one it is shown by first.
pub type Moved = BTreeMap<&'static str, Vec<String>>;

/// A plain letter counts in both cases, as the board's own letters do (`y` and `Y`).
fn chords_for(chord: &str) -> Vec<String> {
    let mut out = vec![chord.to_string()];
    if chord.len() == 1 && chord.chars().all(|c| c.is_ascii_lowercase()) { out.push(format!("shift+{chord}")); }
    out
}
fn typed_char(chord: &str) -> bool { let n = norm(chord); let key = n.trim_start_matches("shift+"); !n.contains("ctrl+") && !n.contains("alt+") && key.chars().count() == 1 }
/// The chords that press a key, moved or not, as norm spells them.
fn presses(k: &Key, moved: &Moved) -> Vec<String> {
    match moved.get(k.name) { Some(c) => c.iter().map(|c| norm(c)).collect(), None => k.keys.iter().map(|c| norm(c)).chain(k.raw.iter().map(|r| norm(&super::keys::Press::char(r.chars().next().unwrap_or(' ')).chord))).collect() }
}
/// The key that opens the board, as presses (herdr's prefix keys, then the key): the chords the board never gets.
fn opener(board: &Option<(Vec<String>, String)>) -> Vec<(String, String)> {
    match board {
        Some((first, key)) if first.is_empty() => vec![(norm(key), format!("{key} opens the board"))],
        Some((first, key)) => first.iter().map(|f| (norm(f), format!("{f} is herdr's prefix, which the key that opens the board starts with (prefix+{key})"))).collect(),
        None => vec![],
    }
}
/// The value read: the keys it moves, and one sentence for each pair that does not count. `board`: the key that opens
/// the board as presses, where it is to be checked.
pub fn read(v: &Value, board: &Option<(Vec<String>, String)>) -> (Moved, Vec<String>) {
    let mut moved = Moved::new();
    let mut bad: Vec<String> = vec![];
    if text(v).is_empty() || text(v) == "none" { return (moved, bad); }
    let Ok(list) = pairs(v) else { return (moved, vec![format!("{SETTING} is not name=chord pairs")]) };
    let names: Vec<&str> = KEYS.iter().map(|k| k.name).collect();
    let mut said: BTreeMap<&'static str, String> = BTreeMap::new();
    for (name, chord) in &list {
        let Some(k) = KEYS.iter().find(|k| k.name.eq_ignore_ascii_case(name)) else {
            bad.push(format!("{name}={chord}: no key is called {name}.{}", nearest(name, &names).map(|n| format!(" Did you mean {n}?")).unwrap_or_default())); continue;
        };
        let Some(chord) = crate::store::settings::key_map::chord_of(chord) else { bad.push(format!("{name}={chord}: that is not a chord")); continue };
        if k.fixed { bad.push(format!("{}={chord}: {} is the text box's own key and cannot be moved", k.name, k.name)); continue; }
        if typed_char(&chord) && k.raw.is_empty() { bad.push(format!("{}={chord}: a plain {chord} is typed into the text box. Give it ctrl or alt", k.name)); continue; }
        if let Some((_, why)) = opener(board).into_iter().find(|(c, _)| chords_for(&chord).iter().any(|x| norm(x) == *c)) { bad.push(format!("{}={chord}: {why}", k.name)); continue; }
        said.insert(k.name, chord.clone());
        moved.insert(k.name, chords_for(&chord));
    }
    // Two keys on one chord: each moved key in it is left out, until no two share one (the built-in ones never do).
    loop {
        let mut clash: Option<(&'static str, String)> = None;
        'find: for k in KEYS.iter().filter(|k| moved.contains_key(k.name)) {
            for other in KEYS.iter().filter(|o| o.name != k.name) {
                let theirs = presses(other, &moved);
                if let Some(c) = presses(k, &moved).into_iter().find(|c| theirs.contains(c)) {
                    let mine = &said[k.name];
                    let why = match said.get(other.name).filter(|_| moved.contains_key(other.name)) {
                        Some(o) => format!("{}={mine}: {}={o} is the same chord. Give each key its own", k.name, other.name),
                        None => format!("{}={mine}: {} is {}'s key already. Move {} too, or choose another", k.name, if c == norm(mine) { mine.clone() } else { c.clone() }, other.name, other.name),
                    };
                    clash = Some((k.name, why));
                    break 'find;
                }
            }
        }
        let Some((name, why)) = clash else { break };
        moved.remove(name);
        bad.push(why);
    }
    (moved, bad)
}

thread_local! {
    static AS_BUILT: Cell<bool> = const { Cell::new(false) };
    static LAST: RefCell<Option<(Arc<Mine>, String, Rc<Moved>)>> = const { RefCell::new(None) };
}
/// The keys as they come, whatever you set: for the made pages (docs/keys.md keeps the built-in chords).
pub fn as_built<T>(f: impl FnOnce() -> T) -> T { let was = AS_BUILT.with(|b| b.replace(true)); let out = f(); AS_BUILT.with(|b| b.set(was)); out }
/// herdr's config.toml as it is on disk now: where it is and when it last changed.
fn config_mark() -> String {
    use std::os::unix::fs::MetadataExt;
    let f = crate::extras::herdr_config();
    let m = std::fs::metadata(&f).map(|m| format!("{}|{}|{}.{}", m.ino(), m.size(), m.mtime(), m.mtime_nsec())).unwrap_or_default();
    format!("{}|{m}", f.display())
}
/// The keys your settings move, as `show` counts them (the key that opens the board included), read again only when
/// your settings or herdr's config changed.
fn now() -> Rc<Moved> {
    let (mine, mark) = (yours(), config_mark());
    if let Some(m) = LAST.with(|l| l.borrow().as_ref().filter(|(was, at, _)| Arc::ptr_eq(was, &mine) && *at == mark).map(|(_, _, m)| m.clone())) { return m; }
    let v = mine.you.get(SETTING).cloned().unwrap_or(Value::Null);
    let opener = if v.is_null() || text(&v) == "none" { None } else { crate::extras::board_presses() };
    let m = Rc::new(read(&v, &opener).0);
    LAST.with(|l| *l.borrow_mut() = Some((mine, mark, m.clone())));
    m
}
/// Your chords for the key with this name, where you moved it.
pub fn chords(name: &str) -> Option<Vec<String>> {
    if AS_BUILT.with(|b| b.get()) { return None; }
    now().get(name).cloned()
}
/// Why pairs of the value you have do not count, one sentence each (for `show`).
pub fn problems(board: &Option<(Vec<String>, String)>) -> Vec<String> {
    let mine = yours();
    mine.you.get(SETTING).map(|v| read(v, board).1).unwrap_or_default().into_iter().map(|p| format!("{SETTING}: {p}")).collect()
}
/// A value about to be set: refused with every pair that would not count.
pub fn refusal(v: &Value, board: &Option<(Vec<String>, String)>) -> Option<String> {
    let bad = read(v, board).1;
    (!bad.is_empty()).then(|| format!("{SETTING}: {}. Nothing was changed.", bad.join("; ")))
}

//! Is a model name one the program takes: checked when a setting names it (`codex/<model>`), so a wrong one is refused
//! there with the names it does take, not found out when the agent fails to start (docs/models.md). The names are the
//! program's own list where it has one (kinds.rs `models`), else its short and full names written in the table.
pub mod listed;
pub mod say;

use super::kinds::{kind, Kind};
use crate::store::settings::values::distance;
pub use listed::{list_command, listed, Listing, Names};

/// At most this many names in a refusal; `herdr-projects models <program>` lists them all.
const SHOWN: usize = 12;

/// The name with what follows its first mark cut off (cursor's `[effort=high]`, pi's `:high`), with the mark. A longer
/// mark is a whole ending (claude's `[1m]`).
fn cut<'a>(a: &Kind, m: &'a str) -> Option<(&'a str, &'static str)> {
    a.models.suffix.iter().find_map(|mark| if mark.len() > 1 { m.strip_suffix(mark).filter(|b| !b.is_empty()).map(|b| (b, *mark)) } else { m.find(mark).filter(|i| *i > 0).map(|i| (&m[..i], *mark)) })
}
fn in_list(names: &Names, a: &Kind, m: &str) -> bool {
    if names.taken.iter().any(|x| x == m) { return true; }
    match cut(a, m) {
        // A name with parameters is that of a model whose variants its list holds: claude-opus-4-8 for claude-opus-4-8-high.
        Some((base, "[")) if !a.models.list.is_empty() => names.taken.iter().any(|x| x == base || x.strip_prefix(base).map(|r| r.starts_with('-')).unwrap_or(false)),
        Some((base, _)) => names.taken.iter().any(|x| x == base),
        None => false,
    }
}
/// The names nearest to what was typed: one that holds it first, then by how few edits make one the other.
fn nearest(m: &str, names: &[String], n: usize) -> Vec<String> {
    let lm = m.to_lowercase();
    let mut v: Vec<(usize, &String)> = names.iter().map(|x| { let lx = x.to_lowercase(); (if lx.contains(&lm) { 0 } else { 1 + distance(&lm, &lx) }, x) }).collect();
    v.sort_by_key(|(d, x)| (*d, x.len()));
    v.into_iter().take(n).map(|(_, x)| x.clone()).collect()
}
/// The names a program takes: its own list, else its short and full names from the table.
pub fn names_of(k: &str, fresh: bool) -> Listing {
    let Some(a) = kind(k) else { return Listing::Unchecked };
    if !a.models.list.is_empty() { return listed(k, fresh); }
    if (a.models.aliases.is_empty() && a.models.known.is_empty()) || listed::own_provider(k) { return Listing::Unchecked; }
    let all: Vec<String> = a.models.aliases.iter().chain(a.models.known.iter()).map(|s| s.to_string()).collect();
    Listing::Names(Names { taken: all.clone(), shown: all })
}
/// Where a program's names come from, as a refusal says it.
fn source(a: &Kind) -> String { if a.models.list.is_empty() { "its short names, each the newest of its kind, and its full names".into() } else { format!("from `{}`", list_command(a.name)) } }
/// Why this program does not take this model, naming the ones it does; None when it takes it, or when its names are
/// not to be checked (no list of its own, a provider of its own, not installed here). A list that could not be read
/// is a refusal too, with why: the name would otherwise go to the program unchecked.
pub fn why_not(k: &str, m: &str) -> Option<String> {
    let a = kind(k)?;
    if m.is_empty() || a.model.is_empty() { return None; }
    let mut names = match names_of(k, false) {
        Listing::Names(n) => n,
        Listing::Unchecked => return None,
        Listing::Failed(why) => return Some(format!("{k}'s list of models could not be read, so \"{m}\" cannot be checked: {why}. Run `{}` in a terminal here to see why and fix that, or leave the model out ({k} then uses its own default).", list_command(k))),
    };
    if in_list(&names, a, m) { return None; }
    if !a.models.list.is_empty() { if let Listing::Names(n) = listed(k, true) { names = n; } } // asked again: the program may have a new one
    if in_list(&names, a, m) { return None; }
    // Listed only as its variants (claude-opus-4-8-high): with parameters in brackets, the name alone is taken.
    if a.models.suffix.contains(&"[") && in_list(&names, a, &format!("{m}[")) {
        return Some(format!("{k} takes \"{m}\" only with its parameters in brackets, as in {k}/{m}[effort=high], or as one of its listed variants ({}).", nearest(m, &names.taken, 3).join(", ")));
    }
    let shown = if names.shown.is_empty() { &names.taken } else { &names.shown };
    let near = nearest(m, shown, 1).into_iter().next().filter(|x| { let d = distance(&m.to_lowercase(), &x.to_lowercase()); x.to_lowercase().contains(&m.to_lowercase()) || d <= 3.max(m.len() / 2) });
    // The nearest ones, in the program's own order (newest first, where it has one).
    let close = nearest(m, shown, SHOWN);
    let some: Vec<String> = shown.iter().filter(|x| close.contains(x)).cloned().collect();
    let more = if shown.len() > SHOWN { format!(", and {} more: `herdr-projects models {k}` lists them all", shown.len() - SHOWN) } else { String::new() };
    Some(format!("{k} has no model \"{m}\".{} It takes {}{more} ({}). Write it after the program: {k}/{}.",
        near.as_ref().map(|n| format!(" Did you mean {n}?")).unwrap_or_default(), some.join(", "), source(a), near.or_else(|| some.first().cloned()).unwrap_or_default()))
}
/// In a few words, how a model is given to this program and where its names come from (`herdr-projects models`).
pub fn how(k: &str) -> String {
    let Some(a) = kind(k) else { return String::new() };
    if a.model.is_empty() { return format!("{k} takes no model when the board starts it: choose it in {k}'s own settings."); }
    if !a.models.list.is_empty() { return format!("the names `{}` lists", list_command(k)); }
    if !a.models.aliases.is_empty() || !a.models.known.is_empty() { return format!("a short name ({}), each the newest of its kind, or a full name such as {}", a.models.aliases.join(", "), a.models.known.first().copied().unwrap_or("")); }
    format!("any name {k} takes (not checked: {k} has no list of them)")
}

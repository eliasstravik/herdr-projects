//! Is a model name one the program takes: checked when a setting names it (`codex/<model>`), so a wrong one is refused
//! there with the names it does take, not found out when the agent fails to start (docs/models.md). The names are the
//! program's own list where it has one (kinds.rs `models`), else its aliases and the start of its full names.
pub mod listed;
pub mod say;

use super::kinds::{kind, program, Kind};
use crate::store::settings::values::distance;
pub use listed::{listed, Names};

/// At most this many names in a refusal; `herdr-projects models <program>` lists them all.
const SHOWN: usize = 12;

/// The name with what follows its first mark cut off (cursor's `[effort=high]`, pi's `:high`), with the mark.
fn cut<'a>(a: &Kind, m: &'a str) -> Option<(&'a str, &'static str)> {
    a.models.suffix.iter().find_map(|mark| m.find(mark).filter(|i| *i > 0).map(|i| (&m[..i], *mark)))
}
fn in_list(names: &Names, a: &Kind, m: &str) -> bool {
    if names.taken.iter().any(|x| x == m) { return true; }
    match cut(a, m) {
        // A name with parameters is that of a model whose variants are listed: claude-opus-4-8 for claude-opus-4-8-high.
        Some((base, "[")) => names.taken.iter().any(|x| x == base || x.strip_prefix(base).map(|r| r.starts_with('-')).unwrap_or(false)),
        Some((base, _)) => names.taken.iter().any(|x| x == base),
        None => false,
    }
}
fn by_rule(a: &Kind, m: &str) -> bool {
    let base = cut(a, m).map(|c| c.0).unwrap_or(m);
    a.models.aliases.contains(&base) || (!a.models.prefix.is_empty() && base.contains(a.models.prefix) && base.len() > a.models.prefix.len())
}
/// The names nearest to what was typed: one that holds it first, then by how few edits make one the other.
fn nearest(m: &str, names: &[String], n: usize) -> Vec<String> {
    let lm = m.to_lowercase();
    let mut v: Vec<(usize, &String)> = names.iter().map(|x| { let lx = x.to_lowercase(); (if lx.contains(&lm) || lm.contains(&lx) { 0 } else { 1 + distance(&lm, &lx) }, x) }).collect();
    v.sort_by_key(|(d, x)| (*d, x.len()));
    v.into_iter().take(n).map(|(_, x)| x.clone()).collect()
}
/// Why this program does not take this model, naming the ones it does; None when it takes it, or when its names
/// cannot be known (no list of its own, or a provider of its own).
pub fn why_not(k: &str, m: &str) -> Option<String> {
    let a = kind(k)?;
    if m.is_empty() || a.model.is_empty() { return None; }
    if a.models.list.is_empty() {
        if a.models.aliases.is_empty() && a.models.prefix.is_empty() { return None; }
        if listed::own_provider(k) || by_rule(a, m) { return None; }
        return Some(format!("{k} has no model \"{m}\". {}", rule_words(a)));
    }
    let names = listed(k, false)?;
    if in_list(&names, a, m) { return None; }
    let names = listed(k, true).unwrap_or(names); // asked again: the program may have a new one
    if in_list(&names, a, m) { return None; }
    // Listed only as its variants (claude-opus-4-8-high): with parameters in brackets, the name alone is taken.
    if a.models.suffix.contains(&"[") && in_list(&names, a, &format!("{m}[")) {
        return Some(format!("{k} takes \"{m}\" only with its parameters in brackets, as in {k}/{m}[effort=high], or as one of its listed variants ({}).", nearest(m, &names.taken, 3).join(", ")));
    }
    let shown = if names.shown.is_empty() { &names.taken } else { &names.shown };
    let near = nearest(m, shown, 1).into_iter().next().filter(|x| { let d = distance(&m.to_lowercase(), &x.to_lowercase()); x.to_lowercase().contains(&m.to_lowercase()) || d <= 3.max(m.len() / 2) });
    let some = if shown.len() > SHOWN { nearest(m, shown, SHOWN) } else { shown.clone() };
    let more = if shown.len() > SHOWN { format!(", and {} more: `herdr-projects models {k}` lists them all", shown.len() - SHOWN) } else { String::new() };
    Some(format!("{k} has no model \"{m}\".{} It takes {}{more} (from `{}`). Write it after the program: {k}/{}.",
        near.as_ref().map(|n| format!(" Did you mean {n}?")).unwrap_or_default(), some.join(", "), list_command(a), near.or_else(|| shown.first().cloned()).unwrap_or_default()))
}
fn list_command(a: &Kind) -> String { std::iter::once(program(a.name)).chain(a.models.list.iter().copied()).collect::<Vec<_>>().join(" ") }
fn rule_words(a: &Kind) -> String {
    let mut w = format!("{} takes", a.name);
    if !a.models.aliases.is_empty() { w.push_str(&format!(" a short name ({})", a.models.aliases.join(", "))); }
    if !a.models.prefix.is_empty() { w.push_str(&format!("{} a full model name with {} in it", if a.models.aliases.is_empty() { "" } else { " or" }, a.models.prefix)); }
    if !a.models.example.is_empty() { w.push_str(&format!(", such as {}", a.models.example)); }
    w.push('.');
    w
}
/// In a few words, how a model is given to this program and where its names come from (docs/agents.md, `models`).
pub fn how(k: &str) -> String {
    let Some(a) = kind(k) else { return String::new() };
    if a.model.is_empty() { return format!("{k} takes no model when the board starts it: choose it in {k}'s own settings."); }
    if !a.models.list.is_empty() { return format!("the names `{}` lists", list_command(a)); }
    if !a.models.aliases.is_empty() || !a.models.prefix.is_empty() { return rule_words(a).split_once(" takes ").map(|(_, r)| r.trim_end_matches('.').to_string()).unwrap_or_default(); }
    format!("any name {k} takes (not checked: {k} has no list of them)")
}

//! The skills of whoever the chat talks to, offered as you type, as the herdr web UI's chat box offers them: a draft
//! that is one word starting with `/` (with Codex `$` too) lists the skills and commands of that program that match
//! it, over the text box (view/skills.rs). Up and down pick one, tab or enter puts it in the box, esc closes the list
//! and keeps the draft. Enter on a word that already is the one picked sends it. The list is read by the loader
//! (board/skills.rs): a key never waits for a file. Words for nobody whose skills this machine can read (another
//! machine, a new task) list nothing, and go as typed (docs/skills.md).
use super::actions::has_agent;
use super::Effect;
use crate::agents::skills::{matching, Skill};
use crate::board::keys::{is, Press, K};
use crate::board::state::{Mode, State};
use crate::json::*;

/// A word a skill could start with: its trigger, then letters, digits, `-`, `_` and `:` only.
fn could_be(d: &str) -> bool { d.starts_with(['/', '$']) && d[1..].chars().all(|c| c.is_alphanumeric() || "-_:".contains(c)) }
/// Whose skills the list is of, as the loader reads them ("task\t<id>", "coord\t<level>"): who gets the words, as
/// enter sends them (actions.rs), while the draft could be a skill's start. "" for nobody.
pub fn wanted(s: &State) -> String {
    if s.mode != Mode::New || s.editing.is_some() || s.ask.is_some() || s.menu.is_some() || !could_be(&s.value()) { return String::new(); }
    if let Some(t) = super::coords::talking(s) { return if t.host.is_empty() { format!("coord\t{}", t.to) } else { String::new() }; }
    if s.reply { return s.current().filter(|r| !r.b("host") && !r.b("archivedAt") && has_agent(r)).map(|r| format!("task\t{}", r.s("id"))).unwrap_or_default(); }
    if s.data.can_dispatch && crate::board::view::textbox::far_coordinator(s).is_none() { "coord\t".into() } else { String::new() }
}
/// What the list shows now: the entries that match the draft, best first. Empty: no list.
pub fn shown(s: &State) -> Vec<&Skill> {
    let of = wanted(s);
    if of.is_empty() || s.skill_closed == s.value() { return vec![]; }
    s.data.skills.as_ref().filter(|(k, _)| *k == of).map(|(_, l)| matching(l, &s.value())).unwrap_or_default()
}
/// The entry picked among `n` shown: the first, until up or down picks another for this draft.
pub fn picked(s: &State, n: usize) -> usize { if s.skill_at.0 == s.value() { s.skill_at.1.min(n.saturating_sub(1)) } else { 0 } }
/// A key while the list shows: true when it was the list's.
pub fn key(s: &mut State, p: &Press) -> bool {
    let list: Vec<Skill> = shown(s).into_iter().cloned().collect();
    if list.is_empty() { return false; }
    let (n, at, word) = (list.len(), picked(s, list.len()), s.value());
    if is(p, K::Up) || is(p, K::Down) { s.skill_at = (word, if is(p, K::Up) { (at + n - 1) % n } else { (at + 1) % n }); return true; }
    if is(p, K::Back) { s.skill_closed = word; return true; }
    if is(p, K::Filter) || is(p, K::Delegate) || (is(p, K::Enter) && list[at].typed != word) { s.input.set_text(&format!("{} ", list[at].typed)); return true; }
    false
}
/// After a key: a draft that now wants a list the loader has not read yet is read at once.
pub fn after(s: &State, fx: &mut Vec<Effect>) {
    let of = wanted(s);
    if !of.is_empty() && s.data.skills.as_ref().map(|x| &x.0) != Some(&of) && !fx.contains(&Effect::Load) { fx.push(Effect::Load); }
}

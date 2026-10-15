//! The made tables: every key of the board (docs/keys.md), every setting (docs/settings.md) and every agent program
//! (docs/agents.md, agents_table.rs), each written
//! between its two marker lines from the one list it is made from, with what those lists must hold checked first.
use crate::board::keys::{by_name, pair, written, GROUPS, KEYS};
use crate::json::*;
use crate::store::settings_list::{Kind, SETTINGS};

fn cell(s: &str) -> String { s.replace('|', "\\|") }
pub fn table(head: &[&str], rows: Vec<Vec<String>>) -> String {
    let mut out = vec![format!("| {} |", head.join(" | ")), format!("|{}|", head.iter().map(|_| "---").collect::<Vec<_>>().join("|"))];
    out.extend(rows.into_iter().map(|r| format!("| {} |", r.iter().map(|c| cell(c)).collect::<Vec<_>>().join(" | "))));
    out.join("\n")
}
/// Words that name a key as {name}, with each key as a page writes it.
fn key_words(text: &str) -> Result<String, String> {
    let mut bad = None;
    let out = crate::re!(r"\{(\w+)\}").replace_all(text, |c: &regex::Captures| match by_name(&c[1]) { Some(k) => written(&crate::board::keys::name(k.id)), None => { bad = Some(c[1].to_string()); String::new() } }).into_owned();
    match bad { Some(id) => Err(format!("\"{{{id}}}\" names no key (crates/flow/src/board/keys.rs)")), None => Ok(out) }
}
pub fn keys_table() -> Result<String, String> {
    for k in KEYS {
        if k.doc.is_none() && k.with.is_none() { return Err(format!("crates/flow/src/board/keys.rs: the key \"{}\" has no `doc` (or `with`, the entry whose row describes it)", k.name)); }
        if let Some(w) = k.with { if by_name(w).and_then(|x| x.doc).is_none() { return Err(format!("crates/flow/src/board/keys.rs: the key \"{}\" is described with \"{w}\", which has no `doc`", k.name)); } }
        for w in [k.help, k.hint, k.doc].into_iter().flatten() { key_words(w).map_err(|e| format!("crates/flow/src/board/keys.rs: in the key \"{}\": {e}", k.name))?; }
        if let Some(g) = k.group {
            if !GROUPS.iter().any(|(x, _)| *x == g) { return Err(format!("crates/flow/src/board/keys.rs: the key \"{}\" is in the group \"{g}\", which is none of plain, ctrl, alt", k.name)); }
            if g != "plain" && !k.keys.iter().all(|c| c.starts_with(&format!("{g}+"))) { return Err(format!("crates/flow/src/board/keys.rs: the key \"{}\" is in the group \"{g}\" and is pressed with {}", k.name, k.keys.join(", "))); }
        }
    }
    // Another chord for the same thing is named in its row (the shifted and "return" twins of a key are not).
    let also = |k: &crate::board::keys::Key| { let more: Vec<String> = k.keys.iter().skip(1).filter(|c| !c.starts_with("shift+") && **c != "return").map(|c| written(c)).collect(); if more.is_empty() { String::new() } else { format!(". {} does the same", more.join(", ")) } };
    let rows = |pick: &dyn Fn(&crate::board::keys::Key) -> bool| -> Result<Vec<Vec<String>>, String> { KEYS.iter().filter(|k| k.doc.is_some() && pick(k)).map(|k| Ok(vec![written(&pair(k.id)), format!("{}{}", key_words(k.doc.unwrap())?, also(k))])).collect() };
    let heads = |g: &str| match g { "plain" => ("**Plain keys**", "Does"), "ctrl" => ("**`Ctrl`: does something**", "Does"), _ => ("**`Alt`: only shows or hides**", "Shows or hides") };
    let mut parts = vec![];
    for (g, _) in GROUPS { let (h, d) = heads(g); parts.push(format!("{h}\n\n{}", table(&["Key", d], rows(&|k| k.group == Some(g))?))); }
    parts.push(format!("**The text box's own** (not a key of the board, and not in the `?` menu)\n\n{}", table(&["Key", "Does"], rows(&|k| k.group.is_none())?)));
    Ok(parts.join("\n\n"))
}
pub fn settings_table() -> Result<String, String> {
    for s in SETTINGS.iter() { if s.doc.is_empty() { return Err(format!("crates/core/src/store/settings_list.rs: the setting \"{}\" has no `doc`", s.name)); } }
    let shown = |v: &Value| format!("`{}`", match v { Value::Array(a) => a.iter().map(text).collect::<Vec<_>>().join(", "), v => text(v) });
    let place = |s: &crate::store::settings_list::Entry| match s.kind { Kind::Board => "you only", Kind::Repo => "repo entry, repo file", Kind::Work if s.repo => "task, repo entry, you, repo file", Kind::Work => "task, repo entry, you" };
    let rows: Result<Vec<Vec<String>>, String> = SETTINGS.iter().map(|s| Ok(vec![format!("`{}`", s.name), shown(&s.builtin), place(s).to_string(), key_words(&s.doc)?])).collect();
    Ok(table(&["Setting", "Built in", "Can be said by", "Means"], rows?))
}
type Make = fn() -> Result<String, String>;
const PARTS: [(&str, &str, &str, Make); 3] = [("docs/keys.md", "keys", "crates/flow/src/board/keys.rs", keys_table), ("docs/settings.md", "settings", "crates/core/src/store/settings_list.rs", settings_table),
    ("docs/agents.md", "agents", "crates/core/src/agents/kinds/", super::agents_table::agents_table)];
fn with_part(file: &str, id: &str, from: &str, body: &str) -> Result<String, String> {
    let text = super::read(file);
    let (start, end) = (format!("<!-- made:{id} "), format!("<!-- /made:{id} -->"));
    let (Some(a), Some(b)) = (text.find(&start), text.find(&end)) else { return Err(format!("{file} has no \"{start}…\" and \"{end}\" lines for its made part")) };
    if b < a { return Err(format!("{file}: its made part's lines are the wrong way round")); }
    Ok(format!("{}{start}Made by `herdr-projects docs` from {from}. Do not edit between these two lines. -->\n{body}\n{}", &text[..a], &text[b..]))
}
/// Every made page that is in git, as it should be now.
pub fn pages() -> Result<Vec<(String, String)>, String> { PARTS.iter().map(|(f, id, from, make)| Ok((f.to_string(), with_part(f, id, from, &make()?)?))).collect() }
/// The made pages that are not what the code says now.
pub fn stale() -> Vec<String> { match pages() { Ok(p) => p.into_iter().filter(|(f, t)| super::read(f) != *t).map(|(f, _)| f).collect(), Err(e) => vec![e] } }
/// Writes what is out of date, and the map in this folder (ignored by git). Returns the pages in git it wrote.
pub fn write() -> Result<Vec<String>, String> {
    let all = pages()?;
    let mut wrote = vec![];
    for (f, t) in all { if super::read(&f) != t { super::write_file(&f, &t); wrote.push(f); } }
    if let Ok(m) = super::map::where_page("") { if super::read(super::map::WHERE) != m { super::write_file(super::map::WHERE, &m); } }
    Ok(wrote)
}

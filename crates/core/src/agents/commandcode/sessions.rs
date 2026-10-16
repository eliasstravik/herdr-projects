//! Finding Command Code's project-scoped sessions by their header, never by the folder's last conversation.
use crate::agents::read_turn::{find_file, head_of, when};
use crate::json::*;
use std::path::{Path, PathBuf};

fn files() -> Vec<PathBuf> { find_file(&crate::store::files::user_home().join(".commandcode/projects"), &|n| n.ends_with(".jsonl"), 1) }
fn header(file: &Path) -> Option<Value> { serde_json::from_str(head_of(file, 16384).lines().next()?).ok() }
fn same(cwd: &str, dir: &Path) -> bool { Path::new(cwd) == dir || std::fs::canonicalize(cwd).ok().zip(std::fs::canonicalize(dir).ok()).is_some_and(|(a, b)| a == b) }
pub fn transcript(session: &str, cwd: &Path) -> Option<PathBuf> {
    if session.is_empty() { return None; }
    files().into_iter().find(|f| header(f).is_some_and(|h| h.s("type") == "session" && h.s("id") == session && same(h.s("cwd"), cwd)))
}
pub fn latest(cwd: &Path, since: i64, was: Option<&Path>, needle: Option<&str>, but: &str) -> Option<String> {
    let mut found = vec![];
    for f in files() {
        let Some(h) = header(&f) else { continue };
        if h.s("type") != "session" || !h.b("id") || h.s("id") == but || when(h.at("timestamp")) < since - 5000 { continue; }
        let meta: Value = std::fs::read_to_string(f.with_extension("meta.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
        if meta.s("entrypoint") == "print" { continue; }
        if !same(h.s("cwd"), cwd) && !was.is_some_and(|w| same(h.s("cwd"), w)) { continue; }
        if let Some(n) = needle { if !head_of(&f, 400000).contains(n) { continue; } }
        found.push((when(h.at("timestamp")), h.t("id")));
    }
    // A task prompt can be present in several restored sessions: ambiguity must never reopen someone else's chat.
    if needle.is_some() { return (found.len() == 1).then(|| found[0].1.clone()); }
    found.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    found.first().map(|(_, id)| id.clone())
}

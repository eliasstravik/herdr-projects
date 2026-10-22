//! Reading a program's skills and commands off the disk: a skill is a folder holding a `SKILL.md` (pi also takes a loose
//! `.md` at its folder's top), a command a `.md` file; what each says of itself is in its header (`name:`,
//! `description:`), else its first line. Read as the herdr web UI's chat box reads them (its server/skills/walk.ts):
//! only a file's first 64 KB, a header line by line, a link that leads nowhere skipped.
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// A skill or a command as found: its name, what it says it does, and its header's fields.
#[derive(Clone, Debug, Default)]
pub struct Found { pub name: String, pub about: String, pub fields: HashMap<String, String> }
/// How deep a skills folder is read: `One` its folders' SKILL.md, `Deep` any folder down that holds one (and a loose
/// `.md` at its top), as pi reads its own.
#[derive(Clone, Copy, PartialEq)]
pub enum Walk { One, Deep }

const MOST: u64 = 64 * 1024;
fn text_of(path: &Path) -> Option<String> {
    if !std::fs::metadata(path).ok()?.is_file() { return None; }
    let mut out = String::new();
    std::fs::File::open(path).ok()?.take(MOST).read_to_string(&mut out).ok()?;
    Some(out)
}
fn unquote(v: &str) -> String {
    let v = v.trim();
    for q in ['"', '\''] { if v.len() >= 2 && v.starts_with(q) && v.ends_with(q) { return v[1..v.len() - 1].to_string(); } }
    v.to_string()
}
/// The header's fields (`key: value`, a `>` or `|` block's lines joined into one), and the text after it.
pub fn header(text: &str) -> (HashMap<String, String>, &str) {
    let mut fields = HashMap::new();
    let t = text.trim_start_matches('\u{feff}');
    let Some(rest) = t.strip_prefix("---").filter(|r| r.starts_with('\n') || r.starts_with("\r\n")) else { return (fields, t) };
    let Some(end) = rest.find("\n---") else { return (fields, t) };
    let body = rest[end + 4..].split_once('\n').map(|x| x.1).unwrap_or("");
    let mut block: Option<String> = None;
    for line in rest[..end].lines() {
        if let Some(k) = &block {
            if line.starts_with([' ', '\t']) || line.trim().is_empty() {
                let e = fields.entry(k.clone()).or_insert_with(String::new);
                if !line.trim().is_empty() { if !e.is_empty() { e.push(' '); } e.push_str(line.trim()); }
                continue;
            }
            block = None;
        }
        let Some((k, v)) = line.split_once(':') else { continue };
        if k.starts_with([' ', '\t']) || k.trim().is_empty() { continue; }
        let (k, v) = (k.trim().to_string(), v.trim());
        if ["|", ">", "|-", ">-", "|+", ">+"].contains(&v) { fields.insert(k.clone(), String::new()); block = Some(k); } else { fields.insert(k, unquote(v)); }
    }
    (fields, body)
}
/// What a file says it does: its header's description, else its first line of text.
fn about_of(fields: &HashMap<String, String>, body: &str) -> String {
    fields.get("description").filter(|d| !d.trim().is_empty()).cloned()
        .unwrap_or_else(|| body.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("").trim_start_matches('#').trim().to_string())
}
/// A name a program takes after its `/`: letters, digits, `-`, `_` and `:`.
fn name_ok(n: &str) -> bool { !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || "-_:".contains(c)) }
/// One skill's file. `need`: one with no description is none (Codex and pi skip it).
fn skill(path: &Path, dir_name: &str, need: bool) -> Option<Found> {
    let text = text_of(path)?;
    let (fields, body) = header(&text);
    if need && fields.get("description").map(|d| d.trim().is_empty()).unwrap_or(true) { return None; }
    let name = fields.get("name").filter(|n| name_ok(n)).cloned().unwrap_or_else(|| dir_name.to_string());
    if !name_ok(&name) { return None; }
    Some(Found { about: about_of(&fields, body), name, fields })
}
fn entries(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = std::fs::read_dir(dir).map(|r| r.flatten().map(|e| (e.file_name().to_string_lossy().to_string(), e.path())).collect()).unwrap_or_default();
    v.sort();
    v.into_iter().filter(|(n, _)| !n.starts_with('.')).collect()
}
/// The skills in a skills folder.
pub fn skills(root: &Path, walk: Walk, need: bool) -> Vec<Found> {
    let mut out = vec![];
    for (name, path) in entries(root) {
        if path.is_dir() {
            if walk == Walk::One || path.join("SKILL.md").is_file() { out.extend(skill(&path.join("SKILL.md"), &name, need)); } else { deep(&path, need, &mut out, 0); }
        } else if walk == Walk::Deep && name.ends_with(".md") { out.extend(skill(&path, name.trim_end_matches(".md"), need)); }
    }
    out
}
fn deep(dir: &Path, need: bool, out: &mut Vec<Found>, depth: usize) {
    if depth > 6 { return; }
    for (name, path) in entries(dir) {
        if !path.is_dir() || name == "node_modules" { continue; }
        if path.join("SKILL.md").is_file() { out.extend(skill(&path.join("SKILL.md"), &name, need)); } else { deep(&path, need, out, depth + 1); }
    }
}
/// The commands in a commands folder: each `.md` file, named by its path in it (`git/commit.md` is `git:commit`).
/// `deep`: its folders too (Claude Code's); else only its top (Codex' and pi's prompts).
pub fn commands(root: &Path, deep: bool) -> Vec<Found> {
    let mut out = vec![];
    fn visit(root: &Path, dir: &Path, deep: bool, out: &mut Vec<Found>, depth: usize) {
        for (name, path) in entries(dir) {
            if path.is_dir() { if deep && depth < 6 { visit(root, &path, deep, out, depth + 1); } continue; }
            let Some(stem) = name.strip_suffix(".md") else { continue };
            let Some(text) = text_of(&path) else { continue };
            let rel = path.parent().and_then(|p| p.strip_prefix(root).ok()).map(|p| p.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect::<Vec<_>>()).unwrap_or_default();
            let name = rel.into_iter().chain(std::iter::once(stem.to_string())).collect::<Vec<_>>().join(":");
            if !name_ok(&name) { continue; }
            let (fields, body) = header(&text);
            out.push(Found { about: about_of(&fields, body), name, fields });
        }
    }
    visit(root, root, deep, &mut out, 0);
    out
}
/// What a skill says it does, down to its first sentence: the rest says when to use it, which is for the agent. A
/// full stop ends a sentence only before a capital, so `e.g. this` and `v1.2` keep their words.
pub fn brief(about: &str) -> String {
    let t: Vec<char> = about.split_whitespace().collect::<Vec<_>>().join(" ").chars().collect();
    for i in 0..t.len() {
        if ".!?".contains(t[i]) && t.get(i + 1) == Some(&' ') && t.get(i + 2).map(|c| c.is_uppercase() || "\"'(".contains(*c)).unwrap_or(false) { return t[..i].iter().collect(); }
    }
    let s: String = t.into_iter().collect();
    s.strip_suffix('.').filter(|x| !x.ends_with('.')).map(String::from).unwrap_or(s)
}

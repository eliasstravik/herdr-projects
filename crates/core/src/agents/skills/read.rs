//! Reading a program's skills and commands off the disk: a skill is a folder holding a `SKILL.md` (some programs also
//! take a loose `.md`), a command a `.md` file; what each says of itself is in its header (`name:`, `description:`),
//! else its first line. Read as the herdr web UI's chat box reads them (its server/skills/walk.ts): only a file's first
//! 64 KB, the header as YAML (line by line where it is not strict YAML), a link that leads nowhere skipped. And the
//! programs' own settings files (JSON with comments, TOML, YAML), read as plain data.
use serde_json::{Map, Value};
use std::io::Read;
use std::path::{Path, PathBuf};

pub type Fields = Map<String, Value>;
/// A skill or a command as found: its name, what it says it does, its header's fields and its file.
#[derive(Clone, Debug, Default)]
pub struct Found { pub name: String, pub about: String, pub fields: Fields, pub path: PathBuf }
impl Found {
    /// A text field of its header.
    pub fn s(&self, k: &str) -> Option<&str> { self.fields.get(k).and_then(Value::as_str) }
    /// The folder its SKILL.md is in, by name.
    pub fn dir(&self) -> String { self.path.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default() }
}
/// How a skills folder is read: `One` its folders' SKILL.md; `Deep` any folder down that holds one; `DeepLoose` that
/// and a loose `.md` at its top (pi's).
#[derive(Clone, Copy, PartialEq)]
pub enum Walk { One, Deep, DeepLoose }

const MOST: u64 = 64 * 1024;
pub fn text_of(path: &Path) -> Option<String> {
    if !std::fs::metadata(path).ok()?.is_file() { return None; }
    let mut out = String::new();
    std::fs::File::open(path).ok()?.take(MOST).read_to_string(&mut out).ok()?;
    Some(out)
}
/// YAML as JSON data (None where it is not YAML).
pub fn yaml(text: &str) -> Option<Value> {
    use yaml_rust2::Yaml;
    fn conv(y: &Yaml) -> Value {
        match y {
            Yaml::String(s) | Yaml::Real(s) => Value::String(s.clone()),
            Yaml::Integer(n) => Value::from(*n),
            Yaml::Boolean(b) => Value::Bool(*b),
            Yaml::Array(a) => Value::Array(a.iter().map(conv).collect()),
            Yaml::Hash(h) => Value::Object(h.iter().filter_map(|(k, v)| Some((match k { Yaml::String(s) => s.clone(), Yaml::Integer(n) => n.to_string(), Yaml::Boolean(b) => b.to_string(), _ => return None }, conv(v)))).collect()),
            _ => Value::Null,
        }
    }
    yaml_rust2::YamlLoader::load_from_str(text).ok()?.first().map(conv)
}
pub fn yaml_file(path: &Path) -> Value { std::fs::read_to_string(path).ok().and_then(|t| yaml(&t)).unwrap_or(Value::Null) }
pub fn toml_file(path: &Path) -> Value { std::fs::read_to_string(path).ok().and_then(|t| toml::from_str::<Value>(&t).ok()).unwrap_or(Value::Null) }
/// JSON, with the comments and trailing commas JSONC allows.
pub fn json_file(path: &Path) -> Value {
    let Ok(t) = std::fs::read_to_string(path) else { return Value::Null };
    if let Ok(v) = serde_json::from_str(&t) { return v; }
    let (mut out, mut it, mut quoted) = (String::new(), t.chars().peekable(), false);
    while let Some(c) = it.next() {
        if quoted { out.push(c); if c == '\\' { if let Some(n) = it.next() { out.push(n); } } else if c == '"' { quoted = false; } continue; }
        match (c, it.peek()) {
            ('"', _) => { quoted = true; out.push(c); }
            ('/', Some('/')) => { for n in it.by_ref() { if n == '\n' { out.push('\n'); break; } } }
            ('/', Some('*')) => { it.next(); let mut last = ' '; for n in it.by_ref() { if last == '*' && n == '/' { break; } last = n; } }
            _ => out.push(c),
        }
    }
    let out = regex::Regex::new(r",(\s*[}\]])").unwrap().replace_all(&out, "$1").to_string();
    serde_json::from_str(&out).unwrap_or(Value::Null)
}
/// The texts of a value that is one text or a list of them.
pub fn texts(v: &Value) -> Vec<String> {
    match v { Value::String(s) if !s.trim().is_empty() => vec![s.trim().to_string()], Value::Array(a) => a.iter().filter_map(Value::as_str).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(), _ => vec![] }
}
fn unquote(v: &str) -> String {
    let v = v.trim();
    for q in ['"', '\''] { if v.len() >= 2 && v.starts_with(q) && v.ends_with(q) { return v[1..v.len() - 1].to_string(); } }
    v.to_string()
}
/// The header's fields, and the text after it. Read as YAML; a header that is not strict YAML is read line by line.
pub fn header(text: &str) -> (Fields, &str) {
    let t = text.trim_start_matches('\u{feff}');
    let Some(rest) = t.strip_prefix("---").filter(|r| r.starts_with('\n') || r.starts_with("\r\n")) else { return (Fields::new(), t) };
    let Some(end) = rest.find("\n---") else { return (Fields::new(), t) };
    let (head, body) = (&rest[..end], rest[end + 4..].split_once('\n').map(|x| x.1).unwrap_or(""));
    if let Some(Value::Object(o)) = yaml(head) { return (o, body); }
    let mut fields = Fields::new();
    for line in head.lines() {
        let Some((k, v)) = line.split_once(':') else { continue };
        if k.starts_with([' ', '\t']) || k.trim().is_empty() { continue; }
        let v = v.trim();
        if !["|", ">", "|-", ">-"].contains(&v) { fields.insert(k.trim().to_string(), Value::String(unquote(v))); }
    }
    (fields, body)
}
/// What a file says it does: its header's description, else its first line of text.
pub fn about_of(fields: &Fields, body: &str) -> String {
    fields.get("description").and_then(Value::as_str).map(|d| d.trim().to_string()).filter(|d| !d.is_empty())
        .unwrap_or_else(|| body.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("").trim_start_matches('#').trim().to_string())
}
/// A name a program takes after its `/`: letters, digits, `-`, `_`, `.` and `:`.
pub fn name_ok(n: &str) -> bool { !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || "-_:.".contains(c)) }
/// One skill's file. `need`: one with no description is none.
pub fn skill(path: &Path, dir_name: &str, need: bool) -> Option<Found> {
    let text = text_of(path)?;
    let (fields, body) = header(&text);
    let about = fields.get("description").and_then(Value::as_str).unwrap_or("").trim().to_string();
    if need && about.is_empty() { return None; }
    let name = fields.get("name").and_then(Value::as_str).map(str::trim).filter(|n| name_ok(n)).map(String::from).unwrap_or_else(|| dir_name.to_string());
    if !name_ok(&name) { return None; }
    Some(Found { about: about_of(&fields, body), name, fields, path: path.to_path_buf() })
}
/// A folder's entries by name, those starting with a dot left out.
pub fn entries(dir: &Path) -> Vec<(String, PathBuf)> {
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
        } else if walk == Walk::DeepLoose && name.ends_with(".md") { out.extend(skill(&path, name.trim_end_matches(".md"), need)); }
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
/// `deep`: its folders too; else only its top.
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
            out.push(Found { about: about_of(&fields, body), name, fields, path: path.clone() });
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
/// Of these paths in the repo at `top`, those its .gitignore leaves out (git itself asked, once).
pub fn git_ignored(top: &Path, paths: &[PathBuf]) -> std::collections::HashSet<PathBuf> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    if paths.is_empty() || !top.join(".git").exists() { return Default::default(); }
    let Ok(mut c) = Command::new("git").arg("-C").arg(top).args(["check-ignore", "--stdin"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else { return Default::default() };
    if let Some(mut i) = c.stdin.take() { let _ = i.write_all(paths.iter().map(|p| format!("{}\n", p.display())).collect::<String>().as_bytes()); }
    c.wait_with_output().map(|o| String::from_utf8_lossy(&o.stdout).lines().map(PathBuf::from).collect()).unwrap_or_default()
}

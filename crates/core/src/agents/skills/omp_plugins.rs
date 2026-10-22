//! omp's plugins beyond Claude Code's (the web UI's server/skills/omp.ts): extensions its settings.json names, and
//! plugins installed as npm packages in its plugins folders (yours and the project's), and which of them, or of the
//! Claude Code plugins, are portable agent plugins (agent-plugins.org), whose skills must keep that spec's rules.
use super::read::{json_file, Found};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";
/// The extensions and npm plugins turned on: (folder, yours). `native`: omp's own folders with their settings.json
/// (the project's, then yours); `scopes`: its plugins folders (yours, then the project's).
pub fn extensions(native: &[(PathBuf, bool)], scopes: &[(PathBuf, bool)], cwd: &Path, home: &Path) -> Vec<(PathBuf, bool)> {
    let mut out: Vec<(PathBuf, bool)> = vec![];
    for (dir, user) in native {
        for p in json_file(&dir.join("settings.json")).get("extensions").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
            out.push((super::path_in(cwd, home, p), *user));
        }
    }
    let off: Vec<String> = json_file(&cwd.join(".omp").join("plugin-overrides.json")).get("disabled").map(super::read::texts).unwrap_or_default();
    let mut installed: BTreeMap<String, (PathBuf, bool)> = BTreeMap::new();
    let mut order: Vec<String> = vec![];
    for (dir, user) in scopes {
        let lock = json_file(&dir.join("omp-plugins.lock.json"));
        let runtime = lock.get("plugins").and_then(Value::as_object).cloned().unwrap_or_default();
        let deps = json_file(&dir.join("package.json")).get("dependencies").and_then(Value::as_object).cloned().unwrap_or_default();
        let mut names: Vec<String> = deps.keys().cloned().collect();
        names.extend(runtime.keys().filter(|k| !deps.contains_key(*k)).cloned());
        for name in names {
            let path = dir.join("node_modules").join(&name);
            let pkg = json_file(&path.join("package.json"));
            let omp = |k: &str| pkg.get(k).is_some_and(|v| !v.is_null() && v != &Value::Bool(false));
            if (!omp("omp") && !omp("pi")) || runtime.get(&name).and_then(|r| r.get("enabled")) == Some(&Value::Bool(false)) || off.contains(&name) { continue; }
            if !installed.contains_key(&name) { order.push(name.clone()); }
            installed.insert(name, (path, *user));
        }
    }
    out.extend(order.iter().map(|n| installed[n].clone()));
    out
}
/// Says it is a portable agent plugin, of any version of the spec.
pub fn targets_portable(dir: &Path) -> bool { json_file(&dir.join("plugin.json")).get("$schema").and_then(Value::as_str).is_some_and(|s| s.starts_with("https://agent-plugins.org/schemas/")) }
/// A valid portable agent plugin (spec 1.0.0): its plugin.json's name and fields as the spec has them.
pub fn portable(dir: &Path) -> bool {
    let m = json_file(&dir.join("plugin.json"));
    let Some(name) = m.get("name").and_then(Value::as_str) else { return false };
    let name_ok = name.len() <= 64 && !name.contains("--") && !name.contains("..") && regex::Regex::new(r"^[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?$").unwrap().is_match(name);
    let texts_ok = ["version", "description", "homepage", "repository", "license"].iter().all(|k| m.get(*k).is_none_or(Value::is_string));
    let keywords_ok = m.get("keywords").is_none_or(|k| k.as_array().is_some_and(|a| a.iter().all(Value::is_string)));
    let author_ok = m.get("author").is_none_or(|a| a.as_object().is_some_and(|o| o.iter().all(|(k, v)| ["name", "email", "url"].contains(&k.as_str()) && v.is_string())));
    m.get("$schema").and_then(Value::as_str) == Some(SCHEMA) && name_ok && texts_ok && keywords_ok && author_ok
}
/// A skill of a portable plugin keeps the spec: named as its folder (lower case words joined by `-`), only the spec's
/// fields, and inside the plugin's folder.
pub fn portable_skill(f: &Found, root: &Path) -> bool {
    let Some(name) = f.s("name") else { return false };
    let allowed = ["name", "description", "license", "allowed-tools", "metadata", "compatibility"];
    let words = name.split('-').all(|w| !w.is_empty() && w.chars().all(char::is_alphanumeric));
    let inside = match (std::fs::canonicalize(&f.path), std::fs::canonicalize(root)) { (Ok(a), Ok(b)) => a.starts_with(b), _ => false };
    name == f.dir() && name.len() <= 64 && name == name.to_lowercase() && words && f.s("description").unwrap_or("").len() <= 1024
        && f.fields.keys().all(|k| allowed.contains(&k.as_str())) && ["license", "allowed-tools", "compatibility"].iter().all(|k| f.fields.get(*k).is_none_or(Value::is_string))
        && f.s("compatibility").is_none_or(|c| c.len() <= 500) && f.fields.get("metadata").is_none_or(|m| m.as_object().is_some_and(|o| o.values().all(Value::is_string))) && inside
}

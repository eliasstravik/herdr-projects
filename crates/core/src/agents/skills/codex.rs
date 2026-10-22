//! Codex's skills and prompts (the web UI's server/skills/codex.ts): skills as `$name`, the project's `.agents/skills`,
//! yours in `~/.agents/skills` and Codex's own folder, the ones it comes with, and those of the plugins turned on in
//! its config.toml (found in its plugin cache, under their own names); your prompts as `/prompts:name`. A skill turned
//! off in config.toml (`[[skills.config]]` with `enabled = false`) is left out.
use super::read::{commands, entries, skills, toml_file, Walk};
use super::{entry, Place, Skill};
use crate::agents::homes::codex_home;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn real(p: &Path) -> PathBuf { std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()) }
pub fn list(p: &Place) -> Vec<Skill> {
    let home = codex_home();
    let config = toml_file(&home.join("config.toml"));
    let off: HashSet<PathBuf> = config.get("skills").and_then(|s| s.get("config")).and_then(Value::as_array).into_iter().flatten()
        .filter(|e| e.get("enabled") == Some(&Value::Bool(false))).filter_map(|e| e.get("path")?.as_str().map(|s| real(Path::new(s)))).collect();
    let dollar = |root: &Path| -> Vec<Skill> {
        skills(root, Walk::One, true).iter().filter(|f| f.s("name").is_some_and(|n| !n.trim().is_empty()) && !off.contains(&real(&f.path))).map(|f| entry(format!("${}", f.name), f)).collect()
    };
    let mut out: Vec<Skill> = commands(&home.join("prompts"), false).iter().map(|f| entry(format!("/prompts:{}", f.name), f)).collect();
    for d in p.projects() { out.extend(dollar(&d.join(".agents").join("skills"))); }
    out.extend(dollar(&p.home.join(".agents").join("skills")));
    out.extend(dollar(&home.join("skills")));
    out.extend(dollar(&home.join("skills").join(".system")));
    out.extend(dollar(Path::new("/etc/codex/skills")));
    for root in plugin_skills(&home.join("plugins").join("cache"), config.get("plugins")) { out.extend(dollar(&root)); }
    out
}
/// The `skills` folders in Codex's plugin cache (marketplace/plugin/version/…) of the plugins turned on: every plugin
/// where config.toml has no `[plugins]` table, else those whose `[plugins."plugin@marketplace"]` says `enabled = true`.
fn plugin_skills(cache: &Path, plugins: Option<&Value>) -> Vec<PathBuf> {
    let on = |parts: &[String]| -> bool {
        let Some(table) = plugins.and_then(Value::as_object) else { return true };
        table.iter().any(|(key, v)| {
            let (plugin, market) = key.split_once('@').unwrap_or((key.as_str(), ""));
            v.get("enabled") == Some(&Value::Bool(true)) && (parts.first().is_some_and(|x| x == key || x == plugin) || (parts.first().is_some_and(|x| x == market) && parts.get(1).is_some_and(|x| x == plugin)))
        })
    };
    let (mut out, mut seen) = (vec![], HashSet::new());
    fn visit(cache: &Path, dir: &Path, depth: usize, on: &dyn Fn(&[String]) -> bool, out: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
        if depth > 8 { return; }
        for (name, path) in entries(dir) {
            if !path.is_dir() { continue; }
            if name == "skills" {
                let parts: Vec<String> = dir.strip_prefix(cache).map(|r| r.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect()).unwrap_or_default();
                if seen.insert(real(&path)) && on(&parts) { out.push(path); }
            } else { visit(cache, &path, depth + 1, on, out, seen); }
        }
    }
    visit(cache, cache, 0, &on, &mut out, &mut seen);
    out
}

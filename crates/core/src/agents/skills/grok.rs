//! Grok's skills, as `/name` (the web UI's server/skills/grok.ts, from Grok's README): the project's `.grok/skills` and
//! `.claude/skills` (where it runs and at the repo's top), yours in Grok's folder and `~/.claude/skills`, the folders
//! its config.toml's `[skills] paths` names, then its plugins' and the installed Claude Code plugins'. A skill needs a
//! name and a description; one under a path of `[skills] ignore` is left out, and so is a project's one that git
//! ignores.
use super::read::{entries, json_file, skills, toml_file, Walk};
use super::{entry, env, path_in, Place, Skill};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn plugin_roots(dir: &Path, installed: bool) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = entries(dir).into_iter().filter(|(_, p)| p.is_dir()).map(|(_, p)| p.join("skills")).collect();
    if installed {
        let j = json_file(&dir.join("installed_plugins.json"));
        for versions in j.get("plugins").and_then(Value::as_object).into_iter().flat_map(|o| o.values()) {
            for x in versions.as_array().into_iter().flatten() { if let Some(i) = x.get("installPath").and_then(Value::as_str) { v.push(PathBuf::from(i).join("skills")); } }
        }
    }
    v
}
pub fn list(p: &Place) -> Vec<Skill> {
    let home = env("GROK_HOME").map(|g| path_in(Path::new("/"), &p.home, &g)).unwrap_or_else(|| p.home.join(".grok"));
    let table = toml_file(&home.join("config.toml")).get("skills").cloned().unwrap_or(Value::Null);
    let abs = |v: Option<&Value>| -> Vec<PathBuf> { v.map(super::read::texts).unwrap_or_default().iter().filter(|x| x.starts_with('/') || x.starts_with('~')).map(|x| path_in(Path::new("/"), &p.home, x)).collect() };
    let (paths, ignore) = (abs(table.get("paths")), abs(table.get("ignore")));
    let mut dirs = vec![p.cwd.clone()];
    dirs.extend(p.roots().into_iter().filter(|d| *d != p.cwd));
    let mut roots: Vec<PathBuf> = dirs.iter().flat_map(|d| [d.join(".grok").join("skills"), d.join(".claude").join("skills")]).collect();
    roots.push(home.join("skills"));
    roots.push(p.home.join(".claude").join("skills"));
    roots.extend(paths);
    for d in &dirs { roots.extend(plugin_roots(&d.join(".grok").join("plugins"), false)); roots.extend(plugin_roots(&d.join(".claude").join("plugins"), false)); }
    roots.extend(plugin_roots(&home.join("plugins"), false));
    roots.extend(plugin_roots(&p.home.join(".claude").join("plugins"), true));
    let found: Vec<_> = roots.iter().flat_map(|r| skills(r, Walk::One, true)).filter(|f| f.s("name").is_some_and(|n| !n.trim().is_empty()))
        .filter(|f| !ignore.iter().any(|i| f.path.parent().is_some_and(|d| d.starts_with(i)))).collect();
    let mine: Vec<PathBuf> = p.top.iter().flat_map(|t| found.iter().filter_map(move |f| f.path.parent().filter(|d| d.starts_with(t)).map(|d| d.join("")))).collect();
    let left_out = p.top.as_deref().map(|t| super::read::git_ignored(t, &mine)).unwrap_or_default();
    found.iter().filter(|f| !f.path.parent().is_some_and(|d| left_out.contains(&d.join("")))).map(|f| entry(format!("/{}", f.name), f)).collect()
}

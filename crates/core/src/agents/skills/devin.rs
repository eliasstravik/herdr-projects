//! Devin's skills, as `/name` by their folder's name (the web UI's server/skills/devin.ts): yours and the project's in
//! Devin's own folders (`devin`, `cognition`, `.agents`), and, unless its config.json's `read_config_from` turns them
//! off, Windsurf's, Claude Code's (skills, and the project's commands) and Copilot's; then its plugins', as
//! `/plugin:name`. A name found in two of these places is told apart by the place (`/claude:review`). A skill whose
//! `triggers` leave out `user` is only for Devin itself.
use super::read::{commands, json_file, skills, Found, Walk};
use super::{env, Place, Skill};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

fn for_you(f: &Found) -> bool { f.fields.get("triggers").and_then(Value::as_array).map(|t| t.iter().any(|x| x.as_str() == Some("user"))).unwrap_or(true) }
pub fn list(p: &Place) -> Vec<Skill> {
    let config_home = env("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| p.home.join(".config"));
    let data_home = env("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| p.home.join(".local").join("share"));
    let project = p.top.clone().unwrap_or_else(|| p.cwd.clone());
    let mut imports = json_file(&config_home.join("devin").join("config.json")).get("read_config_from").and_then(Value::as_object).cloned().unwrap_or_default();
    if let Some(o) = json_file(&project.join(".devin").join("config.json")).get("read_config_from").and_then(Value::as_object) { imports.extend(o.clone()); }
    let on = |k: &str| imports.get(k) != Some(&Value::Bool(false));
    let mut found: Vec<(String, String, Found)> = vec![]; // (place, name, skill)
    let add = |root: PathBuf, ns: &str, deep: bool, plugin: bool, found: &mut Vec<(String, String, Found)>| {
        for f in skills(&root, if deep { Walk::Deep } else { Walk::One }, false).into_iter().filter(for_you) {
            let n = if plugin { format!("{ns}:{}", f.dir()) } else { f.dir() };
            found.push((ns.to_string(), n, f));
        }
    };
    for (base, user) in [(p.home.clone(), true), (project.clone(), false)] {
        let start = found.len();
        add(if user { config_home.join("devin").join("skills") } else { base.join(".devin").join("skills") }, "devin", false, false, &mut found);
        add(if user { config_home.join("cognition").join("skills") } else { base.join(".cognition").join("skills") }, "cognition", false, false, &mut found);
        add(base.join(".agents").join("skills"), "agents", false, false, &mut found);
        let mut seen = BTreeSet::new();
        let mut i = start;
        while i < found.len() { if seen.insert(found[i].1.clone()) { i += 1; } else { found.remove(i); } }
        if on("windsurf") { add(if user { p.home.join(".codeium").join("windsurf").join("skills") } else { base.join(".windsurf").join("skills") }, "windsurf", false, false, &mut found); }
        if on("claude") {
            add(base.join(".claude").join("skills"), "claude", true, false, &mut found);
            if !user { for f in commands(&base.join(".claude").join("commands"), true).into_iter().filter(for_you) { found.push(("claude".into(), f.name.clone(), f)); } }
        }
        if on("copilot") { add(if user { env("COPILOT_HOME").map(PathBuf::from).unwrap_or_else(|| p.home.join(".copilot")).join("skills") } else { base.join(".github").join("skills") }, "copilot", true, false, &mut found); }
    }
    plugins(&data_home.join("devin").join("cli").join("plugins"), 0, &mut |root, name| add(root, name, false, true, &mut found));
    let mut places: HashMap<String, BTreeSet<String>> = HashMap::new();
    for (ns, n, _) in &found { places.entry(n.clone()).or_default().insert(ns.clone()); }
    // The project's comes after yours: it wins within one place.
    let mut out: Vec<Skill> = found.iter().rev().map(|(ns, n, f)| {
        let name = if places[n].len() > 1 { format!("{ns}:{n}") } else { n.clone() };
        Skill { typed: format!("/{name}"), about: super::read::brief(&f.about) }
    }).collect();
    out.dedup_by(|a, b| a.typed == b.typed);
    out
}
/// Devin's plugins: a folder with a plugin.json (`.devin-plugin/`, `.claude-plugin/` or at its top) that names it, with
/// its `skills` folders.
fn plugins(dir: &Path, depth: usize, add: &mut dyn FnMut(PathBuf, &str)) {
    if depth > 6 || !dir.is_dir() { return; }
    if let Some(m) = [".devin-plugin/plugin.json", ".claude-plugin/plugin.json", "plugin.json"].iter().map(|x| dir.join(x)).find(|x| x.is_file()) {
        let m = json_file(&m);
        let Some(name) = m.get("name").and_then(Value::as_str) else { return };
        let roots = match m.get("skills") { None => vec!["skills".to_string()], Some(v) => super::read::texts(v) };
        for r in roots.iter().filter(|r| !r.split('/').any(|x| x == "..")) { add(dir.join(r), name); }
        return;
    }
    for (_, path) in super::read::entries(dir) { if path.is_dir() { plugins(&path, depth + 1, add); } }
}

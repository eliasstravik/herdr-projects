//! Antigravity's (agy) skills, as `/name` (the web UI's server/skills/agy.ts): the project's `.agents`, `.agent`,
//! `_agents` and `_agent` skills folders and the skills their `skills.json` names, yours in `~/.gemini/config/skills`
//! and its plugins', and the ones it comes with, less those that are its own commands. A skill marked
//! `disable-slash-command: true` is left out.
use super::read::{json_file, skill, skills, Found, Walk};
use super::{entry, path_in, Place, Skill};
use serde_json::Value;
use std::path::Path;

/// Its own commands and the screens it opens: a skill it comes with by one of these names is not listed.
const OWN: [&str; 51] = [
    "add-dir", "artifact", "agy-customizations", "antigravity-guide", "btw", "fork", "branch", "browser", "boost",
    "feedback", "changelog", "clear", "new", "codesearch", "cs", "search", "context", "diff", "rewind", "undo", "exit",
    "quit", "help", "open", "goal", "grill-me", "usage", "quota", "schedule", "title", "tasks", "logout", "learn",
    "plan", "rename", "copy", "teamwork-preview", "remote-control", "statusline", "model", "resume", "config",
    "permissions", "agents", "hooks", "mcp", "plugin", "skills", "voice", "effort", "keybindings"
];
fn named(dir: &Path, base: &Path, home: &Path, out: &mut Vec<Found>) {
    let c = json_file(&dir.join("skills.json"));
    for e in c.get("entries").and_then(Value::as_array).into_iter().flatten() {
        let Some(path) = e.get("path").and_then(Value::as_str) else { continue };
        let root = path_in(base, home, path);
        let out_of: Vec<&str> = e.get("exclude").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect();
        match e.get("include_only").and_then(Value::as_array) {
            Some(only) => for n in only.iter().filter_map(Value::as_str).filter(|n| !out_of.contains(n)) { out.extend(skill(&root.join(n).join("SKILL.md"), n, true)); },
            None => out.extend(skills(&root, Walk::One, true).into_iter().filter(|f| !out_of.contains(&f.dir().as_str()))),
        }
    }
}
pub fn list(p: &Place) -> Vec<Skill> {
    let base = p.top.clone().unwrap_or_else(|| p.cwd.clone());
    let mut found: Vec<Found> = vec![];
    for d in p.projects() { for x in [".agents", ".agent", "_agents", "_agent"] { found.extend(skills(&d.join(x).join("skills"), Walk::One, true)); } }
    for d in p.projects() { for x in [".agents", ".agent", "_agents", "_agent"] { named(&d.join(x), &base, &p.home, &mut found); } }
    let config = p.home.join(".gemini").join("config");
    found.extend(skills(&config.join("skills"), Walk::One, true));
    for (_, plugin) in super::read::entries(&config.join("plugins")) { found.extend(skills(&plugin.join("skills"), Walk::One, true)); }
    found.extend(skills(&p.home.join(".gemini").join("antigravity-cli").join("builtin").join("skills"), Walk::One, true).into_iter().filter(|f| !OWN.contains(&f.name.as_str())));
    named(&config, p.top.as_deref().unwrap_or(&config), &p.home, &mut found);
    found.iter().filter(|f| f.fields.get("disable-slash-command") != Some(&Value::Bool(true))).map(|f| entry(format!("/{}", f.name), f)).collect()
}

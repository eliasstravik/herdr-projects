//! Claude Code's skills and commands, all as `/name` (the web UI's server/skills/claude.ts): your skills (the managed
//! ones, `~/.claude/skills`, its `synced` ones as `/anthropic-skills:name`), the project's `.claude/skills`, your and
//! the project's commands (`git/commit.md` is `/git:commit`), and those of the plugins turned on as `/plugin:name`. A
//! skill marked `user-invocable: false` is only for Claude itself.
use super::read::{commands, json_file, skills, Walk};
use super::{entry, Place, Skill};
use crate::agents::homes::claude_home;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn mine(root: PathBuf, prefix: &str) -> Vec<Skill> {
    skills(&root, Walk::One, false).iter()
        .filter(|f| !matches!(f.fields.get("user-invocable"), Some(Value::Bool(false))) && f.s("user-invocable") != Some("false"))
        .map(|f| entry(format!("/{prefix}{}", f.name), f)).collect()
}
fn orders(root: PathBuf, prefix: &str) -> Vec<Skill> { commands(&root, true).iter().map(|f| entry(format!("/{prefix}{}", f.name), f)).collect() }
pub fn list(p: &Place) -> Vec<Skill> {
    let home = claude_home();
    let mut out = mine(PathBuf::from("/etc/claude-code/.claude/skills"), "");
    out.extend(mine(home.join("skills"), ""));
    out.extend(mine(home.join("skills").join("synced"), "anthropic-skills:"));
    for d in p.projects() { out.extend(mine(d.join(".claude").join("skills"), "")); }
    out.extend(orders(home.join("commands"), ""));
    for d in p.projects() { out.extend(orders(d.join(".claude").join("commands"), "")); }
    for (plugin, dir) in plugins(&home) {
        let pre = format!("{plugin}:");
        out.extend(mine(dir.join("skills"), &pre));
        out.extend(orders(dir.join("commands"), &pre));
    }
    out
}
/// The plugins turned on in Claude Code's settings, by name, with the folder each is installed in.
pub fn plugins(home: &Path) -> Vec<(String, PathBuf)> {
    let (on, installed) = (json_file(&home.join("settings.json")), json_file(&home.join("plugins").join("installed_plugins.json")));
    let Some(on) = on.get("enabledPlugins").and_then(Value::as_object) else { return vec![] };
    on.iter().filter(|(_, v)| v.as_bool() == Some(true)).filter_map(|(id, _)| {
        let path = installed.get("plugins")?.get(id)?.get(0)?.get("installPath")?.as_str()?;
        Some((id.split('@').next().unwrap_or(id).to_string(), PathBuf::from(path)))
    }).collect()
}

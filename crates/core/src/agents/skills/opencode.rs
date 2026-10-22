//! OpenCode's commands and skills, all as `/name` (the web UI's server/skills/opencode.ts): the commands of its
//! config (`command` in opencode.json or opencode.jsonc, yours and the project's) and of its `commands` folders; the
//! skill it comes with (`customize-opencode`); skills in `~/.claude/skills`, `~/.agents/skills` and the project's (unless
//! OpenCode is told not to read them), and in its own `skill`/`skills` folders. A skill's name must be its folder's,
//! lower case words joined by `-`. A command wins over a skill of the same name.
use super::read::{commands, json_file, skills, Found, Walk};
use super::{entry, env, Place, Skill};
use serde_json::Value;
use std::path::{Path, PathBuf};

const BUNDLED: &str = "Use ONLY when the user is editing or creating opencode's own configuration: opencode.json, opencode.jsonc, files under .opencode/, or files under ~/.config/opencode/.";
fn in_config(dir: &Path) -> Vec<Skill> {
    ["opencode.json", "opencode.jsonc"].iter().flat_map(|f| {
        let c = json_file(&dir.join(f));
        c.get("command").and_then(Value::as_object).into_iter().flatten().filter(|(_, v)| v.get("template").is_some_and(Value::is_string))
            .map(|(n, v)| Skill { typed: format!("/{n}"), about: super::read::brief(v.get("description").and_then(Value::as_str).unwrap_or("")) }).collect::<Vec<_>>()
    }).collect()
}
fn good(f: &Found) -> bool {
    f.s("name").is_some() && f.name == f.dir() && f.name.len() <= 64 && f.about.len() <= 1024
        && f.name.split('-').all(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
}
pub fn list(p: &Place) -> Vec<Skill> {
    let config = env("OPENCODE_CONFIG_DIR").map(PathBuf::from).unwrap_or_else(|| p.home.join(".config").join("opencode"));
    let projects = p.projects();
    // Nearest first: the first found wins, as the nearer one overrides in OpenCode.
    let mut out = vec![];
    for d in &projects { let o = d.join(".opencode"); out.extend(in_config(&o)); out.extend(commands(&o.join("commands"), false).iter().map(|f| entry(format!("/{}", f.name), f))); out.extend(in_config(d)); }
    out.extend(in_config(&config));
    out.extend(commands(&config.join("commands"), false).iter().map(|f| entry(format!("/{}", f.name), f)));
    let mut roots: Vec<PathBuf> = vec![];
    for d in &projects { for k in ["skill", "skills"] { roots.push(d.join(".opencode").join(k)); } }
    for k in ["skill", "skills"] { roots.push(config.join(k)); }
    let external = !["OPENCODE_DISABLE_EXTERNAL_SKILLS", "OPENCODE_DISABLE_CLAUDE_CODE_SKILLS", "OPENCODE_DISABLE_CLAUDE_CODE"].iter().any(|k| matches!(env(k).as_deref(), Some("1") | Some("true")));
    if external { for d in projects.iter().chain(std::iter::once(&p.home)) { for k in [".claude", ".agents"] { roots.push(d.join(k).join("skills")); } } }
    for r in roots { out.extend(skills(&r, Walk::Deep, true).iter().filter(|f| good(f)).map(|f| entry(format!("/{}", f.name), f))); }
    out.push(Skill { typed: "/customize-opencode".into(), about: super::read::brief(BUNDLED) });
    out
}

//! Hermes' skills, as `/name` in its own spelling (lower case, `-` for spaces and `_`), (the web UI's
//! server/skills/hermes.ts): the project's `.hermes/skills` and `.agents/skills` where its config.yaml trusts the
//! project, then `~/.hermes/skills`, its `create_dir` and `external_dirs`, any depth; its skill bundles; and its
//! `quick_commands`. A skill turned off (`disabled`), one for another system (`platforms`), one named like one of its
//! own commands, and a name two folders of one rank both have, are left out.
use super::read::{entries, header, text_of, texts, yaml_file, Fields};
use super::{env, path_in, Place, Skill};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Its own commands: a skill by one of these names is not listed.
const OWN: [&str; 128] = [
    "new", "reset", "clear", "redraw", "history", "save", "retry", "prompt", "compose", "undo", "title", "handoff",
    "branch", "fork", "worktree", "compress", "compact", "rollback", "snapshot", "snap", "export", "import", "stop",
    "bg", "btw", "tasks", "journey", "learning", "memory-graph", "queue", "q", "steer", "s", "goal", "heartbeat", "hb",
    "refine", "review", "loop", "proactive", "plan", "moa", "subgoal", "status", "egress", "context", "ctx", "whoami",
    "profile", "codex-runtime", "codex_runtime", "personality", "statusbar", "sb", "battery", "timestamps", "ts",
    "diff", "verbose", "focus", "footer", "yolo", "approvals", "reasoning", "fast", "indicator", "voice", "wake",
    "busy", "tools", "toolsets", "memory", "bundles", "pet", "hatch", "generate-pet", "learn", "init", "cron",
    "suggestions", "suggest", "blueprint", "bp", "curator", "kanban", "reload", "reload-mcp", "reload_mcp",
    "reload-skills", "reload_skills", "browser", "help", "palette", "usage", "subscription", "upgrade", "login",
    "topup", "insights", "platforms", "gateway", "copy", "paste", "image", "update", "version", "v", "debug", "quit",
    "exit", "start", "topic", "pause", "approve", "deny", "agents", "sethome", "set-home", "resume", "sessions",
    "config", "model", "skin", "skills", "plugins", "commands", "restart", "platform"
];
const SKIP: [&str; 13] = [".git", ".github", ".hub", ".archive", ".curator_backups", ".locks", ".venv", "venv", "node_modules", "site-packages", "__pycache__", ".tox", ".nox"];
const SUPPORT: [&str; 4] = ["references", "templates", "assets", "scripts"];

/// Every SKILL.md under `root`, also inside a skill's folder (but not its support folders).
fn skill_files(root: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 8 { return; }
    let has = root.join("SKILL.md").is_file();
    if has { out.push(root.join("SKILL.md")); }
    let all = std::fs::read_dir(root).map(|r| { let mut v: Vec<_> = r.flatten().map(|e| (e.file_name().to_string_lossy().to_string(), e.path())).collect(); v.sort(); v }).unwrap_or_default();
    for (n, p) in all { if p.is_dir() && !SKIP.contains(&n.as_str()) && !(has && SUPPORT.contains(&n.as_str())) { skill_files(&p, depth + 1, out); } }
}
pub fn slug(n: &str) -> String {
    let s: String = n.to_lowercase().chars().map(|c| if c == ' ' || c == '_' { '-' } else { c }).filter(|c| c.is_alphanumeric() || *c == '-').collect();
    s.split('-').filter(|w| !w.is_empty()).collect::<Vec<_>>().join("-")
}
fn platform_ok(f: &Fields) -> bool {
    let p = f.get("platforms").map(texts).unwrap_or_default();
    let me = match std::env::consts::OS { "macos" => "macos", "windows" => "windows", o => o };
    p.is_empty() || p.iter().any(|x| x.to_lowercase() == me)
}
pub fn list(p: &Place) -> Vec<Skill> {
    let home = env("HERMES_HOME").map(PathBuf::from).unwrap_or_else(|| p.home.join(".hermes"));
    let config = yaml_file(&home.join("config.yaml"));
    let set = config.get("skills").cloned().unwrap_or(Value::Null);
    let real = |x: &Path| std::fs::canonicalize(x).unwrap_or_else(|_| x.to_path_buf());
    let mut roots: Vec<(PathBuf, u8)> = vec![];
    if let Some(top) = &p.top {
        let trusted = set.get("trusted_project_dirs").map(texts).unwrap_or_default().iter().any(|d| real(&path_in(&home, &p.home, d)) == real(top));
        if set.get("project_discovery") != Some(&Value::Bool(false)) && trusted { for s in [".hermes", ".agents"] { roots.push((top.join(s).join("skills"), 0)); } }
    }
    roots.push((home.join("skills"), 1));
    if let Some(c) = set.get("create_dir").and_then(Value::as_str).filter(|c| !c.trim().is_empty()) { roots.push((path_in(&home, &p.home, c), 2)); }
    for d in set.get("external_dirs").map(texts).unwrap_or_default() { roots.push((path_in(&home, &p.home, &d), 3)); }
    let off: HashSet<String> = set.get("disabled").map(texts).unwrap_or_default().into_iter().collect();
    // Name -> (rank, root, about) of each one found.
    let mut found: HashMap<String, Vec<(u8, PathBuf, String)>> = HashMap::new();
    let mut order: Vec<String> = vec![];
    let mut seen = HashSet::new();
    for (root, rank) in roots {
        if !seen.insert(real(&root)) { continue; }
        let mut files = vec![];
        skill_files(&root, 0, &mut files);
        for f in files {
            let Some(text) = text_of(&f) else { continue };
            let (fields, body) = header(&text);
            if !platform_ok(&fields) { continue; }
            let name = fields.get("name").and_then(Value::as_str).map(String::from).unwrap_or_else(|| f.parent().and_then(|d| d.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
            let about = fields.get("description").and_then(Value::as_str).map(|d| d.trim().to_string()).filter(|d| !d.is_empty())
                .or_else(|| body.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#')).map(|l| l.chars().take(80).collect()))
                .unwrap_or_else(|| format!("Invoke the {name} skill"));
            if !found.contains_key(&name) { order.push(name.clone()); }
            found.entry(name).or_default().push((rank, root.clone(), about));
        }
    }
    let mut out = vec![];
    for name in order {
        let all = &found[&name];
        let top = all.iter().map(|x| x.0).min().unwrap_or(0);
        let first: Vec<_> = all.iter().filter(|x| x.0 == top).collect();
        if first.len() > 1 && !first.iter().all(|x| x.1 == first[0].1 && x.2 == first[0].2) { continue; } // two of one rank: nobody knows which
        let key = slug(&name);
        if off.contains(&name) || key.is_empty() || OWN.contains(&key.as_str()) { continue; }
        out.push(Skill { typed: format!("/{key}"), about: super::read::brief(&first[0].2) });
    }
    let mut first_ones: Vec<Skill> = vec![]; // its quick commands, then its bundles: they win over a skill of their name
    let bundles = env("HERMES_BUNDLES_DIR").map(|d| path_in(&home, &p.home, &d)).unwrap_or_else(|| home.join("skill-bundles"));
    for (file, path) in entries(&bundles).into_iter().filter(|(f, _)| f.ends_with(".yaml") || f.ends_with(".yml")) {
        let b = yaml_file(&path);
        let members = b.get("skills").map(texts).unwrap_or_default();
        let name = slug(b.get("name").and_then(Value::as_str).filter(|n| !n.trim().is_empty()).unwrap_or(file.trim_end_matches(".yaml").trim_end_matches(".yml")));
        if members.is_empty() || name.is_empty() || OWN.contains(&name.as_str()) { continue; }
        let about = b.get("description").and_then(Value::as_str).map(str::trim).filter(|d| !d.is_empty()).map(String::from).unwrap_or_else(|| format!("Load {} skills as a bundle", members.len()));
        first_ones.push(Skill { typed: format!("/{name}"), about: super::read::brief(&about) });
    }
    for (name, v) in config.get("quick_commands").and_then(Value::as_object).into_iter().flatten() {
        if name.is_empty() || OWN.contains(&name.as_str()) { continue; }
        first_ones.insert(0, Skill { typed: format!("/{name}"), about: super::read::brief(v.get("description").and_then(Value::as_str).unwrap_or(&format!("Run /{name}"))) });
    }
    first_ones.extend(out);
    first_ones
}

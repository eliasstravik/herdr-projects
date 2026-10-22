//! omp's skills and commands (the web UI's server/skills/omp.ts): skills as `/skill:name`, found in the folders of the
//! places omp reads (its own `.omp` and agent folder, Claude Code's, `.agent`/`.agents`, Codex's, OpenCode's, the
//! project's `.github`, the Claude Code plugins installed and turned on, its extensions and npm plugins
//! (omp_plugins.rs), its `customDirectories` and managed skills), each place on unless its config.yml's `skills` turn
//! it off; commands as `/name` from the same places' `commands` folders (a plugin's as `/plugin:name`); and prompt
//! templates as `/name`. A name of its own commands is left out.
use super::read::{commands, json_file, skills, texts, yaml_file, Found, Walk};
use super::{env, path_in, Place, Skill};
use serde_json::{Map, Value};
use std::collections::HashSet;
use super::omp_plugins::{extensions, portable, portable_skill, targets_portable};
use std::path::{Path, PathBuf};

/// Its own commands, and the screens it opens: no skill or command by one of these names is listed.
const OWN: [&str; 90] = [
    "login", "logout", "review", "goal", "session", "usage", "security", "setup", "plan", "plan-review", "vibe",
    "guided-goal", "loop", "queue", "switch", "fast", "slow", "extended-context", "computer", "prewalk", "advisor",
    "export", "trace", "dump", "share", "collab", "join", "leave", "browser", "copy", "open", "todo", "jobs", "stats",
    "changelog", "hotkeys", "tools", "context", "extensions", "git", "hub", "branch", "fork", "ssh", "new", "fresh",
    "clear", "delete", "compact", "shake", "handoff", "pin", "btw", "tan", "omfg", "cleanse", "retry", "debug",
    "memory", "rename", "move", "wt", "add-dir", "remove-dir", "dirs", "exit", "restart", "marketplace",
    "reload-plugins", "force", "live", "record", "pause", "quit", "autoresearch", "green", "init", "model", "resume",
    "settings", "agents", "mcp", "plugins", "skills", "effort", "tree", "q", "providers", "models", "status"
];
#[derive(Clone)]
struct Root { path: PathBuf, user: bool, provider: &'static str, plugin: Option<String> }
fn r(path: PathBuf, user: bool, provider: &'static str) -> Root { Root { path, user, provider, plugin: None } }
fn obj(v: &Value) -> Map<String, Value> { v.as_object().cloned().unwrap_or_default() }
/// `*` and `?` as a shell matches them.
fn glob(pat: &str, s: &str) -> bool {
    let re = format!("^{}$", regex::escape(pat).replace(r"\*", ".*").replace(r"\?", "."));
    regex::Regex::new(&re).map(|r| r.is_match(s)).unwrap_or(false)
}
pub fn list(p: &Place) -> Vec<Skill> {
    let home = &p.home;
    let cwd = &p.cwd;
    let config_root = home.join(env("PI_CONFIG_DIR").unwrap_or_else(|| ".omp".into()));
    let base = yaml_file(&config_root.join("agent").join("config.yml"));
    let profile = env("OMP_PROFILE").or_else(|| env("PI_PROFILE")).or_else(|| base.get("profile").and_then(Value::as_str).map(String::from)).unwrap_or_default();
    let named = !profile.is_empty() && profile != "default" && profile != "." && profile != ".." && profile.len() <= 64 && profile.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "._-".contains(c));
    let agent = if named { config_root.join("profiles").join(&profile).join("agent") } else { env("PI_CODING_AGENT_DIR").map(|d| path_in(cwd, home, &d)).unwrap_or_else(|| config_root.join("agent")) };
    let (settings, here) = (yaml_file(&agent.join("config.yml")), yaml_file(&cwd.join(".omp").join("config.yml")));
    let mut sk = obj(settings.get("skills").unwrap_or(&Value::Null)); sk.extend(obj(here.get("skills").unwrap_or(&Value::Null)));
    let mut cm = obj(settings.get("commands").unwrap_or(&Value::Null)); cm.extend(obj(here.get("commands").unwrap_or(&Value::Null)));
    let off = |m: &Map<String, Value>, k: &str| m.get(k) == Some(&Value::Bool(false));
    // From where it runs up to the repo's top (or the home folder).
    let mut up: Vec<PathBuf> = vec![];
    let mut at = Some(cwd.as_path());
    while let Some(d) = at { up.push(d.to_path_buf()); if d.join(".git").exists() || d == home { break; } at = d.parent(); }
    let project = |dir: &str, provider: &'static str| -> Vec<Root> { up.iter().filter(|d| *d != home).map(|d| r(d.join(dir), false, provider)).collect() };
    let claude_dir = env("CLAUDE_CONFIG_DIR").map(|d| path_in(cwd, home, &d)).unwrap_or_else(|| home.join(".claude"));
    let (native_user, native_project) = (r(agent.clone(), true, "native"), r(cwd.join(".omp"), false, "native"));
    let mut native_skills: Vec<Root> = up.iter().map(|d| r(d.join(".omp"), false, "native")).collect(); native_skills.push(native_user.clone());
    let mut claude = vec![r(claude_dir.clone(), true, "claude")]; claude.extend(project(".claude", "claude"));
    let mut agents: Vec<Root> = up.iter().filter(|d| *d != home).flat_map(|d| [".agent", ".agents"].map(|n| r(d.join(n), false, "agents"))).collect();
    agents.extend([".agent", ".agents"].map(|n| r(home.join(n), true, "agents")));
    let codex = vec![r(home.join(".codex"), true, "codex"), r(cwd.join(".codex"), false, "codex")];
    let opencode = vec![r(home.join(".config").join("opencode"), true, "opencode"), r(cwd.join(".opencode"), false, "opencode")];
    let github = r(cwd.join(".github"), false, "github");
    // The Claude Code plugins installed and turned on (its own plugin folders' lists too).
    let top = up.iter().find(|d| d.join(".git").exists()).cloned().unwrap_or_else(|| cwd.clone());
    let mut on = obj(json_file(&claude_dir.join("settings.json")).get("enabledPlugins").unwrap_or(&Value::Null));
    for d in up.iter().rev() { for f in ["settings.json", "settings.local.json"] { on.extend(obj(json_file(&d.join(".claude").join(f)).get("enabledPlugins").unwrap_or(&Value::Null))); } }
    let omp_project = up.iter().find(|d| d.join(".omp").exists() || d.join(".git").exists()).cloned().unwrap_or_else(|| cwd.clone()).join(".omp").join("plugins");
    let mut market: Vec<Root> = vec![];
    for (dir, claude_list, project_scope) in [(claude_dir.join("plugins"), true, false), (config_root.join("plugins"), false, false), (omp_project.clone(), false, true)] {
        for (id, list) in obj(json_file(&dir.join("installed_plugins.json")).get("plugins").unwrap_or(&Value::Null)) {
            for e in list.as_array().into_iter().flatten() {
                let Some(path) = e.get("installPath").and_then(Value::as_str) else { continue };
                if e.get("enabled") == Some(&Value::Bool(false)) { continue; }
                let level_project = project_scope || matches!(e.get("scope").and_then(Value::as_str), Some("project") | Some("local"));
                if claude_list && (on.get(&id) == Some(&Value::Bool(false)) || (level_project && on.get(&id) != Some(&Value::Bool(true)) && e.get("projectPath").and_then(Value::as_str) != top.to_str())) { continue; }
                market.push(Root { path: PathBuf::from(path), user: !level_project, provider: "claude-plugins", plugin: Some(id.split('@').next().unwrap_or(&id).to_string()) });
            }
        }
    }
    // Its extensions and npm plugins; which plugins are portable ones, and which keep the older layout.
    let ext: Vec<Root> = extensions(&[(native_project.path.clone(), false), (native_user.path.clone(), true)], &[(config_root.join("plugins"), true), (omp_project, false)], cwd, home)
        .into_iter().map(|(path, user)| r(path, user, "omp-plugins")).collect();
    let portables: Vec<Root> = market.iter().chain(&ext).filter(|s| portable(&s.path)).cloned().collect();
    let is_portable = |s: &Root| portables.iter().any(|x| x.path == s.path);
    let legacy_ext: Vec<Root> = ext.iter().filter(|s| !targets_portable(&s.path)).cloned().collect();
    let legacy_market: Vec<Root> = market.iter().filter(|s| !targets_portable(&s.path)).cloned().collect();
    let plugin_dirs = |s: &Root, kind: &str| -> Vec<PathBuf> {
        let m = json_file(&s.path.join(".claude-plugin").join("plugin.json"));
        let declared = m.get(kind).or_else(|| if kind == "commands" { m.get("slash-commands") } else { None }).map(texts).unwrap_or_default();
        let mut v: Vec<PathBuf> = if kind == "skills" || declared.is_empty() { vec![s.path.join(kind)] } else { vec![] };
        v.extend(declared.iter().filter(|d| !d.split('/').any(|x| x == "..")).map(|d| s.path.join(d)));
        v
    };
    let third = ["enableCodexUser", "enableClaudeUser", "enableClaudeProject", "enablePiUser", "enablePiProject"].iter().any(|k| !off(&sk, k));
    let lvl = |s: &Root| if s.user { "User" } else { "Project" };
    let enabled = |s: &Root| -> bool {
        match s.provider { "native" => !off(&sk, &format!("enablePi{}", lvl(s))), "agents" => !off(&sk, &format!("enableAgents{}", lvl(s))), "claude" => !off(&sk, &format!("enableClaude{}", lvl(s))), "codex" if s.user => !off(&sk, "enableCodexUser"), _ => third }
    };
    let own: HashSet<&str> = OWN.iter().copied().collect();
    let mut names: HashSet<String> = HashSet::new();
    let mut out: Vec<Skill> = vec![];
    let ignored = sk.get("ignoredSkills").map(texts).unwrap_or_default();
    let included = sk.get("includeSkills").map(texts).unwrap_or_default();
    let disabled: Vec<String> = [settings.get("disabledExtensions"), here.get("disabledExtensions")].into_iter().flatten().flat_map(texts).collect();
    let mut seen_files = HashSet::new();
    let mut add_skills = |dirs: Vec<PathBuf>, spec: Option<&Path>, out: &mut Vec<Skill>| {
        for d in dirs { for f in skills(&d, Walk::One, true) {
            if spec.is_some_and(|root| !portable_skill(&f, root)) { continue; }
            let n = f.name.trim().to_string();
            if n.is_empty() || n.starts_with('.') || f.fields.get("enabled") == Some(&Value::Bool(false)) || disabled.contains(&format!("skill:{n}"))
                || ignored.iter().any(|g| glob(g, &n)) || (!included.is_empty() && !included.iter().any(|g| glob(g, &n))) { continue; }
            if !names.insert(format!("skill:{n}")) || !seen_files.insert(std::fs::canonicalize(&f.path).unwrap_or(f.path.clone())) { continue; }
            out.push(Skill { typed: format!("/skill:{n}"), about: super::read::brief(&f.about) });
        } }
    };
    if !off(&sk, "enabled") && !off(&sk, "enableSkillCommands") {
        for d in sk.get("customDirectories").map(texts).unwrap_or_default() { add_skills(vec![path_in(cwd, home, &d)], None, &mut out); }
        let all: Vec<Root> = native_skills.iter().chain(&legacy_ext).chain(&claude).chain(&portables).chain(&legacy_market).chain(&agents).chain(&codex).chain(&opencode).chain(std::iter::once(&github)).cloned().collect();
        for s in all.iter().filter(|s| enabled(s)) {
            let spec = is_portable(s);
            add_skills(if s.plugin.is_some() && !spec { plugin_dirs(s, "skills") } else { vec![s.path.join("skills")] }, spec.then_some(s.path.as_path()), &mut out);
        }
        add_skills(vec![agent.join("managed-skills")], None, &mut out);
    }
    let mut taken: HashSet<String> = own.iter().map(|s| s.to_string()).collect();
    let short = |f: &Found| -> String { match f.s("description") { Some(d) => d.trim().to_string(), None => { let a = f.about.clone(); if a.chars().count() > 60 { format!("{}...", a.chars().take(60).collect::<String>()) } else { a } } } };
    let mut add = |name: String, about: String, out: &mut Vec<Skill>| { if taken.insert(name.clone()) { out.push(Skill { typed: format!("/{name}"), about: super::read::brief(&about) }); } };
    let claude_cmds = vec![r(claude_dir.clone(), true, "claude"), r(cwd.join(".claude"), false, "claude")];
    let cmd_scopes: Vec<Root> = [native_project.clone(), native_user.clone()].into_iter().chain(legacy_ext.clone()).chain(claude_cmds).chain(legacy_market).chain(agents.clone()).chain(codex.clone()).chain(opencode.clone()).collect();
    for s in cmd_scopes {
        if ["claude", "opencode"].contains(&s.provider) && off(&cm, &format!("enable{}{}", if s.provider == "claude" { "Claude" } else { "Opencode" }, lvl(&s))) { continue; }
        let dirs = if s.plugin.is_some() { plugin_dirs(&s, "commands") } else { vec![s.path.join("commands")] };
        for d in dirs { for f in commands(&d, s.provider == "claude") {
            let leaf = f.path.file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
            let n = if ["codex", "opencode"].contains(&s.provider) { f.s("name").map(String::from).unwrap_or(leaf.clone()) } else { leaf.clone() };
            add(s.plugin.as_ref().map(|pl| format!("{pl}:{n}")).unwrap_or(n), short(&f), &mut out);
            if s.provider == "claude" && f.name.contains(':') { add(f.name.clone(), short(&f), &mut out); }
        } }
    }
    for s in [native_user, native_project].into_iter().chain(legacy_ext).chain(agents).chain(codex).chain(std::iter::once(github)) {
        let dir = s.path.join("prompts");
        for f in commands(&dir, s.provider == "native") {
            let file = f.path.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
            if s.provider == "github" && !file.ends_with(".prompt.md") { continue; }
            let sub = f.name.rsplit_once(':').map(|x| format!(":{}", x.0)).unwrap_or_default();
            let suffix = format!("({}{sub})", if s.user { "user" } else { "project" });
            let stem = file.trim_end_matches(".md").trim_end_matches(".prompt").to_string();
            let n = if ["github", "codex"].contains(&s.provider) { f.s("name").map(String::from).unwrap_or(stem) } else { stem };
            let d = short(&f);
            add(n, if d.is_empty() { suffix } else { format!("{d} {suffix}") }, &mut out);
        }
    }
    out
}

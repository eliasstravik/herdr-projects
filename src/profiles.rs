//! Agent profiles: named launch setups (harness, model, reasoning effort and
//! extra arguments) that the user keeps in `~/.config/herdr-projects/
//! config.toml`, and the per-role allow-lists that say which ones threads and
//! coordinators may use. Agents only ever choose a profile by name.
//!
//! Every Herdr agent kind is also a built-in profile of the same name with no
//! arguments; a user profile of that name replaces it. Built-ins are listed
//! (in `profile list`, `context` and the popup) only when their CLI is
//! installed and looks signed in, but any of them can be named.

use std::collections::BTreeMap;
use std::io::IsTerminal as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use toml_edit::{Array, DocumentMut, Item, Table};

use crate::paths::{Ctx, Env};
use crate::project::{self, Project};

/// Who a profile launches: a thread's worker or a project's coordinator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Thread,
    Coordinator,
}

impl Role {
    pub fn parse(text: &str) -> Result<Role> {
        match text {
            "thread" | "threads" => Ok(Role::Thread),
            "coordinator" | "coordinators" => Ok(Role::Coordinator),
            other => bail!("`{other}` is not a role; use `threads` or `coordinator`"),
        }
    }

    /// The config key of this role's allow-list.
    pub fn list_key(self) -> &'static str {
        match self {
            Role::Thread => "thread_profiles",
            Role::Coordinator => "coordinator_profiles",
        }
    }

    /// The key of this role's default profile, in PROJECT.md and `[defaults]`.
    pub fn default_key(self) -> &'static str {
        match self {
            Role::Thread => "thread_profile",
            Role::Coordinator => "coordinator_profile",
        }
    }

    fn word(self) -> &'static str {
        match self {
            Role::Thread => "threads",
            Role::Coordinator => "the coordinator",
        }
    }
}

/// One `[profiles.<name>]` table as the user wrote it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Entry {
    pub agent: String,
    pub model: String,
    pub effort: String,
    pub args: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub name: String,
    pub entry: Entry,
    /// A bare Herdr kind that no `[profiles]` table replaces.
    pub builtin: bool,
}

impl Profile {
    fn builtin(kind: &str) -> Profile {
        Profile { name: kind.to_string(), entry: Entry { agent: kind.to_string(), ..Entry::default() }, builtin: true }
    }

    pub fn agent(&self) -> &str {
        &self.entry.agent
    }

    /// The agent CLI arguments this profile launches with: the model and
    /// effort flags for its harness, then its own arguments.
    pub fn args(&self) -> Vec<String> {
        let mut args = typed_args(&self.entry.agent, &self.entry.model, &self.entry.effort).unwrap_or_default();
        args.extend(self.entry.args.iter().cloned());
        args
    }

    /// One line for `context`, `profile list` and the popup:
    /// `omp · model gpt-5.5 · effort high · 2 extra args — Cheap tier`.
    pub fn summary(&self) -> String {
        let e = &self.entry;
        let mut parts = vec![e.agent.clone(), if e.model.is_empty() { "default model".into() } else { format!("model {}", e.model) }];
        if !e.effort.is_empty() {
            parts.push(format!("effort {}", e.effort));
        }
        if !e.args.is_empty() {
            parts.push(format!("args: {}", e.args.join(" ")));
        }
        let mut line = parts.join(" · ");
        if !e.description.is_empty() {
            line.push_str(" — ");
            line.push_str(&e.description);
        }
        line
    }
}

/// `[defaults]`: the default profiles `new` writes into a new PROJECT.md.
/// The allow-lists are safety settings: `[safety.default]` for every
/// project, a project's own `[safety."<path>"]` table over it.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Defaults {
    pub thread_profile: Option<String>,
    pub coordinator_profile: Option<String>,
}

/// The profile part of `config.toml`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Config {
    pub profiles: BTreeMap<String, Entry>,
    pub defaults: Defaults,
}

pub fn load(config_dir: &Path) -> Result<Config> {
    let file = config_dir.join("config.toml");
    let Ok(text) = std::fs::read_to_string(&file) else {
        return Ok(Config::default());
    };
    let config: Config = toml::from_str(&text).with_context(|| format!("{} does not parse", file.display()))?;
    for (name, entry) in &config.profiles {
        validate(name, entry).with_context(|| format!("{}: [profiles.{name}]", file.display()))?;
    }
    Ok(config)
}

impl Config {
    /// A profile by name: the user's table, else the built-in of that kind.
    pub fn get(&self, name: &str) -> Option<Profile> {
        match self.profiles.get(name) {
            Some(entry) => Some(Profile { name: name.to_string(), entry: entry.clone(), builtin: false }),
            None => crate::agents::is_kind(name).then(|| Profile::builtin(name)),
        }
    }

    /// The user's profiles, then the built-ins of `detected` kinds they do
    /// not replace.
    pub fn listed(&self, detected: &[String]) -> Vec<Profile> {
        let mut out: Vec<Profile> = self.profiles.keys().filter_map(|n| self.get(n)).collect();
        out.extend(detected.iter().filter(|k| !self.profiles.contains_key(*k)).map(|k| Profile::builtin(k)));
        out
    }

    /// The names `role` may use in a project: its own `[safety]` list, else
    /// `[safety.default]`'s (both merged by `load_safety`), else `None`
    /// (any profile).
    pub fn allowed(&self, safety: &project::Safety, role: Role) -> Option<Vec<String>> {
        match role {
            Role::Thread => safety.thread_profiles.clone(),
            Role::Coordinator => safety.coordinator_profiles.clone(),
        }
    }

    /// The default `new` writes into PROJECT.md for `role`.
    pub fn new_project_default(&self, role: Role) -> String {
        let value = match role {
            Role::Thread => &self.defaults.thread_profile,
            Role::Coordinator => &self.defaults.coordinator_profile,
        };
        value.clone().filter(|v| !v.is_empty()).unwrap_or_else(|| "claude".into())
    }
}

/// A profile name: letters, digits, `.`, `_` and `-`, up to 40 characters.
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 40 || name.starts_with(['-', '.']) || !name.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)) {
        bail!("`{name}` is not a profile name: use letters, digits, `.`, `_` and `-` (at most 40)");
    }
    Ok(())
}

fn validate(name: &str, entry: &Entry) -> Result<()> {
    validate_name(name)?;
    if !crate::agents::is_kind(&entry.agent) {
        bail!("agent `{}` is not a Herdr agent kind ({})", entry.agent, crate::agents::KINDS.join(", "));
    }
    typed_args(&entry.agent, &entry.model, &entry.effort)?;
    Ok(())
}

// ---------------------------------------------------------------- harness flags

/// The effort values a harness's own flag accepts (checked 2026-09-25), or
/// `None` when it has no launch flag for effort: Cursor puts effort in the
/// model id (`gpt-5.6-sol-xhigh`), Gemini CLI in settings.json, and
/// OpenCode's `--variant` is documented for `opencode run` only. For those,
/// use the model id or the profile's `args`.
pub fn effort_values(agent: &str) -> Option<&'static [&'static str]> {
    match agent {
        "claude" => Some(&["low", "medium", "high", "xhigh", "max"]),
        "codex" => Some(&["none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra"]),
        "copilot" => Some(&["none", "minimal", "low", "medium", "high", "xhigh", "max"]),
        "omp" => Some(&["off", "minimal", "low", "medium", "high", "xhigh", "max", "auto"]),
        "pi" => Some(&["off", "minimal", "low", "medium", "high", "xhigh", "max"]),
        _ => None,
    }
}

/// Whether `value` looks like a model name: `opus`, `gpt-5.5`,
/// `anthropic/claude-sonnet-5`, `claude-opus-5-5[1m]`. Never a flag.
pub fn is_model_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with('-')
        && value.chars().all(|c| c.is_ascii_alphanumeric() || "._-/:@+[]".contains(c))
}

/// The model and effort flags for `agent`: `--model NAME` for every harness,
/// and the harness's own effort flag: Claude Code and Copilot CLI `--effort`,
/// Codex `-c model_reasoning_effort="…"`, pi and oh-my-pi `--thinking`.
pub fn typed_args(agent: &str, model: &str, effort: &str) -> Result<Vec<String>> {
    let mut args = Vec::new();
    if !model.is_empty() {
        if !is_model_name(model) {
            bail!("`{model}` does not look like a model name");
        }
        args.extend(["--model".to_string(), model.to_string()]);
    }
    if !effort.is_empty() {
        let Some(values) = effort_values(agent) else {
            bail!("herdr-projects knows no reasoning-effort flag for {agent}; leave effort empty and put the flag in the profile's args");
        };
        if !values.contains(&effort) {
            bail!("effort `{effort}` is not one of {} for {agent}", values.join(", "));
        }
        match agent {
            "claude" | "copilot" => args.extend(["--effort".to_string(), effort.to_string()]),
            "codex" => args.extend(["-c".to_string(), format!("model_reasoning_effort=\"{effort}\"")]),
            _ => args.extend(["--thinking".to_string(), effort.to_string()]),
        }
    }
    Ok(args)
}

// ---------------------------------------------------------------- choosing

/// The profile `role` launches with in `project`: `requested`, else the
/// project's default. Refused when it does not exist or the allow-list does
/// not name it.
pub fn resolve(config: &Config, safety: &project::Safety, settings: &project::Settings, role: Role, requested: Option<&str>, slug: &str) -> Result<Profile> {
    let default = match role {
        Role::Thread => &settings.thread_profile,
        Role::Coordinator => &settings.coordinator_profile,
    };
    let name = requested.unwrap_or(default);
    let profile = config
        .get(name)
        .with_context(|| format!("there is no profile `{name}`; `profile list` shows them, and the user adds one with `profile add`"))?;
    check_allowed(config, safety, role, name, slug)?;
    Ok(profile)
}

pub fn check_allowed(config: &Config, safety: &project::Safety, role: Role, name: &str, slug: &str) -> Result<()> {
    if let Some(allowed) = config.allowed(safety, role)
        && !allowed.iter().any(|a| a == name)
    {
        bail!(
            "profile `{name}` is not allowed for {} in `{slug}`; allowed: {}. Only the user changes this list (`profile allow {} ... --project {slug}`)",
            role.word(),
            if allowed.is_empty() { "none".to_string() } else { allowed.join(", ") },
            role.list_key().trim_end_matches("_profiles"),
        );
    }
    Ok(())
}

/// Everything `thread start`, `open` and the ticker need to launch a role.
pub fn for_project(ctx: &Ctx, project: &Project, role: Role, requested: Option<&str>) -> Result<Profile> {
    let config = load(&ctx.config_dir)?;
    let safety = project.safety(&ctx.config_dir)?;
    let (settings, _) = project.read_project_md()?;
    resolve(&config, &safety, &settings, role, requested, &project.slug)
}

/// The arguments a launch gets. `legacy` is the user's old
/// `thread_agent_args` or `coordinator_agent_args`: they were written for the
/// harness of the project's default profile, so they reach a built-in profile
/// of that harness only, and never another harness (issue #45) or a
/// user profile, which carries its own arguments.
pub fn launch_args(profile: &Profile, legacy: &[String], legacy_agent: &str) -> Vec<String> {
    let mut args = if profile.builtin && profile.agent() == legacy_agent { legacy.to_vec() } else { Vec::new() };
    args.extend(profile.args());
    args
}

/// The harness the legacy arguments belong to: the project's default
/// profile's, `claude` when that cannot be resolved.
pub fn legacy_agent(config: &Config, settings: &project::Settings, role: Role) -> String {
    let default = match role {
        Role::Thread => &settings.thread_profile,
        Role::Coordinator => &settings.coordinator_profile,
    };
    config.get(default).map(|p| p.entry.agent).unwrap_or_else(|| "claude".into())
}

/// `~/` at the start of an argument or after `=` becomes the home folder,
/// for local launches: Herdr passes arguments without a shell.
pub fn expand_home(args: &[String], home: &Path) -> Vec<String> {
    let home = home.to_string_lossy();
    args.iter()
        .map(|a| {
            if let Some(rest) = a.strip_prefix("~/") {
                format!("{home}/{rest}")
            } else if let Some((key, rest)) = a.split_once("=~/") {
                format!("{key}={home}/{rest}")
            } else {
                a.clone()
            }
        })
        .collect()
}

// ---------------------------------------------------------------- detection

/// The executable a Herdr kind starts (Herdr's `interactive_agent_executable`).
pub fn executable(kind: &str) -> &str {
    match kind {
        "cursor" => "cursor-agent",
        "kiro" => "kiro-cli",
        other => other,
    }
}

fn on_path(env: &Env, name: &str) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    env.var("PATH").unwrap_or("").split(':').filter(|d| !d.is_empty()).any(|dir| {
        std::fs::metadata(Path::new(dir).join(name)).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    })
}

/// Whether `kind` looks signed in, from files and variables only (no network,
/// no keychain, no running its CLI): Herdr reports neither installs nor
/// sign-ins it can be asked about. A kind without a known check counts as
/// signed in once installed.
fn signed_in(env: &Env, kind: &str) -> bool {
    let home = &env.home;
    let any_var = |names: &[&str]| names.iter().any(|n| env.var(n).is_some());
    let any_file = |paths: &[PathBuf]| paths.iter().any(|p| p.is_file());
    let file_has = |path: PathBuf, needle: &str| std::fs::read_to_string(path).is_ok_and(|t| t.contains(needle));
    let dir_of = |var: &str, fallback: &str| env.var(var).map(PathBuf::from).unwrap_or_else(|| home.join(fallback));
    match kind {
        "claude" => {
            let config = dir_of("CLAUDE_CONFIG_DIR", ".claude");
            any_var(&["ANTHROPIC_API_KEY", "CLAUDE_CODE_OAUTH_TOKEN", "CLAUDE_CODE_USE_BEDROCK", "CLAUDE_CODE_USE_VERTEX"])
                || any_file(&[config.join(".credentials.json")])
                || file_has(home.join(".claude.json"), "\"oauthAccount\"")
                || file_has(config.join(".claude.json"), "\"oauthAccount\"")
        }
        "codex" => any_var(&["OPENAI_API_KEY", "CODEX_ACCESS_TOKEN"]) || any_file(&[dir_of("CODEX_HOME", ".codex").join("auth.json")]),
        "gemini" => {
            any_var(&["GEMINI_API_KEY", "GOOGLE_API_KEY", "GOOGLE_GENAI_USE_VERTEXAI"]) || any_file(&[home.join(".gemini/oauth_creds.json")])
        }
        "copilot" => any_var(&["COPILOT_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"]) || dir_of("COPILOT_HOME", ".copilot").is_dir(),
        "opencode" => {
            let data = env.var("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"));
            any_file(&[data.join("opencode/auth.json")]) || any_var(&["ANTHROPIC_API_KEY", "OPENAI_API_KEY"])
        }
        // The token is in the keychain; the config file appears at sign-in.
        "cursor" => any_var(&["CURSOR_API_KEY"]) || home.join(".cursor/cli-config.json").is_file(),
        "pi" => any_file(&[dir_of("PI_CODING_AGENT_DIR", ".pi/agent").join("auth.json")]) || any_var(&["ANTHROPIC_API_KEY", "OPENAI_API_KEY", "GEMINI_API_KEY"]),
        "omp" => any_file(&[home.join(".omp/agent/agent.db"), home.join(".omp/agent/.env"), home.join(".omp/.env")]) || any_var(&["ANTHROPIC_API_KEY", "OPENAI_API_KEY", "GEMINI_API_KEY"]),
        _ => true,
    }
}

/// The kinds whose CLI is on `PATH` and looks signed in, in Herdr's order.
pub fn detect(env: &Env) -> Vec<String> {
    crate::agents::KINDS.iter().filter(|k| on_path(env, executable(k)) && signed_in(env, k)).map(|k| k.to_string()).collect()
}

// ---------------------------------------------------------------- editing

/// A change to the user's profile settings. Only a person makes one: at a
/// terminal (`profile add`, `edit`, `remove`, `allow`, `default`) or in the
/// popup, never an agent's shell tool.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Add { name: String, entry: Entry },
    Edit { name: String, agent: Option<String>, model: Option<String>, effort: Option<String>, description: Option<String>, args: Option<Vec<String>> },
    Remove { name: String },
    /// `names: None` removes the list: every profile is allowed.
    Allow { role: Role, project: Option<PathBuf>, names: Option<Vec<String>> },
    Default { role: Role, name: String },
}

pub fn require_person() -> Result<()> {
    if !std::io::stdin().is_terminal() {
        bail!("profiles and their allow-lists are the user's: this command must be run by a person at a terminal (or from the projects popup); standard input is not a terminal");
    }
    Ok(())
}

fn table<'a>(doc: &'a mut DocumentMut, key: &str) -> Result<&'a mut Table> {
    if doc.get(key).is_none() {
        let mut t = Table::new();
        t.set_implicit(true);
        doc[key] = Item::Table(t);
    }
    doc[key].as_table_mut().with_context(|| format!("`{key}` in config.toml is not a table"))
}

fn string_array(names: &[String]) -> Item {
    let mut array = Array::new();
    for n in names {
        array.push(n.as_str());
    }
    toml_edit::value(array)
}

/// Applies `change` to config.toml's text, keeping everything else as it is,
/// and checks the result still loads. Returns the new text and a message.
pub fn apply_in(text: &str, change: &Change) -> Result<(String, String)> {
    let mut doc = text.parse::<DocumentMut>().context("config.toml does not parse")?;
    let current: Config = toml::from_str(text).context("config.toml does not parse")?;
    let message = match change {
        Change::Add { name, entry } => {
            validate(name, entry)?;
            if current.profiles.contains_key(name) {
                bail!("profile `{name}` already exists; `profile edit {name}` changes it");
            }
            let profiles = table(&mut doc, "profiles")?;
            let mut t = Table::new();
            t["agent"] = toml_edit::value(&entry.agent);
            for (key, value) in [("model", &entry.model), ("effort", &entry.effort), ("description", &entry.description)] {
                if !value.is_empty() {
                    t[key] = toml_edit::value(value);
                }
            }
            if !entry.args.is_empty() {
                t["args"] = string_array(&entry.args);
            }
            profiles[name] = Item::Table(t);
            let replaces = if crate::agents::is_kind(name) { format!(" (it replaces the built-in `{name}`)") } else { String::new() };
            format!("added profile `{name}`{replaces}: {}", Profile { name: name.clone(), entry: entry.clone(), builtin: false }.summary())
        }
        Change::Edit { name, agent, model, effort, description, args } => {
            let mut entry = current.profiles.get(name).cloned().with_context(|| format!("there is no profile `{name}` in config.toml; `profile add` makes one"))?;
            if let Some(agent) = agent {
                if *agent != entry.agent && effort.is_none() && effort_values(agent).is_none() {
                    // Another harness: an effort it cannot take goes.
                    entry.effort.clear();
                }
                entry.agent = agent.clone();
            }
            for (field, value) in [(&mut entry.model, model), (&mut entry.effort, effort), (&mut entry.description, description)] {
                if let Some(value) = value {
                    *field = value.clone();
                }
            }
            if let Some(args) = args {
                entry.args = args.clone();
            }
            validate(name, &entry)?;
            let t = doc["profiles"][name.as_str()].as_table_mut().with_context(|| format!("[profiles.{name}] is not a table"))?;
            t["agent"] = toml_edit::value(&entry.agent);
            for (key, value) in [("model", &entry.model), ("effort", &entry.effort), ("description", &entry.description)] {
                if value.is_empty() {
                    t.remove(key);
                } else {
                    t[key] = toml_edit::value(value);
                }
            }
            if entry.args.is_empty() {
                t.remove("args");
            } else {
                t["args"] = string_array(&entry.args);
            }
            format!("changed profile `{name}`: {}", Profile { name: name.clone(), entry, builtin: false }.summary())
        }
        Change::Remove { name } => {
            if !current.profiles.contains_key(name) {
                bail!("there is no profile `{name}` in config.toml{}", if crate::agents::is_kind(name) { " (a built-in profile cannot be removed)" } else { "" });
            }
            table(&mut doc, "profiles")?.remove(name);
            let back = if crate::agents::is_kind(name) { format!("; the built-in `{name}` is back") } else { String::new() };
            format!("removed profile `{name}`{back}")
        }
        Change::Allow { role, project, names } => {
            if let Some(names) = names {
                for n in names {
                    validate_name(n)?;
                    if current.get(n).is_none() {
                        bail!("there is no profile `{n}`");
                    }
                }
            }
            let (key, scope) = match project {
                Some(dir) => (dir.to_string_lossy().into_owned(), format!("in {}", dir.display())),
                None => (crate::safety::DEFAULT_TABLE.to_string(), "in every project without its own list".to_string()),
            };
            let safety = table(&mut doc, "safety")?;
            if safety.get(&key).is_none() {
                safety[&key] = Item::Table(Table::new());
            }
            let t = safety[&key].as_table_mut().context("the [safety] entry is not a table")?;
            match names {
                Some(names) => {
                    t[role.list_key()] = string_array(names);
                    format!("{} may use {} {scope}", role.word(), if names.is_empty() { "no profile".into() } else { names.join(", ") })
                }
                None => {
                    t.remove(role.list_key());
                    format!("{} may use every profile {scope}", role.word())
                }
            }
        }
        Change::Default { role, name } => {
            validate_name(name)?;
            if current.get(name).is_none() {
                bail!("there is no profile `{name}`");
            }
            table(&mut doc, "defaults")?[role.default_key()] = toml_edit::value(name);
            format!("new projects start with {} = \"{name}\"", role.default_key())
        }
    };
    let text = doc.to_string();
    let parsed: Config = toml::from_str(&text).context("the edited config.toml does not parse")?;
    for (name, entry) in &parsed.profiles {
        validate(name, entry)?;
    }
    Ok((text, message))
}

/// Applies `change` to `<config_dir>/config.toml`. The caller has made sure a
/// person asked for it.
pub fn apply(config_dir: &Path, change: &Change) -> Result<String> {
    let file = config_dir.join("config.toml");
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    let (edited, message) = apply_in(&text, change)?;
    std::fs::create_dir_all(config_dir)?;
    project::write_atomic(&file, edited.as_bytes())?;
    Ok(message)
}

/// Writes a new project's default profiles into its PROJECT.md.
pub fn write_project_defaults(project: &Project, thread: &str, coordinator: &str) -> Result<()> {
    let text = std::fs::read_to_string(project.project_md())?;
    let text = crate::settings::set_in(&text, "thread_profile", thread)?;
    let text = crate::settings::set_in(&text, "coordinator_profile", coordinator)?;
    project::write_atomic(&project.project_md(), text.as_bytes())
}

/// `profile list [--project SLUG]`.
pub fn list_text(ctx: &Ctx, project: Option<&Project>) -> Result<String> {
    use std::fmt::Write as _;
    let config = load(&ctx.config_dir)?;
    let detected = detect(ctx.env);
    let mut out = String::new();
    let _ = writeln!(out, "Profiles (built-ins: the agent CLIs installed and signed in here: {}):", if detected.is_empty() { "none found".to_string() } else { detected.join(", ") });
    for p in config.listed(&detected) {
        let _ = writeln!(out, "  {:<14} {}{}", p.name, p.summary(), if p.builtin { "  (built-in)" } else { "" });
    }
    let _ = writeln!(out, "\nDefaults for new projects: thread_profile = {}, coordinator_profile = {}", config.new_project_default(Role::Thread), config.new_project_default(Role::Coordinator));
    if let Some(project) = project {
        let safety = project.safety(&ctx.config_dir)?;
        let (settings, _) = project.read_project_md()?;
        let _ = writeln!(out, "\nIn `{}`:", project.slug);
        for role in [Role::Thread, Role::Coordinator] {
            let default = match role {
                Role::Thread => &settings.thread_profile,
                Role::Coordinator => &settings.coordinator_profile,
            };
            let allowed = config.allowed(&safety, role).map(|l| if l.is_empty() { "none".to_string() } else { l.join(", ") }).unwrap_or_else(|| "every profile".into());
            let _ = writeln!(out, "  {:<21} {default}\n  {:<21} {allowed}", role.default_key(), role.list_key());
        }
    } else {
        for role in [Role::Thread, Role::Coordinator] {
            let global = project::load_safety(&ctx.config_dir, Path::new("")).unwrap_or_default();
            let allowed = config.allowed(&global, role).map(|l| l.join(", ")).unwrap_or_else(|| "every profile".into());
            let _ = writeln!(out, "Allowed for {} in projects without their own list: {allowed}", role.word());
        }
    }
    Ok(out)
}

/// The `context` block: the profiles the coordinator may choose for threads,
/// one line each, and what it may open another coordinator with.
pub fn context_text(ctx: &Ctx, project: &Project, settings: &project::Settings) -> String {
    use std::fmt::Write as _;
    let (config, safety) = match (load(&ctx.config_dir), project.safety(&ctx.config_dir)) {
        (Ok(c), Ok(s)) => (c, s),
        (Err(e), _) | (_, Err(e)) => return format!("config-error: {e:#}\n"),
    };
    let detected = detect(ctx.env);
    let mut out = String::new();
    for role in [Role::Thread, Role::Coordinator] {
        let default = match role {
            Role::Thread => &settings.thread_profile,
            Role::Coordinator => &settings.coordinator_profile,
        };
        let names = usable(&config, &safety, role, &detected, default);
        let _ = writeln!(out, "{} (--profile NAME; default {default}):", if role == Role::Thread { "Thread profiles" } else { "Coordinator profiles" });
        if names.is_empty() {
            let _ = writeln!(out, "- (none allowed; ask the user)");
        }
        for p in names {
            let _ = writeln!(out, "- {}: {}", p.name, p.summary());
        }
    }
    out
}

/// The profiles to offer `role` in a project: the listed ones it may use,
/// plus its default when that is allowed but not listed (an uninstalled
/// built-in, say).
pub fn usable(config: &Config, safety: &project::Safety, role: Role, detected: &[String], default: &str) -> Vec<Profile> {
    let allowed = config.allowed(safety, role);
    let ok = |name: &str| allowed.as_ref().is_none_or(|l| l.iter().any(|a| a == name));
    let mut out: Vec<Profile> = config.listed(detected).into_iter().filter(|p| ok(&p.name)).collect();
    if let Some(list) = &allowed {
        for name in list {
            if !out.iter().any(|p| &p.name == name)
                && let Some(p) = config.get(name)
            {
                out.push(p);
            }
        }
    }
    if ok(default)
        && !out.iter().any(|p| p.name == default)
        && let Some(p) = config.get(default)
    {
        out.insert(0, p);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|a| a.to_string()).collect()
    }

    const CONFIG: &str = "root = \"~/p\" # mine\n\n[profiles.luna]\nagent = \"omp\"\nargs = [\"--config\", \"~/.omp/agent/luna.yml\"]\ndescription = \"Cheap tier\"\n\n[profiles.claude]\nagent = \"claude\"\nargs = [\"--dangerously-skip-permissions\"]\n\n[profiles.deep]\nagent = \"codex\"\nmodel = \"gpt-5.5\"\neffort = \"high\"\n\n[safety.default]\nthread_profiles = [\"claude\", \"luna\", \"deep\"]\n\n[safety.\"/p/demo\"]\nthread_profiles = [\"luna\"]\n";

    /// The merged safety settings of `/p/demo` (`own`) or another project.
    fn safety_of(text: &str, dir: &str) -> project::Safety {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("config.toml"), text).unwrap();
        project::load_safety(tmp.path(), Path::new(dir)).unwrap()
    }

    fn settings(thread: &str) -> project::Settings {
        project::Settings { thread_profile: thread.into(), ..project::Settings::default() }
    }

    #[test]
    fn profiles_resolve_to_their_harness_flags() {
        let config: Config = toml::from_str(CONFIG).unwrap();
        let deep = config.get("deep").unwrap();
        assert_eq!(deep.args(), strings(&["--model", "gpt-5.5", "-c", "model_reasoning_effort=\"high\""]));
        assert_eq!(config.get("luna").unwrap().args(), strings(&["--config", "~/.omp/agent/luna.yml"]));
        // The user's `claude` replaces the built-in; `codex` stays built-in.
        let claude = config.get("claude").unwrap();
        assert!(!claude.builtin && claude.args() == ["--dangerously-skip-permissions"]);
        let codex = config.get("codex").unwrap();
        assert!(codex.builtin && codex.args().is_empty() && codex.agent() == "codex");
        assert_eq!(config.get("gpt"), None);
        assert_eq!(typed_args("claude", "opus", "xhigh").unwrap(), strings(&["--model", "opus", "--effort", "xhigh"]));
        assert_eq!(typed_args("omp", "", "low").unwrap(), strings(&["--thinking", "low"]));
        assert!(typed_args("gemini", "", "high").is_err(), "no known effort flag");
        assert!(typed_args("claude", "", "ultra").is_err());
        assert!(typed_args("claude", "--yolo", "").is_err());
        assert_eq!(expand_home(&strings(&["--config", "~/a.yml", "--x=~/b", "~x"]), Path::new("/h")), strings(&["--config", "/h/a.yml", "--x=/h/b", "~x"]));
    }

    #[test]
    fn allow_lists_refuse_other_profiles() {
        let config: Config = toml::from_str(CONFIG).unwrap();
        let own = safety_of(CONFIG, "/p/demo");
        let none = safety_of(CONFIG, "/p/other");
        // The project's own list beats [safety.default].
        assert_eq!(resolve(&config, &own, &settings("luna"), Role::Thread, None, "demo").unwrap().name, "luna");
        let error = resolve(&config, &own, &settings("luna"), Role::Thread, Some("deep"), "demo").unwrap_err().to_string();
        assert!(error.contains("not allowed for threads in `demo`") && error.contains("allowed: luna"), "{error}");
        // A default that the list does not name is refused too.
        assert!(resolve(&config, &own, &settings("claude"), Role::Thread, None, "demo").is_err());
        // [safety.default] applies without a project list.
        assert_eq!(resolve(&config, &none, &settings("claude"), Role::Thread, Some("deep"), "x").unwrap().name, "deep");
        assert!(resolve(&config, &none, &settings("claude"), Role::Thread, Some("gemini"), "x").is_err());
        // No list for the coordinator: any profile, built-ins included.
        assert_eq!(none.coordinator_profiles, None);
        assert_eq!(resolve(&config, &none, &settings("claude"), Role::Coordinator, Some("gemini"), "x").unwrap().agent(), "gemini");
        assert!(resolve(&config, &none, &settings("claude"), Role::Coordinator, Some("nope"), "x").unwrap_err().to_string().contains("no profile `nope`"));
        // An empty list allows nothing.
        let empty = project::Safety { coordinator_profiles: Some(vec![]), ..project::Safety::default() };
        assert!(resolve(&config, &empty, &settings("claude"), Role::Coordinator, None, "x").unwrap_err().to_string().contains("allowed: none"));
    }

    #[test]
    fn old_settings_keep_working_and_legacy_args_follow_their_harness() {
        // No profiles at all: every kind is a built-in, everything allowed.
        let config = Config::default();
        let safety = project::Safety { thread_agent_args: strings(&["--dangerously-skip-permissions"]), ..project::Safety::default() };
        let old = settings("claude");
        let claude = resolve(&config, &safety, &old, Role::Thread, None, "x").unwrap();
        let legacy = legacy_agent(&config, &old, Role::Thread);
        assert_eq!(launch_args(&claude, &safety.thread_agent_args, &legacy), strings(&["--dangerously-skip-permissions"]));
        // Issue #45: a Codex thread no longer gets Claude's flag.
        let codex = resolve(&config, &safety, &old, Role::Thread, Some("codex"), "x").unwrap();
        assert!(launch_args(&codex, &safety.thread_agent_args, &legacy).is_empty());
        // A user profile carries its own arguments only.
        let config: Config = toml::from_str(CONFIG).unwrap();
        let luna = config.get("luna").unwrap();
        assert_eq!(launch_args(&luna, &safety.thread_agent_args, "omp"), strings(&["--config", "~/.omp/agent/luna.yml"]));
        // PROJECT.md's old keys read as the default profiles.
        let (s, _) = project::parse_project_md("+++\nthread_agent = \"codex\"\ncoordinator_agent = \"gemini\"\n+++\n").unwrap();
        assert_eq!((s.thread_profile.as_str(), s.coordinator_profile.as_str()), ("codex", "gemini"));
    }

    #[test]
    fn built_ins_are_the_installed_and_signed_in_kinds() {
        use std::os::unix::fs::PermissionsExt as _;
        let home = tempfile::tempdir().unwrap();
        let bin = home.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        for name in ["claude", "codex", "cursor-agent", "omp", "gemini"] {
            let path = bin.join(name);
            std::fs::write(&path, "#!/bin/sh\n").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        std::fs::write(bin.join("pi"), "not executable").unwrap();
        std::fs::create_dir_all(home.path().join(".codex")).unwrap();
        std::fs::write(home.path().join(".codex/auth.json"), "{}").unwrap();
        std::fs::write(home.path().join(".claude.json"), "{\"oauthAccount\":{}}").unwrap();
        std::fs::create_dir_all(home.path().join(".omp/agent")).unwrap();
        std::fs::write(home.path().join(".omp/agent/agent.db"), "").unwrap();
        let path = bin.to_string_lossy().into_owned();
        let env = Env::for_test(home.path(), &[("PATH", &path), ("CURSOR_API_KEY", "k")]);
        // gemini is installed but not signed in; pi is not executable.
        assert_eq!(detect(&env), strings(&["claude", "codex", "cursor", "omp"]));
        let config: Config = toml::from_str(CONFIG).unwrap();
        let names: Vec<String> = config.listed(&detect(&env)).into_iter().map(|p| p.name).collect();
        assert_eq!(names, strings(&["claude", "deep", "luna", "codex", "cursor", "omp"]));
    }

    #[test]
    fn changes_edit_config_toml_in_place() {
        let luna = Entry { agent: "omp".into(), args: strings(&["--config", "~/.omp/agent/sol.yml"]), ..Entry::default() };
        let (text, _) = apply_in(CONFIG, &Change::Add { name: "sol".into(), entry: luna.clone() }).unwrap();
        assert!(text.starts_with("root = \"~/p\" # mine\n"), "the rest stays: {text}");
        let config: Config = toml::from_str(&text).unwrap();
        assert_eq!(config.profiles["sol"], luna);
        assert!(apply_in(&text, &Change::Add { name: "sol".into(), entry: luna.clone() }).is_err(), "already exists");
        assert!(apply_in("", &Change::Add { name: "x".into(), entry: Entry { agent: "gpt".into(), ..Entry::default() } }).is_err());
        assert!(apply_in("", &Change::Add { name: "bad name".into(), entry: luna.clone() }).is_err());

        let edit = Change::Edit { name: "deep".into(), agent: None, model: Some(String::new()), effort: Some("xhigh".into()), description: None, args: Some(strings(&["--search"])) };
        let (text, _) = apply_in(&text, &edit).unwrap();
        let deep = toml::from_str::<Config>(&text).unwrap().profiles["deep"].clone();
        assert_eq!((deep.model.as_str(), deep.effort.as_str(), deep.args.clone()), ("", "xhigh", strings(&["--search"])));
        // Switching harness drops an effort the new one cannot take.
        let (text, _) = apply_in(&text, &Change::Edit { name: "deep".into(), agent: Some("gemini".into()), model: None, effort: None, description: None, args: None }).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap().profiles["deep"].effort, "");

        let allow = Change::Allow { role: Role::Coordinator, project: Some("/p/demo".into()), names: Some(strings(&["claude"])) };
        let (text, _) = apply_in(&text, &allow).unwrap();
        assert!(text.contains("[safety.\"/p/demo\"]\nthread_profiles = [\"luna\"]\ncoordinator_profiles = [\"claude\"]"), "{text}");
        assert!(apply_in(&text, &Change::Allow { role: Role::Thread, project: None, names: Some(strings(&["nope"])) }).is_err());
        let (text, _) = apply_in(&text, &Change::Allow { role: Role::Thread, project: None, names: None }).unwrap();
        assert_eq!(safety_of(&text, "/p/other").thread_profiles, None);
        let (text, _) = apply_in(&text, &Change::Allow { role: Role::Coordinator, project: None, names: Some(strings(&["luna"])) }).unwrap();
        assert!(text.contains("[safety.default]\ncoordinator_profiles = [\"luna\"]"), "{text}");
        assert_eq!(safety_of(&text, "/p/demo").coordinator_profiles, Some(strings(&["claude"])), "the project's own list still wins");
        let (text, _) = apply_in(&text, &Change::Default { role: Role::Thread, name: "luna".into() }).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap().new_project_default(Role::Thread), "luna");

        let (text, message) = apply_in(&text, &Change::Remove { name: "claude".into() }).unwrap();
        assert!(message.contains("built-in `claude` is back"));
        assert!(apply_in(&text, &Change::Remove { name: "claude".into() }).is_err());
        // The safety table still loads with its new keys.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.toml"), &text).unwrap();
        let safety = project::load_safety(dir.path(), Path::new("/p/demo")).unwrap();
        assert_eq!(safety.coordinator_profiles, Some(strings(&["claude"])));
    }
}

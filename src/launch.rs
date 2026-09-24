//! Administrator-owned launch defaults and named profiles. Project/thread files
//! may select a permitted name, but never supply the trusted arguments behind it.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::project::Safety;

#[derive(Clone, Copy)]
pub enum Role {
    Coordinator,
    Thread,
}

pub struct Launch {
    pub agent: String,
    pub args: Vec<String>,
    pub profile: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SafetyOverrides {
    start_threads: Option<String>,
    coordinator_agent_args: Option<Vec<String>>,
    thread_agent_args: Option<Vec<String>>,
    routine_commands: Option<bool>,
    coordinator_profile: Option<String>,
    thread_profile: Option<String>,
    coordinator_profiles: Option<Vec<String>>,
    thread_profiles: Option<Vec<String>>,
}

impl SafetyOverrides {
    fn apply(self, safety: &mut Safety) {
        if let Some(value) = self.start_threads { safety.start_threads = value; }
        if let Some(value) = self.coordinator_agent_args { safety.coordinator_agent_args = value; }
        if let Some(value) = self.thread_agent_args { safety.thread_agent_args = value; }
        if let Some(value) = self.routine_commands { safety.routine_commands = value; }
        if let Some(value) = self.coordinator_profile { safety.coordinator_profile = value; }
        if let Some(value) = self.thread_profile { safety.thread_profile = value; }
        if let Some(value) = self.coordinator_profiles { safety.coordinator_profiles = value; }
        if let Some(value) = self.thread_profiles { safety.thread_profiles = value; }
    }
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Defaults {
    coordinator_agent: Option<String>,
    thread_agent: Option<String>,
    safety: SafetyOverrides,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    agent: String,
    args: Vec<String>,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Config {
    defaults: Defaults,
    safety: BTreeMap<String, SafetyOverrides>,
    profiles: BTreeMap<String, Profile>,
}

fn load(config_dir: &Path) -> Result<Config> {
    let file = config_dir.join("config.toml");
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(error) => return Err(error).with_context(|| format!("could not read {}", file.display())),
    };
    toml::from_str(&text).with_context(|| format!("{} does not parse", file.display()))
}

fn validate_agent(agent: &str) -> Result<()> {
    if !crate::agents::is_kind(agent) {
        bail!("`{agent}` is not a Herdr agent kind; `herdr agent start --help` lists them");
    }
    Ok(())
}

/// Creation defaults are copied into PROJECT.md. Existing projects retain their
/// settings; changing a global default never silently switches an existing agent.
pub fn creation_agents(config_dir: &Path, coordinator: Option<String>, thread: Option<String>) -> Result<(String, String)> {
    let defaults = load(config_dir)?.defaults;
    let coordinator = coordinator.or(defaults.coordinator_agent).unwrap_or_else(|| "claude".into());
    let thread = thread.or(defaults.thread_agent).unwrap_or_else(|| "claude".into());
    validate_agent(&coordinator)?;
    validate_agent(&thread)?;
    Ok((coordinator, thread))
}

/// Per-key inheritance: an omitted project key inherits the global setting;
/// an explicit empty array/string or false replaces it rather than merging.
pub fn load_safety(config_dir: &Path, canonical_project_dir: &Path) -> Result<Safety> {
    let mut config = load(config_dir)?;
    let mut safety = Safety::default();
    config.defaults.safety.apply(&mut safety);
    if let Some(overrides) = config.safety.remove(&*canonical_project_dir.to_string_lossy()) {
        overrides.apply(&mut safety);
    }
    if !matches!(safety.start_threads.as_str(), "propose" | "auto") {
        bail!("{}: start_threads must be \"propose\" or \"auto\", not {:?}", config_dir.join("config.toml").display(), safety.start_threads);
    }
    Ok(safety)
}

/// A profile is a complete launch configuration, not an argument overlay. Only
/// the administrator's role-specific allow-list grants access to it. An explicit
/// kind opts out of the default profile, preserving the ordinary kind selector.
pub fn resolve(
    config_dir: &Path,
    safety: &Safety,
    role: Role,
    default_agent: &str,
    agent: Option<&str>,
    profile: Option<&str>,
) -> Result<Launch> {
    if agent.is_some() && profile.is_some() {
        bail!("choose --agent or --profile, not both");
    }
    let (default_profile, allowed, args, role_name) = match role {
        Role::Coordinator => (&safety.coordinator_profile, &safety.coordinator_profiles, &safety.coordinator_agent_args, "coordinator"),
        Role::Thread => (&safety.thread_profile, &safety.thread_profiles, &safety.thread_agent_args, "thread"),
    };
    let selected = profile.or_else(|| (agent.is_none() && !default_profile.is_empty()).then_some(default_profile.as_str()));
    let Some(name) = selected else {
        let agent = agent.unwrap_or(default_agent);
        validate_agent(agent)?;
        return Ok(Launch { agent: agent.to_string(), args: args.clone(), profile: None });
    };
    crate::project::validate_slug(name).context("a profile name must be a lower-case slug")?;
    if !allowed.iter().any(|allowed| allowed == name) {
        bail!("profile `{name}` is not allowed for {role_name} launches; ask the administrator to set {role_name}_profiles in config.toml");
    }
    let mut config = load(config_dir)?;
    let profile = config.profiles.remove(name).with_context(|| format!("unknown trusted launch profile `{name}` in {}", config_dir.join("config.toml").display()))?;
    validate_agent(&profile.agent)?;
    Ok(Launch { agent: profile.agent, args: profile.args, profile: Some(name.to_string()) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(text: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.toml"), text).unwrap();
        dir
    }

    #[test]
    fn safety_inherits_per_key_and_explicit_values_clear_defaults() {
        let dir = config(r#"
[defaults]
coordinator_agent = "omp"
thread_agent = "codex"
[defaults.safety]
start_threads = "auto"
routine_commands = true
coordinator_agent_args = ["--config", "/admin/luna.yml"]
thread_agent_args = ["--config", "/admin/astra.yml"]
thread_profile = "astra"
thread_profiles = ["astra", "sol"]
[safety."/projects/demo"]
routine_commands = false
thread_agent_args = []
thread_profile = ""
thread_profiles = ["sol"]
"#);
        assert_eq!(creation_agents(dir.path(), None, None).unwrap(), ("omp".into(), "codex".into()));
        let safety = load_safety(dir.path(), Path::new("/projects/demo")).unwrap();
        assert_eq!(safety.start_threads, "auto");
        assert!(!safety.routine_commands);
        assert_eq!(safety.coordinator_agent_args, ["--config", "/admin/luna.yml"]);
        assert!(safety.thread_agent_args.is_empty());
        assert!(safety.thread_profile.is_empty());
        assert_eq!(safety.thread_profiles, ["sol"]);
        let other = load_safety(dir.path(), Path::new("/projects/other")).unwrap();
        assert!(other.routine_commands);
        assert_eq!(other.thread_agent_args, ["--config", "/admin/astra.yml"]);
        assert_eq!(other.thread_profile, "astra");
    }

    #[test]
    fn profiles_replace_legacy_args_and_require_role_permission() {
        let dir = config(r#"
[profiles.astra]
agent = "omp"
args = ["--config", "/admin/full config/astra.yml"]
[profiles.sol]
agent = "omp"
args = ["--config", "/admin/sol.yml"]
"#);
        let safety = Safety {
            thread_profile: "astra".into(),
            thread_profiles: vec!["astra".into(), "sol".into(), "missing".into()],
            thread_agent_args: vec!["--config".into(), "/legacy.yml".into()],
            ..Safety::default()
        };
        let launch = resolve(dir.path(), &safety, Role::Thread, "claude", None, None).unwrap();
        assert_eq!(launch.agent, "omp");
        assert_eq!(launch.profile.as_deref(), Some("astra"));
        assert_eq!(launch.args, ["--config", "/admin/full config/astra.yml"]);
        let sol = resolve(dir.path(), &safety, Role::Thread, "claude", None, Some("sol")).unwrap();
        assert_eq!(sol.args, ["--config", "/admin/sol.yml"]);
        let plain = resolve(dir.path(), &safety, Role::Thread, "claude", Some("codex"), None).unwrap();
        assert_eq!(plain.agent, "codex");
        assert!(plain.profile.is_none());
        assert_eq!(plain.args, safety.thread_agent_args);
        assert!(resolve(dir.path(), &safety, Role::Coordinator, "omp", None, Some("astra")).is_err());
        assert!(resolve(dir.path(), &safety, Role::Thread, "omp", None, Some("missing")).is_err());
        assert!(resolve(dir.path(), &safety, Role::Thread, "omp", Some("omp"), Some("astra")).is_err());
        assert!(resolve(dir.path(), &safety, Role::Thread, "omp", None, Some("../astra")).is_err());
    }

    #[test]
    fn invalid_config_does_not_fall_back_to_untrusted_launch() {
        for text in ["[defaults", "[defaults]\ncoordinator_agent = 'unknown'", "[profiles.bad]\nagent = 'omp'\narg = []"] {
            let dir = config(text);
            assert!(creation_agents(dir.path(), None, None).is_err());
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("config.toml")).unwrap();
        assert!(load_safety(dir.path(), Path::new("/demo")).is_err());
    }
}

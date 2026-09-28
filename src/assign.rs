//! Who a TASKS.md task may be assigned to: `me`, the thread profiles this
//! project allows here, and other machines with their profiles. Machines come
//! from `herdr machine list`, whose profiles are fetched by running
//! `herdr-projects profile list --names` there over ssh, and from
//! `[machines.<label>] profiles = [...]` in config.toml, which names the
//! profiles of a machine this one cannot reach (a sandboxed VM). The ssh
//! lookups are cached in `<root>/.machines.json` for an hour, so `context`
//! does not reach other machines on every turn.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::paths::Ctx;
use crate::project::{self, Project};
use crate::tasks::Owner;

/// How long the lookups of other machines' profiles are reused.
pub const CACHE_SECONDS: u64 = 3600;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Machine {
    pub name: String,
    pub profiles: Vec<String>,
    /// Why its profiles could not be fetched; empty when they were, or when
    /// it has no SSH target to fetch them with.
    #[serde(default)]
    pub error: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Cache {
    checked: u64,
    machines: Vec<Machine>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Assignable {
    /// The thread profiles this project allows on this machine.
    pub local: Vec<String>,
    pub machines: Vec<Machine>,
}

/// `[machines.<label>]` in config.toml: `ssh` (used by `remote`) and
/// `profiles`, which only the user writes.
#[derive(Debug, Default, Deserialize)]
struct ConfigMachine {
    #[serde(default)]
    ssh: String,
    #[serde(default)]
    profiles: Vec<String>,
}

fn configured(config_dir: &Path) -> BTreeMap<String, ConfigMachine> {
    #[derive(Deserialize, Default)]
    struct Config {
        #[serde(default)]
        machines: BTreeMap<String, ConfigMachine>,
    }
    std::fs::read_to_string(config_dir.join("config.toml"))
        .ok()
        .and_then(|text| toml::from_str::<Config>(&text).ok())
        .unwrap_or_default()
        .machines
}

fn cache_path(root: &Path) -> PathBuf {
    root.join(".machines.json")
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Profile names from `profile list --names`, or from the plain `profile
/// list` of a version without `--names`.
pub fn parse_names(text: &str) -> Vec<String> {
    let lines: Vec<&str> = if text.starts_with("Profiles (") {
        text.lines().skip(1).take_while(|l| !l.trim().is_empty()).filter_map(|l| l.split_whitespace().next()).collect()
    } else {
        text.lines().map(str::trim).collect()
    };
    lines.into_iter().filter(|n| crate::profiles::validate_name(n).is_ok()).map(str::to_string).collect()
}

/// Each machine Herdr saves or config.toml names with an ssh target, and the
/// profiles it lists itself. One ssh call per machine.
fn fetch(ctx: &Ctx) -> Vec<Machine> {
    let mut targets: BTreeMap<String, String> = BTreeMap::new();
    for (name, entry) in configured(&ctx.config_dir) {
        if !entry.ssh.is_empty() {
            targets.insert(name, entry.ssh);
        }
    }
    for (name, target) in crate::remote::saved_machines(ctx.runner, &ctx.env.herdr_bin()) {
        if !target.is_empty() || !targets.contains_key(&name) {
            targets.insert(name, target);
        }
    }
    const SCRIPT: &str = "PATH=\"$HOME/.local/bin:$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH\"\nherdr-projects profile list --names 2>/dev/null || herdr-projects profile list";
    targets
        .into_iter()
        .map(|(name, target)| {
            if target.is_empty() {
                return Machine { name, profiles: Vec::new(), error: "no SSH target".into() };
            }
            match crate::remote::ssh(ctx.runner, &target, SCRIPT, None, crate::remote::SSH_TIMEOUT) {
                Ok(out) if out.success() => Machine { name, profiles: parse_names(&out.stdout), error: String::new() },
                Ok(out) => Machine { name, profiles: Vec::new(), error: first_line(&out.error_text()) },
                Err(error) => Machine { name, profiles: Vec::new(), error: first_line(&format!("{error:#}")) },
            }
        })
        .collect()
}

fn first_line(text: &str) -> String {
    let line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("unreachable").trim();
    line.chars().take(120).collect()
}

/// The fetched machines: from the cache while it is younger than
/// `CACHE_SECONDS`, else looked up again and cached. `refresh` always looks.
fn remote(ctx: &Ctx, refresh: bool) -> Vec<Machine> {
    let path = cache_path(&ctx.root);
    let cached = std::fs::read_to_string(&path).ok().and_then(|text| serde_json::from_str::<Cache>(&text).ok());
    if !refresh
        && let Some(cache) = cached
        && now().saturating_sub(cache.checked) < CACHE_SECONDS
    {
        return cache.machines;
    }
    let machines = fetch(ctx);
    if let Ok(json) = serde_json::to_vec_pretty(&Cache { checked: now(), machines: machines.clone() }) {
        let _ = project::write_atomic(&path, &json);
    }
    machines
}

/// Everything a task in `project` may be assigned to.
pub fn load(ctx: &Ctx, project: &Project, refresh: bool) -> Result<Assignable> {
    let config = crate::profiles::load(&ctx.config_dir)?;
    let safety = project.safety(&ctx.config_dir)?;
    let (settings, _) = project.read_project_md()?;
    let detected = crate::profiles::detect(ctx.env);
    let local = crate::profiles::usable(&config, &safety, crate::profiles::Role::Thread, &detected, &settings.thread_profile).into_iter().map(|p| p.name).collect();
    Ok(Assignable { local, machines: merge(remote(ctx, refresh), configured(&ctx.config_dir)) })
}

/// The fetched machines plus config.toml's, each with the profiles either
/// source names. A machine config.toml gives profiles is not an error.
fn merge(fetched: Vec<Machine>, configured: BTreeMap<String, ConfigMachine>) -> Vec<Machine> {
    let mut machines: BTreeMap<String, Machine> = fetched.into_iter().map(|m| (m.name.clone(), m)).collect();
    for (name, entry) in configured {
        let machine = machines.entry(name.clone()).or_insert_with(|| Machine { name, ..Machine::default() });
        for profile in entry.profiles {
            if !machine.profiles.contains(&profile) {
                machine.profiles.push(profile);
            }
        }
        if machine.error == "no SSH target" || !machine.profiles.is_empty() {
            machine.error.clear();
        }
    }
    machines.into_values().filter(|m| crate::tasks::is_machine_name(&m.name)).collect()
}

impl Assignable {
    fn machine(&self, name: &str) -> Option<&Machine> {
        self.machines.iter().find(|m| m.name == name)
    }

    /// Refuses an owner that is not `me`, one of the project's profiles here,
    /// a known machine, or a profile that machine lists.
    pub fn check(&self, owner: &Owner) -> Result<()> {
        match owner {
            Owner::Unassigned | Owner::Me | Owner::Agent { profile: None, machine: None } => Ok(()),
            Owner::Other(text) => bail!("`{text}` is not an owner: use me, a profile, @machine or profile@machine ({})", self.line()),
            Owner::Agent { profile: Some(profile), machine: None } => {
                if self.local.contains(profile) {
                    return Ok(());
                }
                bail!("there is no profile `{profile}` for threads here; assignable: {}", self.line())
            }
            Owner::Agent { profile, machine: Some(name) } => {
                let Some(machine) = self.machine(name) else {
                    bail!("there is no machine `{name}`: it is not in `herdr machine list` or [machines] in config.toml; assignable: {}", self.line());
                };
                let Some(profile) = profile else { return Ok(()) };
                if machine.profiles.contains(profile) {
                    return Ok(());
                }
                if machine.profiles.is_empty() && !machine.error.is_empty() {
                    bail!("the profiles of `{name}` are unknown ({}); `assignable --refresh` looks again, or assign `@{name}` and let its coordinator pick", machine.error);
                }
                bail!("`{name}` has no profile `{profile}`; it has: {}", if machine.profiles.is_empty() { "none listed".to_string() } else { machine.profiles.join(", ") })
            }
        }
    }

    /// `claude, codex-fast, @m1: claude|pi, @box (not reached)`.
    pub fn line(&self) -> String {
        let mut parts: Vec<String> = self.local.clone();
        for m in &self.machines {
            parts.push(match (m.profiles.is_empty(), m.error.is_empty()) {
                (false, _) => format!("@{}: {}", m.name, m.profiles.join("|")),
                (true, true) => format!("@{}", m.name),
                (true, false) => format!("@{} (not reached)", m.name),
            });
        }
        if parts.is_empty() { "(none; only me)".into() } else { parts.join(", ") }
    }
}

/// The one `Assignable:` line of `context`; never reaches another machine
/// while the cache is fresh.
pub fn context_line(ctx: &Ctx, project: &Project) -> String {
    match load(ctx, project, false) {
        Ok(assignable) => format!("Assignable (TASKS.md owners, besides me): {}\n", assignable.line()),
        Err(error) => format!("config-error: assignable: {error:#}\n"),
    }
}

/// `assignable <slug> [--refresh] [--check OWNER]`.
pub fn run(ctx: &Ctx, slug: &str, refresh: bool, check: Option<&str>) -> Result<()> {
    let project = Project::load(&ctx.root, slug)?;
    let assignable = load(ctx, &project, refresh)?;
    if let Some(owner) = check {
        let text = owner.trim().trim_start_matches('(').trim_end_matches(')');
        assignable.check(&Owner::parse(text))?;
        println!("`{text}` is a valid owner");
        return Ok(());
    }
    let mut out = String::new();
    let _ = writeln!(out, "me: the user");
    let _ = writeln!(out, "Profiles here: {}", if assignable.local.is_empty() { "(none allowed)".to_string() } else { assignable.local.join(", ") });
    if assignable.machines.is_empty() {
        let _ = writeln!(out, "Machines: none (herdr machine list is empty and config.toml has no [machines])");
    }
    for m in &assignable.machines {
        let profiles = if m.profiles.is_empty() { "profiles unknown".to_string() } else { m.profiles.join(", ") };
        let error = if m.error.is_empty() { String::new() } else { format!(" ({})", m.error) };
        let _ = writeln!(out, "@{}: {profiles}{error}", m.name);
    }
    print!("{out}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::fake::{fail, ok};

    fn agent(profile: Option<&str>, machine: Option<&str>) -> Owner {
        Owner::Agent { profile: profile.map(str::to_string), machine: machine.map(str::to_string) }
    }

    #[test]
    fn names_come_from_either_list_format() {
        assert_eq!(parse_names("claude\ncodex-fast\n\n"), ["claude", "codex-fast"]);
        let old = "Profiles (built-ins: the agent CLIs installed and signed in here: claude):\n  luna           omp · default model\n  claude         claude · default model  (built-in)\n\nDefaults for new projects: thread_profile = claude\n";
        assert_eq!(parse_names(old), ["luna", "claude"]);
        assert!(parse_names("error: unexpected argument\n").is_empty());
    }

    #[test]
    fn owners_are_checked_against_profiles_and_machines() {
        let assignable = Assignable {
            local: vec!["claude".into(), "codex-fast".into()],
            machines: vec![
                Machine { name: "m1".into(), profiles: vec!["claude".into(), "pi".into()], error: String::new() },
                Machine { name: "far".into(), profiles: vec![], error: "timed out".into() },
            ],
        };
        for good in [Owner::Me, Owner::Unassigned, agent(Some("codex-fast"), None), agent(None, Some("m1")), agent(Some("pi"), Some("m1")), agent(None, Some("far")), agent(None, None)] {
            assignable.check(&good).unwrap();
        }
        assert!(assignable.check(&agent(Some("pi"), None)).unwrap_err().to_string().contains("no profile `pi`"));
        assert!(assignable.check(&agent(Some("codex-fast"), Some("m1"))).unwrap_err().to_string().contains("it has: claude, pi"));
        assert!(assignable.check(&agent(None, Some("m9"))).is_err());
        assert!(assignable.check(&agent(Some("claude"), Some("far"))).unwrap_err().to_string().contains("unknown (timed out)"));
        assert!(assignable.check(&Owner::Other("Bob".into())).is_err());
        assert_eq!(assignable.line(), "claude, codex-fast, @m1: claude|pi, @far (not reached)");
        assert_eq!(Assignable::default().line(), "(none; only me)");
    }

    #[test]
    fn herdr_machines_are_fetched_once_an_hour_and_config_adds_names() {
        let world = crate::scenarios::World::new();
        let project = world.project("demo", "a.sock");
        let ctx = world.ctx();
        std::fs::create_dir_all(&ctx.config_dir).unwrap();
        std::fs::write(ctx.config_dir.join("config.toml"), "[machines.vm]\nprofiles = [\"claude\", \"cheap\"]\n\n[machines.m1]\nprofiles = [\"extra\"]\n").unwrap();
        world.runner.on("machine list --json", ok(r#"[{"id":"1","label":"m1","target":"me@m1"},{"id":"2","label":"down","target":"me@down"}]"#));
        world.runner.on_fn(|c| c.program == "ssh" && c.args.iter().any(|a| a == "me@m1"), |_| Ok(ok("claude\npi\n")));
        world.runner.on("me@down", fail(255, "ssh: connect to host down port 22: Connection timed out"));
        let first = load(&ctx, &project, false).unwrap();
        let line = first.line();
        assert!(line.ends_with("@down (not reached), @m1: claude|pi|extra, @vm: claude|cheap"), "{line}");
        assert_eq!(world.runner.count("ssh"), 2);
        // Fresh cache: no ssh, and config.toml edits still show at once.
        let ctx = world.ctx();
        std::fs::create_dir_all(&ctx.config_dir).unwrap();
        std::fs::write(ctx.config_dir.join("config.toml"), "[machines.vm]\nprofiles = [\"claude\"]\n").unwrap();
        let again = load(&ctx, &project, false).unwrap();
        assert_eq!(world.runner.count("ssh"), 2);
        assert!(again.line().ends_with("@m1: claude|pi, @vm: claude"), "{}", again.line());
        // A stale cache or --refresh looks again.
        let stale = serde_json::json!({ "checked": now() - CACHE_SECONDS - 1, "machines": [] });
        std::fs::write(cache_path(&world.root), stale.to_string()).unwrap();
        load(&ctx, &project, false).unwrap();
        assert_eq!(world.runner.count("ssh"), 4);
        load(&ctx, &project, true).unwrap();
        assert_eq!(world.runner.count("ssh"), 6);
        assert!(context_line(&ctx, &project).starts_with("Assignable (TASKS.md owners, besides me): "));
    }
}

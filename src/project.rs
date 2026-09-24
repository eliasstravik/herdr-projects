//! Project folders under the root: slugs, settings, status, the per-project
//! lock and the coordinator record.

use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub const MAX_SLUG: usize = 40;
pub const BODY_WARN_CHARS: usize = 16_000;

/// A slug matches `[a-z0-9][a-z0-9-]*` and is at most 40 characters. Every
/// subcommand validates the slug it is given before building any path from it.
pub fn validate_slug(slug: &str) -> Result<()> {
    let mut chars = slug.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let rest_ok = chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !first_ok || !rest_ok || slug.len() > MAX_SLUG {
        bail!("`{slug}` is not a valid slug (lower-case letters, digits and hyphens, at most {MAX_SLUG} characters)");
    }
    Ok(())
}

/// Lower-cases and turns each run of other characters into one hyphen. Used for
/// project names and for thread titles in branch names.
pub fn slugify(text: &str) -> String {
    let mut slug = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            slug.push(c);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let mut slug: String = slug.chars().take(MAX_SLUG).collect();
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}

/// The slug `new` gives a project name, refusing names that look like paths.
pub fn slug_from_name(name: &str) -> Result<String> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        bail!("a project name may not contain `/`, `\\` or `..`");
    }
    let slug = slugify(name);
    if slug.is_empty() {
        bail!("`{name}` has no letters or digits to make a slug from");
    }
    validate_slug(&slug)?;
    Ok(slug)
}

/// Words split on `-` and `_`, each with its first letter upper-cased:
/// `herdr-projects` becomes `Herdr Projects`. Plain title case, so `gtm-ai`
/// becomes `Gtm Ai`; a user who wants `GTM AI` sets `name` in PROJECT.md.
pub fn humanize(slug: &str) -> String {
    slug.split(['-', '_'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map(|first| first.to_uppercase().chain(chars).collect::<String>()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The name a project shows: `name` as given unless it is empty or looks like
/// a slug (lower-case letters, digits, `-` and `_` only), else the humanized
/// form of it or of `slug`. A herdr workspace never shows a bare slug, which
/// would read the same as a repository's own workspace.
pub fn display_name(name: &str, slug: &str) -> String {
    let name = name.trim();
    let base = if name.is_empty() { slug } else { name };
    let slug_like = base.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if slug_like { humanize(base) } else { base.to_string() }
}

/// Writes through a temporary file in the same directory plus a rename. It never
/// creates parent directories: only `new` creates a project's directories.
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let dir = path.parent().context("path has no parent")?;
    let name = path.file_name().context("path has no file name")?;
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let result = (|| -> Result<()> {
        let mut file = File::create(&tmp)?;
        file.write_all(contents)?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.with_context(|| format!("could not write {}", path.display()))
}

pub fn now() -> String {
    jiff::Timestamp::now()
        .round(jiff::Unit::Second)
        .map(|t| t.to_string())
        .unwrap_or_default()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Repo {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine: Option<String>,
}

/// `PROJECT.md` front matter. `repos` is last so the TOML tables follow the
/// plain keys when `new` serializes it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub name: String,
    pub goal: String,
    pub coordinator_agent: String,
    pub thread_agent: String,
    pub max_parallel_threads: u32,
    pub auto_resolve_days: u32,
    pub nudge: bool,
    /// Silences every notification for the project except errors.
    pub mute: bool,
    pub repos: Vec<Repo>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            name: String::new(),
            goal: String::new(),
            coordinator_agent: "claude".into(),
            thread_agent: "claude".into(),
            max_parallel_threads: 3,
            auto_resolve_days: 7,
            // On by default (W15): the ticker prompts only a coordinator that
            // has been idle for a minute, because on herdr 0.9.1 a prompt
            // merges with half-typed text (docs/herdr-notes.md, stage 2).
            nudge: true,
            mute: false,
            repos: Vec::new(),
        }
    }
}

/// Splits `+++` TOML front matter from the body.
pub fn parse_project_md(text: &str) -> Result<(Settings, String)> {
    let rest = text
        .strip_prefix("+++\n")
        .context("PROJECT.md must start with a `+++` line")?;
    let (front, body) = match rest.split_once("\n+++\n") {
        Some(parts) => parts,
        None => rest
            .strip_suffix("\n+++")
            .map(|front| (front, ""))
            .context("PROJECT.md front matter has no closing `+++` line")?,
    };
    let settings: Settings = toml::from_str(front).context("PROJECT.md front matter does not parse")?;
    Ok((settings, body.trim_start_matches('\n').to_string()))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Active,
    Paused,
    Archived,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Status::Active => "active",
            Status::Paused => "paused",
            Status::Archived => "archived",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
struct ProjectState {
    status: Status,
}

/// The session and workspace the project belongs to, and the coordinator pane
/// `open` last started or focused. Any agent whose working directory is `cwd`
/// is a coordinator; the ticker lists them in `.state/coordinators.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Coordinator {
    pub socket: String,
    /// Empty when the session was chosen by socket path alone.
    pub session: String,
    pub workspace_id: String,
    pub tab_id: String,
    pub pane_id: String,
    pub agent_name: String,
    /// The canonical project folder.
    pub cwd: String,
    /// The Herdr agent kind `open` last started.
    pub agent: String,
    /// The last native session id Herdr reported for that kind, for resume.
    pub agent_session: String,
    pub updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Safety {
    pub start_threads: String,
    pub coordinator_agent_args: Vec<String>,
    pub thread_agent_args: Vec<String>,
    /// Launch flags for one agent kind, used instead of `thread_agent_args`
    /// for threads of that kind, with optional per-machine lists.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub thread_agents: std::collections::BTreeMap<String, KindArgs>,
    pub routine_commands: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct KindArgs {
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub machines: std::collections::BTreeMap<String, Vec<String>>,
}

impl Default for Safety {
    fn default() -> Self {
        Safety {
            start_threads: "propose".into(),
            coordinator_agent_args: Vec::new(),
            thread_agent_args: Vec::new(),
            thread_agents: std::collections::BTreeMap::new(),
            routine_commands: false,
        }
    }
}

impl Safety {
    /// The user's launch flags for a thread of `kind` on `machine` (empty for
    /// a local thread): that kind's list for the machine, else the kind's
    /// list, else `thread_agent_args`.
    pub fn thread_args_for(&self, kind: &str, machine: &str) -> Vec<String> {
        match self.thread_agents.get(kind) {
            Some(config) => config.machines.get(machine).unwrap_or(&config.args).clone(),
            None => self.thread_agent_args.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub slug: String,
}

/// Held while reading and rewriting anything under `threads/`, `inbox/` or
/// `.state/`. Never held across a herdr, git, gh, ssh or scp call.
pub struct ProjectLock {
    _file: File,
}

impl Project {
    /// An existing project. Validates the slug before building any path.
    pub fn load(root: &Path, slug: &str) -> Result<Project> {
        validate_slug(slug)?;
        let project = Project {
            root: root.to_path_buf(),
            slug: slug.to_string(),
        };
        if !project.project_md().is_file() {
            bail!("no project `{slug}` in {}", root.display());
        }
        Ok(project)
    }

    pub fn dir(&self) -> PathBuf {
        self.root.join(&self.slug)
    }

    pub fn project_md(&self) -> PathBuf {
        self.dir().join("PROJECT.md")
    }

    pub fn state_dir(&self) -> PathBuf {
        self.dir().join(".state")
    }

    /// The canonical folder (symlinks resolved): the key of the project's
    /// `[safety]` table and of its routine approvals.
    pub fn canonical_dir(&self) -> PathBuf {
        std::fs::canonicalize(self.dir()).unwrap_or_else(|_| self.dir())
    }

    /// Takes the per-project lock. The lock file is opened without creating
    /// parent directories, and the project is re-checked afterwards, so a
    /// `delete` that lands mid-operation cannot be resurrected by a writer.
    pub fn lock(&self) -> Result<ProjectLock> {
        let path = self.state_dir().join("lock");
        let file = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .with_context(|| format!("project `{}` is gone ({})", self.slug, path.display()))?;
        file.lock()?;
        if !self.project_md().is_file() {
            bail!("project `{}` is gone", self.slug);
        }
        Ok(ProjectLock { _file: file })
    }

    pub fn read_project_md(&self) -> Result<(Settings, String)> {
        let text = std::fs::read_to_string(self.project_md())
            .with_context(|| format!("could not read {}", self.project_md().display()))?;
        parse_project_md(&text)
    }

    pub fn status(&self) -> Status {
        read_json::<ProjectState>(&self.state_dir().join("project.json"))
            .unwrap_or_default()
            .status
    }

    pub fn set_status(&self, status: Status) -> Result<()> {
        let _lock = self.lock()?;
        write_json(&self.state_dir().join("project.json"), &ProjectState { status })
    }

    pub fn coordinator(&self) -> Option<Coordinator> {
        read_json(&self.state_dir().join("coordinator.json"))
    }

    /// Read-modify-write of `coordinator.json` under the lock: re-reads the
    /// file, lets `change` touch only the fields its step owns, writes.
    pub fn update_coordinator(&self, change: impl FnOnce(&mut Coordinator)) -> Result<Coordinator> {
        let _lock = self.lock()?;
        let mut record = self.coordinator().unwrap_or_default();
        change(&mut record);
        record.updated = now();
        write_json(&self.state_dir().join("coordinator.json"), &record)?;
        Ok(record)
    }

    pub fn safety(&self, config_dir: &Path) -> Result<Safety> {
        load_safety(config_dir, &self.canonical_dir())
    }
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    write_atomic(path, text.as_bytes())
}

/// The effective safety settings: `[safety."<canonical project path>"]` in
/// `<config_dir>/config.toml`, with defaults for an absent table or key.
pub fn load_safety(config_dir: &Path, canonical_project_dir: &Path) -> Result<Safety> {
    #[derive(Deserialize, Default)]
    struct Config {
        #[serde(default)]
        safety: std::collections::BTreeMap<String, Safety>,
    }
    let file = config_dir.join("config.toml");
    let Ok(text) = std::fs::read_to_string(&file) else {
        return Ok(Safety::default());
    };
    let mut config: Config =
        toml::from_str(&text).with_context(|| format!("{} does not parse", file.display()))?;
    let safety = config
        .safety
        .remove(&*canonical_project_dir.to_string_lossy())
        .unwrap_or_default();
    if !matches!(safety.start_threads.as_str(), "propose" | "auto") {
        bail!(
            "{}: start_threads must be \"propose\" or \"auto\", not {:?}",
            file.display(),
            safety.start_threads
        );
    }
    Ok(safety)
}

/// Slugs of the projects in `root`: folders that contain `PROJECT.md`. Entries
/// whose names start with a dot are ignored. A missing root has no projects.
pub fn list_slugs(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut slugs: Vec<String> = entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.') && validate_slug(name).is_ok())
        .filter(|name| root.join(name).join("PROJECT.md").is_file())
        .collect();
    slugs.sort();
    slugs
}

/// `PATH[@MACHINE]` as given to `new --repo`.
pub fn parse_repo_arg(arg: &str) -> Repo {
    if let Some((path, machine)) = arg.rsplit_once('@') {
        let label_like = !machine.is_empty()
            && machine
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
        if label_like && !path.is_empty() {
            return Repo {
                path: path.to_string(),
                machine: Some(machine.to_string()),
            };
        }
    }
    Repo {
        path: arg.to_string(),
        machine: None,
    }
}

const TASKS_TEMPLATE: &str = "# Tasks\n\n## Backlog\n";

const INSTRUCTIONS_TEMPLATE: &str = "\
# Instructions

Standing instructions for this project. Every thread starts from this text and
from the project's memory. Replace this paragraph with how you want work done:
conventions, what to check before finishing, what never to do. Ask the
coordinator to change it, or edit it here.

The settings above, between the `+++` lines, are changed from the projects
popup or by asking the coordinator.
";

/// The folders every project has. `uploads/` is yours (files for threads),
/// `library/` holds what threads produced.
pub const SUBDIRS: [&str; 9] = ["memory", "scratch", "routines", "threads", "inbox", "inbox/done", "library", "uploads", ".state"];

/// The text of `AGENTS.md`. Harnesses load it from every ancestor of their
/// working directory, and tab threads run under `threads/<id>/`, so it says
/// who is who by working directory. `prefix` is `<absolute binary> --root
/// <root>`: bare `hp` is on no harness's `PATH`.
pub fn agents_md(name: &str, slug: &str, prefix: &str) -> String {
    format!(
        "# {name}\n\n\
         This folder is the home of the Herdr project \"{name}\" (`{slug}`). Written by herdr-projects; `doctor --fix` refreshes it.\n\n\
         If your working directory is exactly this folder, you are the coordinator of {name}: run `{prefix} skill` now and follow what it prints, and run `{prefix} context {slug}` now and whenever you need project state.\n\n\
         If your working directory is under `threads/`, you are a thread: your brief is in your own folder (`.herdr-project/{slug}-<id>/brief.md`); ignore the rest of this file.\n"
    )
}

/// The command prefix `AGENTS.md` carries, so `doctor` can check that its
/// binary still exists.
pub fn prefix_in_agents_md(text: &str) -> Option<String> {
    let line = text.lines().find(|l| l.starts_with("If your working directory is exactly this folder"))?;
    let start = line.find('`')? + 1;
    let rest = &line[start..];
    let end = rest.find(" skill`")?;
    Some(rest[..end].to_string())
}

pub const PR_FOLLOWUP: &str = "routines/pr-followup.md";

const PR_FOLLOWUP_TEMPLATE: &str = "+++\non = \"pr\"\nevents = [\"checks-failed\", \"review\"]\nenabled = true\n+++\n\nFix the failing checks and address the new review comments on your pull request. Read them with `gh`, push the fixes, reply where a reviewer asked something, and then rewrite your report. If a comment asks for something outside your task, say so in the report instead of doing it.\n";

/// The ready-made `pr` routine (enabled by default; the popup or the
/// coordinator turns it off). Written by `new` and by `doctor --fix` when
/// missing.
pub fn write_default_routine(project: &Project) -> Result<bool> {
    let path = project.dir().join(PR_FOLLOWUP);
    if path.exists() {
        return Ok(false);
    }
    write_atomic(&path, PR_FOLLOWUP_TEMPLATE.as_bytes())?;
    Ok(true)
}

/// Writes `AGENTS.md`, `CLAUDE.md` (a relative symbolic link to it) and
/// creates `uploads/`. Idempotent; used by `new` and by `doctor --fix`.
pub fn write_priming(project: &Project, prefix: &str) -> Result<()> {
    let dir = project.dir();
    let (settings, _) = project.read_project_md()?;
    let name = display_name(&settings.name, &project.slug);
    let agents = dir.join("AGENTS.md");
    if let Ok(existing) = std::fs::read_to_string(&agents)
        && !existing.contains("Written by herdr-projects")
    {
        // Someone else's AGENTS.md: keep its text beside ours, once.
        let kept = dir.join("AGENTS.md.before-herdr-projects");
        if !kept.exists() {
            std::fs::rename(&agents, &kept)?;
        }
    }
    write_atomic(&agents, agents_md(&name, &project.slug, prefix).as_bytes())?;
    let claude = dir.join("CLAUDE.md");
    let link_ok = std::fs::read_link(&claude).is_ok_and(|target| target == Path::new("AGENTS.md"));
    if !link_ok {
        if std::fs::symlink_metadata(&claude).is_ok() {
            // A regular file or a link elsewhere: keep its text beside it, once.
            let kept = dir.join("CLAUDE.md.before-herdr-projects");
            if !kept.exists() {
                std::fs::rename(&claude, &kept)?;
            } else {
                std::fs::remove_file(&claude)?;
            }
        }
        std::os::unix::fs::symlink("AGENTS.md", &claude).with_context(|| format!("could not link {}", claude.display()))?;
    }
    if !dir.join("uploads").is_dir() {
        std::fs::create_dir(dir.join("uploads"))?;
    }
    write_default_routine(project)?;
    Ok(())
}

/// What `doctor` finds wrong with a project's priming files, as short notes.
pub fn priming_problems(project: &Project, prefix: &str) -> Vec<String> {
    let dir = project.dir();
    let mut problems = Vec::new();
    match std::fs::read_to_string(dir.join("AGENTS.md")) {
        Err(_) => problems.push("AGENTS.md is missing".into()),
        Ok(text) => match prefix_in_agents_md(&text) {
            None => problems.push("AGENTS.md does not name the binary".into()),
            Some(found) => {
                let binary = found.split(" --root ").next().unwrap_or("").trim_matches('\'');
                if !Path::new(binary).is_file() {
                    problems.push(format!("AGENTS.md points at a binary that does not exist ({binary})"));
                } else if found != prefix {
                    problems.push("AGENTS.md names another binary or root than this one".into());
                }
            }
        },
    }
    if !std::fs::read_link(dir.join("CLAUDE.md")).is_ok_and(|t| t == Path::new("AGENTS.md")) {
        problems.push("CLAUDE.md is not a link to AGENTS.md".into());
    }
    if !dir.join("uploads").is_dir() {
        problems.push("uploads/ is missing".into());
    }
    if !dir.join(PR_FOLLOWUP).exists() {
        problems.push("routines/pr-followup.md is missing".into());
    }
    problems
}

/// Creates the folder and skeleton files. The only code path that creates a
/// project's directories. Fails if the slug exists.
pub fn create(root: &Path, name: &str, goal: &str, repos: Vec<Repo>) -> Result<Project> {
    let slug = slug_from_name(name)?;
    let project = Project {
        root: root.to_path_buf(),
        slug: slug.clone(),
    };
    let dir = project.dir();
    if dir.exists() {
        bail!("`{slug}` already exists in {}", root.display());
    }
    let repos = repos
        .into_iter()
        .map(|repo| match repo.machine {
            // A remote path is stored as it is on its own machine.
            Some(_) => repo,
            None => Repo {
                path: std::fs::canonicalize(&repo.path)
                    .or_else(|_| std::path::absolute(&repo.path))
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or(repo.path),
                machine: None,
            },
        })
        .collect();
    let settings = Settings {
        name: display_name(name, &slug),
        goal: goal.to_string(),
        repos,
        ..Settings::default()
    };
    let front = toml::to_string(&settings)?;

    std::fs::create_dir_all(root)?;
    std::fs::create_dir(&dir).with_context(|| format!("could not create {}", dir.display()))?;
    for sub in SUBDIRS {
        std::fs::create_dir_all(dir.join(sub))?;
    }
    write_atomic(
        &dir.join("MEMORY.md"),
        b"# Memory\n\nOne line per memory file: `- [title](memory/file.md): what it holds`.\n",
    )?;
    write_atomic(&dir.join("TASKS.md"), TASKS_TEMPLATE.as_bytes())?;
    write_atomic(&dir.join(PR_FOLLOWUP), PR_FOLLOWUP_TEMPLATE.as_bytes())?;
    write_json(&project.state_dir().join("project.json"), &ProjectState::default())?;
    // PROJECT.md last: a folder without it is not a project, so a half-made
    // skeleton is never picked up by `list` or the ticker.
    write_atomic(
        &project.project_md(),
        format!("+++\n{front}+++\n\n{INSTRUCTIONS_TEMPLATE}").as_bytes(),
    )?;
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_folders_with_project_md_count() {
        let root = tempfile::tempdir().unwrap();
        for name in ["b", "a", ".trash", "empty", "Not_A_Slug"] {
            std::fs::create_dir(root.path().join(name)).unwrap();
        }
        for name in ["b", "a", ".trash", "Not_A_Slug"] {
            std::fs::write(root.path().join(name).join("PROJECT.md"), "").unwrap();
        }
        assert_eq!(list_slugs(root.path()), ["a", "b"]);
        assert!(list_slugs(&root.path().join("missing")).is_empty());
    }

    #[test]
    fn slug_validation() {
        for good in ["a", "demo", "demo-2", "0x", &"a".repeat(40)] {
            assert!(validate_slug(good).is_ok(), "{good}");
        }
        for bad in ["", "-a", "A", "a_b", "a/b", "../x", "a b", ".", "..", &"a".repeat(41)] {
            assert!(validate_slug(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_slug_like_name_is_humanized_and_a_typed_name_is_kept() {
        assert_eq!(humanize("herdr-projects"), "Herdr Projects");
        assert_eq!(humanize("gtm_ai"), "Gtm Ai");
        assert_eq!(humanize("-v2--api-"), "V2 Api");
        assert_eq!(display_name("herdr-projects", "herdr-projects"), "Herdr Projects");
        assert_eq!(display_name("", "herdr-projects"), "Herdr Projects");
        assert_eq!(display_name("  ", "demo"), "Demo");
        for typed in ["GTM AI", "my project", "Demo", "herdr-Projects"] {
            assert_eq!(display_name(typed, "x"), typed);
        }
    }

    #[test]
    fn create_stores_a_display_name_and_keeps_the_slug() {
        let root = tempfile::tempdir().unwrap();
        let project = create(root.path(), "herdr-projects", "", vec![]).unwrap();
        assert_eq!(project.slug, "herdr-projects");
        assert_eq!(project.read_project_md().unwrap().0.name, "Herdr Projects");
        let project = create(root.path(), "GTM AI", "", vec![]).unwrap();
        assert_eq!(project.slug, "gtm-ai");
        assert_eq!(project.read_project_md().unwrap().0.name, "GTM AI");
    }

    #[test]
    fn slug_derivation_and_name_refusals() {
        assert_eq!(slug_from_name("My Demo  Project!").unwrap(), "my-demo-project");
        assert_eq!(slug_from_name("  Ünï 42 ").unwrap(), "n-42");
        assert_eq!(slug_from_name(&"x".repeat(60)).unwrap().len(), 40);
        for bad in ["../x", "a/b", "a\\b", "..", "!!!", ""] {
            assert!(slug_from_name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn create_writes_the_skeleton_and_refuses_a_second_time() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().join("root");
        let project = create(
            &root,
            "Demo",
            "Ship \"it\"",
            vec![parse_repo_arg("/srv/app@box"), parse_repo_arg("/no/such/repo")],
        )
        .unwrap();
        assert_eq!(project.slug, "demo");
        for sub in ["memory", "scratch", "routines", "threads", "inbox/done", "library", "uploads", ".state"] {
            assert!(project.dir().join(sub).is_dir(), "{sub}");
        }
        assert!(project.dir().join("MEMORY.md").is_file());
        assert!(project.dir().join("TASKS.md").is_file());
        let (settings, body) = project.read_project_md().unwrap();
        assert_eq!(settings.name, "Demo");
        assert_eq!(settings.goal, "Ship \"it\"");
        assert_eq!(settings.coordinator_agent, "claude");
        assert_eq!(settings.max_parallel_threads, 3);
        assert_eq!(settings.auto_resolve_days, 7);
        assert!(settings.nudge);
        assert!(project.dir().join(PR_FOLLOWUP).is_file());
        assert!(crate::routine::load_all(&project).1.is_empty(), "the default routine parses");
        assert_eq!(
            settings.repos,
            vec![
                Repo { path: "/srv/app".into(), machine: Some("box".into()) },
                Repo { path: "/no/such/repo".into(), machine: None },
            ]
        );
        assert!(body.starts_with("# Instructions"));
        assert_eq!(project.status(), Status::Active);
        assert!(create(&root, "demo", "", vec![]).is_err());
    }

    #[test]
    fn priming_files_are_written_linked_and_checked() {
        let root = tempfile::tempdir().unwrap();
        let project = create(root.path(), "Demo Project", "", vec![]).unwrap();
        let prefix = format!("{} --root {}", std::env::current_exe().unwrap().display(), root.path().display());
        write_priming(&project, &prefix).unwrap();
        let text = std::fs::read_to_string(project.dir().join("AGENTS.md")).unwrap();
        assert!(text.contains("you are the coordinator of Demo Project"));
        assert!(text.contains(&format!("`{prefix} skill`")));
        assert!(text.contains(&format!("`{prefix} context demo-project`")));
        assert!(text.contains("under `threads/`, you are a thread"));
        assert_eq!(prefix_in_agents_md(&text).as_deref(), Some(prefix.as_str()));
        assert_eq!(std::fs::read_link(project.dir().join("CLAUDE.md")).unwrap(), Path::new("AGENTS.md"));
        assert!(project.dir().join("uploads").is_dir());
        assert!(priming_problems(&project, &prefix).is_empty());

        // Idempotent, and a stale binary path is reported.
        write_priming(&project, &prefix).unwrap();
        let stale = agents_md("Demo Project", "demo-project", "/no/such/binary --root /r");
        std::fs::write(project.dir().join("AGENTS.md"), stale).unwrap();
        let problems = priming_problems(&project, &prefix);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("does not exist"));
        write_priming(&project, &prefix).unwrap();
        assert!(priming_problems(&project, &prefix).is_empty());

        // A foreign AGENTS.md is kept beside ours.
        std::fs::write(project.dir().join("AGENTS.md"), "codex notes").unwrap();
        write_priming(&project, &prefix).unwrap();
        assert_eq!(std::fs::read_to_string(project.dir().join("AGENTS.md.before-herdr-projects")).unwrap(), "codex notes");
        // A hand-written CLAUDE.md is kept beside the link, not lost.
        std::fs::remove_file(project.dir().join("CLAUDE.md")).unwrap();
        std::fs::write(project.dir().join("CLAUDE.md"), "mine").unwrap();
        assert!(priming_problems(&project, &prefix).iter().any(|p| p.contains("CLAUDE.md")));
        write_priming(&project, &prefix).unwrap();
        assert_eq!(std::fs::read_to_string(project.dir().join("CLAUDE.md.before-herdr-projects")).unwrap(), "mine");
        assert!(priming_problems(&project, &prefix).is_empty());
    }

    #[test]
    fn front_matter_parsing() {
        let (settings, body) =
            parse_project_md("+++\nname = \"X\"\nnudge = true\n+++\n\nBody\n+++\nmore\n").unwrap();
        assert_eq!(settings.name, "X");
        assert!(settings.nudge);
        assert_eq!(settings.thread_agent, "claude");
        assert_eq!(body, "Body\n+++\nmore\n");
        assert!(parse_project_md("no front matter").is_err());
        assert!(parse_project_md("+++\nname = \n+++\n").is_err());
        assert!(parse_project_md("+++\nname = \"X\"\n").is_err());
        let (_, body) = parse_project_md("+++\nname = \"X\"\n+++").unwrap();
        assert_eq!(body, "");
    }

    #[test]
    fn repo_arg_parsing() {
        assert_eq!(parse_repo_arg("/a/b").machine, None);
        assert_eq!(parse_repo_arg("/a/b@m1").machine.as_deref(), Some("m1"));
        assert_eq!(parse_repo_arg("/a/b@m1").path, "/a/b");
        // An `@` inside a path is not a machine.
        assert_eq!(parse_repo_arg("/a@b/c").machine, None);
        assert_eq!(parse_repo_arg("/a@b/c").path, "/a@b/c");
    }

    #[test]
    fn thread_launch_flags_can_be_set_per_kind_and_per_machine() {
        let config = tempfile::tempdir().unwrap();
        std::fs::write(
            config.path().join("config.toml"),
            "[safety.\"/projects/demo\"]\nthread_agent_args = [\"--dangerously-skip-permissions\"]\n\
             [safety.\"/projects/demo\".thread_agents.codex]\nargs = [\"--dangerously-bypass-approvals-and-sandbox\"]\n\
             [safety.\"/projects/demo\".thread_agents.codex.machines]\nlaptop = [\"--yolo\"]\n",
        )
        .unwrap();
        let safety = load_safety(config.path(), Path::new("/projects/demo")).unwrap();
        // A listed kind uses its own list, per machine where one is given.
        assert_eq!(safety.thread_args_for("codex", ""), ["--dangerously-bypass-approvals-and-sandbox"]);
        assert_eq!(safety.thread_args_for("codex", "laptop"), ["--yolo"]);
        assert_eq!(safety.thread_args_for("codex", "other"), ["--dangerously-bypass-approvals-and-sandbox"]);
        // Any other kind keeps thread_agent_args, as before.
        assert_eq!(safety.thread_args_for("claude", ""), ["--dangerously-skip-permissions"]);
        assert_eq!(safety.thread_args_for("opencode", "laptop"), ["--dangerously-skip-permissions"]);
        // With no tables at all nothing changes.
        assert_eq!(Safety::default().thread_args_for("codex", ""), Vec::<String>::new());
    }

    #[test]
    fn safety_defaults_and_overrides_keyed_by_canonical_path() {
        let config = tempfile::tempdir().unwrap();
        let here = Path::new("/projects/demo");
        assert_eq!(load_safety(config.path(), here).unwrap(), Safety::default());

        std::fs::write(
            config.path().join("config.toml"),
            "root = \"/projects\"\n\n[safety.\"/projects/demo\"]\nstart_threads = \"auto\"\nthread_agent_args = [\"--x\"]\n",
        )
        .unwrap();
        let safety = load_safety(config.path(), here).unwrap();
        assert_eq!(safety.start_threads, "auto");
        assert_eq!(safety.thread_agent_args, ["--x"]);
        assert!(!safety.routine_commands);
        assert!(safety.coordinator_agent_args.is_empty());
        assert_eq!(
            load_safety(config.path(), Path::new("/projects/other")).unwrap(),
            Safety::default()
        );

        std::fs::write(
            config.path().join("config.toml"),
            "[safety.\"/projects/demo\"]\nstart_threads = \"yolo\"\n",
        )
        .unwrap();
        assert!(load_safety(config.path(), here).is_err());
    }

    #[test]
    fn writers_drop_their_write_when_project_md_is_gone() {
        let root = tempfile::tempdir().unwrap();
        let project = create(root.path(), "demo", "", vec![]).unwrap();
        std::fs::remove_file(project.project_md()).unwrap();
        assert!(project.update_coordinator(|c| c.pane_id = "w1:p1".into()).is_err());
        assert!(project.coordinator().is_none());

        // A deleted folder is not recreated by taking the lock.
        std::fs::remove_dir_all(project.dir()).unwrap();
        assert!(project.lock().is_err());
        assert!(!project.dir().exists());
    }

    #[test]
    fn coordinator_updates_keep_other_fields() {
        let root = tempfile::tempdir().unwrap();
        let project = create(root.path(), "demo", "", vec![]).unwrap();
        project.update_coordinator(|c| c.socket = "/s".into()).unwrap();
        project.update_coordinator(|c| c.agent_session = "sess".into()).unwrap();
        let record = project.coordinator().unwrap();
        assert_eq!(record.socket, "/s");
        assert_eq!(record.agent_session, "sess");
        assert!(std::fs::read_dir(project.state_dir())
            .unwrap()
            .flatten()
            .all(|e| !e.file_name().to_string_lossy().ends_with(".tmp")));
    }
}

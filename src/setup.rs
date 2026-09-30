//! `configure` and `unconfigure`: the edits the plugin makes to the user's
//! files, each recorded in an ownership journal so `unconfigure` restores
//! exactly what `configure` changed. Hook files are edited as JSONC through
//! a concrete syntax tree, so comments and formatting survive.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use jsonc_parser::cst::{CstInputValue, CstRootNode};
use serde::{Deserialize, Serialize};

use crate::paths::{Ctx, Env};
use crate::remote::quote;

/// A harness with a native, user-level hook system or extension folder that
/// can put text in the model's context. Every other agent learns to report
/// from its thread brief or the coordinator skill alone; adding a harness
/// here is one entry.
pub struct Harness {
    /// Herdr's agent kind, which is also the `hook --agent` value.
    pub agent: &'static str,
    /// The variable that moves the harness's config directory, if any.
    home_env: Option<&'static str>,
    /// The config directory under the home directory.
    home: &'static str,
    /// The hook file, relative to the config directory.
    file: &'static str,
    /// The harness's own names for SessionStart, UserPromptSubmit and PostToolUse.
    pub events: [&'static str; 3],
    /// What the plugin writes into the hook file.
    format: Format,
    /// The hook timeout in the harness's unit.
    timeout: u64,
    /// The injected text goes in top-level `additionalContext`, not `hookSpecificOutput`.
    pub top_level_output: bool,
}

/// The shape of a harness's hook file.
#[derive(PartialEq)]
enum Format {
    /// Claude Code's `{matcher, hooks: [...]}` entries in a shared JSON file.
    Nested,
    /// Flat `{type, command, timeoutSec}` entries in a `version: 1` file of our
    /// own (Copilot CLI).
    Flat,
    /// A TypeScript extension file of our own (Pi, OMP) that runs the hook
    /// command with Claude Code's event names and payload.
    Extension,
}

/// The extension Pi and OMP load; they share its events and result shapes.
const EXTENSION: &str = include_str!("../assets/extensions/herdr-projects.ts");
/// The first line of [`EXTENSION`], which marks an extension file as ours.
const EXTENSION_MARKER: &str = "// herdr-projects: progress extension";

pub const HARNESSES: [Harness; 7] = [
    Harness { agent: "claude", home_env: Some("CLAUDE_CONFIG_DIR"), home: ".claude", file: "settings.json", events: ["SessionStart", "UserPromptSubmit", "PostToolUse"], format: Format::Nested, timeout: 10, top_level_output: false },
    Harness { agent: "codex", home_env: Some("CODEX_HOME"), home: ".codex", file: "hooks.json", events: ["SessionStart", "UserPromptSubmit", "PostToolUse"], format: Format::Nested, timeout: 10, top_level_output: false },
    // Factory Droid: Claude Code's format, in its settings.json.
    Harness { agent: "droid", home_env: None, home: ".factory", file: "settings.json", events: ["SessionStart", "UserPromptSubmit", "PostToolUse"], format: Format::Nested, timeout: 10, top_level_output: false },
    // Gemini CLI: its own event names; timeouts in milliseconds.
    Harness { agent: "gemini", home_env: None, home: ".gemini", file: "settings.json", events: ["SessionStart", "BeforeAgent", "AfterTool"], format: Format::Nested, timeout: 10_000, top_level_output: false },
    // Copilot CLI reads every file in hooks/, so ours is a file of its own.
    // PascalCase event names select its Claude-style payload (snake_case,
    // `hook_event_name`); prompt-submit output is dropped, but the event
    // still clears an answered question.
    Harness { agent: "copilot", home_env: Some("COPILOT_HOME"), home: ".copilot", file: "hooks/herdr-projects.json", events: ["SessionStart", "UserPromptSubmit", "PostToolUse"], format: Format::Flat, timeout: 10, top_level_output: true },
    // Pi and OMP load every extension in agent/extensions/, so ours is a file of its own.
    Harness { agent: "pi", home_env: None, home: ".pi", file: "agent/extensions/herdr-projects.ts", events: ["SessionStart", "UserPromptSubmit", "PostToolUse"], format: Format::Extension, timeout: 10, top_level_output: true },
    Harness { agent: "omp", home_env: None, home: ".omp", file: "agent/extensions/herdr-projects.ts", events: ["SessionStart", "UserPromptSubmit", "PostToolUse"], format: Format::Extension, timeout: 10, top_level_output: true },
];

pub const AGENTS: [&str; 7] = ["claude", "codex", "droid", "gemini", "copilot", "pi", "omp"];

pub fn harness(agent: &str) -> Option<&'static Harness> {
    HARNESSES.iter().find(|h| h.agent == agent)
}

/// The harness a hook command was written for (`… hook --agent <name> …`).
fn harness_of(command: &str) -> &'static Harness {
    let agent = command.split(" hook --agent ").nth(1).and_then(|rest| rest.split_whitespace().next()).unwrap_or("claude");
    harness(agent).unwrap_or(&HARNESSES[0])
}

/// One file the plugin edited: its text before the first edit, after the last
/// one, what kind of edit, and the hook command (for hook files).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Owned {
    pub before: Option<String>,
    pub after: String,
    pub kind: String,
    #[serde(default)]
    pub command: Option<String>,
}

pub type Journal = BTreeMap<String, Owned>;

pub fn journal_path(config_dir: &Path) -> PathBuf {
    config_dir.join("owned.json")
}

pub fn load_journal(config_dir: &Path) -> Journal {
    crate::project::read_json(&journal_path(config_dir)).unwrap_or_default()
}

pub fn save_journal(config_dir: &Path, journal: &Journal) -> Result<()> {
    std::fs::create_dir_all(config_dir)?;
    crate::project::write_json(&journal_path(config_dir), journal)
}

/// Reads a config file, refusing a file that is a symbolic link (a dotfile
/// manager's link would be replaced by a plain file) rather than editing it.
pub fn read(path: &Path) -> Result<Option<String>> {
    if let Ok(m) = std::fs::symlink_metadata(path) {
        ensure!(!m.file_type().is_symlink(), "refusing to edit {}: it is a symbolic link; edit its target's hooks by hand or pass --claude-home/--codex-home", path.display());
    }
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Replaces a file's text only if it still reads as `before`.
pub fn replace(path: &Path, before: &Option<String>, after: &str) -> Result<()> {
    ensure!(&read(path)? == before, "{} changed while configuring; run the command again", path.display());
    std::fs::create_dir_all(path.parent().context("config path has no parent")?)?;
    let tmp = path.with_file_name(format!(".herdr-projects-{}.tmp", std::process::id()));
    std::fs::write(&tmp, after)?;
    if path.exists() {
        std::fs::set_permissions(&tmp, std::fs::metadata(path)?.permissions())?;
    }
    if &read(path)? != before {
        let _ = std::fs::remove_file(&tmp);
        bail!("{} changed while configuring; run the command again", path.display());
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// The removal baseline after a repeated `configure`: the original text when
/// nothing else changed in between, else the current text with our entries
/// taken out, so a later `unconfigure` keeps edits made since.
pub fn removal_baseline(previous: &Owned, current: &Owned) -> Result<Option<String>> {
    if current.before.as_ref() == Some(&previous.after) {
        return Ok(previous.before.clone());
    }
    current
        .before
        .as_deref()
        .map(|text| remove_ours(&current.kind, text, current.command.as_deref()))
        .transpose()
}

/// A file's text with only this plugin's entries taken out.
fn remove_ours(kind: &str, text: &str, command: Option<&str>) -> Result<String> {
    match kind {
        "hooks" => hooks(text, command.context("missing hook command")?, true),
        "config" => crate::sidebar::config_edit(text, &crate::sidebar::Spec { key: String::new(), tab_command: command.unwrap_or("").to_string() }, true),
        other => bail!("unknown ownership kind {other}"),
    }
}

/// Herdr's config file: `HERDR_CONFIG_PATH`, else `$XDG_CONFIG_HOME/herdr`,
/// else `~/.config/herdr/config.toml`.
pub fn herdr_config_path(env: &Env) -> PathBuf {
    if let Some(path) = env.var("HERDR_CONFIG_PATH") {
        return PathBuf::from(path);
    }
    env.var("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| env.home.join(".config")).join("herdr/config.toml")
}

/// The tab-bar command: absolute paths, since it runs under `/bin/sh -lc` on
/// the server with no plugin environment.
pub fn tab_command(binary: &Path, root: &Path) -> String {
    format!("{} --root {} needs-you --line", quote(&binary.to_string_lossy()), quote(&root.to_string_lossy()))
}

fn hook_entry(harness: &Harness, command: &str) -> serde_json::Value {
    if harness.format == Format::Flat {
        serde_json::json!({"type":"command","command":command,"timeoutSec":harness.timeout})
    } else {
        serde_json::json!({"matcher":"*","hooks":[{"type":"command","command":command,"timeout":harness.timeout}]})
    }
}

fn cst_value(value: &serde_json::Value) -> CstInputValue {
    match value {
        serde_json::Value::Object(map) => CstInputValue::Object(map.iter().map(|(k, v)| (k.clone(), cst_value(v))).collect()),
        serde_json::Value::Array(items) => CstInputValue::Array(items.iter().map(cst_value).collect()),
        serde_json::Value::String(text) => text.as_str().into(),
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(0).into(),
        serde_json::Value::Bool(b) => (*b).into(),
        serde_json::Value::Null => CstInputValue::Null,
    }
}

/// Whether a hook entry is one of ours: it runs `herdr-projects … hook`.
fn is_our_entry(value: &serde_json::Value) -> bool {
    let ours = |h: &serde_json::Value| h["command"].as_str().is_some_and(|c| c.contains("herdr-projects") && c.contains(" hook --agent "));
    ours(value) || value["hooks"].as_array().is_some_and(|hooks| hooks.iter().any(ours))
}

/// Adds (or removes) the plugin's hook entry under each event, keeping
/// everything else, including comments. Any earlier entry of ours (a moved
/// binary) is replaced on add. Idempotent.
pub fn hooks(input: &str, command: &str, remove: bool) -> Result<String> {
    let harness = harness_of(command);
    let root = CstRootNode::parse(input, &Default::default()).context("hook file does not parse")?;
    let obj = root.object_value().context("hook configuration must be a JSON object")?;
    if harness.format == Format::Flat && !remove && obj.get("version").is_none() {
        obj.append("version", 1u64.into());
    }
    let hooks = match obj.get("hooks") {
        Some(p) => p.object_value().context("`hooks` must be an object")?,
        None if remove => return Ok(input.into()),
        None => obj.append("hooks", CstInputValue::Object(vec![])).object_value().unwrap(),
    };
    let expected = hook_entry(harness, command);
    for event in harness.events {
        let entries = match hooks.get(event) {
            Some(p) => p.array_value().with_context(|| format!("`hooks.{event}` must be an array"))?,
            None if remove => continue,
            None => hooks.append(event, CstInputValue::Array(vec![])).array_value().unwrap(),
        };
        let mut found = false;
        for entry in entries.elements() {
            let value = entry.to_serde_value();
            if value.as_ref() == Some(&expected) {
                if remove {
                    entry.remove();
                } else {
                    found = true;
                }
            } else if value.as_ref().is_some_and(is_our_entry) {
                // Ours, but with another command (the binary moved): replaced.
                entry.remove();
            }
        }
        if !remove && !found {
            entries.append(cst_value(&expected));
        }
    }
    Ok(root.to_string())
}

/// The extension file that runs `command`.
fn extension(command: &str) -> String {
    EXTENSION.replacen("\"__HOOK_COMMAND__\"", &serde_json::Value::from(command).to_string(), 1)
}

/// Why an existing hook file must be left alone, if it must. A shared JSON
/// file never is (only our entries change). An extension file is ours to
/// rewrite while it still reads as we last wrote it (`written`, from the
/// journal), or, unjournaled, while it carries our marker.
fn left_alone(harness: &Harness, text: Option<&str>, written: Option<&str>) -> Option<&'static str> {
    let text = text.filter(|_| harness.format == Format::Extension)?;
    match written {
        Some(written) if written != text => Some("edited since configure"),
        None if !text.starts_with(EXTENSION_MARKER) => Some("not this plugin's extension"),
        _ => None,
    }
}

/// A hook file's text with the plugin's hooks for `command` in place.
pub fn with_hooks(agent: &str, text: Option<&str>, command: &str) -> Result<String> {
    match harness(agent).map(|h| &h.format) {
        Some(Format::Extension) => Ok(extension(command)),
        _ => hooks(text.unwrap_or("{}"), command, false),
    }
}

/// Whether a hook file runs `command` as this release writes it: a JSON file
/// that has the command, an extension file that is exactly this release's.
pub fn hooks_current(agent: &str, text: &str, command: &str) -> bool {
    match harness(agent).map(|h| &h.format) {
        Some(Format::Extension) => text == extension(command),
        _ => text.contains(command),
    }
}

/// The ownership kind a harness's hook file is journaled as.
fn kind(harness: &Harness) -> &'static str {
    if harness.format == Format::Extension { "extension" } else { "hooks" }
}

/// The hook command for a harness: the absolute binary path and the root,
/// because hooks run outside the plugin environment.
/// It always exits 0 and never writes to standard error: harnesses treat a
/// failing UserPromptSubmit hook (exit 2) as "block this prompt", in every
/// session on the machine, so a missing or older binary must not do that.
pub fn hook_command(binary: &Path, root: &Path, agent: &str) -> String {
    format!("{} --root {} hook --agent {agent} 2>/dev/null || true", quote(&binary.to_string_lossy()), quote(&root.to_string_lossy()))
}

/// Where each harness keeps its hooks; `--claude-home`/`--codex-home`
/// override those two.
pub fn hook_file(env: &Env, agent: &str, claude_home: Option<&Path>, codex_home: Option<&Path>) -> PathBuf {
    let harness = harness(agent).unwrap_or(&HARNESSES[0]);
    let flag = match agent {
        "claude" => claude_home,
        "codex" => codex_home,
        _ => None,
    };
    flag.map(Path::to_path_buf)
        .or_else(|| harness.home_env.and_then(|name| env.var(name)).map(PathBuf::from))
        .unwrap_or_else(|| env.home.join(harness.home))
        .join(harness.file)
}

/// The harness's config directory: `configure` without `--clients` picks the
/// harnesses whose directory exists.
pub fn harness_installed(env: &Env, agent: &str, claude_home: Option<&Path>, codex_home: Option<&Path>) -> bool {
    let file = hook_file(env, agent, claude_home, codex_home);
    let depth = harness(agent).map_or(1, |h| h.file.split('/').count());
    file.ancestors().nth(depth).is_some_and(Path::is_dir)
}

/// The skill bundled with the plugin, linked into each harness by `configure`.
pub const SKILL: &str = "autoproject";

/// The bundled skill in the plugin checkout this binary was built in, so the
/// link follows the installed plugin, not the directory `configure` ran in.
pub fn skill_source() -> Option<PathBuf> {
    crate::update::own_root().map(|root| root.join("skill").join(SKILL))
}

/// Where a harness looks for user skills: Claude Code's `<config dir>/skills`,
/// Codex's user scope `~/.agents/skills` (not under `CODEX_HOME`). A skills
/// directory that is itself a link is resolved, so a shared directory gets
/// one link and one journal key; a missing one is resolved through its
/// parent, so the key stays the same once it exists.
pub fn skill_link(env: &Env, agent: &str, claude_home: Option<&Path>) -> PathBuf {
    let dir = match agent {
        "claude" => claude_home
            .map(Path::to_path_buf)
            .or_else(|| env.var("CLAUDE_CONFIG_DIR").map(PathBuf::from))
            .unwrap_or_else(|| env.home.join(".claude"))
            .join("skills"),
        _ => env.home.join(".agents/skills"),
    };
    let resolved = std::fs::canonicalize(&dir).or_else(|_| std::fs::canonicalize(dir.parent().unwrap_or(&dir)).map(|p| p.join("skills")));
    resolved.unwrap_or(dir).join(SKILL)
}

#[derive(Debug, PartialEq)]
pub enum SkillState {
    /// A link to `source`.
    Ours,
    Missing,
    /// A link to somewhere else: ours from an older checkout when journaled.
    Elsewhere(PathBuf),
    /// A directory or file: never touched.
    Foreign,
}

pub fn skill_state(link: &Path, source: &Path) -> SkillState {
    let Ok(meta) = std::fs::symlink_metadata(link) else {
        return SkillState::Missing;
    };
    if !meta.file_type().is_symlink() {
        return SkillState::Foreign;
    }
    match std::fs::read_link(link) {
        Ok(target) if target == source => SkillState::Ours,
        Ok(target) => SkillState::Elsewhere(target),
        Err(_) => SkillState::Foreign,
    }
}

pub struct ConfigureOptions {
    /// Harnesses from [`AGENTS`]; empty means every one whose config
    /// directory exists.
    pub clients: Vec<String>,
    pub claude_home: Option<PathBuf>,
    pub codex_home: Option<PathBuf>,
    pub dry_run: bool,
    /// Install the progress hooks; `false` links only the skill (`doctor --fix`).
    pub hooks: bool,
    /// Also edit Herdr's config.toml: sidebar rows, popup key, tab-bar entry.
    pub sidebar: bool,
    /// The popup key (default: the one already configured, else `prefix+a`).
    pub key: Option<String>,
    pub herdr_config: Option<PathBuf>,
    /// The skill directory to link (`skill_source()`); `None` links nothing.
    pub skill: Option<PathBuf>,
}

/// Whether the standalone agent-progress plugin's hooks are installed in a
/// hook file: `doctor` tells the user to remove them with that plugin's own
/// `unconfigure`, since this plugin never edits another plugin's entries.
pub fn has_agent_progress_hooks(text: &str) -> bool {
    text.contains("herdr-progress") && text.contains(" hook --agent ")
}

/// Installs the hooks. Every edit is journaled before it is made, so a killed
/// run never leaves hooks `unconfigure` cannot identify as its own.
pub fn configure(ctx: &Ctx, options: &ConfigureOptions) -> Result<Vec<String>> {
    let binary = crate::paths::binary()?;
    let clients: Vec<String> = if options.clients.is_empty() {
        AGENTS
            .into_iter()
            .filter(|c| harness_installed(ctx.env, c, options.claude_home.as_deref(), options.codex_home.as_deref()))
            .map(str::to_owned)
            .collect()
    } else {
        options.clients.clone()
    };
    let mut journal = load_journal(&ctx.config_dir);
    let mut edits: Vec<(PathBuf, Owned)> = Vec::new();
    let mut notes = Vec::new();
    for (client, harness) in clients.iter().filter(|_| options.hooks).filter_map(|c| Some((c, harness(c)?))) {
        let file = hook_file(ctx.env, client, options.claude_home.as_deref(), options.codex_home.as_deref());
        let command = hook_command(&binary, &ctx.root, client);
        let before = read(&file)?;
        let written = journal.get(&*file.to_string_lossy()).filter(|o| o.kind == "extension").map(|o| o.after.as_str());
        if let Some(reason) = left_alone(harness, before.as_deref(), written) {
            notes.push(format!("{}: left alone, {reason}; move it away and run `configure` again", file.display()));
            continue;
        }
        let after = with_hooks(client, before.as_deref(), &command)?;
        if before.as_deref().is_some_and(has_agent_progress_hooks) {
            notes.push(format!("{} also runs the standalone agent-progress hooks; run that plugin's `unconfigure` (see `doctor`) so only one set fires", file.display()));
        }
        if before.as_deref() == Some(after.as_str()) {
            notes.push(format!("{}: hooks already in place", file.display()));
            continue;
        }
        notes.push(format!("{}: {} hook entries for `{command}`", file.display(), if before.is_some() { "adding" } else { "creating with" }));
        edits.push((file, Owned { before, after, kind: kind(harness).into(), command: Some(command) }));
    }
    let mut links: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    if let Some(source) = &options.skill {
        let mut seen = Vec::new();
        for client in clients.iter().filter(|c| matches!(c.as_str(), "claude" | "codex")) {
            let link = skill_link(ctx.env, client, options.claude_home.as_deref());
            if seen.contains(&link) {
                continue;
            }
            seen.push(link.clone());
            if !source.join("SKILL.md").is_file() {
                notes.push(format!("{}: no bundled skill at {}; not linked", link.display(), source.display()));
                break;
            }
            let journaled = journal.get(&link.to_string_lossy().into_owned()).is_some_and(|o| o.kind == "skill");
            match skill_state(&link, source) {
                SkillState::Ours => {
                    notes.push(format!("{}: skill link already in place", link.display()));
                    if !journaled {
                        links.push((link, None));
                    }
                }
                SkillState::Missing => {
                    notes.push(format!("{}: linking the `{SKILL}` skill to {}", link.display(), source.display()));
                    links.push((link, Some(source.clone())));
                }
                SkillState::Elsewhere(old) if journaled => {
                    notes.push(format!("{}: relinking the `{SKILL}` skill from {} to {}", link.display(), old.display(), source.display()));
                    links.push((link, Some(source.clone())));
                }
                SkillState::Elsewhere(_) | SkillState::Foreign => {
                    notes.push(format!("{}: left alone, it is not this plugin's link; move it away and run `configure` again to install the bundled `{SKILL}` skill", link.display()));
                }
            }
        }
    }
    if options.sidebar {
        let file = options.herdr_config.clone().unwrap_or_else(|| herdr_config_path(ctx.env));
        let before = read(&file)?;
        let text = before.clone().unwrap_or_default();
        let current_key = text
            .parse::<toml_edit::DocumentMut>()
            .ok()
            .and_then(|doc| {
                doc.get("keys")?.get("command")?.as_array_of_tables()?.iter().find(|t| t.get("command").and_then(|c| c.as_str()) == Some(crate::sidebar::POPUP_ACTION))?.get("key")?.as_str().map(str::to_string)
            });
        let key = options.key.clone().or(current_key).unwrap_or_else(|| crate::sidebar::DEFAULT_KEY.to_string());
        let defaults = ctx.runner.run(&crate::runner::Cmd::new(ctx.env.herdr_bin(), crate::herdr::CALL_TIMEOUT).arg("--default-config")).ok().filter(|o| o.success()).map(|o| o.stdout).unwrap_or_default();
        let builtin = crate::sidebar::builtin_keys(&defaults);
        if builtin.is_empty() {
            notes.push("could not read Herdr's built-in key map (`herdr --default-config`); the popup key was checked against your config only".into());
        }
        if let Some(conflict) = crate::sidebar::key_conflict(&text, &key, &builtin) {
            bail!("{conflict}; pick another popup key with `configure --key <key>`");
        }
        let command = tab_command(&binary, &ctx.root);
        let after = crate::sidebar::config_edit(&text, &crate::sidebar::Spec { key: key.clone(), tab_command: command.clone() }, false)?;
        if before.as_deref() == Some(after.as_str()) {
            notes.push(format!("{}: sidebar rows, popup key `{key}` and tab-bar entry already in place", file.display()));
        } else {
            crate::sidebar::check_config(&ctx.env.herdr_bin(), ctx.runner, &after, &ctx.config_dir)?;
            notes.push(format!("{}: adding the project grouping rows, the popup key `{key}` and the tab-bar entry", file.display()));
            edits.push((file, Owned { before, after, kind: "config".into(), command: Some(command) }));
        }
    }
    if options.dry_run {
        return Ok(notes);
    }
    for (path, edit) in &edits {
        let key = path.to_string_lossy().into_owned();
        let mut owned = edit.clone();
        if owned.kind == "extension" {
            // The whole file is ours: `unconfigure` removes it.
            owned.before = None;
        } else if let Some(previous) = journal.get(&key) {
            owned.before = removal_baseline(previous, &owned)?;
        }
        journal.insert(key, owned);
    }
    let source = options.skill.as_ref().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    for (link, _) in &links {
        journal.insert(link.to_string_lossy().into_owned(), Owned { before: None, after: source.clone(), kind: "skill".into(), command: None });
    }
    save_journal(&ctx.config_dir, &journal)?;
    for (index, (path, edit)) in edits.iter().enumerate() {
        if let Err(error) = replace(path, &edit.before, &edit.after) {
            for (path, edit) in edits[..index].iter().rev() {
                if read(path)?.as_deref() == Some(edit.after.as_str()) {
                    match &edit.before {
                        Some(text) => replace(path, &Some(edit.after.clone()), text)?,
                        None => std::fs::remove_file(path)?,
                    }
                }
            }
            return Err(error);
        }
    }
    for (link, source) in &links {
        let Some(source) = source else { continue };
        if std::fs::symlink_metadata(link).is_ok() {
            std::fs::remove_file(link)?;
        }
        std::fs::create_dir_all(link.parent().context("skill link has no parent")?)?;
        std::os::unix::fs::symlink(source, link).with_context(|| format!("could not link {}", link.display()))?;
    }
    Ok(notes)
}

/// Removes exactly what `configure` added: the file goes back to its journaled
/// text when nothing else changed, else only our entries are taken out.
pub fn unconfigure(ctx: &Ctx) -> Result<Vec<String>> {
    let journal = load_journal(&ctx.config_dir);
    let mut notes = Vec::new();
    let mut remaining = journal.clone();
    for (key, owned) in &journal {
        let path = Path::new(key);
        if owned.kind == "skill" {
            match skill_state(path, Path::new(&owned.after)) {
                SkillState::Ours => {
                    std::fs::remove_file(path)?;
                    notes.push(format!("{key}: skill link removed"));
                }
                SkillState::Missing => notes.push(format!("{key}: already gone")),
                _ => notes.push(format!("{key}: no longer this plugin's link; left alone")),
            }
            remaining.remove(key);
            continue;
        }
        let current = read(path)?;
        if current.as_deref() == Some(owned.after.as_str()) {
            match &owned.before {
                Some(text) => replace(path, &current, text)?,
                None => std::fs::remove_file(path)?,
            }
            notes.push(format!("{key}: restored"));
        } else if owned.kind == "extension" && current.is_some() {
            notes.push(format!("{key}: edited since configure; left alone"));
        } else if let Some(text) = &current {
            let cleaned = remove_ours(&owned.kind, text, owned.command.as_deref())?;
            if cleaned != *text {
                replace(path, &current, &cleaned)?;
            }
            notes.push(format!("{key}: edited since configure; only the plugin's entries were removed"));
        } else {
            notes.push(format!("{key}: already gone"));
        }
        remaining.remove(key);
    }
    save_journal(&ctx.config_dir, &remaining)?;
    if journal.is_empty() {
        notes.push("nothing was configured".into());
    }
    Ok(notes)
}

/// The session the user's shell or the plugin action talks to, if reachable.
fn session_herdr<'a>(ctx: &'a Ctx) -> Option<crate::herdr::Herdr<'a>> {
    let session = crate::paths::resolve_session(&crate::paths::SessionFlags::default(), ctx.env, ctx.runner).ok()?;
    let herdr = crate::herdr::Herdr::new(ctx.env.herdr_bin(), &session.socket, ctx.runner);
    herdr.reachable().then_some(herdr)
}

/// `herdr server reload-config`, so server-side settings (the tab-bar entry,
/// keys) apply without a restart.
pub fn reload_config(ctx: &Ctx) {
    if let Some(herdr) = session_herdr(ctx) {
        match herdr.call(&["server", "reload-config"], crate::herdr::CALL_TIMEOUT) {
            Ok(_) => println!("reloaded the Herdr server's config"),
            Err(error) => println!("could not reload the Herdr config ({error}); run `herdr server reload-config`"),
        }
    }
}

/// After `configure`: reload, then the default by-need agent order.
pub fn apply_live(ctx: &Ctx) {
    reload_config(ctx);
    apply_view(ctx);
    println!("Sidebar rows are drawn by your Herdr client: if they are not visible yet, run `reload config` in Herdr (prefix+shift+r).");
}

/// The default agent view, once the sidebar is configured. Herdr holds one
/// view and has no way to read it, so this replaces another tool's view; it
/// is applied at startup, after `configure` and on `unfocus` only.
pub fn apply_view(ctx: &Ctx) {
    let configured = load_journal(&ctx.config_dir).values().any(|o| o.kind == "config");
    if !configured {
        return;
    }
    if let Some(herdr) = session_herdr(ctx) {
        let _ = herdr.agent_view_set(crate::sidebar::default_view());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CMD: &str = "'/p/herdr-projects' --root /r hook --agent claude 2>/dev/null || true";

    #[test]
    fn the_hook_command_never_fails_even_with_a_missing_binary() {
        let command = hook_command(Path::new("/no/such/herdr-projects"), Path::new("/r"), "claude");
        let out = std::process::Command::new("/bin/sh").args(["-c", &command]).output().unwrap();
        assert!(out.status.success());
        assert!(out.stdout.is_empty() && out.stderr.is_empty());
    }

    #[test]
    fn existing_hooks_comments_and_user_edits_survive() {
        let original = "{\n// user's comment\n\"theme\": \"dark\",\"hooks\":{\"SessionStart\":[{\"hooks\":[{\"command\":\"keep\"}]}]}}";
        let added = hooks(original, CMD, false).unwrap();
        assert!(added.contains("// user's comment"));
        assert!(added.contains("keep"));
        assert_eq!(added.matches("hook --agent claude").count(), 3);
        assert_eq!(hooks(&added, CMD, false).unwrap(), added);
        let removed = hooks(&added, CMD, true).unwrap();
        assert!(!removed.contains("herdr-projects"));
        assert!(removed.contains("keep"));
        assert!(hooks("[]", CMD, false).is_err());
    }

    #[test]
    fn a_moved_binary_replaces_the_old_entries_and_other_plugins_are_left_alone() {
        let old = hooks("{}", CMD, false).unwrap();
        let moved = hooks(&old, "'/new/herdr-projects' --root /r hook --agent claude", false).unwrap();
        assert!(!moved.contains("/p/herdr-projects"));
        assert_eq!(moved.matches("/new/herdr-projects").count(), 3);
        let with_other = "{\"hooks\":{\"PostToolUse\":[{\"matcher\":\"*\",\"hooks\":[{\"type\":\"command\",\"command\":\"'/x/herdr-progress' hook --agent claude\",\"timeout\":10}]}]}}";
        let added = hooks(with_other, CMD, false).unwrap();
        assert!(added.contains("herdr-progress"));
        assert!(has_agent_progress_hooks(&added));
        let removed = hooks(&added, CMD, true).unwrap();
        assert!(removed.contains("herdr-progress") && !removed.contains("herdr-projects"));
    }

    #[test]
    fn each_harness_gets_its_own_file_event_names_and_entry_shape() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[("COPILOT_HOME", "/c")]);
        assert_eq!(hook_file(&env, "droid", None, None), home.path().join(".factory/settings.json"));
        assert_eq!(hook_file(&env, "gemini", None, None), home.path().join(".gemini/settings.json"));
        assert_eq!(hook_file(&env, "copilot", None, None), Path::new("/c/hooks/herdr-projects.json"));
        // Copilot's hooks/ folder may not exist yet: its config directory counts.
        std::fs::create_dir_all(home.path().join(".copilot")).unwrap();
        let env = Env::for_test(home.path(), &[]);
        assert!(harness_installed(&env, "copilot", None, None));
        assert!(!harness_installed(&env, "gemini", None, None));

        let gemini = hooks("{\"theme\":\"x\"}", "/b/herdr-projects --root /r hook --agent gemini", false).unwrap();
        let value: serde_json::Value = serde_json::from_str(&gemini).unwrap();
        for event in ["SessionStart", "BeforeAgent", "AfterTool"] {
            assert_eq!(value["hooks"][event][0]["hooks"][0]["timeout"], 10_000, "{gemini}");
        }
        assert!(value["hooks"]["PostToolUse"].is_null());

        let command = "/b/herdr-projects --root /r hook --agent copilot";
        let copilot = hooks("{}", command, false).unwrap();
        let value: serde_json::Value = serde_json::from_str(&copilot).unwrap();
        assert_eq!(value["version"], 1);
        for event in ["SessionStart", "UserPromptSubmit", "PostToolUse"] {
            assert_eq!(value["hooks"][event], serde_json::json!([{"type":"command","command":command,"timeoutSec":10}]), "{copilot}");
        }
        assert_eq!(hooks(&copilot, command, false).unwrap(), copilot);
        let moved = hooks(&copilot, "/new/herdr-projects --root /r hook --agent copilot", false).unwrap();
        assert_eq!(moved.matches("herdr-projects").count(), 3);
        assert!(!hooks(&copilot, command, true).unwrap().contains("herdr-projects"));
    }

    #[test]
    fn removal_baseline_keeps_the_original_or_the_users_later_edits() {
        let previous = Owned { before: Some("original".into()), after: "configured".into(), kind: "hooks".into(), command: Some(CMD.into()) };
        let unchanged = Owned { before: Some("configured".into()), after: "configured2".into(), kind: "hooks".into(), command: Some(CMD.into()) };
        assert_eq!(removal_baseline(&previous, &unchanged).unwrap().as_deref(), Some("original"));
        let edited_text = format!("{}\n", hooks("{\"theme\":\"dark\"}", CMD, false).unwrap());
        let edited = Owned { before: Some(edited_text.clone()), after: "x".into(), kind: "hooks".into(), command: Some(CMD.into()) };
        let baseline = removal_baseline(&previous, &edited).unwrap().unwrap();
        assert!(baseline.contains("dark") && !baseline.contains("herdr-projects"));
    }

    #[test]
    fn configure_and_unconfigure_round_trip_byte_for_byte_and_keep_user_additions() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        let claude = home.path().join("claude");
        let codex = home.path().join("codex");
        std::fs::create_dir_all(&claude).unwrap();
        std::fs::create_dir_all(&codex).unwrap();
        let original = "{\n  // mine\n  \"permissions\": {\"allow\": [\"Bash(ls:*)\"]},\n  \"hooks\": {\"Stop\": [{\"hooks\": [{\"type\": \"command\", \"command\": \"say done\"}]}]}\n}\n";
        std::fs::write(claude.join("settings.json"), original).unwrap();
        let runner = crate::runner::fake::FakeRunner::new();
        let ctx = Ctx { env: &env, root: home.path().join("root"), config_dir: home.path().join("cfg"), runner: &runner, detached_ticker: false };
        let options = ConfigureOptions { clients: vec![], claude_home: Some(claude.clone()), codex_home: Some(codex.clone()), dry_run: true, hooks: true, sidebar: false, key: None, herdr_config: None, skill: None };
        let notes = configure(&ctx, &options).unwrap();
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert_eq!(std::fs::read_to_string(claude.join("settings.json")).unwrap(), original, "dry run changed a file");

        let options = ConfigureOptions { dry_run: false, ..options };
        configure(&ctx, &options).unwrap();
        let configured = std::fs::read_to_string(claude.join("settings.json")).unwrap();
        assert!(configured.contains("// mine") && configured.contains("say done"));
        assert_eq!(configured.matches("hook --agent claude").count(), 3);
        let codex_text = std::fs::read_to_string(codex.join("hooks.json")).unwrap();
        assert_eq!(codex_text.matches("hook --agent codex").count(), 3);
        assert_eq!(load_journal(&ctx.config_dir).len(), 2);
        // Idempotent.
        configure(&ctx, &options).unwrap();
        assert_eq!(std::fs::read_to_string(claude.join("settings.json")).unwrap(), configured);

        // Unconfigure: byte-identical when nothing else changed; the created file is removed.
        unconfigure(&ctx).unwrap();
        assert_eq!(std::fs::read_to_string(claude.join("settings.json")).unwrap(), original);
        assert!(!codex.join("hooks.json").exists());
        assert!(load_journal(&ctx.config_dir).is_empty());

        // A user edit made after configure survives unconfigure.
        configure(&ctx, &options).unwrap();
        let text = std::fs::read_to_string(claude.join("settings.json")).unwrap();
        std::fs::write(claude.join("settings.json"), text.replace("\"theme\"", "\"theme\"").replacen("{\n", "{\n  \"model\": \"opus\",\n", 1)).unwrap();
        unconfigure(&ctx).unwrap();
        let after = std::fs::read_to_string(claude.join("settings.json")).unwrap();
        assert!(after.contains("\"model\": \"opus\"") && after.contains("say done") && !after.contains("herdr-projects"));
    }

    #[test]
    fn pi_and_omp_get_an_extension_file_that_runs_the_hook_and_unconfigure_removes_it() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        std::fs::create_dir_all(home.path().join(".pi")).unwrap();
        std::fs::create_dir_all(home.path().join(".omp")).unwrap();
        let runner = crate::runner::fake::FakeRunner::new();
        let ctx = Ctx { env: &env, root: home.path().join("root"), config_dir: home.path().join("cfg"), runner: &runner, detached_ticker: false };
        let options = ConfigureOptions { clients: vec![], claude_home: None, codex_home: None, dry_run: false, hooks: true, sidebar: false, key: None, herdr_config: None, skill: None };
        configure(&ctx, &options).unwrap();
        let pi = home.path().join(".pi/agent/extensions/herdr-projects.ts");
        let omp = home.path().join(".omp/agent/extensions/herdr-projects.ts");
        let pi_text = std::fs::read_to_string(&pi).unwrap();
        assert!(pi_text.contains("hook --agent pi") && pi_text.contains("pi.on(\"session_start\""), "{pi_text}");
        assert!(std::fs::read_to_string(&omp).unwrap().contains("hook --agent omp"));
        // Idempotent.
        configure(&ctx, &options).unwrap();
        assert_eq!(std::fs::read_to_string(&pi).unwrap(), pi_text);

        unconfigure(&ctx).unwrap();
        assert!(!pi.exists() && !omp.exists());
        assert!(load_journal(&ctx.config_dir).is_empty());
    }

    #[test]
    fn an_extension_file_that_is_not_ours_is_left_alone() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        let theirs = home.path().join(".pi/agent/extensions/herdr-projects.ts");
        std::fs::create_dir_all(theirs.parent().unwrap()).unwrap();
        std::fs::write(&theirs, "export default function () {}\n").unwrap();
        let runner = crate::runner::fake::FakeRunner::new();
        let ctx = Ctx { env: &env, root: home.path().join("root"), config_dir: home.path().join("cfg"), runner: &runner, detached_ticker: false };
        let options = ConfigureOptions { clients: vec!["pi".into()], claude_home: None, codex_home: None, dry_run: false, hooks: true, sidebar: false, key: None, herdr_config: None, skill: None };
        let notes = configure(&ctx, &options).unwrap();
        assert_eq!(std::fs::read_to_string(&theirs).unwrap(), "export default function () {}\n");
        assert!(notes.iter().any(|n| n.contains("left alone")), "{notes:?}");
        assert!(load_journal(&ctx.config_dir).is_empty());
    }

    #[test]
    fn unconfigure_leaves_an_extension_the_user_edited() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        std::fs::create_dir_all(home.path().join(".omp")).unwrap();
        let runner = crate::runner::fake::FakeRunner::new();
        let ctx = Ctx { env: &env, root: home.path().join("root"), config_dir: home.path().join("cfg"), runner: &runner, detached_ticker: false };
        let options = ConfigureOptions { clients: vec!["omp".into()], claude_home: None, codex_home: None, dry_run: false, hooks: true, sidebar: false, key: None, herdr_config: None, skill: None };
        configure(&ctx, &options).unwrap();
        let file = home.path().join(".omp/agent/extensions/herdr-projects.ts");
        let edited = std::fs::read_to_string(&file).unwrap() + "// mine\n";
        std::fs::write(&file, &edited).unwrap();
        let notes = unconfigure(&ctx).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), edited);
        assert!(notes.iter().any(|n| n.contains("left alone")), "{notes:?}");
    }

    #[test]
    fn configure_rewrites_an_extension_it_wrote_but_never_one_the_user_edited() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        std::fs::create_dir_all(home.path().join(".omp")).unwrap();
        let runner = crate::runner::fake::FakeRunner::new();
        let ctx = Ctx { env: &env, root: home.path().join("root"), config_dir: home.path().join("cfg"), runner: &runner, detached_ticker: false };
        let options = ConfigureOptions { clients: vec!["omp".into()], claude_home: None, codex_home: None, dry_run: false, hooks: true, sidebar: false, key: None, herdr_config: None, skill: None };
        configure(&ctx, &options).unwrap();
        let file = home.path().join(".omp/agent/extensions/herdr-projects.ts");
        let current = std::fs::read_to_string(&file).unwrap();

        // What an older release wrote, still as journaled: rewritten.
        let older = current.replace("TIMEOUT_MS = 10_000", "TIMEOUT_MS = 5_000");
        std::fs::write(&file, &older).unwrap();
        let mut journal = load_journal(&ctx.config_dir);
        journal.get_mut(&*file.to_string_lossy()).unwrap().after = older;
        save_journal(&ctx.config_dir, &journal).unwrap();
        configure(&ctx, &options).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), current);

        // Edited by the user: left alone.
        let edited = current.clone() + "// mine\n";
        std::fs::write(&file, &edited).unwrap();
        let notes = configure(&ctx, &options).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), edited);
        assert!(notes.iter().any(|n| n.contains("edited")), "{notes:?}");
    }

    #[test]
    fn the_skill_is_linked_once_into_a_shared_skills_dir_and_foreign_ones_are_left_alone() {
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        let claude = home.path().join("claude");
        let shared = home.path().join(".agents/skills");
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::create_dir_all(home.path().join("codex")).unwrap();
        std::fs::create_dir_all(&claude).unwrap();
        // Like this Mac: Claude's skills dir is itself a link to ~/.agents/skills.
        std::os::unix::fs::symlink(&shared, claude.join("skills")).unwrap();
        let source = home.path().join("plugin/skill/autoproject");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "---\nname: autoproject\n---\n").unwrap();
        let runner = crate::runner::fake::FakeRunner::new();
        let ctx = Ctx { env: &env, root: home.path().join("root"), config_dir: home.path().join("cfg"), runner: &runner, detached_ticker: false };
        let options = |dry_run: bool, skill: &Path| ConfigureOptions { clients: vec![], claude_home: Some(claude.clone()), codex_home: Some(home.path().join("codex")), dry_run, hooks: true, sidebar: false, key: None, herdr_config: None, skill: Some(skill.to_path_buf()) };
        let link = std::fs::canonicalize(&shared).unwrap().join(SKILL);

        // A plain directory already there (the old personal copy) is never touched.
        std::fs::create_dir_all(shared.join(SKILL)).unwrap();
        let notes = configure(&ctx, &options(false, &source)).unwrap();
        assert!(notes.iter().any(|n| n.contains("left alone")), "{notes:?}");
        assert_eq!(skill_state(&link, &source), SkillState::Foreign);
        std::fs::remove_dir(shared.join(SKILL)).unwrap();
        unconfigure(&ctx).unwrap();

        // Dry run: nothing linked.
        configure(&ctx, &options(true, &source)).unwrap();
        assert_eq!(skill_state(&link, &source), SkillState::Missing);

        let notes = configure(&ctx, &options(false, &source)).unwrap();
        assert_eq!(notes.iter().filter(|n| n.contains("linking")).count(), 1, "{notes:?}");
        assert_eq!(skill_state(&link, &source), SkillState::Ours);
        assert!(claude.join("skills").join(SKILL).join("SKILL.md").is_file());
        assert!(claude.join("skills").is_symlink(), "the shared dir link was replaced");

        // A moved plugin checkout relinks our own link.
        let moved = home.path().join("moved/skill/autoproject");
        std::fs::create_dir_all(&moved).unwrap();
        std::fs::write(moved.join("SKILL.md"), "x").unwrap();
        configure(&ctx, &options(false, &moved)).unwrap();
        assert_eq!(skill_state(&link, &moved), SkillState::Ours);

        // Unconfigure removes only our link; a foreign link in its place survives.
        unconfigure(&ctx).unwrap();
        assert_eq!(skill_state(&link, &moved), SkillState::Missing);
        assert!(shared.is_dir());
        configure(&ctx, &options(false, &moved)).unwrap();
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(home.path(), &link).unwrap();
        let notes = unconfigure(&ctx).unwrap();
        assert!(notes.iter().any(|n| n.contains("left alone")), "{notes:?}");
        assert!(link.is_symlink());
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_config_is_refused_without_touching_the_target() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("real.json");
        let link = dir.path().join("settings.json");
        std::fs::write(&target, "{\"user\":true}").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(read(&link).is_err());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "{\"user\":true}");
    }

    #[test]
    fn configure_edits_herdrs_config_checks_the_key_and_unconfigure_restores_it() {
        use crate::runner::fake::{FakeRunner, fail, ok};
        let home = tempfile::tempdir().unwrap();
        let env = Env::for_test(home.path(), &[]);
        let config = home.path().join("herdr.toml");
        let original = "# my theme\n[theme]\nname = \"catppuccin\"\n";
        std::fs::write(&config, original).unwrap();
        let runner = FakeRunner::new();
        runner.on("--default-config", ok("[keys]\n# previous_tab = \"prefix+p\"\n"));
        runner.on("config check", ok(""));
        let ctx = Ctx { env: &env, root: home.path().join("root"), config_dir: home.path().join("cfg"), runner: &runner, detached_ticker: false };
        let options = |key: Option<&str>| ConfigureOptions { clients: vec!["claude".into()], claude_home: Some(home.path().join("claude")), codex_home: None, dry_run: false, hooks: true, sidebar: true, key: key.map(str::to_string), herdr_config: Some(config.clone()), skill: None };
        std::fs::create_dir_all(home.path().join("claude")).unwrap();

        // A key Herdr already uses is refused before anything is written.
        assert!(configure(&ctx, &options(Some("prefix+p"))).unwrap_err().to_string().contains("previous_tab"));
        assert_eq!(std::fs::read_to_string(&config).unwrap(), original);

        configure(&ctx, &options(None)).unwrap();
        let text = std::fs::read_to_string(&config).unwrap();
        assert!(text.contains("# my theme") && text.contains("prefix+a") && text.contains("$hp_sub") && text.contains("needs-you --line"));
        assert_eq!(runner.count("config check"), 1);
        // A second run keeps the configured key.
        configure(&ctx, &options(None)).unwrap();
        assert_eq!(std::fs::read_to_string(&config).unwrap(), text);

        unconfigure(&ctx).unwrap();
        assert_eq!(std::fs::read_to_string(&config).unwrap(), original);

        // Herdr rejecting the candidate changes nothing.
        let rejecting = FakeRunner::new();
        rejecting.on("--default-config", ok(""));
        rejecting.on("config check", fail(1, "bad row"));
        let ctx = Ctx { runner: &rejecting, ..ctx };
        assert!(configure(&ctx, &options(None)).is_err());
        assert_eq!(std::fs::read_to_string(&config).unwrap(), original);
    }
}

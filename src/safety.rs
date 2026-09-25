//! Yolo mode and the other user-only safety settings: `safety show`, `safety
//! set` and `safety yolo`, and the popup's safety rows. They live in the
//! user's `config.toml`, per project (`[safety."<path>"]`) or for all projects
//! (`[safety.default]`). The CLI changes them only for a person at a terminal
//! who confirms; the popup, itself a terminal, writes them in-process.

use std::io::{BufRead, IsTerminal, Write as _};
use std::path::Path;

use anyhow::{Context, Result, bail};
use toml_edit::{DocumentMut, Item, Table};

use crate::paths::Ctx;
use crate::project::{self, Project, SafetyLayer};

/// The `[safety.<key>]` table every project falls back to. Project tables are
/// keyed by an absolute path, so this name never clashes with one.
pub const DEFAULT_TABLE: &str = "default";

/// The keys `safety set` accepts, with what they hold.
pub const KEYS: [(&str, &str); 5] = [
    ("yolo", "on/off"),
    ("start_threads", "propose/auto"),
    ("coordinator_agent_args", "arguments"),
    ("thread_agent_args", "arguments"),
    ("routine_commands", "on/off"),
];

/// What the popup and the CLI say after a change.
pub const RESTART_NOTE: &str = "agents launched from now on use it; running agents keep their flags until restarted (r on a thread, quit and reopen the coordinator)";

/// The flag that makes an agent of `kind` stop asking for permission, or
/// `None` when no such flag is known for the kind. Each harness has its own:
/// Claude Code's flag makes Codex refuse to start, so there is no shared list.
/// Pi has no permission prompts, so it needs none.
pub fn yolo_flags(kind: &str) -> Option<&'static [&'static str]> {
    Some(match kind {
        "claude" => &["--dangerously-skip-permissions"],
        "codex" => &["--dangerously-bypass-approvals-and-sandbox"],
        "gemini" | "qwen" => &["--yolo"],
        "cursor" => &["--force"],
        "opencode" => &["--auto"],
        "copilot" => &["--allow-all-tools"],
        "amp" => &["--dangerously-allow-all"],
        "pi" => &[],
        _ => return None,
    })
}

/// Which table a change goes to.
pub enum Target {
    Global,
    Project(Project),
}

impl Target {
    /// `--global` or a project slug.
    pub fn parse(ctx: &Ctx, word: &str) -> Result<Target> {
        if word == "--global" {
            return Ok(Target::Global);
        }
        Ok(Target::Project(Project::load(&ctx.root, word)?))
    }

    fn table(&self) -> String {
        match self {
            Target::Global => DEFAULT_TABLE.to_string(),
            Target::Project(p) => p.canonical_dir().to_string_lossy().into_owned(),
        }
    }

    fn label(&self) -> String {
        match self {
            Target::Global => "all projects".to_string(),
            Target::Project(p) => p.slug.clone(),
        }
    }

    fn word(&self) -> String {
        match self {
            Target::Global => "--global".to_string(),
            Target::Project(p) => p.slug.clone(),
        }
    }
}

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

fn parse_on_off(value: &str) -> Result<bool> {
    match value {
        "on" | "true" | "yes" => Ok(true),
        "off" | "false" | "no" => Ok(false),
        other => bail!("`{other}` is not on or off"),
    }
}

/// The TOML value `words` set `key` to, or `None` for `default`: remove the
/// key so the table below applies. Argument lists split on whitespace, and no
/// words at all is the empty list.
fn value_item(key: &str, words: &[String]) -> Result<Option<Item>> {
    if words.len() == 1 && words[0] == "default" {
        return Ok(None);
    }
    let one = || -> Result<&str> {
        match words {
            [word] => Ok(word.as_str()),
            _ => bail!("`{key}` takes one value"),
        }
    };
    Ok(Some(match key {
        "yolo" | "routine_commands" => toml_edit::value(parse_on_off(one()?)?),
        "start_threads" => match one()? {
            value @ ("propose" | "auto") => toml_edit::value(value),
            other => bail!("start_threads is propose or auto, not `{other}`"),
        },
        "coordinator_agent_args" | "thread_agent_args" => {
            let mut array = toml_edit::Array::new();
            for arg in words.iter().flat_map(|w| w.split_whitespace()) {
                array.push(arg);
            }
            toml_edit::value(array)
        }
        other => bail!("unknown safety setting `{other}`; one of: {}", KEYS.iter().map(|(k, _)| *k).collect::<Vec<_>>().join(", ")),
    }))
}

/// Sets (or, with `None`, removes) `[safety.<table>] key` in config.toml's
/// text, keeping the rest of the file as it was. An emptied table goes.
pub fn set_in(text: &str, table: &str, key: &str, item: Option<Item>) -> Result<String> {
    let mut doc = text.parse::<DocumentMut>().context("config.toml does not parse")?;
    if !doc.contains_key("safety") {
        let mut safety = Table::new();
        safety.set_implicit(true);
        doc["safety"] = Item::Table(safety);
    }
    let safety = doc["safety"].as_table_mut().context("`safety` in config.toml is not a table")?;
    match item {
        Some(item) => {
            if !safety.contains_key(table) {
                safety[table] = Item::Table(Table::new());
            }
            safety[table].as_table_mut().with_context(|| format!("`safety.{table}` is not a table"))?[key] = item;
        }
        None => {
            if let Some(t) = safety.get_mut(table).and_then(Item::as_table_mut) {
                t.remove(key);
                if t.is_empty() {
                    safety.remove(table);
                }
            }
        }
    }
    let edited = doc.to_string();
    project::load_safety_layers_from(&edited, "config.toml", Path::new(""))?;
    Ok(edited)
}

/// Writes one change and returns what the popup and the CLI print.
pub fn apply(ctx: &Ctx, target: &Target, key: &str, words: &[String]) -> Result<String> {
    let item = value_item(key, words)?;
    let path = ctx.config_dir.join("config.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let edited = set_in(&text, &target.table(), key, item)?;
    std::fs::create_dir_all(&ctx.config_dir)?;
    project::write_atomic(&path, edited.as_bytes())?;
    let shown = rows(&ctx.config_dir, target)?.into_iter().find(|r| r.key == key).map(|r| r.text()).unwrap_or_default();
    Ok(format!("{}: {key} = {shown}; {RESTART_NOTE}", target.label()))
}

/// `safety set <slug|--global> <key> <value…>` and `safety yolo <slug|--global>
/// on|off|default`: only a person at a terminal who types `y`. Coding agents
/// run commands without a terminal, so they are refused; one that fakes a
/// terminal is not stopped by this, only by its own permission prompts.
pub fn set_cli(ctx: &Ctx, target: &str, key: &str, words: &[String]) -> Result<()> {
    let target = Target::parse(ctx, target)?;
    value_item(key, words)?;
    let command = format!("herdr-projects safety set {} {key} {}", target.word(), words.join(" "));
    if !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
        bail!(
            "safety settings are changed only by a person: in the projects popup (settings) or by running `{}` in a terminal. Standard input is not a terminal",
            command.trim_end()
        );
    }
    let before = rows(&ctx.config_dir, &target)?.into_iter().find(|r| r.key == key).map(|r| r.text()).unwrap_or_default();
    if key == "yolo" && words.first().is_some_and(|w| parse_on_off(w).unwrap_or(false)) {
        println!("Yolo mode for {}: threads start without asking, and agents launch with their", target.label());
        println!("harness's skip-permissions flag, so they run commands and edit files without asking.");
    }
    print!("{}: {key} is {before}; set it to `{}`? [y/N] ", target.label(), words.join(" "));
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    if !matches!(line.trim(), "y" | "Y" | "yes") {
        bail!("not changed");
    }
    println!("{}", apply(ctx, &target, key, words)?);
    Ok(())
}

/// One safety setting as the popup and `safety show` list it.
#[derive(Debug, Clone, PartialEq)]
pub struct SafetyRow {
    pub key: &'static str,
    pub value: String,
    /// Where the value comes from: `project`, `all projects` or `built-in`.
    pub source: &'static str,
    /// Why a value differs from what is written (yolo forces start_threads).
    pub note: String,
}

impl SafetyRow {
    pub fn text(&self) -> String {
        let mut text = self.value.clone();
        if !self.note.is_empty() {
            text.push_str(&format!(" ({})", self.note));
        }
        text
    }
}

/// The effective settings for `target`, each with where it comes from.
pub fn rows(config_dir: &Path, target: &Target) -> Result<Vec<SafetyRow>> {
    let (default, own) = match target {
        Target::Global => (project::load_safety_layers(config_dir, Path::new(""))?.0, SafetyLayer::default()),
        Target::Project(p) => project::load_safety_layers(config_dir, &p.canonical_dir())?,
    };
    let own_source = match target {
        Target::Global => "all projects",
        Target::Project(_) => "project",
    };
    fn pick<T: Clone>(own: &Option<T>, default: &Option<T>, own_source: &'static str, builtin: T) -> (T, &'static str) {
        match (own, default) {
            (Some(v), _) => (v.clone(), own_source),
            (None, Some(v)) => (v.clone(), "all projects"),
            (None, None) => (builtin, "built-in"),
        }
    }
    let args = |v: Vec<String>| if v.is_empty() { "(none)".to_string() } else { v.join(" ") };
    let (yolo, yolo_source) = pick(&own.yolo, &default.yolo, own_source, false);
    let (start, start_source) = pick(&own.start_threads, &default.start_threads, own_source, "propose".to_string());
    let (coordinator, coordinator_source) = pick(&own.coordinator_agent_args, &default.coordinator_agent_args, own_source, Vec::new());
    let (thread, thread_source) = pick(&own.thread_agent_args, &default.thread_agent_args, own_source, Vec::new());
    let (commands, commands_source) = pick(&own.routine_commands, &default.routine_commands, own_source, false);
    Ok(vec![
        SafetyRow { key: "yolo", value: on_off(yolo).into(), source: yolo_source, note: String::new() },
        SafetyRow {
            key: "start_threads",
            value: if yolo { "auto".into() } else { start },
            source: start_source,
            note: if yolo { "yolo".into() } else { String::new() },
        },
        SafetyRow { key: "coordinator_agent_args", value: args(coordinator), source: coordinator_source, note: String::new() },
        SafetyRow { key: "thread_agent_args", value: args(thread), source: thread_source, note: String::new() },
        SafetyRow { key: "routine_commands", value: on_off(commands).into(), source: commands_source, note: String::new() },
    ])
}

/// What the yolo flag adds for each of `kinds`, or that none is known.
fn flags_text(kinds: &[&str]) -> Vec<String> {
    let mut seen = Vec::new();
    for kind in kinds {
        if seen.iter().any(|(k, _)| k == kind) {
            continue;
        }
        let text = match yolo_flags(kind) {
            Some([]) => "nothing (it never asks)".to_string(),
            Some(flags) => flags.join(" "),
            None => "no known flag: its agents still ask".to_string(),
        };
        seen.push((kind.to_string(), text));
    }
    seen.into_iter().map(|(k, t)| format!("{k}: {t}")).collect()
}

/// What `safety show` prints: the effective settings, where each comes from,
/// and how the user changes them.
pub fn show_text(ctx: &Ctx, target: &Target) -> Result<String> {
    let mut out = format!("Safety settings for {} (yours; no agent may change them):\n", target.label());
    for row in rows(&ctx.config_dir, target)? {
        out.push_str(&format!("  {:<24} {}   [{}]\n", row.key, row.text(), row.source));
    }
    let kinds: Vec<String> = match target {
        Target::Global => vec!["claude".into(), "codex".into()],
        Target::Project(p) => {
            // The harnesses of the project's default profiles.
            let (s, _) = p.read_project_md().unwrap_or_default();
            let config = crate::profiles::load(&ctx.config_dir).unwrap_or_default();
            [&s.coordinator_profile, &s.thread_profile].iter().map(|n| config.get(n).map(|p| p.entry.agent).unwrap_or_else(|| n.to_string())).collect()
        }
    };
    let kinds: Vec<&str> = kinds.iter().map(String::as_str).collect();
    out.push_str(&format!("\nYolo starts threads without asking and adds each harness's flag ({}).\n", flags_text(&kinds).join("; ")));
    out.push_str("Routine commands are not part of yolo: they stay off until you turn them on and approve each one.\n");
    if let Target::Project(project) = target {
        let safety = project.safety(&ctx.config_dir)?;
        let config = crate::profiles::load(&ctx.config_dir)?;
        out.push_str("\n");
        for role in [crate::profiles::Role::Thread, crate::profiles::Role::Coordinator] {
            let list = config.allowed(&safety, role).map(|l| if l.is_empty() { "none".to_string() } else { l.join(", ") }).unwrap_or_else(|| "every profile".into());
            out.push_str(&format!("  {:<24} {list}\n", role.list_key()));
        }
        out.push_str(&format!("Profiles and these lists: `herdr-projects profile list --project {}`; you change them in the popup's settings or with `profile add|edit|remove|allow`.\n", project.slug));
    }
    let word = target.word();
    out.push_str(&format!(
        "\nChange them in the projects popup (settings: Y toggles yolo, ↵ edits a row) or in a terminal:\n  herdr-projects safety yolo {word} on|off|default\n  herdr-projects safety set {word} <key> <value>|default\n"
    ));
    out.push_str("A change reaches agents launched after it; running agents keep their flags until restarted.\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split(' ').filter(|w| !w.is_empty()).map(str::to_string).collect()
    }

    #[test]
    fn each_harness_gets_its_own_flag() {
        assert_eq!(yolo_flags("claude").unwrap(), ["--dangerously-skip-permissions"]);
        assert_eq!(yolo_flags("codex").unwrap(), ["--dangerously-bypass-approvals-and-sandbox"]);
        assert_eq!(yolo_flags("gemini").unwrap(), ["--yolo"]);
        assert!(yolo_flags("pi").unwrap().is_empty());
        assert_eq!(yolo_flags("kiro"), None);
        for kind in crate::agents::KINDS {
            for flag in yolo_flags(kind).unwrap_or_default() {
                assert!(crate::agents::split_model_args(kind, &[flag.to_string()]).0.is_empty(), "{kind}: a yolo flag is never a model flag");
            }
        }
    }

    #[test]
    fn values_are_checked_and_default_removes_the_key() {
        assert!(value_item("yolo", &words("on")).unwrap().is_some());
        assert!(value_item("yolo", &words("maybe")).is_err());
        assert!(value_item("yolo", &words("on off")).is_err());
        assert!(value_item("start_threads", &words("yolo")).is_err());
        assert!(value_item("start_threads", &words("default")).unwrap().is_none());
        assert!(value_item("whatever", &words("x")).is_err());
        let item = value_item("thread_agent_args", &["--a --b".to_string(), "c".to_string()]).unwrap().unwrap();
        assert_eq!(item.to_string(), "[\"--a\", \"--b\", \"c\"]");
        assert_eq!(value_item("thread_agent_args", &[]).unwrap().unwrap().to_string(), "[]");
    }

    #[test]
    fn edits_keep_the_rest_of_the_file_and_empty_tables_go() {
        let text = "root = \"/p\" # mine\n\n[machines.box]\nhost = \"box\"\n";
        let on = set_in(text, "default", "yolo", Some(toml_edit::value(true))).unwrap();
        assert!(on.starts_with("root = \"/p\" # mine\n") && on.contains("[machines.box]"));
        assert!(on.contains("[safety.default]\nyolo = true"), "{on}");
        let project = set_in(&on, "/p/demo", "start_threads", Some(toml_edit::value("auto"))).unwrap();
        assert!(project.contains("[safety.\"/p/demo\"]\nstart_threads = \"auto\""), "{project}");
        let (default, own) = project::load_safety_layers_from(&project, "config.toml", Path::new("/p/demo")).unwrap();
        assert_eq!((default.yolo, own.start_threads.as_deref()), (Some(true), Some("auto")));
        let back = set_in(&project, "/p/demo", "start_threads", None).unwrap();
        let back = set_in(&back, "default", "yolo", None).unwrap();
        assert!(!back.contains("safety."), "{back}");
        assert!(back.contains("[machines.box]"));
    }

    #[test]
    fn apply_writes_the_table_and_rows_show_where_values_come_from() {
        let root = tempfile::tempdir().unwrap();
        let project = project::create(root.path(), "demo", "", vec![]).unwrap();
        let env = crate::paths::Env::for_test(root.path(), &[]);
        let runner = crate::runner::fake::FakeRunner::new();
        let ctx = Ctx { env: &env, root: root.path().to_path_buf(), config_dir: root.path().join("cfg"), runner: &runner, detached_ticker: false };
        let target = Target::Project(project.clone());
        assert_eq!(rows(&ctx.config_dir, &target).unwrap()[0].source, "built-in");

        let message = apply(&ctx, &Target::Global, "yolo", &words("on")).unwrap();
        assert!(message.starts_with("all projects: yolo = on;") && message.contains("restarted"), "{message}");
        let r = rows(&ctx.config_dir, &target).unwrap();
        assert_eq!((r[0].value.as_str(), r[0].source), ("on", "all projects"));
        assert_eq!(r[1].text(), "auto (yolo)");
        assert!(project.safety(&ctx.config_dir).unwrap().yolo);

        apply(&ctx, &target, "yolo", &words("off")).unwrap();
        let r = rows(&ctx.config_dir, &target).unwrap();
        assert_eq!((r[0].value.as_str(), r[0].source, r[1].value.as_str()), ("off", "project", "propose"));
        assert!(!project.safety(&ctx.config_dir).unwrap().yolo, "the project's own value wins");
        apply(&ctx, &target, "yolo", &words("default")).unwrap();
        assert!(project.safety(&ctx.config_dir).unwrap().yolo, "back to the all-projects value");

        let shown = show_text(&ctx, &target).unwrap();
        assert!(shown.contains("claude: --dangerously-skip-permissions") && shown.contains("safety yolo demo on|off|default"), "{shown}");
    }
}

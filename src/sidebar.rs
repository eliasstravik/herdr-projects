//! What the Herdr sidebar shows for projects: per agent row a name
//! (`--display-agent`), `$hp_state` and `$hp_activity`; per project Space row
//! `$hp`; the project headings of `grouping`; a tab-bar count; the default
//! agent view, grouped by project. Also the `config.toml` edit `configure`
//! makes to render them.

use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value};

use crate::herdr::{CALL_TIMEOUT, Herdr};
use crate::project::{self, Project};
use crate::thread::{Group, Live, Thread};

pub const TOKEN_TTL_MS: u64 = 300_000;
/// Tokens this plugin wrote before 0.2.0; cleared on every report.
pub const OLD_TOKENS: [&str; 4] = ["project", "thread", "review", "rank"];
pub const POPUP_ACTION: &str = "herdr-projects.open-popup";
pub const DEFAULT_KEY: &str = "prefix+a";
/// Minutes without a self-report after which a working row says "Nm quiet".
pub const QUIET_SECS: i64 = 300;

/// The short word of a group, as the sidebar and tab bar show it.
pub fn word(group: Group) -> &'static str {
    match group {
        Group::WaitingOnYou => "needs you",
        Group::ReadyForReview => "review",
        Group::Working => "working",
        Group::Landing => "landing",
        Group::Idle => "idle",
        Group::Resolved => "resolved",
    }
}

/// Groups that need the user: counted in the project row and the tab bar.
pub fn needs_you(group: Group) -> bool {
    matches!(group, Group::WaitingOnYou | Group::ReadyForReview)
}

fn pr_number(url: &str) -> Option<&str> {
    url.rsplit('/').next().filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

/// Line 3 of a thread row: the group word and one fact.
pub fn state_line(group: Group, thread: &Thread, live: &Live, now: jiff::Timestamp) -> String {
    let percent = live.self_report.as_ref().and_then(|r| r.percent).filter(|p| *p < 100).map(|p| format!("~{p}%"));
    let pr = pr_number(&thread.pr).map(|n| format!("PR #{n}"));
    let fact = match group {
        Group::WaitingOnYou if !live.pane_exists => Some("pane closed".to_string()),
        Group::WaitingOnYou if thread.status == crate::thread::Status::Failed => Some("failed".to_string()),
        Group::WaitingOnYou if live.agent_state.as_deref() == Some("blocked") => Some("blocked".to_string()),
        Group::WaitingOnYou => percent,
        Group::ReadyForReview | Group::Landing => pr.or_else(|| Some("report".into())),
        Group::Working => match (&live.self_report, percent) {
            (_, Some(p)) if live.report_age_secs < QUIET_SECS => Some(p),
            _ => {
                let since = match &live.self_report {
                    Some(record) => live.report_age_secs.max(now.as_second() - record.reported_at),
                    None => crate::thread::seconds_since(&thread.last_state_change, now),
                };
                (since >= QUIET_SECS && !thread.prompt_pending).then(|| format!("{}m quiet", since / 60))
            }
        },
        Group::Idle | Group::Resolved => None,
    };
    match fact {
        Some(fact) => format!("{} · {fact}", word(group)),
        None => word(group).to_string(),
    }
}

/// Row 1 of a thread: its id and title, capped by Herdr at 80 characters.
pub fn thread_display(thread: &Thread) -> String {
    format!("{} · {}", thread.id, thread.title)
}

/// A coordinator's row name. The project heading above it names the project.
pub fn coordinator_display() -> String {
    "coordinator".into()
}

/// Reports one pane's row: display name, project, rank and state, with a TTL
/// so the row falls back to Herdr's own when the ticker stops. Never `--seq`:
/// token patches are per key and `hp_activity` comes from `report`.
pub fn report_pane(herdr: &Herdr, pane: &str, display: &str, slug: &str, group: Group, state: &str) {
    let ttl = TOKEN_TTL_MS.to_string();
    let rank = group.rank().to_string();
    let tokens = [format!("hp_project={slug}"), format!("hp_rank={rank}"), format!("hp_state={state}")];
    let mut args = vec!["pane", "report-metadata", pane, "--source", crate::herdr::SOURCE, "--display-agent", display, "--ttl-ms", &ttl];
    for token in &tokens {
        args.push("--token");
        args.push(token);
    }
    for old in OLD_TOKENS {
        args.push("--clear-token");
        args.push(old);
    }
    let _ = herdr.call(&args, CALL_TIMEOUT);
}

/// Clears every token and the display name this plugin set on a pane.
pub fn clear_pane(herdr: &Herdr, pane: &str) {
    if pane.is_empty() {
        return;
    }
    let mut args = vec!["pane", "report-metadata", pane, "--source", crate::herdr::SOURCE, "--clear-display-agent"];
    for name in ["hp_project", "hp_rank", "hp_state", "hp_activity"].into_iter().chain(crate::grouping::TOKENS).chain(OLD_TOKENS) {
        args.push("--clear-token");
        args.push(name);
    }
    let _ = herdr.call(&args, CALL_TIMEOUT);
}

/// `2 need you · 3 working`, `paused`, or `idle`.
pub fn project_line(groups: &[Group], paused: bool) -> String {
    if paused {
        return "paused".into();
    }
    let count = |f: &dyn Fn(Group) -> bool| groups.iter().filter(|g| f(**g)).count();
    let mut parts = Vec::new();
    let need = count(&|g| needs_you(g));
    if need > 0 {
        parts.push(format!("{need} need you"));
    }
    for (group, label) in [(Group::Working, "working"), (Group::Landing, "landing")] {
        let n = count(&|g| g == group);
        if n > 0 {
            parts.push(format!("{n} {label}"));
        }
    }
    if parts.is_empty() { "idle".into() } else { parts.join(" · ") }
}

pub fn report_workspace(herdr: &Herdr, workspace: &str, line: &str) {
    if workspace.is_empty() {
        return;
    }
    let ttl = TOKEN_TTL_MS.to_string();
    let token = format!("hp={line}");
    let _ = herdr.call(&["workspace", "report-metadata", workspace, "--source", crate::herdr::SOURCE, "--token", &token, "--ttl-ms", &ttl], CALL_TIMEOUT);
}

pub fn clear_workspace(herdr: &Herdr, workspace: &str) {
    if workspace.is_empty() {
        return;
    }
    let mut args = vec!["workspace", "report-metadata", workspace, "--source", crate::herdr::SOURCE, "--clear-token", "hp"];
    for name in crate::grouping::TOKENS {
        args.push("--clear-token");
        args.push(name);
    }
    let _ = herdr.call(&args, CALL_TIMEOUT);
}

/// The groups of a project's open threads, as the ticker last persisted them.
pub fn recorded_groups(project: &Project) -> Vec<Group> {
    crate::thread::list(project)
        .into_iter()
        .filter(|t| t.status != crate::thread::Status::Resolved)
        .filter_map(|t| Group::from_token(&t.last_group))
        .collect()
}

/// `needs-you --line`, the tab-bar entry: `projects: N need you`, from the
/// thread records the ticker persists. Nothing when none need the user or when
/// the ticker is not running (a stale count must not sit in the tab bar).
pub fn needs_you_line(root: &Path) -> Option<String> {
    if crate::ticker::lock_state(root) == crate::ticker::LockState::Free {
        return None;
    }
    let n: usize = project::list_slugs(root)
        .iter()
        .filter_map(|slug| Project::load(root, slug).ok())
        .filter(|p| p.status() == project::Status::Active)
        .map(|p| recorded_groups(&p).into_iter().filter(|g| needs_you(*g)).count())
        .sum();
    (n > 0).then(|| format!("projects: {n} need you"))
}

// ---------------------------------------------------------------- agent view

/// The default view: agents in project blocks (`hp_group`, see `grouping`),
/// each by need; agents without it last.
pub fn default_view() -> serde_json::Value {
    serde_json::json!({
        "source": crate::herdr::SOURCE,
        "label": "projects",
        "sort": [{ "field": { "token": "hp_group" }, "order": "asc" }, { "field": { "token": "hp_rank" }, "order": "asc" }],
    })
}

/// `focus <slug>`: only that project's agents, by need.
pub fn project_view(slug: &str) -> serde_json::Value {
    serde_json::json!({
        "source": crate::herdr::SOURCE,
        "label": format!("project: {slug}"),
        "filter": { "op": "eq", "field": { "token": "hp_project" }, "value": slug },
        "sort": [{ "field": { "token": "hp_group" }, "order": "asc" }, { "field": { "token": "hp_rank" }, "order": "asc" }],
    })
}

// ---------------------------------------------------------------- config.toml

/// What `configure` adds to the user's Herdr config.
pub struct Spec {
    pub key: String,
    /// The tab-bar command (absolute binary path; runs under `/bin/sh -lc`).
    pub tab_command: String,
}

fn row_state() -> Value {
    let mut rules = Array::new();
    let mut red = InlineTable::new();
    red.insert("starts_with", "needs you".into());
    red.insert("fg", "#f38ba8".into());
    red.insert("bold", true.into());
    rules.push(red);
    let mut yellow = InlineTable::new();
    yellow.insert("starts_with", "review".into());
    yellow.insert("fg", "#f9e2af".into());
    rules.push(yellow);
    let mut style = InlineTable::new();
    style.insert("token", "$hp_state".into());
    style.insert("rules", rules.into());
    let mut row = Array::new();
    row.push(style);
    row.into()
}

fn row_single(token: &str, dim: bool) -> Value {
    let mut style = InlineTable::new();
    style.insert("token", token.into());
    if dim {
        style.insert("dim", true.into());
    }
    let mut row = Array::new();
    row.push(style);
    row.into()
}

/// A card's first row: the project heading in Herdr's accent (lilac), the
/// `other` heading, the project line, or a blank that keeps the card's own
/// rows in the same column as the heading card's.
fn row_heading(note: bool) -> Value {
    let mut row = Array::new();
    let mut top = InlineTable::new();
    top.insert("token", "$hp_top".into());
    top.insert("fg", ACCENT.into());
    top.insert("bold", true.into());
    row.push(top);
    let mut other = InlineTable::new();
    other.insert("token", "$hp_other".into());
    other.insert("bold", true.into());
    other.insert("dim", true.into());
    row.push(other);
    if note {
        let mut line = InlineTable::new();
        line.insert("token", "$hp_note".into());
        line.insert("dim", true.into());
        row.push(line);
    }
    row.into()
}

/// Herdr's accent colour.
pub const ACCENT: &str = "#cba6f7";

fn same(a: &Value, b: &Value) -> bool {
    a.to_string().split_whitespace().collect::<String>() == b.to_string().split_whitespace().collect::<String>()
}

/// `first` rows go at the top, `wanted` rows at the bottom.
fn edit_rows(item: &mut Item, first: &[Value], wanted: &[Value], ours: &[&str], remove: bool) -> Result<()> {
    let rows = item.as_array_mut().context("sidebar rows must be an array")?;
    if remove {
        rows.retain(|v| !first.iter().chain(wanted).any(|w| same(v, w)));
        return Ok(());
    }
    for want in first.iter().rev() {
        if rows.iter().any(|v| same(v, want)) {
            continue;
        }
        let token = ours.iter().find(|t| want.to_string().contains(*t)).copied().unwrap_or("");
        if !token.is_empty() && rows.iter().any(|v| v.to_string().contains(&format!("\"{token}\""))) {
            continue;
        }
        ensure!(rows.len() < 16, "the sidebar already has 16 rows; remove one before configuring");
        rows.insert(0, want.clone());
    }
    for want in wanted {
        if rows.iter().any(|v| same(v, want)) {
            continue;
        }
        // A row the user edited that still carries our token stays theirs.
        let token = ours.iter().find(|t| want.to_string().contains(*t)).copied().unwrap_or("");
        if !token.is_empty() && rows.iter().any(|v| v.to_string().contains(&format!("\"{token}\""))) {
            continue;
        }
        ensure!(rows.len() < 16, "the sidebar already has 16 rows; remove one before configuring");
        rows.push(want.clone());
    }
    Ok(())
}

fn table_mut<'a>(parent: &'a mut Item, key: &str) -> Result<&'a mut Item> {
    let table = parent.as_table_like_mut().with_context(|| format!("`{key}`'s parent must be a table"))?;
    if table.get(key).is_none() {
        let mut t = Table::new();
        t.set_implicit(true);
        table.insert(key, Item::Table(t));
    }
    Ok(table.get_mut(key).unwrap())
}

/// Adds (or removes) the rows, the popup key and the tab-bar entry. Existing
/// rows, keys and entries of the user are never touched. Idempotent.
pub fn config_edit(input: &str, spec: &Spec, remove: bool) -> Result<String> {
    let mut doc = input.parse::<DocumentMut>().context("config.toml does not parse")?;
    let agent_rows = [row_state(), row_single("$hp_activity", true), row_single("$hp_tail", false)];
    let space_rows = [row_single("$hp", false), row_single("$hp_tail", false)];
    let agent_first = [row_heading(true)];
    let space_first = [row_heading(false)];
    let agent_ours = ["$hp_state", "$hp_activity", "$hp_tail", "$hp_top"];

    // Agent rows, and every per-harness override (which replaces `rows`).
    {
        let ui = table_mut(doc.as_item_mut(), "ui")?;
        let sidebar = table_mut(ui, "sidebar")?;
        let agents = table_mut(sidebar, "agents")?;
        if agents.get("rows").is_none() && !remove {
            let mut defaults = Array::new();
            let mut first = Array::new();
            for t in ["state_icon", "machine", "workspace", "tab"] {
                first.push(t);
            }
            defaults.push(first);
            let mut second = Array::new();
            second.push("agent");
            defaults.push(second);
            agents["rows"] = toml_edit::value(defaults);
        }
        if let Some(rows) = agents.get_mut("rows").filter(|v| !v.is_none()) {
            edit_rows(rows, &agent_first, &agent_rows, &agent_ours, remove)?;
        }
        if let Some(overrides) = agents.get_mut("rows_by_agent").filter(|v| !v.is_none()) {
            for (_, rows) in overrides.as_table_like_mut().context("rows_by_agent must be a table")?.iter_mut() {
                edit_rows(rows, &agent_first, &agent_rows, &agent_ours, remove)?;
            }
        }
        let spaces = table_mut(sidebar, "spaces")?;
        if spaces.get("rows").is_none() && !remove {
            let mut defaults = Array::new();
            let mut first = Array::new();
            first.push("state_icon");
            first.push("workspace");
            defaults.push(first);
            let mut second = Array::new();
            second.push("branch");
            second.push("git_status");
            defaults.push(second);
            spaces["rows"] = toml_edit::value(defaults);
        }
        if let Some(rows) = spaces.get_mut("rows").filter(|v| !v.is_none()) {
            edit_rows(rows, &space_first, &space_rows, &["$hp_tail", "$hp_top", "$hp"], remove)?;
        }

        // Tab bar: one command entry.
        let is_ours = |v: &Value| v.as_inline_table().and_then(|t| t.get("command")).and_then(Value::as_str).is_some_and(|c| c.contains("needs-you --line"));
        if ui.get("tab_bar_right").is_none() && !remove {
            ui["tab_bar_right"] = toml_edit::value(Array::new());
        }
        if let Some(entries) = ui.get_mut("tab_bar_right").and_then(Item::as_array_mut) {
            entries.retain(|v| !is_ours(v) || (!remove && v.as_inline_table().and_then(|t| t.get("command")).and_then(Value::as_str) == Some(spec.tab_command.as_str())));
            if !remove && !entries.iter().any(is_ours) {
                let mut entry = InlineTable::new();
                entry.insert("type", "command".into());
                entry.insert("command", spec.tab_command.as_str().into());
                entry.insert("interval_seconds", 15.into());
                entry.insert("timeout_seconds", 5.into());
                entries.push(entry);
            }
        }
    }

    // The popup key: one [[keys.command]] entry.
    {
        let keys = table_mut(doc.as_item_mut(), "keys")?;
        let table = keys.as_table_like_mut().context("`keys` must be a table")?;
        if table.get("command").is_none() && !remove {
            table.insert("command", Item::ArrayOfTables(ArrayOfTables::new()));
        }
        if let Some(commands) = table.get_mut("command").and_then(Item::as_array_of_tables_mut) {
            let ours = |t: &Table| t.get("command").and_then(Item::as_str) == Some(POPUP_ACTION);
            if remove {
                commands.retain(|t| !ours(t));
            } else if let Some(existing) = commands.iter_mut().find(|t| ours(t)) {
                existing["key"] = toml_edit::value(spec.key.as_str());
            } else {
                let mut entry = Table::new();
                entry["key"] = toml_edit::value(spec.key.as_str());
                entry["type"] = toml_edit::value("plugin_action");
                entry["command"] = toml_edit::value(POPUP_ACTION);
                entry["description"] = toml_edit::value("Projects");
                commands.push(entry);
            }
        }
    }
    Ok(doc.to_string())
}

/// Herdr's built-in bindings, from the commented `# action = "key"` lines
/// under `[keys]` in `herdr --default-config`.
pub fn builtin_keys(default_config: &str) -> Vec<(String, String)> {
    let mut inside = false;
    let mut keys = Vec::new();
    for line in default_config.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == "[keys]";
            continue;
        }
        if !inside {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix('#') else {
            continue;
        };
        // The commented `[[keys.command]]` example is not a binding.
        if rest.trim_start().starts_with("[[") {
            inside = false;
            continue;
        }
        let Some((action, value)) = rest.split_once('=') else {
            continue;
        };
        let action = action.trim();
        if action.is_empty() || action.contains(' ') || matches!(action, "key" | "type" | "command" | "description" | "width" | "height") {
            continue;
        }
        let value = value.trim();
        let Some(key) = value.strip_prefix('"').and_then(|v| v.split('"').next()) else {
            continue;
        };
        if !key.is_empty() {
            keys.push((action.to_string(), key.to_string()));
        }
    }
    keys
}

/// Why `key` cannot be the popup key, or `None` when it is free: bound in the
/// user's `[keys]`, by another `[[keys.command]]`, or in Herdr's built-in map
/// (unless the user rebound that action to another key).
pub fn key_conflict(config: &str, key: &str, builtin: &[(String, String)]) -> Option<String> {
    let doc = config.parse::<DocumentMut>().ok()?;
    let keys = doc.get("keys").and_then(Item::as_table_like);
    let mut overridden = Vec::new();
    if let Some(keys) = keys {
        for (action, value) in keys.iter() {
            if action == "command" {
                continue;
            }
            let bound: Vec<String> = match value {
                Item::Value(Value::String(s)) => vec![s.value().clone()],
                Item::Value(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
                _ => Vec::new(),
            };
            if bound.iter().any(|b| b == key) {
                return Some(format!("`{key}` is bound to `{action}` in your [keys]"));
            }
            overridden.push(action.to_string());
        }
        if let Some(commands) = keys.get("command").and_then(Item::as_array_of_tables) {
            for c in commands.iter() {
                if c.get("key").and_then(Item::as_str) == Some(key) && c.get("command").and_then(Item::as_str) != Some(POPUP_ACTION) {
                    return Some(format!("`{key}` already runs `{}`", c.get("command").and_then(Item::as_str).unwrap_or("a custom command")));
                }
            }
        }
    }
    builtin
        .iter()
        .find(|(action, bound)| bound == key && !overridden.contains(action))
        .map(|(action, _)| format!("`{key}` is Herdr's built-in `{action}` key"))
}

/// Checks a candidate config with `herdr config check` before it goes live.
pub fn check_config(herdr_bin: &str, runner: &dyn crate::runner::Runner, text: &str, scratch: &Path) -> Result<()> {
    std::fs::create_dir_all(scratch)?;
    let candidate = scratch.join(format!("config-check-{}.toml", std::process::id()));
    std::fs::write(&candidate, text)?;
    let out = runner.run(&crate::runner::Cmd::new(herdr_bin, CALL_TIMEOUT).env("HERDR_CONFIG_PATH", candidate.to_string_lossy()).args(["config", "check"]));
    let _ = std::fs::remove_file(&candidate);
    let out = out?;
    if !out.success() {
        bail!("Herdr rejected the proposed config ({}); nothing was changed", out.error_text());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thread::Status;

    fn spec() -> Spec {
        Spec { key: "prefix+a".into(), tab_command: "/bin/hp --root /r needs-you --line".into() }
    }

    #[test]
    fn config_edit_adds_once_keeps_user_rows_and_removes_exactly_its_own() {
        let original = "# mine\n[ui.sidebar.agents]\nrows = [[\"agent\"], [{ token = \"$github\" }]]\n[ui.sidebar.agents.rows_by_agent]\nclaude = [[\"agent\"]]\n\n[[keys.command]]\nkey = \"prefix+e\"\ntype = \"plugin_action\"\ncommand = \"other.toggle\"\n";
        let added = config_edit(original, &spec(), false).unwrap();
        assert!(added.contains("# mine") && added.contains("$github") && added.contains("other.toggle"));
        assert_eq!(added.matches("$hp_state").count(), 2, "{added}");
        assert_eq!(added.matches("$hp_activity").count(), 2);
        assert_eq!(added.matches("\"$hp\"").count(), 1);
        assert_eq!(added.matches(POPUP_ACTION).count(), 1);
        assert_eq!(added.matches("needs-you --line").count(), 1);
        assert_eq!(config_edit(&added, &spec(), false).unwrap(), added);
        // Herdr's parser sees the result as valid TOML.
        assert!(added.parse::<DocumentMut>().is_ok());

        let removed = config_edit(&added, &spec(), true).unwrap();
        for ours in ["$hp_state", "$hp_activity", "\"$hp\"", POPUP_ACTION, "needs-you"] {
            assert!(!removed.contains(ours), "{ours} left in\n{removed}");
        }
        assert!(removed.contains("$github") && removed.contains("other.toggle") && removed.contains("# mine"));

        // Another key, and a moved binary, replace ours rather than adding.
        let moved = config_edit(&added, &Spec { key: "prefix+y".into(), tab_command: "/new/hp --root /r needs-you --line".into() }, false).unwrap();
        assert_eq!(moved.matches(POPUP_ACTION).count(), 1);
        assert!(moved.contains("prefix+y") && !moved.contains("/bin/hp --root"));
        assert_eq!(moved.matches("needs-you --line").count(), 1);
    }

    #[test]
    fn the_heading_row_goes_first_and_the_tail_row_last() {
        let added = config_edit("[ui.sidebar.agents]\nrows = [[\"agent\"]]\n[ui.sidebar.spaces]\nrows = [[\"workspace\"]]\n", &spec(), false).unwrap();
        let doc = added.parse::<DocumentMut>().unwrap();
        for (section, note) in [("agents", true), ("spaces", false)] {
            let rows = doc["ui"]["sidebar"][section]["rows"].as_array().unwrap();
            let first = rows.get(0).unwrap().to_string();
            assert!(first.contains("$hp_top") && first.contains(ACCENT) && first.contains("$hp_other"), "{first}");
            assert_eq!(first.contains("$hp_note"), note);
            assert!(rows.get(rows.len() - 1).unwrap().to_string().contains("$hp_tail"));
        }
        assert_eq!(config_edit(&added, &spec(), false).unwrap(), added);
        let removed = config_edit(&added, &spec(), true).unwrap();
        assert!(!removed.contains("$hp_top") && !removed.contains("$hp_tail"), "{removed}");
    }

    #[test]
    fn an_empty_config_gets_herdrs_default_rows_plus_ours() {
        let added = config_edit("", &spec(), false).unwrap();
        assert!(added.contains("\"state_icon\", \"machine\", \"workspace\", \"tab\""), "{added}");
        assert!(added.contains("\"branch\", \"git_status\""));
        let full = format!("[ui.sidebar.agents]\nrows = [{}]\n", vec!["[\"agent\"]"; 16].join(","));
        assert!(config_edit(&full, &spec(), false).is_err());
    }

    #[test]
    fn keys_are_checked_against_the_users_and_herdrs_bindings() {
        let defaults = "[ui]\n# x = 1\n[keys]\n# prefix = \"ctrl+b\"\n# previous_tab = \"prefix+p\"\n# rename_pane = \"prefix+shift+p\"\n# open_worktree = \"\"    # optional\n# [[keys.command]]\n# key = \"prefix+alt+g\"\n# command = \"lazygit\"\n[other]\n# nope = \"prefix+a\"\n";
        let builtin = builtin_keys(defaults);
        assert_eq!(builtin, [("prefix".to_string(), "ctrl+b".to_string()), ("previous_tab".into(), "prefix+p".into()), ("rename_pane".into(), "prefix+shift+p".into())]);
        assert!(key_conflict("", "prefix+a", &builtin).is_none());
        assert!(key_conflict("", "prefix+p", &builtin).unwrap().contains("previous_tab"));
        // Rebinding previous_tab frees prefix+p.
        assert!(key_conflict("[keys]\nprevious_tab = \"prefix+[\"\n", "prefix+p", &builtin).is_none());
        assert!(key_conflict("[keys]\ndetach = \"prefix+a\"\n", "prefix+a", &builtin).unwrap().contains("detach"));
        let taken = "[[keys.command]]\nkey = \"prefix+a\"\ntype = \"pane\"\ncommand = \"lazygit\"\n";
        assert!(key_conflict(taken, "prefix+a", &builtin).unwrap().contains("lazygit"));
        let ours = "[[keys.command]]\nkey = \"prefix+a\"\ntype = \"plugin_action\"\ncommand = \"herdr-projects.open-popup\"\n";
        assert!(key_conflict(ours, "prefix+a", &builtin).is_none());
    }

    fn live(state: &str) -> Live {
        Live { pane_exists: true, agent_state: Some(state.into()), ..Live::default() }
    }

    #[test]
    fn state_lines_carry_the_word_and_one_fact() {
        let now: jiff::Timestamp = "2026-09-23T12:00:00Z".parse().unwrap();
        let t = Thread { status: Status::Open, last_state_change: "2026-09-23T11:40:00Z".into(), pr: "https://github.com/o/r/pull/4".into(), ..Thread::default() };
        let with = |activity: &str, percent: Option<u8>, age: i64, state: &str| {
            let record = crate::progress::Record { activity: activity.into(), percent, reported_at: now.as_second() - age, ..Default::default() };
            Live { self_report: Some(record), report_age_secs: age, ..live(state) }
        };
        assert_eq!(state_line(Group::WaitingOnYou, &t, &with("Waiting for you", Some(55), 10, "idle"), now), "needs you · ~55%");
        assert_eq!(state_line(Group::WaitingOnYou, &t, &live("blocked"), now), "needs you · blocked");
        assert_eq!(state_line(Group::WaitingOnYou, &t, &Live::default(), now), "needs you · pane closed");
        assert_eq!(state_line(Group::ReadyForReview, &t, &live("idle"), now), "review · PR #4");
        assert_eq!(state_line(Group::Landing, &t, &live("idle"), now), "landing · PR #4");
        assert_eq!(state_line(Group::Working, &t, &with("Testing", Some(40), 30, "working"), now), "working · ~40%");
        // A harness without self-reports, silent for 20 minutes.
        assert_eq!(state_line(Group::Working, &t, &live("working"), now), "working · 20m quiet");
        assert_eq!(state_line(Group::Working, &t, &with("Testing", Some(40), 720, "working"), now), "working · 12m quiet");
        assert_eq!(state_line(Group::Idle, &t, &live("idle"), now), "idle");
        let no_pr = Thread { pr: String::new(), ..t };
        assert_eq!(state_line(Group::ReadyForReview, &no_pr, &live("idle"), now), "review · report");
    }

    #[test]
    fn project_lines_count_what_needs_you() {
        use Group::*;
        assert_eq!(project_line(&[WaitingOnYou, ReadyForReview, Working, Working, Working, Idle], false), "2 need you · 3 working");
        assert_eq!(project_line(&[Landing], false), "1 landing");
        assert_eq!(project_line(&[Idle], false), "idle");
        assert_eq!(project_line(&[WaitingOnYou], true), "paused");
    }
}

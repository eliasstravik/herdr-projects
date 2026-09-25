//! What the Herdr sidebar shows for projects: per agent row a name
//! (`--display-agent`); the project rails of `grouping` in the agents and the
//! Spaces list; a tab-bar count; the default agent view, grouped by project.
//! Also the `config.toml` edit `configure` makes to render them.

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

/// A coordinator's row name. The rail's corner above it names the project.
pub fn coordinator_display() -> String {
    "coordinator".into()
}

/// Reports one pane's row: display name, project and rank, with a TTL so the
/// row falls back to Herdr's own when the ticker stops. Never `--seq`: token
/// patches are per key and the rail tokens come from `grouping`.
pub fn report_pane(herdr: &Herdr, pane: &str, display: &str, slug: &str, group: Group) {
    let ttl = TOKEN_TTL_MS.to_string();
    let rank = group.rank().to_string();
    let tokens = [format!("hp_project={slug}"), format!("hp_rank={rank}")];
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
    for name in ["hp_project", "hp_rank"].into_iter().chain(OLD_TOKENS) {
        args.push("--clear-token");
        args.push(name);
    }
    let _ = herdr.call(&args, CALL_TIMEOUT);
    crate::grouping::clear(herdr, "pane", pane);
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

/// Clears the rail tokens of a Space.
pub fn clear_workspace(herdr: &Herdr, workspace: &str) {
    crate::grouping::clear(herdr, "workspace", workspace);
}

/// A card's sub-line: what the state line adds to Herdr's own state word,
/// then the agent's last activity. `working · ~40%` and `Writing tests` give
/// `~40% · Writing tests`; a bare `idle` or `working` gives nothing.
pub fn sub_line(state_line: &str, activity: &str) -> String {
    let fact = match state_line {
        "idle" | "working" | "resolved" => "",
        line => line.strip_prefix("working · ").unwrap_or(line),
    };
    [fact, activity.trim()].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" · ")
}

/// A project's rail: needs you over working over idle.
pub fn rail(groups: &[Group], paused: bool) -> crate::grouping::Rail {
    use crate::grouping::Rail;
    if paused {
        Rail::Idle
    } else if groups.iter().any(|g| needs_you(*g)) {
        Rail::Needs
    } else if groups.iter().any(|g| matches!(g, Group::Working | Group::Landing)) {
        Rail::Working
    } else {
        Rail::Idle
    }
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

/// A row of every state's rail token for `slots` (row 0 holds the corner
/// and the connector, never both on one card).
fn rail_row(slots: &[&str]) -> Value {
    use crate::grouping::{Rail, token};
    let mut row = Array::new();
    for slot in slots {
        for rail in Rail::ALL {
            let mut t = InlineTable::new();
            t.insert("token", format!("${}", token(slot, rail)).into());
            t.insert("fg", rail.color().into());
            if *slot == "top" {
                t.insert("bold", true.into());
            }
            if rail == Rail::Other {
                t.insert("dim", true.into());
            }
            row.push(t);
        }
    }
    row.into()
}

fn row_gap() -> Value {
    let mut t = InlineTable::new();
    t.insert("token", "$hp_gap".into());
    let mut row = Array::new();
    row.push(t);
    row.into()
}

fn row_of(tokens: &[&str]) -> Value {
    let mut row = Array::new();
    for t in tokens {
        row.push(*t);
    }
    row.into()
}

/// Herdr's built-in rows (0.9.1), which `configure` wrote into a config that
/// had none.
fn herdr_agent_rows() -> [Value; 2] {
    [row_of(&["state_icon", "machine", "workspace", "tab"]), row_of(&["agent"])]
}

fn herdr_space_rows() -> [Value; 2] {
    [row_of(&["state_icon", "workspace"]), row_of(&["branch", "git_status"])]
}

/// The agent card that replaces Herdr's built-in rows: one line, status icon
/// on the rail, then the name and Herdr's state word.
fn agent_card() -> Value {
    let mut row = Array::new();
    row.push("state_icon");
    let mut agent = InlineTable::new();
    agent.insert("token", "agent".into());
    agent.insert("bold", true.into());
    agent.insert("dim", false.into());
    row.push(agent);
    row.push("state_text");
    row.into()
}

/// The Space card that replaces Herdr's built-in rows: one line, with the
/// branch after the name so no row of Herdr's sits on the rail, and `home`
/// in place of a home Space's name (see `grouping::HOME_MARK`).
fn space_card() -> Value {
    let mut row = Array::new();
    row.push("state_icon");
    let mut rule = InlineTable::new();
    rule.insert("contains", crate::grouping::HOME_MARK.to_string().into());
    rule.insert("hide", true.into());
    let mut rules = Array::new();
    rules.push(rule);
    let mut workspace = InlineTable::new();
    workspace.insert("token", "workspace".into());
    workspace.insert("rules", rules.into());
    row.push(workspace);
    let mut home = InlineTable::new();
    home.insert("token", "$hp_home".into());
    row.push(home);
    let mut branch = InlineTable::new();
    branch.insert("token", "branch".into());
    branch.insert("dim", true.into());
    row.push(branch);
    row.push("git_status");
    row.into()
}

/// The 0.2.17/0.2.18 row tokens: a row made only of these is removed.
const LEGACY_ROW_TOKENS: [&str; 7] = ["$hp_top", "$hp_other", "$hp_note", "$hp_state", "$hp_activity", "$hp_tail", "$hp"];

/// The tokens a row names, in order.
fn row_tokens(row: &Value) -> Vec<String> {
    row.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| match v {
                    Value::String(s) => Some(s.value().clone()),
                    Value::InlineTable(t) => t.get("token").and_then(Value::as_str).map(str::to_string),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// A row this plugin owns: every token is one of ours (now or before 0.2.19).
fn ours(row: &Value) -> bool {
    let current: Vec<String> = crate::grouping::tokens().iter().map(|t| format!("${t}")).collect();
    let tokens = row_tokens(row);
    !tokens.is_empty() && tokens.iter().all(|t| LEGACY_ROW_TOKENS.contains(&t.as_str()) || current.contains(t))
}

fn same(a: &Value, b: &Value) -> bool {
    a.to_string().split_whitespace().collect::<String>() == b.to_string().split_whitespace().collect::<String>()
}

/// Rebuilds one `rows` array: drops every row of ours (today's and the
/// 0.2.17/0.2.18 ones), swaps Herdr's built-in rows for `card` when they are
/// exactly Herdr's (never rows the user wrote), then puts `first` at the top
/// and `last` at the bottom. With `remove`, only drops ours and puts Herdr's
/// built-in rows back in place of `card`.
fn edit_rows(item: &mut Item, first: &[Value], last: &[Value], builtin: &[Value], card: Option<&Value>, remove: bool) -> Result<()> {
    let rows = item.as_array_mut().context("sidebar rows must be an array")?;
    rows.retain(|v| !ours(v));
    let current: Vec<Value> = rows.iter().cloned().collect();
    if let Some(card) = card {
        let replace = |rows: &mut Array, with: &[Value]| {
            rows.clear();
            for row in with {
                rows.push(row.clone());
            }
        };
        let is_builtin = current.len() == builtin.len() && current.iter().zip(builtin).all(|(a, b)| same(a, b));
        let is_card = current.len() == 1 && same(&current[0], card);
        if !remove && is_builtin {
            replace(rows, std::slice::from_ref(card));
        } else if remove && is_card && row_tokens(card).iter().any(|t| t.starts_with("$hp")) {
            // Only a card that names a token of ours is ours to take back.
            replace(rows, builtin);
        }
    }
    if remove {
        return Ok(());
    }
    ensure!(rows.len() + first.len() + last.len() <= 16, "the sidebar already has {} rows; remove one before configuring", rows.len());
    for row in first.iter().rev() {
        rows.insert(0, row.clone());
    }
    for row in last {
        rows.push(row.clone());
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
    let agent_first = [rail_row(&["top", "con"])];
    let agent_last = [rail_row(&["sub"]), rail_row(&["end"]), row_gap()];
    let space_first = [rail_row(&["top", "con"])];
    let space_last = [rail_row(&["end"]), row_gap()];
    let (agent_card, space_card) = (agent_card(), space_card());

    // Agent rows, and every per-harness override (which replaces `rows`).
    {
        let ui = table_mut(doc.as_item_mut(), "ui")?;
        let sidebar = table_mut(ui, "sidebar")?;
        let agents = table_mut(sidebar, "agents")?;
        if agents.get("rows").is_none() && !remove {
            agents["rows"] = toml_edit::value(Array::from_iter(herdr_agent_rows()));
        }
        if let Some(rows) = agents.get_mut("rows").filter(|v| !v.is_none()) {
            edit_rows(rows, &agent_first, &agent_last, &herdr_agent_rows(), Some(&agent_card), remove)?;
        }
        if let Some(overrides) = agents.get_mut("rows_by_agent").filter(|v| !v.is_none()) {
            for (_, rows) in overrides.as_table_like_mut().context("rows_by_agent must be a table")?.iter_mut() {
                edit_rows(rows, &agent_first, &agent_last, &herdr_agent_rows(), None, remove)?;
            }
        }
        let spaces = table_mut(sidebar, "spaces")?;
        if spaces.get("rows").is_none() && !remove {
            spaces["rows"] = toml_edit::value(Array::from_iter(herdr_space_rows()));
        }
        if let Some(rows) = spaces.get_mut("rows").filter(|v| !v.is_none()) {
            edit_rows(rows, &space_first, &space_last, &herdr_space_rows(), Some(&space_card), remove)?;
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

    fn rows(text: &str, section: &str) -> Vec<String> {
        let doc = text.parse::<DocumentMut>().unwrap();
        doc["ui"]["sidebar"][section]["rows"].as_array().unwrap().iter().map(|r| r.to_string()).collect()
    }

    #[test]
    fn config_edit_adds_once_keeps_user_rows_and_removes_exactly_its_own() {
        let original = "# mine\n[ui.sidebar.agents]\nrows = [[\"agent\"], [{ token = \"$github\" }]]\n[ui.sidebar.agents.rows_by_agent]\nclaude = [[\"agent\"]]\n\n[[keys.command]]\nkey = \"prefix+e\"\ntype = \"plugin_action\"\ncommand = \"other.toggle\"\n";
        let added = config_edit(original, &spec(), false).unwrap();
        assert!(added.contains("# mine") && added.contains("$github") && added.contains("other.toggle"));
        assert_eq!(added.matches("\"$hp_top_n\"").count(), 3, "agents, the override and spaces: {added}");
        assert_eq!(added.matches("\"$hp_sub_w\"").count(), 2);
        assert_eq!(added.matches(POPUP_ACTION).count(), 1);
        assert_eq!(added.matches("needs-you --line").count(), 1);
        assert_eq!(config_edit(&added, &spec(), false).unwrap(), added);
        // Herdr's parser sees the result as valid TOML.
        assert!(added.parse::<DocumentMut>().is_ok());
        // The user's own rows stay theirs, between the corner and the sub-line.
        let agents = rows(&added, "agents");
        assert!(agents[0].contains("$hp_top_w") && agents[0].contains("$hp_con_w"));
        assert_eq!(agents[1].replace(' ', ""), "[\"agent\"]");
        assert!(agents[2].contains("$github"));
        assert!(agents[3].contains("$hp_sub_n") && agents[4].contains("$hp_end_i") && agents[5].contains("$hp_gap"));

        let removed = config_edit(&added, &spec(), true).unwrap();
        for ours in ["$hp_", POPUP_ACTION, "needs-you"] {
            assert!(!removed.contains(ours), "{ours} left in\n{removed}");
        }
        assert!(removed.contains("$github") && removed.contains("other.toggle") && removed.contains("# mine"));
        assert_eq!(rows(&removed, "spaces").len(), 2, "Herdr's own Space rows come back: {removed}");

        // Another key, and a moved binary, replace ours rather than adding.
        let moved = config_edit(&added, &Spec { key: "prefix+y".into(), tab_command: "/new/hp --root /r needs-you --line".into() }, false).unwrap();
        assert_eq!(moved.matches(POPUP_ACTION).count(), 1);
        assert!(moved.contains("prefix+y") && !moved.contains("/bin/hp --root"));
        assert_eq!(moved.matches("needs-you --line").count(), 1);
    }

    #[test]
    fn herdrs_built_in_rows_become_one_line_cards_and_user_rows_stay() {
        let added = config_edit("", &spec(), false).unwrap();
        let agents = rows(&added, "agents");
        assert_eq!(agents.len(), 5, "{added}");
        assert!(agents[1].contains("\"state_icon\"") && agents[1].contains("\"agent\"") && agents[1].contains("\"state_text\"") && !agents[1].contains("machine"));
        let spaces = rows(&added, "spaces");
        assert_eq!(spaces.len(), 4, "{added}");
        assert!(spaces[1].contains("$hp_home") && spaces[1].contains("hide = true") && spaces[1].contains("\"branch\""));
        assert!(spaces[1].contains(crate::grouping::HOME_MARK));
        // Rows the user wrote are never swapped.
        let mine = "[ui.sidebar.spaces]\nrows = [[\"workspace\"], [\"branch\"]]\n";
        let kept = rows(&config_edit(mine, &spec(), false).unwrap(), "spaces");
        assert_eq!(kept[1].replace(' ', ""), "[\"workspace\"]");
        let full = format!("[ui.sidebar.agents]\nrows = [{}]\n", vec!["[\"agent\"]"; 14].join(","));
        assert!(config_edit(&full, &spec(), false).is_err());
    }

    #[test]
    fn the_0_2_18_rows_are_migrated_with_nothing_left_over() {
        // The M1's config as 0.2.18 left it: Herdr's rows plus the headings.
        let old = r##"[ui]
sidebar = { agents = { rows = [[{ token = "$hp_top", fg = "#cba6f7", bold = true }, { token = "$hp_other", bold = true, dim = true }, { token = "$hp_note", dim = true }],["state_icon", "machine", "workspace", "tab"], ["agent"], [{ token = "$hp_state", rules = [{ starts_with = "needs you", fg = "#f38ba8", bold = true }, { starts_with = "review", fg = "#f9e2af" }] }], [{ token = "$hp_activity", dim = true }], [{ token = "$hp_tail" }]] } , spaces = { rows = [[{ token = "$hp_top", fg = "#cba6f7", bold = true }, { token = "$hp_other", bold = true, dim = true }],["state_icon", "workspace"], ["branch", "git_status"], [{ token = "$hp" }], [{ token = "$hp_tail" }]] } }
"##;
        let migrated = config_edit(old, &spec(), false).unwrap();
        for legacy in ["\"$hp_top\"", "\"$hp_other\"", "\"$hp_note\"", "\"$hp_state\"", "\"$hp_activity\"", "\"$hp_tail\"", "\"$hp\""] {
            assert!(!migrated.contains(legacy), "{legacy} left in\n{migrated}");
        }
        assert_eq!(rows(&migrated, "agents").len(), 5, "{migrated}");
        assert!(!rows(&migrated, "agents")[1].contains("machine"), "Herdr's rows became the card");
        assert_eq!(rows(&migrated, "spaces").len(), 4);
        assert_eq!(config_edit(&migrated, &spec(), false).unwrap(), migrated);
        // The Mac mini's own card and $github row survive untouched.
        let mine = r##"[ui.sidebar.agents]
rows = [[{ token = "$hp_top", fg = "#cba6f7", bold = true }, { token = "$hp_other", bold = true, dim = true }, { token = "$hp_note", dim = true }],
  ["state_icon", { token = "agent", bold = true, dim = false }, "state_text"],
  [{ token = "$github", dim = false }], [{ token = "$hp_state" }], [{ token = "$hp_activity", dim = true }], [{ token = "$hp_tail" }],
]
"##;
        let agents = rows(&config_edit(mine, &spec(), false).unwrap(), "agents");
        assert_eq!(agents.len(), 6);
        assert!(agents[1].contains("state_text") && agents[2].contains("$github"));
    }

    #[test]
    fn sub_lines_keep_what_herdrs_state_word_does_not_say() {
        assert_eq!(sub_line("working · ~40%", "Writing tests"), "~40% · Writing tests");
        assert_eq!(sub_line("working", ""), "");
        assert_eq!(sub_line("idle", "Done"), "Done");
        assert_eq!(sub_line("review · report", ""), "review · report");
        assert_eq!(sub_line("needs you · blocked", " Waiting for you "), "needs you · blocked · Waiting for you");
        assert_eq!(sub_line("", "Reading code"), "Reading code");
    }

    #[test]
    fn a_rail_says_needs_you_over_working_over_idle() {
        use crate::grouping::Rail;
        use Group::*;
        assert_eq!(rail(&[Idle, Working, ReadyForReview], false), Rail::Needs);
        assert_eq!(rail(&[Idle, Landing], false), Rail::Working);
        assert_eq!(rail(&[Idle], false), Rail::Idle);
        assert_eq!(rail(&[WaitingOnYou], true), Rail::Idle);
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

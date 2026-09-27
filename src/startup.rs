//! User-owned, opt-in native onboarding. No task/tool approval automation.
use crate::{
    herdr::Herdr,
    paths::Ctx,
    project::{self, Project},
    thread,
};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

#[derive(Default, Deserialize, Clone, Copy)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    pub folder_trust: bool,
    pub mcp_enablement: bool,
}

pub fn policy(config_dir: &Path) -> Result<Policy> {
    #[derive(Default, Deserialize)]
    struct Config {
        #[serde(default)]
        startup: Policy,
    }
    let path = config_dir.join("config.toml");
    if !path.exists() {
        return Ok(Policy::default());
    }
    Ok(toml::from_str::<Config>(&std::fs::read_to_string(path)?)?.startup)
}

#[derive(Debug, PartialEq)]
struct Action {
    category: &'static str,
    key: &'static str,
}

fn row(line: &str) -> (&str, bool) {
    let line = line.trim();
    let focused = line.starts_with('❯') || line.starts_with('›');
    let line = line.trim_start_matches(['❯', '›']).trim();
    let line = if let Some((n, rest)) = line.split_once(". ") {
        if n.chars().all(|c| c.is_ascii_digit()) {
            rest
        } else {
            line
        }
    } else {
        line
    };
    (line.trim(), focused)
}

fn choose(
    lines: &[String],
    labels: &[&str],
    target: usize,
    category: &'static str,
) -> Option<Action> {
    if lines.iter().filter(|l| row(l).1).count() != 1 {
        return None;
    }
    let rows: Vec<_> = lines
        .iter()
        .map(|l| row(l))
        .filter(|(text, _)| labels.contains(text))
        .collect();
    if rows.len() != labels.len()
        || rows
            .iter()
            .zip(labels)
            .any(|((text, _), label)| text != label)
    {
        return None;
    }
    let selected: Vec<_> = rows
        .iter()
        .enumerate()
        .filter(|(_, (_, focused))| *focused)
        .map(|(i, _)| i)
        .collect();
    if selected.len() != 1 {
        return None;
    }
    Some(Action {
        category,
        key: if selected[0] == target {
            "enter"
        } else if selected[0] < target {
            "down"
        } else {
            "up"
        },
    })
}

/// Exact known native menu structures, never a generic Yes/Allow matcher.
fn action(kind: &str, screen: &str, policy: Policy) -> Option<Action> {
    let lines = crate::prompt_box::plain_lines(screen);
    let has = |s: &str| lines.iter().any(|l| l.trim() == s);
    let starts = |s: &str| lines.iter().any(|l| l.trim().starts_with(s));
    // Hook trust and managed-settings approval are deliberately outside scope.
    if starts("Hooks need review") || has("Managed settings require approval") {
        return None;
    }
    // Native notices wrap at pane width. Anchor at the question's line,
    // then normalize whitespace across its continuation lines, not the menu.
    let codex_notice = lines
        .iter()
        .position(|line| line.trim().starts_with("Trust this folder?"))
        .map(|start| {
            lines[start..]
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    if kind == "codex"
        && policy.folder_trust
        && has("Folder access")
        && codex_notice.starts_with("Trust this folder? Codex can read, edit, and run files here,")
    {
        return choose(&lines, &["Trust and continue", "Quit"], 0, "folder_trust");
    }
    if kind != "claude" {
        return None;
    }
    if policy.folder_trust
        && (has("Accessing workspace:") || has("Do you trust the files in this folder?"))
    {
        for labels in [
            vec!["No, exit", "Yes, I trust this folder"],
            vec![
                "No, continue without these permissions",
                "Yes, I trust this folder",
            ],
            vec!["Yes, I trust this folder", "No, exit"],
        ] {
            if let Some(a) = choose(
                &lines,
                &labels,
                labels
                    .iter()
                    .position(|l| *l == "Yes, I trust this folder")?,
                "folder_trust",
            ) {
                return Some(a);
            }
        }
    }
    if !policy.mcp_enablement {
        return None;
    }
    if starts("New MCP server found in this project:") {
        return choose(
            &lines,
            &[
                "Use this MCP server",
                "Use this and all future MCP servers in this project",
                "Continue without using this MCP server",
            ],
            1,
            "mcp_enablement",
        );
    }
    // Require the entire checkbox list to be visible; never submit a partial
    // scroll viewport. Every checkbox is explicitly checked before submission.
    let count = lines.iter().find_map(|l| {
        l.trim()
            .strip_suffix(" new MCP servers found in this project")
            .and_then(|n| n.parse::<usize>().ok())
    })?;
    if !has("Select any you wish to enable.") || !(2..=32).contains(&count) {
        return None;
    }
    let rows: Vec<_> = lines
        .iter()
        .map(|l| row(l))
        .filter(|(l, _)| l.starts_with("[ ] ") || l.starts_with("[✓] ") || *l == "Enable selected")
        .collect();
    if rows.len() != count + 1 || rows.last()?.0 != "Enable selected" {
        return None;
    }
    let selected: Vec<_> = rows
        .iter()
        .enumerate()
        .filter(|(_, (_, focused))| *focused)
        .map(|(i, _)| i)
        .collect();
    if selected.len() != 1 {
        return None;
    }
    let target = rows
        .iter()
        .position(|(l, _)| l.starts_with("[ ] "))
        .unwrap_or(count);
    Some(Action {
        category: "mcp_enablement",
        key: if selected[0] < target {
            "down"
        } else if selected[0] > target {
            "up"
        } else if target == count {
            "enter"
        } else {
            "space"
        },
    })
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    identity: String,
    complete: bool,
    ready_since: String,
    observed: String,
    observed_since: String,
    claims: Vec<String>,
}

/// One bounded poll, shared by the ticker and Buzz adapter. Persist the key
/// claim before sending it: an ambiguous failure must not replay Enter.
pub fn advance(
    config_dir: &Path,
    project: &Project,
    herdr: &Herdr,
    pane: &str,
) -> Result<&'static str> {
    advance_at(config_dir, project, herdr, pane, jiff::Timestamp::now())
}

pub(crate) fn advance_at(
    config_dir: &Path,
    project: &Project,
    herdr: &Herdr,
    pane: &str,
    now: jiff::Timestamp,
) -> Result<&'static str> {
    let policy = policy(config_dir)?;
    if !policy.folder_trust && !policy.mcp_enablement {
        return Ok("disabled");
    }
    let _lock = project.lock()?;
    let agents = herdr.agent_list()?;
    let Some(agent) = agents.iter().find(|a| a.pane_id == pane) else {
        return Ok("waiting");
    };
    let home = project.canonical_dir().to_string_lossy().into_owned();
    let records = thread::list(project);
    let worker = records.iter().find(|t| thread::agent_matches(t, agent));
    let managed = match worker {
        Some(t) => t.status == thread::Status::Open && t.prompt_pending,
        None => agent.works_in(&home),
    };
    if !managed {
        bail!("startup target is not a pending worker or coordinator of this project");
    }
    if agent.terminal_id.is_empty() {
        return Ok("waiting");
    }
    let path = project.state_dir().join("startup.json");
    let mut states: BTreeMap<String, State> = if path.exists() {
        serde_json::from_slice(&std::fs::read(&path)?)?
    } else {
        BTreeMap::new()
    };
    let state = states
        .entry(format!("{}:{pane}", herdr.scope()))
        .or_default();
    let identity = format!("{:?}", (&herdr.scope(), &agent.terminal_id, &agent.agent));
    if state.identity != identity {
        *state = State {
            identity,
            ..State::default()
        };
    }
    if state.complete {
        return Ok("complete");
    }
    // Once real work starts, this terminal can never approve onboarding again.
    if agent.agent_status == "working" {
        state.complete = true;
        project::write_json(&path, &states)?;
        return Ok("complete");
    }
    let screen = herdr.agent_screen(pane)?;
    let Some(action) = action(&agent.agent, &screen, policy) else {
        state.observed.clear();
        state.observed_since.clear();
        if agent.ready() && crate::prompt_box::ready_for_brief(&agent.agent, &screen) {
            if state.ready_since.is_empty() {
                state.ready_since = now.to_string();
            }
            state.complete = thread::seconds_since(&state.ready_since, now) >= 3;
        } else {
            state.ready_since.clear();
        }
        let result = if state.complete {
            "complete"
        } else if agent.agent_status == "blocked" || crate::prompt_box::trust_dialog(&screen) {
            "manual"
        } else {
            "waiting"
        };
        project::write_json(&path, &states)?;
        return Ok(result);
    };
    state.ready_since.clear();
    let digest = Sha256::digest(
        crate::prompt_box::plain_lines(&screen)
            .join("\n")
            .as_bytes(),
    );
    let claim = format!("{}:{}:{digest:x}", action.category, action.key);
    if state.claims.contains(&claim) || state.claims.len() >= 128 {
        return Ok("held");
    }
    let age = state
        .observed_since
        .parse::<jiff::Timestamp>()
        .ok()
        .map(|t| (now.as_millisecond() - t.as_millisecond()) / 1000);
    if state.observed != claim || !age.is_some_and(|age| (0..=90).contains(&age)) {
        state.observed = claim;
        state.observed_since = now.to_string();
        project::write_json(&path, &states)?;
        return Ok("waiting");
    }
    if age.unwrap_or(0) < 3 {
        return Ok("waiting");
    }

    // Re-read native identity and exact screen immediately before the write.
    let fresh = herdr.agent_list()?;
    if !fresh.iter().any(|a| {
        a.pane_id == pane
            && a.terminal_id == agent.terminal_id
            && a.agent == agent.agent
            && a.agent_status == agent.agent_status
            && a.state_change_seq == agent.state_change_seq
            && a.cwd == agent.cwd
    }) || herdr.agent_screen(pane)? != screen
    {
        return Ok("waiting");
    }
    state.claims.push(claim);
    state.observed.clear();
    state.observed_since.clear();
    project::write_json(&path, &states)?;
    herdr.agent_send_keys(pane, &[action.key.to_string()])?;
    Ok("accepted")
}

pub fn command(ctx: &Ctx, slug: &str, pane: &str) -> Result<()> {
    let project = Project::load(&ctx.root, slug)?;
    let record = project
        .coordinator()
        .ok_or_else(|| anyhow::anyhow!("project has no recorded Herdr socket"))?;
    let herdr = Herdr::new(ctx.env.herdr_bin(), &record.socket, ctx.runner);
    let state = advance(&ctx.config_dir, &project, &herdr, pane)?;
    println!("{}", serde_json::json!({"state": state}));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const ON: Policy = Policy {
        folder_trust: true,
        mcp_enablement: true,
    };
    const CLAUDE: &str = "Accessing workspace:\n/srv/demo\n❯ No, exit\n  Yes, I trust this folder\nEnter to confirm · Esc to cancel";
    const CODEX: &str = "Folder access\n/srv/demo\nTrust this folder? Codex can read, edit, and run files here,\nsubject to your permission settings.\n› 1. Trust and continue\n  2. Quit\nenter continue and create sandbox · esc quit";
    const MCP: &str = "New MCP server found in this project: example\n  Use this MCP server\n  Use this and all future MCP servers in this project\n❯ Continue without using this MCP server\nEnter to confirm";

    #[test]
    fn exact_folder_choosers_navigate_then_confirm_selected_accept() {
        assert_eq!(action("claude", CLAUDE, ON).unwrap().key, "down");
        assert_eq!(
            action(
                "claude",
                &CLAUDE.replace("❯ No", "  No").replace("  Yes", "❯ Yes"),
                ON
            )
            .unwrap()
            .key,
            "enter"
        );
        assert_eq!(action("codex", CODEX, ON).unwrap().key, "enter");
        assert_eq!(
            action(
                "codex",
                &CODEX.replace("› 1.", "  1.").replace("  2.", "› 2."),
                ON
            )
            .unwrap()
            .key,
            "up"
        );
        assert!(action("pi", CODEX, ON).is_none());
        assert!(action("claude", CLAUDE, Policy::default()).is_none());
        assert!(
            action(
                "claude",
                CLAUDE,
                Policy {
                    mcp_enablement: true,
                    ..Policy::default()
                }
            )
            .is_none()
        );
    }

    #[test]
    fn codex_narrow_pane_wrapped_folder_notice_keeps_exact_menu_guard() {
        // Live worker report: the first line ends at "files", the next at "Folder".
        let screen = "  Folder access\n  /srv/typefree\n\n  Trust this folder? Codex can read, edit, and run files\n  here, subject to your permission settings. Folder\n  settings can run code automatically, even without a\n  model request. Continue only if you trust these files.\n  Your trust decision will be saved.\n\n› 1. Trust and continue\n  2. Quit\n\n  enter continue · esc quit";
        assert_eq!(action("codex", screen, ON).unwrap().key, "enter");
        let quit_selected = screen.replace("› 1.", "  1.").replace("  2.", "› 2.");
        assert_eq!(action("codex", &quit_selected, ON).unwrap().key, "up");
        for changed in [
            screen.replace("Folder access", "Tool approval"),
            screen.replace("Trust and continue", "Allow command"),
            screen.replace("  Trust this folder?", "  Quoted: Trust this folder?"),
            screen.replace("  here, subject", "  elsewhere, subject"),
        ] {
            assert!(action("codex", &changed, ON).is_none());
        }
        assert!(action("codex", screen, Policy::default()).is_none());
    }

    #[test]
    fn single_mcp_prefers_accept_all_and_multiselect_checks_every_visible_server() {
        assert_eq!(action("claude", MCP, ON).unwrap().key, "up");
        let selected = MCP
            .replace("❯ Continue", "  Continue")
            .replace("  Use this and", "❯ Use this and");
        assert_eq!(action("claude", &selected, ON).unwrap().key, "enter");
        assert!(
            action(
                "claude",
                MCP,
                Policy {
                    folder_trust: true,
                    ..Policy::default()
                }
            )
            .is_none()
        );
        let multi = "2 new MCP servers found in this project\nSelect any you wish to enable.\n❯ [ ] first\n  [✓] second\n  Enable selected\nSpace to select · Esc to reject all";
        assert_eq!(action("claude", multi, ON).unwrap().key, "space");
        let checked = multi.replace("[ ]", "[✓]");
        assert_eq!(action("claude", &checked, ON).unwrap().key, "down");
        let submit = checked
            .replace("❯ [✓]", "  [✓]")
            .replace("  Enable selected", "❯ Enable selected");
        assert_eq!(action("claude", &submit, ON).unwrap().key, "enter");
        assert!(
            action("claude", &submit.replace("2 new", "8 new"), ON).is_none(),
            "a scrolled list cannot approve hidden rows"
        );
    }

    #[test]
    fn unknown_ambiguous_tool_hook_and_auth_prompts_never_get_keys() {
        for screen in [
            "Would you like to run the following command?\n❯ Yes, proceed\n  No",
            "Do you want to delete these files?\n❯ Yes\n  No",
            "Hooks need review\n❯ Trust all and continue\n  Quit",
            "MCP Expo OAuth authentication required\n❯ Sign in\n  Cancel",
            "Managed settings require approval\n❯ Yes, I trust these settings\n  No, exit Claude Code",
            "New MCP server found in this project: example\n❯ Allow tool execution\n  Deny",
        ] {
            assert!(action("claude", screen, ON).is_none());
            assert!(action("codex", screen, ON).is_none());
        }
        assert!(action("claude", &CLAUDE.replace("❯", " "), ON).is_none());
        assert!(action("claude", &CLAUDE.replace("  Yes", "❯ Yes"), ON).is_none());
        assert!(
            action(
                "claude",
                &format!("{CLAUDE}\n❯ unrelated tool approval"),
                ON
            )
            .is_none()
        );
        assert!(
            action(
                "claude",
                &CLAUDE.replace("Accessing workspace:", "Assistant quoted:"),
                ON
            )
            .is_none()
        );
        let ansi = CLAUDE.replace("❯ No, exit", "\u{1b}[31m❯ No, exit\u{1b}[0m");
        assert_eq!(action("claude", &ansi, ON).unwrap().key, "down");
    }

    #[test]
    fn policy_is_persistent_user_config_and_defaults_off() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!policy(dir.path()).unwrap().folder_trust);
        std::fs::write(
            dir.path().join("config.toml"),
            "[startup]\nfolder_trust = true\nmcp_enablement = true\n",
        )
        .unwrap();
        assert!(policy(dir.path()).unwrap().mcp_enablement);
        std::fs::write(
            dir.path().join("config.toml"),
            "[startup]\nallow_everything = true\n",
        )
        .unwrap();
        assert!(policy(dir.path()).is_err());
    }
}

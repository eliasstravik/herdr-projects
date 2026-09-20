//! Typed calls to the herdr CLI. Differences between herdr versions stay here.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::runner::{Cmd, Output, Runner};

pub const MIN_VERSION: Version = Version(0, 9, 1);
pub const CALL_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u64, pub u64, pub u64);

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// Parses `herdr 0.9.1` and `herdr 0.9.2-preview.3`; a pre-release suffix is ignored.
pub fn parse_version(text: &str) -> Option<Version> {
    let token = text
        .split_whitespace()
        .find(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit()))?;
    let core = token.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    Some(Version(parts.next()??, parts.next()??, parts.next()??))
}

/// A herdr command that does not talk to a server (`--version`, `session list`).
fn bare(bin: &str) -> Cmd {
    Cmd::new(bin, CALL_TIMEOUT)
}

pub fn version(bin: &str, runner: &dyn Runner) -> Result<Version> {
    let out = runner.run(&bare(bin).arg("--version"))?;
    if !out.success() {
        bail!("`{bin} --version` failed: {}", out.error_text());
    }
    parse_version(&out.stdout)
        .with_context(|| format!("could not read a version from `{}`", out.stdout.trim()))
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SessionInfo {
    pub name: String,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub running: bool,
    pub socket_path: PathBuf,
}

pub fn session_list(bin: &str, runner: &dyn Runner) -> Result<Vec<SessionInfo>> {
    #[derive(Deserialize)]
    struct Reply {
        sessions: Vec<SessionInfo>,
    }
    let out = runner.run(&bare(bin).args(["session", "list", "--json"]))?;
    if !out.success() {
        bail!("`{bin} session list` failed: {}", out.error_text());
    }
    let reply: Reply =
        serde_json::from_str(&out.stdout).context("`herdr session list --json` output changed")?;
    Ok(reply.sessions)
}

/// herdr bound to one session's socket. Every call a project makes goes through
/// this, so a project always talks to the session it was opened in.
pub struct Herdr<'a> {
    bin: String,
    socket: PathBuf,
    /// A saved SSH machine: every call is forwarded as `herdr --machine M ...`.
    machine: Option<String>,
    runner: &'a dyn Runner,
}

fn decode_reply(command: &str, out: Output) -> Result<serde_json::Value, HerdrError> {
    if out.timed_out {
        return Err(HerdrError {
            code: "timeout".into(),
            message: format!("`herdr {}` timed out", command),
        });
    }
    // herdr prints one JSON object; on failure it carries `error`, and
    // which stream it lands on is not something to depend on.
    let reply = [&out.stdout, &out.stderr]
        .into_iter()
        .find_map(|text| serde_json::from_str::<serde_json::Value>(text.trim()).ok());
    if let Some(reply) = reply {
        if let Some(error) = reply.get("error") {
            return Err(HerdrError {
                code: error["code"].as_str().unwrap_or("failed").to_string(),
                message: error["message"].as_str().unwrap_or("").to_string(),
            });
        }
        if out.success() {
            return Ok(reply.get("result").cloned().unwrap_or(serde_json::Value::Null));
        }
    }
    Err(HerdrError {
        code: "failed".into(),
        message: format!("`herdr {}`: {}", command, out.error_text()),
    })
}

impl<'a> Herdr<'a> {
    pub fn new(bin: impl Into<String>, socket: impl Into<PathBuf>, runner: &'a dyn Runner) -> Self {
        Herdr {
            bin: bin.into(),
            socket: socket.into(),
            machine: None,
            runner,
        }
    }

    /// The same session, with calls forwarded to a saved machine.
    pub fn on_machine(&self, machine: &str) -> Herdr<'a> {
        Herdr {
            bin: self.bin.clone(),
            socket: self.socket.clone(),
            machine: (!machine.is_empty()).then(|| machine.to_string()),
            runner: self.runner,
        }
    }

    /// `HERDR_SESSION` is removed so an inherited value can never compete with
    /// the socket this project recorded.
    pub fn cmd(&self, timeout: Duration) -> Cmd {
        let cmd = Cmd::new(&self.bin, timeout)
            .env("HERDR_SOCKET_PATH", self.socket.to_string_lossy())
            .env_remove("HERDR_SESSION");
        match &self.machine {
            Some(machine) => cmd.args(["--machine", machine]),
            None => cmd,
        }
    }

    /// True when the socket file exists and the server answers.
    pub fn reachable(&self) -> bool {
        if !self.socket.exists() {
            return false;
        }
        self.runner
            .run(&self.cmd(CALL_TIMEOUT).args(["status", "server"]))
            .map(|out| out.success())
            .unwrap_or(false)
    }
}

/// A herdr call that failed. `code` is herdr's own error code (for example
/// `agent_blocked` or `pane_not_found`), or `timeout` / `unreachable` / `failed`
/// when herdr never answered with one.
#[derive(Debug, Clone, PartialEq)]
pub struct HerdrError {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for HerdrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "herdr: {} ({})", self.message, self.code)
    }
}

impl std::error::Error for HerdrError {}

pub const AGENT_START_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Deserialize, PartialEq, Default)]
pub struct Pane {
    pub pane_id: String,
    pub tab_id: String,
    pub workspace_id: String,
    #[serde(default)]
    pub cwd: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default)]
pub struct Agent {
    pub pane_id: String,
    pub tab_id: String,
    pub workspace_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub agent: String,
    #[serde(default)]
    pub agent_status: String,
    #[serde(default)]
    pub cwd: String,
}

impl Agent {
    /// The one "ready for a prompt" predicate: state `idle` or `done`.
    pub fn ready(&self) -> bool {
        ready_state(&self.agent_status)
    }
}

pub fn ready_state(state: &str) -> bool {
    matches!(state, "idle" | "done")
}

#[derive(Debug, Clone, PartialEq)]
pub struct Created {
    pub workspace_id: String,
    pub tab_id: String,
    pub pane_id: String,
}

impl<'a> Herdr<'a> {
    /// Runs one herdr command and returns the `result` object of its JSON reply.
    pub fn call(&self, args: &[&str], timeout: Duration) -> Result<serde_json::Value, HerdrError> {
        let cmd = self.cmd(timeout).args(args.iter().copied());
        let out = self.runner.run(&cmd).map_err(|e| HerdrError {
            code: "unreachable".into(),
            message: format!("{e:#}"),
        })?;
        decode_reply(&args.join(" "), out)
    }

    fn call_as<T: serde::de::DeserializeOwned>(
        &self,
        args: &[&str],
        field: &str,
    ) -> Result<T, HerdrError> {
        let result = self.call(args, CALL_TIMEOUT)?;
        serde_json::from_value(result[field].clone()).map_err(|e| HerdrError {
            code: "failed".into(),
            message: format!("`herdr {}` reply changed: {e}", args.join(" ")),
        })
    }

    pub fn pane_list(&self) -> Result<Vec<Pane>, HerdrError> {
        self.call_as(&["pane", "list"], "panes")
    }

    pub fn agent_list(&self) -> Result<Vec<Agent>, HerdrError> {
        self.call_as(&["agent", "list"], "agents")
    }

    fn created(result: &serde_json::Value) -> Result<Created, HerdrError> {
        let pane = &result["root_pane"];
        match (
            pane["workspace_id"].as_str(),
            pane["tab_id"].as_str(),
            pane["pane_id"].as_str(),
        ) {
            (Some(w), Some(t), Some(p)) => Ok(Created {
                workspace_id: w.into(),
                tab_id: t.into(),
                pane_id: p.into(),
            }),
            _ => Err(HerdrError {
                code: "failed".into(),
                message: "herdr's create reply has no root_pane ids".into(),
            }),
        }
    }

    pub fn workspace_create(&self, cwd: &Path, label: &str, focus: bool) -> Result<Created, HerdrError> {
        let cwd = cwd.to_string_lossy();
        let focus = if focus { "--focus" } else { "--no-focus" };
        let result = self.call(
            &["workspace", "create", "--cwd", &cwd, "--label", label, focus],
            CALL_TIMEOUT,
        )?;
        Self::created(&result)
    }

    pub fn tab_create(&self, workspace: &str, cwd: &Path, label: &str, focus: bool) -> Result<Created, HerdrError> {
        let cwd = cwd.to_string_lossy();
        let focus = if focus { "--focus" } else { "--no-focus" };
        let result = self.call(
            &["tab", "create", "--workspace", workspace, "--cwd", &cwd, "--label", label, focus],
            CALL_TIMEOUT,
        )?;
        Self::created(&result)
    }

    /// Creates a worktree-backed workspace. Returns the ids and the checkout
    /// path as herdr reports it (on the machine the call ran on).
    pub fn worktree_create(&self, repo: &str, branch: &str, base: &str, label: &str) -> Result<(Created, String, String), HerdrError> {
        let result = self.call(
            &["worktree", "create", "--cwd", repo, "--branch", branch, "--base", base, "--label", label, "--no-focus"],
            Duration::from_secs(20),
        )?;
        Self::worktree_reply(&result)
    }

    /// `--cwd <repo>` is required: without it herdr answers `worktree_not_found`
    /// even for a path git lists (checked on 0.9.1).
    pub fn worktree_open(&self, repo: &str, path: &str, label: &str) -> Result<(Created, String, String), HerdrError> {
        let result = self.call(
            &["worktree", "open", "--cwd", repo, "--path", path, "--label", label, "--no-focus"],
            Duration::from_secs(20),
        )?;
        Self::worktree_reply(&result)
    }

    /// (ids, checkout path, pane working directory)
    fn worktree_reply(result: &serde_json::Value) -> Result<(Created, String, String), HerdrError> {
        let created = Self::created(result)?;
        let path = result["worktree"]["path"]
            .as_str()
            .or_else(|| result["workspace"]["worktree"]["checkout_path"].as_str())
            .unwrap_or_default()
            .to_string();
        let cwd = result["root_pane"]["cwd"].as_str().unwrap_or(&path).to_string();
        if path.is_empty() {
            return Err(HerdrError { code: "failed".into(), message: "herdr's worktree reply has no path".into() });
        }
        Ok((created, path, cwd))
    }

    /// Never passes `--force`: herdr refuses a dirty worktree and that refusal
    /// is reported unchanged.
    pub fn worktree_remove(&self, workspace: &str) -> Result<(), HerdrError> {
        self.call(&["worktree", "remove", "--workspace", workspace], Duration::from_secs(20)).map(|_| ())
    }

    /// A workspace's label, as the sidebar shows it.
    pub fn workspace_label(&self, workspace: &str) -> Result<String, HerdrError> {
        let result = self.call(&["workspace", "get", workspace], CALL_TIMEOUT)?;
        Ok(result["workspace"]["label"].as_str().unwrap_or_default().to_string())
    }

    pub fn workspace_rename(&self, workspace: &str, label: &str) -> Result<(), HerdrError> {
        self.call(&["workspace", "rename", workspace, label], CALL_TIMEOUT).map(|_| ())
    }

    /// The working directory herdr reports for a new tab's pane.
    pub fn pane_cwd(&self, pane: &str) -> Result<String, HerdrError> {
        let result = self.call(&["pane", "get", pane], CALL_TIMEOUT)?;
        Ok(result["pane"]["cwd"].as_str().unwrap_or_default().to_string())
    }

    /// Starts an agent in a pane that is at a shell prompt. Success means herdr
    /// detected the agent and it is ready for input.
    pub fn agent_start(&self, name: &str, kind: &str, pane: &str, agent_args: &[String]) -> Result<Agent, HerdrError> {
        let command = self.agent_start_command(name, kind, pane, agent_args);
        Self::agent_start_result(self.runner.run(&command))
    }

    pub fn agent_start_command(&self, name: &str, kind: &str, pane: &str, agent_args: &[String]) -> Cmd {
        let timeout_ms = AGENT_START_TIMEOUT.as_millis().to_string();
        let mut args = vec!["agent", "start", name, "--kind", kind, "--pane", pane, "--timeout", &timeout_ms];
        if !agent_args.is_empty() {
            args.push("--");
            args.extend(agent_args.iter().map(String::as_str));
        }
        self.cmd(AGENT_START_TIMEOUT + Duration::from_secs(5)).args(args)
    }

    pub fn agent_start_result(output: Result<Output>) -> Result<Agent, HerdrError> {
        let out = output.map_err(|e| HerdrError { code: "unreachable".into(), message: format!("{e:#}") })?;
        let result = decode_reply("agent start", out)?;
        serde_json::from_value(result["agent"].clone()).map_err(|e| HerdrError {
            code: "failed".into(),
            message: format!("`herdr agent start` reply changed: {e}"),
        })
    }

    /// Submits a prompt. herdr's parser takes positionals first and options
    /// after them, and has no `--` separator here; text in the second
    /// position is accepted even when it starts with a dash (checked on 0.9.1).
    pub fn agent_prompt(&self, target: &str, text: &str) -> Result<(), HerdrError> {
        self.call(&["agent", "prompt", target, text], CALL_TIMEOUT).map(|_| ())
    }

    /// Reads a pane's recent terminal output. Unlike every other call this
    /// one answers with the terminal text itself, not JSON, so it takes the
    /// runner directly; a JSON body here means herdr refused, and that is an
    /// error rather than an empty pane (an empty pane reads as "no receipt").
    ///
    /// `recent-unwrapped` carries history, which is what a receipt wants, but
    /// herdr can only capture it while the agent is idle (`agent_not_idle` on
    /// 0.9.1) — and an agent given its brief is working within the second, so
    /// that is the ordinary case, not the rare one. herdr's own advice is
    /// `--source visible`, which reads the drawn screen whatever the agent is
    /// doing, so a busy pane falls back to it rather than going unread.
    pub fn agent_read(&self, target: &str, lines: u32) -> Result<String, HerdrError> {
        match self.read_source(target, lines, "recent-unwrapped") {
            Err(error) if error.code == "agent_not_idle" => self.read_source(target, lines, "visible"),
            other => other,
        }
    }

    fn read_source(&self, target: &str, lines: u32, source: &str) -> Result<String, HerdrError> {
        let lines = lines.to_string();
        let args = ["agent", "read", target, "--source", source, "--lines", &lines];
        let cmd = self.cmd(CALL_TIMEOUT).args(args);
        let out = self.runner.run(&cmd).map_err(|e| HerdrError {
            code: "unreachable".into(),
            message: format!("{e:#}"),
        })?;
        if out.timed_out {
            return Err(HerdrError { code: "timeout".into(), message: "`herdr agent read` timed out".into() });
        }
        if !out.success() {
            let coded = [&out.stdout, &out.stderr]
                .into_iter()
                .find_map(|t| serde_json::from_str::<serde_json::Value>(t.trim()).ok())
                .and_then(|reply| {
                    let error = reply.get("error")?;
                    Some(HerdrError {
                        code: error["code"].as_str().unwrap_or("failed").to_string(),
                        message: error["message"].as_str().unwrap_or("").to_string(),
                    })
                });
            return Err(coded.unwrap_or_else(|| HerdrError {
                code: "failed".into(),
                message: format!("`herdr agent read {target}`: {}", out.error_text()),
            }));
        }
        Ok(out.stdout)
    }

    pub fn agent_focus(&self, target: &str) -> Result<(), HerdrError> {
        self.call(&["agent", "focus", target], CALL_TIMEOUT).map(|_| ())
    }

    pub fn notification_show(&self, title: &str, body: &str) -> Result<(), HerdrError> {
        self.call(&["notification", "show", title, "--body", body], CALL_TIMEOUT).map(|_| ())
    }

    /// Display tokens on a pane row, always with a TTL so they fade if the
    /// ticker stops.
    pub fn pane_report_tokens(&self, pane: &str, tokens: &[(&str, &str)], ttl: Duration) -> Result<(), HerdrError> {
        let ttl = ttl.as_millis().to_string();
        let pairs: Vec<String> = tokens.iter().map(|(k, v)| format!("{k}={v}")).collect();
        let mut args = vec!["pane", "report-metadata", pane, "--source", SOURCE, "--ttl-ms", &ttl];
        for pair in &pairs {
            args.push("--token");
            args.push(pair);
        }
        self.call(&args, CALL_TIMEOUT).map(|_| ())
    }

    pub fn pane_clear_tokens(&self, pane: &str, names: &[&str]) -> Result<(), HerdrError> {
        let mut args = vec!["pane", "report-metadata", pane, "--source", SOURCE];
        for name in names {
            args.push("--clear-token");
            args.push(name);
        }
        self.call(&args, CALL_TIMEOUT).map(|_| ())
    }
}

pub const SOURCE: &str = "herdr-projects";

impl<'a> Herdr<'a> {
    fn request(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, HerdrError> {
        let line = serde_json::json!({ "id": "herdr-projects", "method": method, "params": params }).to_string();
        let reply = self
            .runner
            .socket_request(&self.socket, &line, CALL_TIMEOUT)
            .map_err(|e| HerdrError { code: "unreachable".into(), message: format!("{e:#}") })?;
        let reply: serde_json::Value = serde_json::from_str(reply.trim())
            .map_err(|e| HerdrError { code: "failed".into(), message: format!("herdr's reply to {method} did not parse: {e}") })?;
        match reply.get("error") {
            Some(error) => Err(HerdrError {
                code: error["code"].as_str().unwrap_or("failed").to_string(),
                message: error["message"].as_str().unwrap_or("").to_string(),
            }),
            None => Ok(reply["result"].clone()),
        }
    }

    /// Filters the sidebar's agents to one project and sorts them by attention:
    /// the coordinator (rank 0) first, then by the group's display-order digit.
    /// herdr holds one transient view, so this replaces any other tool's view.
    pub fn agent_view_set_project(&self, slug: &str) -> Result<(), HerdrError> {
        self.request(
            "agent.view.set",
            serde_json::json!({
                "source": SOURCE,
                "label": format!("project: {slug}"),
                "filter": { "op": "eq", "field": { "token": "project" }, "value": slug },
                "sort": [{ "field": { "token": "rank" }, "order": "asc" }],
            }),
        )
        .map(|_| ())
    }

    pub fn agent_view_clear(&self) -> Result<(), HerdrError> {
        self.request("agent.view.clear", serde_json::json!({})).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions() {
        assert_eq!(parse_version("herdr 0.9.0\n"), Some(Version(0, 9, 0)));
        assert_eq!(parse_version("herdr 0.9.2-preview.3"), Some(Version(0, 9, 2)));
        assert_eq!(parse_version("0.10.0"), Some(Version(0, 10, 0)));
        assert_eq!(parse_version("herdr"), None);
        assert!(Version(0, 9, 0) < MIN_VERSION);
        assert!(Version(0, 10, 0) > MIN_VERSION);
    }

    #[test]
    fn agent_read_returns_terminal_text_not_json() {
        use crate::runner::fake::{FakeRunner, fail, ok};
        // `herdr agent read` prints the pane as it is drawn. There is no
        // global `--json`, so this one call cannot go through `call`.
        let runner = FakeRunner::new();
        runner.on("agent read", ok("> Read .herdr-project/demo-t-0001/brief.md and do what it says.\n"));
        let herdr = Herdr::new("herdr", "/tmp/s.sock", &runner);
        let text = herdr.agent_read("w2:p1", 40).unwrap();
        assert!(text.contains(".herdr-project/demo-t-0001/brief.md"));
        let call = runner.calls.borrow().last().unwrap().clone();
        assert!(call.display().contains("--source recent-unwrapped"), "{}", call.display());
        assert!(call.display().contains("--lines 40"), "{}", call.display());

        // A failure is an error, never an empty pane: an empty pane would read
        // as "no receipt" and silently look like a lost brief.
        let broken = FakeRunner::new();
        broken.on("agent read", fail(1, r#"{"error":{"code":"pane_not_found","message":"gone"}}"#));
        let herdr = Herdr::new("herdr", "/tmp/s.sock", &broken);
        assert_eq!(herdr.agent_read("w2:p1", 40).unwrap_err().code, "pane_not_found");
    }

    #[test]
    fn a_busy_pane_falls_back_to_the_drawn_screen() {
        use crate::runner::fake::{FakeRunner, fail, ok};
        // herdr can only capture history while the agent is idle, and an agent
        // that has its brief is working. Its own advice is `--source visible`.
        let runner = FakeRunner::new();
        runner.on(
            "--source recent-unwrapped",
            fail(1, r#"{"error":{"code":"agent_not_idle","message":"cannot read 200 lines while w2:p1 is working"}}"#),
        );
        runner.on("--source visible", ok("> Read .herdr-project/demo-t-0001/brief.md and do what it says.\n"));
        let herdr = Herdr::new("herdr", "/tmp/s.sock", &runner);
        let text = herdr.agent_read("w2:p1", 200).unwrap();
        assert!(text.contains(".herdr-project/demo-t-0001/brief.md"));
        assert_eq!(runner.count("agent read"), 2, "history first, then the screen");

        // Any other refusal stays an error: only an unreadable history is
        // worth a second, weaker read.
        let gone = FakeRunner::new();
        gone.on("agent read", fail(1, r#"{"error":{"code":"pane_not_found","message":"gone"}}"#));
        let herdr = Herdr::new("herdr", "/tmp/s.sock", &gone);
        assert_eq!(herdr.agent_read("w2:p1", 200).unwrap_err().code, "pane_not_found");
        assert_eq!(gone.count("agent read"), 1, "no retry for a pane that is not there");
    }
}

//! The terminal backend: the one interface the board runs its spaces, tabs, panes and agents through, and the two
//! programs behind it, herdr (herdr_cli.rs) and Rex (rex/). Which one a handle talks to is in its socket: a herdr
//! socket path, or `rex:` and a Rex server (`rex:` alone is the local one). Every answer has herdr's JSON shape
//! (`workspace_id`, `tab_id`, `pane_id`, `agent_status`, …), so the code above never asks which one it is.
use super::cli::HRes;
use crate::json::*;

/// What every terminal backend does. The ids it answers are its own; a caller hands them back unchanged.
pub trait TerminalBackend {
    /// `herdr` or `rex`.
    fn name(&self) -> &'static str;
    /// One herdr command line as it is. Rex answers the few the board sends this way (`workspace get`, `rename`,
    /// `report-metadata`) and refuses the rest with `unsupported`.
    fn call(&self, a: Vec<String>, timeout_ms: u64) -> HRes<Value>;
    // ---- spaces and tabs
    fn workspaces(&self) -> HRes<Vec<Value>>;
    fn workspace_get(&self, id: &str) -> Option<Value>;
    fn workspace_create(&self, cwd: &str, label: &str, env: &[(String, String)]) -> HRes<Value>;
    fn workspace_close(&self, id: &str) -> HRes<Value>;
    fn workspace_focus(&self, id: &str) -> HRes<Value>;
    fn tab_focus(&self, id: &str) -> HRes<Value>;
    fn tab_create(&self, workspace: &str, cwd: &str, label: &str, env: &[(String, String)]) -> HRes<Value>;
    fn tabs(&self, workspace: &str) -> HRes<Vec<Value>>;
    fn tab_get(&self, id: &str) -> Option<Value>;
    fn tab_rename(&self, id: &str, label: &str) -> Option<Value>;
    fn tab_close(&self, id: &str) -> Option<Value>;
    fn worktree_create(&self, cwd: &str, branch: &str, base: &str, label: &str) -> HRes<Value>;
    fn worktree_open(&self, cwd: &str, path: &str, label: &str) -> HRes<Value>;
    fn worktree_remove(&self, workspace: &str, force: bool) -> HRes<Value>;
    // ---- panes
    fn panes(&self, workspace: Option<&str>) -> HRes<Vec<Value>>;
    fn pane_get(&self, id: &str) -> Option<Value>;
    fn pane_layout(&self, id: &str) -> Option<Value>;
    fn pane_zoom(&self, id: &str, on: bool) -> HRes<Value>;
    fn pane_split(&self, id: &str, cwd: &str, direction: &str, env: &[(String, String)]) -> HRes<Value>;
    /// herdr's `process_info`: `shell_pid`, `foreground_process_group_id`, `foreground_processes` [{pid, name}].
    fn process_info(&self, id: &str) -> Option<Value>;
    fn pane_move(&self, id: &str, workspace: &str, label: &str) -> HRes<Value>;
    fn pane_close(&self, id: &str) -> Option<Value>;
    /// A command typed at the pane's shell and entered. Errors as herdr's when the shell is not at its prompt.
    fn pane_run(&self, id: &str, command: &str) -> HRes<Value>;
    fn pane_read(&self, id: &str, source: &str, lines: Option<i64>) -> HRes<String>;
    fn metadata(&self, id: &str, extra: &[String]) -> Option<Value>;
    /// Text typed into a pane as it is, with no enter.
    fn pane_text(&self, id: &str, text: &str) -> HRes<Value>;
    // ---- agents
    fn agents(&self) -> HRes<Vec<Value>>;
    /// The agent in a pane, or None when no agent is there.
    fn agent_get(&self, target: &str) -> HRes<Option<Value>>;
    /// Can this backend see when an agent of `kind` works? Refused (`integration_missing`) when it cannot.
    fn integration_ready(&self, kind: &str) -> HRes<()>;
    /// Start `kind` in the pane's shell under `name` and wait until it is seen there. Its agent record.
    fn agent_start(&self, name: &str, kind: &str, pane: &str, extra: &[String], timeout_ms: u64) -> HRes<Value>;
    /// Paste a prompt and submit it, confirmed. `agent_blocked` while the agent waits for an answer.
    fn agent_prompt(&self, target: &str, text: &str) -> HRes<Value>;
    fn agent_keys(&self, target: &str, keys: &[&str]) -> HRes<Value>;
    fn agent_focus(&self, target: &str) -> HRes<Value>;
    fn agent_read(&self, target: &str, source: &str, lines: Option<i64>) -> HRes<String>;
    // ---- the rest
    fn open_board(&self) -> HRes<Option<String>>;
    fn open_board_popup(&self, width: &str, height: &str) -> HRes<Value>;
    fn plugins(&self) -> Vec<Value>;
    fn plugin_config_dir(&self, id: &str) -> String;
    fn machines(&self) -> Vec<Value>;
    /// Take a pane's terminal back from whatever else is attached to it.
    fn take_terminal(&self, pane: &str, cols: u16, rows: u16) -> HRes<()>;
}

/// The prefix of a Rex socket.
pub const REX: &str = "rex:";
/// Is this socket a Rex server's?
pub fn is_rex(socket: &str) -> bool { socket.starts_with(REX) }
/// The Rex server a Rex socket names: None for the local one.
pub fn rex_server(socket: &str) -> Option<String> { socket.strip_prefix(REX).filter(|s| !s.is_empty()).map(String::from) }
/// The socket of a Rex server (None: the local one).
pub fn rex_socket(server: Option<&str>) -> String { format!("{REX}{}", server.unwrap_or("")) }
/// The backend a socket names.
pub fn for_socket(socket: Option<&str>) -> Box<dyn TerminalBackend> {
    match socket.filter(|s| is_rex(s)) {
        Some(s) => Box::new(super::rex::Rex::new(rex_server(s))),
        None => Box::new(super::herdr_cli::HerdrCli { socket: socket.map(String::from) }),
    }
}
/// The socket a repo's new work goes to: its `backend` setting. `herdr` keeps `here` (the board's own session);
/// `rex` is the local Rex server, unless the board itself runs on a Rex server.
pub fn socket_for(backend: &str, here: &str) -> String {
    match backend { "rex" if is_rex(here) => here.to_string(), "rex" => rex_socket(None), _ if is_rex(here) => super::cli::herdr_socket(), _ => here.to_string() }
}

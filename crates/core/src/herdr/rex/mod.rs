//! The Rex backend: the board's spaces, tabs, panes and agents on a Rex server. A project (a repo) is one Rex
//! session; one of the board's spaces is one or more windows in it (a tab is a window, with the space's worktree as
//! its folder); a pane is a block. Rex has no agents of its own: the board starts the program in a block's shell and
//! reads its turns from the status the program reports (OSC 7501, `program_status`). What Rex does not hold is in
//! the registry (registry.rs). The parts: running rex (run.rs), spaces and tabs (layout.rs), worktrees
//! (worktrees.rs), panes and their screens (screen.rs), agents (agents.rs, turns.rs). docs/rex.md says what differs.
pub mod agents;
pub mod layout;
pub mod registry;
pub mod run;
pub mod screen;
pub mod turns;
pub mod worktrees;

use super::backend::TerminalBackend;
use super::cli::HRes;
use crate::json::*;
use run::err;

/// One Rex server: None is the local one.
#[derive(Clone, Debug, Default)]
pub struct Rex { pub server: Option<String> }
impl Rex {
    pub fn new(server: Option<String>) -> Self { Rex { server } }
    pub fn srv(&self) -> Option<&str> { self.server.as_deref() }
    /// The socket that names this server (backend.rs).
    pub fn socket(&self) -> String { super::backend::rex_socket(self.srv()) }
}
fn unsupported(what: &str) -> super::HerdrError { err("unsupported", format!("Rex cannot {what} (docs/rex.md).")) }

impl TerminalBackend for Rex {
    fn name(&self) -> &'static str { "rex" }
    fn call(&self, a: Vec<String>, _timeout_ms: u64) -> HRes<Value> { self.herdr_line(&a) }
    fn workspaces(&self) -> HRes<Vec<Value>> { self.spaces() }
    fn workspace_get(&self, id: &str) -> Option<Value> { self.spaces().ok()?.into_iter().find(|w| w.s("workspace_id") == id) }
    fn workspace_create(&self, cwd: &str, label: &str, env: &[(String, String)]) -> HRes<Value> { self.space_create(cwd, label, env, Value::Null) }
    fn workspace_close(&self, id: &str) -> HRes<Value> { self.space_close(id) }
    fn workspace_focus(&self, id: &str) -> HRes<Value> { self.space_focus(id) }
    fn tab_focus(&self, id: &str) -> HRes<Value> { self.window_focus(id) }
    fn tab_create(&self, workspace: &str, cwd: &str, label: &str, env: &[(String, String)]) -> HRes<Value> { self.window_create(workspace, cwd, label, env) }
    fn tabs(&self, workspace: &str) -> HRes<Vec<Value>> { self.windows(workspace) }
    fn tab_get(&self, id: &str) -> Option<Value> { self.window_get(id) }
    fn tab_rename(&self, id: &str, label: &str) -> Option<Value> { self.window_rename(id, label) }
    fn tab_close(&self, id: &str) -> Option<Value> { self.window_close(id) }
    fn worktree_create(&self, cwd: &str, branch: &str, base: &str, label: &str) -> HRes<Value> { self.tree_create(cwd, branch, base, label) }
    fn worktree_open(&self, cwd: &str, path: &str, label: &str) -> HRes<Value> { self.tree_open(cwd, path, label) }
    fn worktree_remove(&self, workspace: &str, force: bool) -> HRes<Value> { self.tree_remove(workspace, force) }
    fn panes(&self, workspace: Option<&str>) -> HRes<Vec<Value>> { self.blocks(workspace) }
    fn pane_get(&self, id: &str) -> Option<Value> { self.block_get(id) }
    /// Rex lays a window out in fractions, not in cells: no size to hand over (held.rs falls back).
    fn pane_layout(&self, _id: &str) -> Option<Value> { None }
    fn pane_zoom(&self, id: &str, on: bool) -> HRes<Value> { run::rex(self.srv(), &super::cli::args(&["block", "zoom", "-b", id, if on { "--on" } else { "--off" }]), "", 15000).map(|_| json!({ "type": "ok" })) }
    fn pane_split(&self, id: &str, cwd: &str, direction: &str, env: &[(String, String)]) -> HRes<Value> { self.block_split(id, cwd, direction, env) }
    fn process_info(&self, id: &str) -> Option<Value> { self.processes(id) }
    /// Rex moves a block only inside its own session: the board's move to another space is refused.
    fn pane_move(&self, _id: &str, _workspace: &str, _label: &str) -> HRes<Value> { Err(unsupported("move a block to another session")) }
    fn pane_close(&self, id: &str) -> Option<Value> { self.block_close(id) }
    fn pane_run(&self, id: &str, command: &str) -> HRes<Value> { self.run_line(id, command) }
    fn pane_read(&self, id: &str, source: &str, lines: Option<i64>) -> HRes<String> { self.read(id, source, lines) }
    /// Rex keeps no structured metadata on a block: nothing to report it to.
    fn metadata(&self, _id: &str, _extra: &[String]) -> Option<Value> { None }
    fn pane_text(&self, id: &str, text: &str) -> HRes<Value> { self.type_text(id, text) }
    fn agents(&self) -> HRes<Vec<Value>> { self.agent_list() }
    fn agent_get(&self, target: &str) -> HRes<Option<Value>> { self.agent_one(target) }
    /// Rex sees a turn only through the program's own OSC 7501 reports: they are not checked here (docs/rex.md).
    fn integration_ready(&self, _kind: &str) -> HRes<()> { Ok(()) }
    fn agent_start(&self, name: &str, kind: &str, pane: &str, extra: &[String], timeout_ms: u64) -> HRes<Value> { self.start(name, kind, pane, extra, timeout_ms) }
    fn agent_prompt(&self, target: &str, text: &str) -> HRes<Value> { self.prompt(target, text) }
    fn agent_keys(&self, target: &str, keys: &[&str]) -> HRes<Value> { self.keys(target, keys) }
    fn agent_focus(&self, target: &str) -> HRes<Value> { let b = self.block_of(target)?; run::rex(self.srv(), &super::cli::args(&["block", "focus", &b]), "", 15000).map(|_| json!({ "type": "ok" })) }
    fn agent_read(&self, target: &str, source: &str, lines: Option<i64>) -> HRes<String> { let b = self.block_of(target)?; self.read(&b, source, lines) }
    fn open_board(&self) -> HRes<Option<String>> { self.board().map(Some) }
    /// No popup on Rex yet: the board opens in its own window.
    fn open_board_popup(&self, _width: &str, _height: &str) -> HRes<Value> { self.board().map(|b| json!({ "pane": b })) }
    fn plugins(&self) -> Vec<Value> { vec![] }
    fn plugin_config_dir(&self, _id: &str) -> String { String::new() }
    /// Other machines are herdr's: Rex reaches another server with `rex:<host>` instead.
    fn machines(&self) -> Vec<Value> { vec![] }
    fn take_terminal(&self, _pane: &str, _cols: u16, _rows: u16) -> HRes<()> { Err(unsupported("hand a block's terminal over")) }
}

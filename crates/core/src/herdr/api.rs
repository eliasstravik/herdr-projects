//! The board's handle on its terminal: one session of herdr or one Rex server, named by its socket (backend.rs). It
//! passes each call to that backend, and keeps what is the same on both: a start tried again until the shell is at
//! its prompt, what runs in a pane, and how an agent is ended. `None` as the socket means whatever this process
//! would talk to.
use super::backend::{for_socket, TerminalBackend};
use super::cli::{args, kids, HRes, HerdrError};
use crate::json::*;

#[derive(Clone, Debug, Default)]
pub struct Herdr { pub socket: Option<String> }

impl Herdr {
    pub fn new(socket: Option<String>) -> Self { Herdr { socket } }
    /// The backend this handle's socket names.
    /// No socket inside a Rex block is that Rex server (`HERDR_PROJECTS_SOCKET`); elsewhere herdr's own choice.
    pub fn backend(&self) -> Box<dyn TerminalBackend> { for_socket(self.rex_socket().as_deref().or(self.socket.as_deref())) }
    fn rex_socket(&self) -> Option<String> {
        match &self.socket { Some(s) => Some(s.clone()).filter(|s| super::backend::is_rex(s)), None => Some(super::cli::current_socket()).filter(|s| super::backend::is_rex(s)) }
    }
    pub fn is_rex(&self) -> bool { self.rex_socket().is_some() }
    pub fn socket_path(&self) -> String { self.socket.clone().unwrap_or_else(super::cli::current_socket) }
    pub fn call(&self, a: Vec<String>) -> HRes<Value> { self.backend().call(a, 30000) }
    pub fn call_t(&self, a: Vec<String>, timeout_ms: u64) -> HRes<Value> { self.backend().call(a, timeout_ms) }
    pub fn alive(&self) -> bool { if self.is_rex() { self.backend().workspaces().is_ok() } else { self.call_t(args(&["workspace", "list"]), 5000).is_ok() } }
    // ---- layout
    pub fn workspaces(&self) -> HRes<Vec<Value>> { self.backend().workspaces() }
    pub fn workspace_get(&self, id: &str) -> Option<Value> { self.backend().workspace_get(id) }
    pub fn workspace_create(&self, cwd: &str, label: &str, env: &[(String, String)]) -> HRes<Value> { self.backend().workspace_create(cwd, label, env) }
    pub fn workspace_close(&self, id: &str) -> HRes<Value> { self.backend().workspace_close(id) }
    pub fn workspace_focus(&self, id: &str) -> HRes<Value> { self.backend().workspace_focus(id) }
    pub fn tab_focus(&self, id: &str) -> HRes<Value> { self.backend().tab_focus(id) }
    /// A new tab in a space, with one pane at its shell. Never focused: the human stays where they are.
    pub fn tab_create(&self, workspace: &str, cwd: &str, label: &str, env: &[(String, String)]) -> HRes<Value> { self.backend().tab_create(workspace, cwd, label, env) }
    pub fn tabs(&self, workspace: &str) -> HRes<Vec<Value>> { self.backend().tabs(workspace) }
    pub fn tab_get(&self, id: &str) -> Option<Value> { self.backend().tab_get(id) }
    pub fn tab_rename(&self, id: &str, label: &str) -> Option<Value> { self.backend().tab_rename(id, label) }
    pub fn tab_close(&self, id: &str) -> Option<Value> { self.backend().tab_close(id) }
    pub fn worktree_create(&self, cwd: &str, branch: &str, base: &str, label: &str) -> HRes<Value> { self.backend().worktree_create(cwd, branch, base, label) }
    pub fn worktree_open(&self, cwd: &str, path: &str, label: &str) -> HRes<Value> { self.backend().worktree_open(cwd, path, label) }
    pub fn worktree_remove(&self, workspace: &str, force: bool) -> HRes<Value> { self.backend().worktree_remove(workspace, force) }
    // ---- panes
    pub fn panes(&self, workspace: Option<&str>) -> HRes<Vec<Value>> { self.backend().panes(workspace) }
    pub fn pane_get(&self, id: &str) -> Option<Value> { self.backend().pane_get(id) }
    /// Where a pane sits in its window: its tab's area, every pane's rect, and whether one is zoomed.
    pub fn pane_layout(&self, id: &str) -> Option<Value> { self.backend().pane_layout(id) }
    /// Put the keyboard on one pane of the tab that shows: zoom it (zooming focuses), and take the zoom off again
    /// unless the tab was zoomed (herdr 0.9.3 has no "focus this pane").
    pub fn pane_focus(&self, id: &str) -> HRes<()> {
        if self.is_rex() { return self.backend().agent_focus(id).map(|_| ()); } // a block is focused by its id
        let Some(before) = self.pane_layout(id) else { return Ok(()) };
        if before.s("focused_pane_id") == id { return Ok(()); }
        self.backend().pane_zoom(id, true)?;
        if !before.b("zoomed") { let _ = self.backend().pane_zoom(id, false); }
        Ok(())
    }
    pub fn pane_split(&self, id: &str, cwd: &str, direction: &str, env: &[(String, String)]) -> HRes<Value> { self.backend().pane_split(id, cwd, direction, env) }
    fn process_info(&self, id: &str) -> Option<Value> { self.backend().process_info(id) }
    /// Is nothing but its shell running in the pane? Unknown counts as busy.
    pub fn pane_idle(&self, id: &str) -> bool {
        let Some(i) = self.process_info(id) else { return false };
        let shell = i.at("shell_pid");
        if !crate::gate::is_int(shell) { return false; }
        let sp = shell.as_i64().unwrap_or(-1);
        i.i("foreground_process_group_id") == sp && i.arr("foreground_processes").iter().all(|p| p.i("pid") == sp) && kids(sp).map(|k| k.is_empty()).unwrap_or(false)
    }
    /// Does nothing run in the pane but the program `agent` in its foreground (and the shell that started it)? Unknown:
    /// no. Beside the program, its process group may hold only its helpers: one wired to the group itself on its input
    /// and output (an MCP server, procs.rs), or the agent's own program under a wrapper (`codex` installed with npm). A
    /// command is at work, in the program's group (on the terminal, a file or `/dev/null`) or in a group of its own.
    /// Under the group, a known agent program (`agents::kinds`: pi, codex, claude, ...) keeps helpers in groups of
    /// their own while idle (MCP servers, tool drivers), wired to it the same way: its own affair, not counted. Any
    /// other process under the group is a command it left running. A job of the pane's shell (typed in the pane, a
    /// child of the shell and not of the agent) counts whatever runs.
    pub fn pane_childless(&self, id: &str, agent: &str) -> bool {
        let Some(i) = self.process_info(id) else { return false };
        let procs: Vec<&Value> = i.arr("foreground_processes").iter().filter(|p| crate::gate::is_int(p.at("pid"))).collect();
        let front: Vec<i64> = procs.iter().map(|p| p.i("pid")).collect();
        if front.is_empty() { return false; }
        let lead = i.i("foreground_process_group_id");
        let helper = |p: &Value| p.i("pid") == lead || (!agent.is_empty() && p.s("name") == agent) || super::procs::wired_to(p.i("pid"), &front) == Some(true);
        if !procs.iter().all(|p| helper(p)) { return false; }
        let known = crate::agents::kinds::kind_named(agent).is_some();
        let own = |p: &i64| front.contains(p) || (known && super::procs::wired_to(*p, &front) == Some(true));
        for pid in &front { match kids(*pid) { Ok(k) if k.iter().all(|p| own(p)) => {} _ => return false } }
        if !crate::gate::is_int(i.at("shell_pid")) { return true; }
        kids(i.i("shell_pid")).map(|k| k.iter().all(|p| front.contains(p))).unwrap_or(false)
    }
    /// A pane, with whatever runs in it, into a new tab of another space. Never focused.
    pub fn pane_move(&self, id: &str, workspace: &str, label: &str) -> HRes<Value> { self.backend().pane_move(id, workspace, label) }
    pub fn pane_close(&self, id: &str) -> Option<Value> { self.backend().pane_close(id) }
    /// Run a command in the pane, at its shell: typed and entered.
    pub fn pane_run(&self, id: &str, command: &str) -> HRes<Value> { self.when_at_prompt(id, || self.backend().pane_run(id, command)) }
    /// End the program in the pane's foreground, so the pane is at its shell again. True once no agent is seen there.
    pub fn pane_quit(&self, id: &str, wait_ms: i64) -> bool {
        let gone = || matches!(self.agent_get(id), Ok(None));
        let info = self.process_info(id);
        let shell = info.as_ref().map(|i| i.i("shell_pid")).unwrap_or(-1);
        let pids: Vec<i64> = info.map(|i| i.arr("foreground_processes").iter().map(|p| p.i("pid")).filter(|p| *p > 1 && *p != shell).collect()).unwrap_or_default();
        for sig in [libc::SIGTERM, libc::SIGKILL] {
            if pids.is_empty() || gone() { break; }
            for pid in &pids { match super::cli::fake() { Some(f) => f.kill(*pid), None => unsafe { libc::kill(*pid as i32, sig); } } }
            let until = crate::clock::now() + wait_ms / 2;
            while crate::clock::now() < until && !gone() { std::thread::sleep(std::time::Duration::from_millis(200)); }
        }
        gone()
    }
    pub fn pane_read(&self, id: &str, source: &str, lines: Option<i64>) -> HRes<String> { self.backend().pane_read(id, source, lines) }
    pub fn metadata(&self, id: &str, extra: &[String]) -> Option<Value> { self.backend().metadata(id, extra) }
    // ---- agents
    pub fn agents(&self) -> HRes<Vec<Value>> { self.backend().agents() }
    /// The agent in a pane (or by name), or None when no agent is there.
    pub fn agent_get(&self, target: &str) -> HRes<Option<Value>> { self.backend().agent_get(target) }
    pub fn integration_ready(&self, kind: &str) -> HRes<()> { self.backend().integration_ready(kind) }
    pub fn agent_start(&self, name: &str, kind: &str, pane: &str, extra: &[String], timeout_ms: u64) -> HRes<Value> {
        self.integration_ready(kind)?;
        let b = self.backend();
        // A shell not at its prompt yet takes no long line: waited for and tried again (shell_ready.rs).
        match self.when_at_prompt(pane, || b.agent_start(name, kind, pane, extra, timeout_ms)) {
            Ok(r) => Ok(r),
            // A program that quits as it starts (on an argument it does not know) leaves the backend waiting for it
            // until the timeout. What it printed says why.
            // One that runs on, unseen, is behind a launcher the backend cannot see through (kilo and qodercli from
            // npm: a node script that starts the program as its child).
            Err(e) if e.code == "timeout" && self.agent_get(pane).ok().flatten().is_none() => Err(match self.quit_with(pane, kind) {
                Some(why) => HerdrError { code: e.code, message: format!("{kind} quit as it started: {why}") },
                None => HerdrError { message: format!("{}: {} did not see {kind} come up in its pane. If it runs there, it is behind a launcher that hides it (kilo and qodercli installed with npm are): install its standalone program (docs/agents.md).", e.message, b.name()), code: e.code },
            }),
            Err(e) => Err(e),
        }
    }
    /// The error a program printed in the pane as it quit: its first line saying "error" after the shell prompt it
    /// was started from (the prompt the shell shows again, last), or None.
    fn quit_with(&self, pane: &str, kind: &str) -> Option<String> {
        let text = self.pane_read(pane, "recent", Some(40)).ok()?;
        let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        let (prompt, before) = lines.split_last()?;
        let from = before.iter().rposition(|l| l.starts_with(prompt) && l.contains(kind)).map(|i| i + 1).unwrap_or(0);
        before[from..].iter().find(|l| l.to_lowercase().contains("error")).map(|l| l.chars().take(300).collect())
    }
    pub fn agent_prompt(&self, target: &str, text: &str) -> HRes<Value> { self.backend().agent_prompt(target, text) }
    /// Text typed into a pane as it is, with no enter.
    pub fn pane_text(&self, id: &str, text: &str) -> HRes<Value> { self.backend().pane_text(id, text) }
    pub fn agent_keys(&self, target: &str, keys: &[&str]) -> HRes<Value> { self.backend().agent_keys(target, keys) }
    pub fn agent_focus(&self, target: &str) -> HRes<Value> { self.backend().agent_focus(target) }
    pub fn agent_read(&self, target: &str, source: &str, lines: Option<i64>) -> HRes<String> { self.backend().agent_read(target, source, lines) }
    // ---- the rest
    /// The board in a tab of its own, an ordinary pane that stays when you go elsewhere. Answers that pane's id.
    pub fn open_board(&self) -> HRes<Option<String>> { self.backend().open_board() }
    /// The board in a popup. herdr refuses with `ui_busy` while a popup, anybody's, is open.
    pub fn open_board_popup(&self, width: &str, height: &str) -> HRes<Value> { self.backend().open_board_popup(width, height) }
    /// The plugins herdr has installed.
    pub fn plugins(&self) -> Vec<Value> { self.backend().plugins() }
    pub fn plugin_config_dir(&self, id: &str) -> String { self.backend().plugin_config_dir(id) }
    pub fn machines(&self) -> Vec<Value> { self.backend().machines() }
}
/// Takes a pane's terminal back from whatever else is attached to it.
pub fn take_terminal(h: &Herdr, pane: &str, cols: u16, rows: u16) -> HRes<()> { h.backend().take_terminal(pane, cols, rows) }
/// The size herdr shows a pane at, from `pane layout`: its rectangle, or the whole tab while it is the zoomed one.
pub fn shown_of(layout: &Value, pane: &str) -> Option<(u16, u16, bool)> {
    let panes = layout.arr("panes");
    let mine = panes.iter().find(|p| p.s("pane_id") == pane)?;
    let r = if layout.b("zoomed") && layout.s("focused_pane_id") == pane && layout.at("area").is_object() { layout.at("area") } else { mine.at("rect") };
    (r.i("width") > 0 && r.i("height") > 0).then(|| (r.i("width") as u16, r.i("height") as u16, panes.len() > 1))
}
/// Reading a terminal. A deep read is only possible (and only harmless) when the agent is idle or done.
pub fn read_screen(h: &Herdr, pane: &str, status: Option<&str>, lines: i64) -> String {
    let r = match status {
        Some("idle") | Some("done") => h.agent_read(pane, "recent-unwrapped", Some(lines)),
        None => h.pane_read(pane, "visible", None),
        _ => h.agent_read(pane, "visible", None),
    };
    match r {
        Ok(s) => s,
        Err(e) => {
            if e.code == "agent_not_idle" { if let Ok(s) = h.agent_read(pane, "visible", None) { return s; } }
            h.pane_read(pane, "visible", None).unwrap_or_default()
        }
    }
}

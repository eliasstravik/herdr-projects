//! herdr's commands, bound to one herdr session (by socket path): spaces, tabs, panes, agents and the rest. `None` as
//! the socket means whatever this process would talk to.
use super::cli::{args, kids, run, run_raw, HRes, HerdrError};
use crate::json::*;

#[derive(Clone, Debug, Default)]
pub struct Herdr { pub socket: Option<String> }
fn envs(env: &[(String, String)]) -> Vec<String> { env.iter().flat_map(|(k, v)| ["--env".to_string(), format!("{k}={v}")]).collect() }

/// Sidebar-only task rows never count as a running coding CLI.
pub const TASK_AGENT: &str = "herdr-task";

impl Herdr {
    pub fn new(socket: Option<String>) -> Self { Herdr { socket } }
    pub fn socket_path(&self) -> String { self.socket.clone().unwrap_or_else(super::cli::current_socket) }
    pub fn call(&self, a: Vec<String>) -> HRes<Value> { run(self.socket.as_deref(), &a, 30000, "") }
    pub fn call_t(&self, a: Vec<String>, timeout_ms: u64) -> HRes<Value> { run(self.socket.as_deref(), &a, timeout_ms, "") }
    pub fn alive(&self) -> bool { self.call_t(args(&["workspace", "list"]), 5000).is_ok() }
    // ---- layout
    pub fn workspaces(&self) -> HRes<Vec<Value>> { Ok(self.call(args(&["workspace", "list"]))?.arr("workspaces").to_vec()) }
    pub fn workspace_get(&self, id: &str) -> Option<Value> { self.call(args(&["workspace", "get", id])).ok().map(|r| r.at("workspace").clone()).filter(|v| !v.is_null()) }
    pub fn workspace_create(&self, cwd: &str, label: &str, env: &[(String, String)]) -> HRes<Value> {
        let mut a = args(&["workspace", "create", "--cwd", cwd]);
        if !label.is_empty() { a.extend(args(&["--label", label])); }
        a.extend(envs(env)); a.push("--no-focus".into());
        self.call(a)
    }
    pub fn workspace_close(&self, id: &str) -> HRes<Value> { self.call(args(&["workspace", "close", id])) }
    pub fn workspace_focus(&self, id: &str) -> HRes<Value> { self.call(args(&["workspace", "focus", id])) }
    pub fn tab_focus(&self, id: &str) -> HRes<Value> { self.call(args(&["tab", "focus", id])) }
    /// A new tab in a space, with one pane at its shell. Never focused: the human stays where they are.
    pub fn tab_create(&self, workspace: &str, cwd: &str, label: &str, env: &[(String, String)]) -> HRes<Value> {
        let mut a = args(&["tab", "create", "--workspace", workspace]);
        if !cwd.is_empty() { a.extend(args(&["--cwd", cwd])); }
        if !label.is_empty() { a.extend(args(&["--label", label])); }
        a.extend(envs(env)); a.push("--no-focus".into());
        self.call(a)
    }
    pub fn tabs(&self, workspace: &str) -> HRes<Vec<Value>> { Ok(self.call(args(&["tab", "list", "--workspace", workspace]))?.arr("tabs").to_vec()) }
    pub fn tab_get(&self, id: &str) -> Option<Value> { self.call(args(&["tab", "get", id])).ok().map(|r| r.at("tab").clone()).filter(|v| !v.is_null()) }
    pub fn tab_rename(&self, id: &str, label: &str) -> Option<Value> { self.call(args(&["tab", "rename", id, label])).ok() }
    pub fn tab_close(&self, id: &str) -> Option<Value> { self.call(args(&["tab", "close", id])).ok() }
    pub fn worktree_create(&self, cwd: &str, branch: &str, base: &str, label: &str) -> HRes<Value> {
        let mut a = args(&["worktree", "create", "--cwd", cwd, "--branch", branch]);
        if !base.is_empty() { a.extend(args(&["--base", base])); }
        if !label.is_empty() { a.extend(args(&["--label", label])); }
        a.push("--no-focus".into());
        self.call_t(a, 120000)
    }
    pub fn worktree_open(&self, cwd: &str, path: &str, label: &str) -> HRes<Value> {
        let mut a = args(&["worktree", "open", "--cwd", cwd, "--path", path]);
        if !label.is_empty() { a.extend(args(&["--label", label])); }
        a.push("--no-focus".into());
        self.call_t(a, 60000)
    }
    pub fn worktree_remove(&self, workspace: &str, force: bool) -> HRes<Value> {
        let mut a = args(&["worktree", "remove", "--workspace", workspace]);
        if force { a.push("--force".into()); }
        self.call_t(a, 120000)
    }
    // ---- panes
    pub fn panes(&self, workspace: Option<&str>) -> HRes<Vec<Value>> {
        let mut a = args(&["pane", "list"]);
        if let Some(w) = workspace { a.extend(args(&["--workspace", w])); }
        Ok(self.call(a)?.arr("panes").to_vec())
    }
    pub fn pane_get(&self, id: &str) -> Option<Value> { self.call(args(&["pane", "get", id])).ok().map(|r| r.at("pane").clone()).filter(|v| !v.is_null()) }
    /// Where a pane sits in herdr's own window: its tab's area, every pane's rect, and whether one is zoomed.
    pub fn pane_layout(&self, id: &str) -> Option<Value> { self.call_t(args(&["pane", "layout", "--pane", id]), 3000).ok().map(|r| r.at("layout").clone()).filter(|v| !v.is_null()) }
    /// Put the keyboard on one pane of the tab that shows: zoom it (zooming focuses), and take the zoom off again
    /// unless the tab was zoomed (herdr 0.9.3 has no "focus this pane").
    pub fn pane_focus(&self, id: &str) -> HRes<()> {
        let Some(before) = self.pane_layout(id) else { return Ok(()) };
        if before.s("focused_pane_id") == id { return Ok(()); }
        self.call(args(&["pane", "zoom", id, "--on"]))?;
        if !before.b("zoomed") { let _ = self.call(args(&["pane", "zoom", id, "--off"])); }
        Ok(())
    }
    pub fn pane_split(&self, id: &str, cwd: &str, direction: &str, env: &[(String, String)]) -> HRes<Value> {
        let mut a = args(&["pane", "split", id, "--direction", direction]);
        if !cwd.is_empty() { a.extend(args(&["--cwd", cwd])); }
        a.extend(envs(env)); a.push("--no-focus".into());
        Ok(self.call(a)?.at("pane").clone())
    }
    fn process_info(&self, id: &str) -> Option<Value> { self.call(args(&["pane", "process-info", "--pane", id])).ok().map(|r| r.at("process_info").clone()) }
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
    pub fn pane_childless(&self, id: &str, agent: &str) -> bool {
        let Some(i) = self.process_info(id) else { return false };
        let procs: Vec<&Value> = i.arr("foreground_processes").iter().filter(|p| crate::gate::is_int(p.at("pid"))).collect();
        let front: Vec<i64> = procs.iter().map(|p| p.i("pid")).collect();
        if front.is_empty() { return false; }
        let lead = i.i("foreground_process_group_id");
        let helper = |p: &Value| p.i("pid") == lead || (!agent.is_empty() && p.s("name") == agent) || super::procs::wired_to(p.i("pid"), &front) == Some(true);
        if !procs.iter().all(|p| helper(p)) { return false; }
        for pid in &front { match kids(*pid) { Ok(k) if k.iter().all(|p| front.contains(p)) => {} _ => return false } }
        if !crate::gate::is_int(i.at("shell_pid")) { return true; }
        kids(i.i("shell_pid")).map(|k| k.iter().all(|p| front.contains(p))).unwrap_or(false)
    }
    /// A pane, with whatever runs in it, into a new tab of another space. Never focused.
    pub fn pane_move(&self, id: &str, workspace: &str, label: &str) -> HRes<Value> {
        let mut a = args(&["pane", "move", id, "--new-tab", "--workspace", workspace]);
        if !label.is_empty() { a.extend(args(&["--label", label])); }
        a.push("--no-focus".into());
        Ok(self.call(a)?.at("move_result").clone())
    }
    pub fn pane_close(&self, id: &str) -> Option<Value> { self.call(args(&["pane", "close", id])).ok() }
    /// Run a command in the pane, at its shell: typed and entered.
    pub fn pane_run(&self, id: &str, command: &str) -> HRes<Value> { self.when_at_prompt(id, || self.call(args(&["pane", "run", id, command]))) }
    /// End the program in the pane's foreground, so the pane is at its shell again. True once herdr sees no agent there.
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
    pub fn pane_read(&self, id: &str, source: &str, lines: Option<i64>) -> HRes<String> {
        let mut a = args(&["pane", "read", id, "--source", source]);
        if let Some(n) = lines { a.extend(args(&["--lines", &n.to_string()])); }
        run_raw(self.socket.as_deref(), &a, 30000, "")
    }
    pub fn metadata(&self, id: &str, extra: &[String]) -> Option<Value> {
        let mut a = args(&["pane", "report-metadata", id, "--source", "plugin:herdr-projects"]);
        a.extend(extra.iter().cloned());
        self.call(a).ok()
    }
    // ---- agents
    pub fn agents(&self) -> HRes<Vec<Value>> { Ok(self.call(args(&["agent", "list"]))?.arr("agents").iter().filter(|a| a.s("agent") != TASK_AGENT).cloned().collect()) }
    /// The agent in a pane (or by name), or None when no agent is there.
    pub fn agent_get(&self, target: &str) -> HRes<Option<Value>> {
        match self.call(args(&["agent", "get", target])) {
            Ok(r) => Ok(Some(r.at("agent").clone()).filter(|v| !v.is_null() && v.s("agent") != TASK_AGENT)),
            Err(e) if ["agent_not_found", "not_found", "agent_not_running", "pane_not_found"].contains(&e.code.as_str()) => Ok(None),
            Err(e) => Err(e),
        }
    }
    /// A program herdr sees at work only through its own integration is never started without it (kinds.rs
    /// `needs_integration`): herdr would never say its turn ended. Its status unknown (an older herdr): it is started.
    pub fn integration_ready(&self, kind: &str) -> HRes<()> {
        let Some(k) = crate::agents::kinds::kind(kind).filter(|k| k.needs_integration) else { return Ok(()) };
        let status = run_raw(self.socket.as_deref(), &args(&["integration", "status"]), 15000, "").unwrap_or_default();
        let line = status.lines().find(|l| l.trim_start().starts_with(&format!("{}:", k.integration)) || l.trim_start().starts_with(&format!("{} (", k.integration)));
        if line.map(|l| l.contains("not installed")).unwrap_or(false) {
            return Err(HerdrError { code: "integration_missing".into(), message: format!("{kind} needs herdr's own integration, or herdr cannot see when it works: run `herdr integration install {}`, then try again.", k.integration) });
        }
        Ok(())
    }
    pub fn agent_start(&self, name: &str, kind: &str, pane: &str, extra: &[String], timeout_ms: u64) -> HRes<Value> {
        self.integration_ready(kind)?;
        let mut a = args(&["agent", "start", name, "--kind", kind, "--pane", pane, "--timeout", &timeout_ms.to_string()]);
        if !extra.is_empty() { a.push("--".into()); a.extend(extra.iter().cloned()); }
        // A shell not at its prompt yet takes no long line: waited for and tried again (shell_ready.rs).
        match self.when_at_prompt(pane, || self.call_t(a.clone(), timeout_ms + 15000).map(|r| r.at("agent").clone())) {
            Ok(r) => Ok(r),
            // A program that quits as it starts (on an argument it does not know) leaves herdr waiting for it until the
            // timeout. What it printed says why.
            // One that runs on, unseen, is behind a launcher herdr cannot see through (kilo and qodercli from npm: a node
            // script that starts the program as its child).
            Err(e) if e.code == "timeout" && self.agent_get(pane).ok().flatten().is_none() => Err(match self.quit_with(pane, kind) {
                Some(why) => HerdrError { code: e.code, message: format!("{kind} quit as it started: {why}") },
                None => HerdrError { message: format!("{}: herdr did not see {kind} come up in its pane. If it runs there, it is behind a launcher that hides it from herdr (kilo and qodercli installed with npm are): install its standalone program (docs/agents.md).", e.message), code: e.code },
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
    pub fn agent_prompt(&self, target: &str, text: &str) -> HRes<Value> { self.call_t(args(&["agent", "prompt", target, text]), 60000) }
    /// Text typed into a pane as it is, with no enter.
    pub fn pane_text(&self, id: &str, text: &str) -> HRes<Value> { self.call(args(&["pane", "send-text", id, text])) }
    pub fn agent_keys(&self, target: &str, keys: &[&str]) -> HRes<Value> { let mut a = args(&["agent", "send-keys", target]); a.extend(args(keys)); self.call(a) }
    pub fn agent_focus(&self, target: &str) -> HRes<Value> { self.call(args(&["agent", "focus", target])) }
    pub fn agent_read(&self, target: &str, source: &str, lines: Option<i64>) -> HRes<String> {
        let mut a = args(&["agent", "read", target, "--source", source]);
        if let Some(n) = lines { a.extend(args(&["--lines", &n.to_string()])); }
        run_raw(self.socket.as_deref(), &a, 60000, "")
    }
    // ---- the rest
    /// The board in a tab of its own, an ordinary pane that stays when you go elsewhere. Answers that pane's id (herdr
    /// answers the pane itself).
    pub fn open_board(&self) -> HRes<Option<String>> {
        let p = self.call(args(&["plugin", "pane", "open", "--plugin", "herdr-projects", "--entrypoint", "board", "--placement", "tab"]))?.at("plugin_pane").at("pane").clone();
        Ok(p.as_str().or(p.so("pane_id")).filter(|s| !s.is_empty()).map(String::from))
    }
    /// The board in herdr's popup. herdr refuses with `ui_busy` while a popup, anybody's, is open.
    pub fn open_board_popup(&self, width: &str, height: &str) -> HRes<Value> {
        let mut a = args(&["plugin", "pane", "open", "--plugin", "herdr-projects", "--entrypoint", "board", "--placement", "popup"]);
        if !width.is_empty() { a.extend(args(&["--width", width])); }
        if !height.is_empty() { a.extend(args(&["--height", height])); }
        self.call(a)
    }
    /// The plugins herdr has installed.
    pub fn plugins(&self) -> Vec<Value> {
        run_raw(None, &args(&["plugin", "list", "--json"]), 8000, "").ok().and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .map(|j| if j.at("result").at("plugins").is_array() { j.at("result").arr("plugins").to_vec() } else { j.arr("plugins").to_vec() }).unwrap_or_default()
    }
    pub fn plugin_config_dir(&self, id: &str) -> String { run_raw(None, &args(&["plugin", "config-dir", id]), 5000, "").map(|s| s.trim().to_string()).unwrap_or_default() }
    pub fn machines(&self) -> Vec<Value> {
        let Some(j) = run_raw(None, &args(&["machine", "list", "--json"]), 30000, "").ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { return vec![] };
        if let Some(a) = j.as_array() { return a.clone(); }
        if j.at("machines").is_array() { j.arr("machines").to_vec() } else { j.at("result").arr("machines").to_vec() }
    }
}
/// Takes a pane's terminal back from whatever else is attached to it.
pub fn take_terminal(h: &Herdr, pane: &str, cols: u16, rows: u16) -> HRes<()> {
    let id = h.call_t(args(&["pane", "get", pane]), 5000)?.at("pane").t("terminal_id");
    if id.is_empty() { return Err(HerdrError { code: "pane_not_found".into(), message: "herdr does not know this pane.".into() }); }
    run_raw(h.socket.as_deref(), &args(&["terminal", "session", "control", &id, "--takeover", "--cols", &cols.to_string(), "--rows", &rows.to_string()]), 5000, "")?;
    Ok(())
}
/// The size herdr shows a pane at, from `pane layout`: its rectangle, or the whole tab while it is the zoomed one.
pub fn shown_of(layout: &Value, pane: &str) -> Option<(u16, u16, bool)> {
    let panes = layout.arr("panes");
    let mine = panes.iter().find(|p| p.s("pane_id") == pane)?;
    let r = if layout.b("zoomed") && layout.s("focused_pane_id") == pane && layout.at("area").is_object() { layout.at("area") } else { mine.at("rect") };
    (r.i("width") > 0 && r.i("height") > 0).then(|| (r.i("width") as u16, r.i("height") as u16, panes.len() > 1))
}
/// Automatic screen checks must never scroll a CLI's conversation or send it navigation keys.
pub fn read_screen(h: &Herdr, pane: &str) -> String {
    h.pane_read(pane, "visible", None).unwrap_or_default()
}

//! The herdr backend: each terminal call as one herdr command line, bound to one herdr session (by socket path).
//! `None` as the socket means whatever this process would talk to.
use super::backend::TerminalBackend;
use super::cli::{args, run, run_raw, HRes, HerdrError};
use crate::json::*;

#[derive(Clone, Debug, Default)]
pub struct HerdrCli { pub socket: Option<String> }
fn envs(env: &[(String, String)]) -> Vec<String> { env.iter().flat_map(|(k, v)| ["--env".to_string(), format!("{k}={v}")]).collect() }
impl HerdrCli {
    fn c(&self, a: Vec<String>) -> HRes<Value> { run(self.socket.as_deref(), &a, 30000, "") }
    fn ct(&self, a: Vec<String>, timeout_ms: u64) -> HRes<Value> { run(self.socket.as_deref(), &a, timeout_ms, "") }
}

impl TerminalBackend for HerdrCli {
    fn name(&self) -> &'static str { "herdr" }
    fn call(&self, a: Vec<String>, timeout_ms: u64) -> HRes<Value> { self.ct(a, timeout_ms) }
    fn workspaces(&self) -> HRes<Vec<Value>> { Ok(self.c(args(&["workspace", "list"]))?.arr("workspaces").to_vec()) }
    fn workspace_get(&self, id: &str) -> Option<Value> { self.c(args(&["workspace", "get", id])).ok().map(|r| r.at("workspace").clone()).filter(|v| !v.is_null()) }
    fn workspace_create(&self, cwd: &str, label: &str, env: &[(String, String)]) -> HRes<Value> {
        let mut a = args(&["workspace", "create", "--cwd", cwd]);
        if !label.is_empty() { a.extend(args(&["--label", label])); }
        a.extend(envs(env)); a.push("--no-focus".into());
        self.c(a)
    }
    fn workspace_close(&self, id: &str) -> HRes<Value> { self.c(args(&["workspace", "close", id])) }
    fn workspace_focus(&self, id: &str) -> HRes<Value> { self.c(args(&["workspace", "focus", id])) }
    fn tab_focus(&self, id: &str) -> HRes<Value> { self.c(args(&["tab", "focus", id])) }
    fn tab_create(&self, workspace: &str, cwd: &str, label: &str, env: &[(String, String)]) -> HRes<Value> {
        let mut a = args(&["tab", "create", "--workspace", workspace]);
        if !cwd.is_empty() { a.extend(args(&["--cwd", cwd])); }
        if !label.is_empty() { a.extend(args(&["--label", label])); }
        a.extend(envs(env)); a.push("--no-focus".into());
        self.c(a)
    }
    fn tabs(&self, workspace: &str) -> HRes<Vec<Value>> { Ok(self.c(args(&["tab", "list", "--workspace", workspace]))?.arr("tabs").to_vec()) }
    fn tab_get(&self, id: &str) -> Option<Value> { self.c(args(&["tab", "get", id])).ok().map(|r| r.at("tab").clone()).filter(|v| !v.is_null()) }
    fn tab_rename(&self, id: &str, label: &str) -> Option<Value> { self.c(args(&["tab", "rename", id, label])).ok() }
    fn tab_close(&self, id: &str) -> Option<Value> { self.c(args(&["tab", "close", id])).ok() }
    fn worktree_create(&self, cwd: &str, branch: &str, base: &str, label: &str) -> HRes<Value> {
        let mut a = args(&["worktree", "create", "--cwd", cwd, "--branch", branch]);
        if !base.is_empty() { a.extend(args(&["--base", base])); }
        if !label.is_empty() { a.extend(args(&["--label", label])); }
        a.push("--no-focus".into());
        self.ct(a, 120000)
    }
    fn worktree_open(&self, cwd: &str, path: &str, label: &str) -> HRes<Value> {
        let mut a = args(&["worktree", "open", "--cwd", cwd, "--path", path]);
        if !label.is_empty() { a.extend(args(&["--label", label])); }
        a.push("--no-focus".into());
        self.ct(a, 60000)
    }
    fn worktree_remove(&self, workspace: &str, force: bool) -> HRes<Value> {
        let mut a = args(&["worktree", "remove", "--workspace", workspace]);
        if force { a.push("--force".into()); }
        self.ct(a, 120000)
    }
    fn panes(&self, workspace: Option<&str>) -> HRes<Vec<Value>> {
        let mut a = args(&["pane", "list"]);
        if let Some(w) = workspace { a.extend(args(&["--workspace", w])); }
        Ok(self.c(a)?.arr("panes").to_vec())
    }
    fn pane_get(&self, id: &str) -> Option<Value> { self.c(args(&["pane", "get", id])).ok().map(|r| r.at("pane").clone()).filter(|v| !v.is_null()) }
    fn pane_layout(&self, id: &str) -> Option<Value> { self.ct(args(&["pane", "layout", "--pane", id]), 3000).ok().map(|r| r.at("layout").clone()).filter(|v| !v.is_null()) }
    fn pane_zoom(&self, id: &str, on: bool) -> HRes<Value> { self.c(args(&["pane", "zoom", id, if on { "--on" } else { "--off" }])) }
    fn pane_split(&self, id: &str, cwd: &str, direction: &str, env: &[(String, String)]) -> HRes<Value> {
        let mut a = args(&["pane", "split", id, "--direction", direction]);
        if !cwd.is_empty() { a.extend(args(&["--cwd", cwd])); }
        a.extend(envs(env)); a.push("--no-focus".into());
        Ok(self.c(a)?.at("pane").clone())
    }
    fn process_info(&self, id: &str) -> Option<Value> { self.c(args(&["pane", "process-info", "--pane", id])).ok().map(|r| r.at("process_info").clone()) }
    fn pane_move(&self, id: &str, workspace: &str, label: &str) -> HRes<Value> {
        let mut a = args(&["pane", "move", id, "--new-tab", "--workspace", workspace]);
        if !label.is_empty() { a.extend(args(&["--label", label])); }
        a.push("--no-focus".into());
        Ok(self.c(a)?.at("move_result").clone())
    }
    fn pane_close(&self, id: &str) -> Option<Value> { self.c(args(&["pane", "close", id])).ok() }
    fn pane_run(&self, id: &str, command: &str) -> HRes<Value> { self.c(args(&["pane", "run", id, command])) }
    fn pane_read(&self, id: &str, source: &str, lines: Option<i64>) -> HRes<String> {
        let mut a = args(&["pane", "read", id, "--source", source]);
        if let Some(n) = lines { a.extend(args(&["--lines", &n.to_string()])); }
        run_raw(self.socket.as_deref(), &a, 30000, "")
    }
    fn metadata(&self, id: &str, extra: &[String]) -> Option<Value> {
        let mut a = args(&["pane", "report-metadata", id, "--source", "plugin:herdr-projects"]);
        a.extend(extra.iter().cloned());
        self.c(a).ok()
    }
    fn pane_text(&self, id: &str, text: &str) -> HRes<Value> { self.c(args(&["pane", "send-text", id, text])) }
    fn agents(&self) -> HRes<Vec<Value>> { Ok(self.c(args(&["agent", "list"]))?.arr("agents").to_vec()) }
    fn agent_get(&self, target: &str) -> HRes<Option<Value>> {
        match self.c(args(&["agent", "get", target])) {
            Ok(r) => Ok(Some(r.at("agent").clone()).filter(|v| !v.is_null())),
            Err(e) if ["agent_not_found", "not_found", "agent_not_running", "pane_not_found"].contains(&e.code.as_str()) => Ok(None),
            Err(e) => Err(e),
        }
    }
    /// A program herdr sees at work only through its own integration is never started without it (kinds.rs
    /// `needs_integration`): herdr would never say its turn ended. Its status unknown (an older herdr): it is started.
    fn integration_ready(&self, kind: &str) -> HRes<()> {
        let Some(k) = crate::agents::kinds::kind(kind).filter(|k| k.needs_integration) else { return Ok(()) };
        let status = run_raw(self.socket.as_deref(), &args(&["integration", "status"]), 15000, "").unwrap_or_default();
        let line = status.lines().find(|l| l.trim_start().starts_with(&format!("{}:", k.integration)) || l.trim_start().starts_with(&format!("{} (", k.integration)));
        if line.map(|l| l.contains("not installed")).unwrap_or(false) {
            return Err(HerdrError { code: "integration_missing".into(), message: format!("{kind} needs herdr's own integration, or herdr cannot see when it works: run `herdr integration install {}`, then try again.", k.integration) });
        }
        Ok(())
    }
    fn agent_start(&self, name: &str, kind: &str, pane: &str, extra: &[String], timeout_ms: u64) -> HRes<Value> {
        let mut a = args(&["agent", "start", name, "--kind", kind, "--pane", pane, "--timeout", &timeout_ms.to_string()]);
        if !extra.is_empty() { a.push("--".into()); a.extend(extra.iter().cloned()); }
        self.ct(a, timeout_ms + 15000).map(|r| r.at("agent").clone())
    }
    fn agent_prompt(&self, target: &str, text: &str) -> HRes<Value> { self.ct(args(&["agent", "prompt", target, text]), 60000) }
    fn agent_keys(&self, target: &str, keys: &[&str]) -> HRes<Value> { let mut a = args(&["agent", "send-keys", target]); a.extend(args(keys)); self.c(a) }
    fn agent_focus(&self, target: &str) -> HRes<Value> { self.c(args(&["agent", "focus", target])) }
    fn agent_read(&self, target: &str, source: &str, lines: Option<i64>) -> HRes<String> {
        let mut a = args(&["agent", "read", target, "--source", source]);
        if let Some(n) = lines { a.extend(args(&["--lines", &n.to_string()])); }
        run_raw(self.socket.as_deref(), &a, 60000, "")
    }
    fn open_board(&self) -> HRes<Option<String>> {
        let p = self.c(args(&["plugin", "pane", "open", "--plugin", "herdr-projects", "--entrypoint", "board", "--placement", "tab"]))?.at("plugin_pane").at("pane").clone();
        Ok(p.as_str().or(p.so("pane_id")).filter(|s| !s.is_empty()).map(String::from))
    }
    fn open_board_popup(&self, width: &str, height: &str) -> HRes<Value> {
        let mut a = args(&["plugin", "pane", "open", "--plugin", "herdr-projects", "--entrypoint", "board", "--placement", "popup"]);
        if !width.is_empty() { a.extend(args(&["--width", width])); }
        if !height.is_empty() { a.extend(args(&["--height", height])); }
        self.c(a)
    }
    fn plugins(&self) -> Vec<Value> {
        run_raw(None, &args(&["plugin", "list", "--json"]), 8000, "").ok().and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .map(|j| if j.at("result").at("plugins").is_array() { j.at("result").arr("plugins").to_vec() } else { j.arr("plugins").to_vec() }).unwrap_or_default()
    }
    fn plugin_config_dir(&self, id: &str) -> String { run_raw(None, &args(&["plugin", "config-dir", id]), 5000, "").map(|s| s.trim().to_string()).unwrap_or_default() }
    fn machines(&self) -> Vec<Value> {
        let Some(j) = run_raw(None, &args(&["machine", "list", "--json"]), 30000, "").ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { return vec![] };
        if let Some(a) = j.as_array() { return a.clone(); }
        if j.at("machines").is_array() { j.arr("machines").to_vec() } else { j.at("result").arr("machines").to_vec() }
    }
    fn take_terminal(&self, pane: &str, cols: u16, rows: u16) -> HRes<()> {
        let id = self.ct(args(&["pane", "get", pane]), 5000)?.at("pane").t("terminal_id");
        if id.is_empty() { return Err(HerdrError { code: "pane_not_found".into(), message: "herdr does not know this pane.".into() }); }
        run_raw(self.socket.as_deref(), &args(&["terminal", "session", "control", &id, "--takeover", "--cols", &cols.to_string(), "--rows", &rows.to_string()]), 5000, "")?;
        Ok(())
    }
}

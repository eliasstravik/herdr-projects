//! Panes on Rex: a pane is a block. Listing them in herdr's shape, splitting, closing, typing into one, running a
//! command at its shell, reading its screen, and the processes in it.
use super::{registry, run, Rex};
use crate::herdr::cli::{args, HRes};
use crate::json::*;

/// The shells a block shows at its prompt: no program of its own running in it.
pub const SHELLS: [&str; 6] = ["zsh", "bash", "fish", "sh", "dash", "login"];

impl Rex {
    pub fn pane_out(&self, block: &str, space: &str, window: &str, cwd: &str, fg_cwd: &str) -> Value {
        json!({ "pane_id": block, "workspace_id": space, "tab_id": window, "terminal_id": block, "cwd": cwd,
            "foreground_cwd": if fg_cwd.is_empty() { cwd } else { fg_cwd }, "agent": Value::Null, "agent_status": "unknown" })
    }
    /// The session and window of a block.
    pub fn where_is(&self, block: &str) -> HRes<(String, String)> {
        let reg = registry::read(self.srv());
        let window = reg.at("blocks").at(block).t("window");
        if let Some((_, s)) = registry::space_of_window(&reg, &window) { return Ok((s.t("session"), window)); }
        for s in self.sessions()? {
            let sid = s.t("session_id");
            if let Some(w) = self.view(&sid).ok().as_ref().map(Self::windows_of).unwrap_or_default().into_iter().find(|w| w.2.iter().any(|b| b == block)) { return Ok((sid, w.0)); }
        }
        Err(run::err("pane_not_found", "Rex has no such block"))
    }
    /// The panes of one space, or of all of them, each with its agent as herdr would show it.
    pub fn blocks(&self, space: Option<&str>) -> HRes<Vec<Value>> {
        let reg = registry::read(self.srv());
        let mut out = vec![];
        for (id, s) in reg.at("spaces").as_object().cloned().unwrap_or_default() {
            if space.is_some_and(|w| w != id) { continue; }
            let Ok(view) = self.view(s.s("session")) else { continue };
            for (window, _, blocks) in Self::windows_of(&view).into_iter().filter(|w| s.arr("windows").iter().any(|x| x.as_str() == Some(&w.0))) {
                for b in blocks {
                    let cwd = Some(reg.at("blocks").at(&b).t("cwd")).filter(|c| !c.is_empty()).unwrap_or_else(|| s.t("cwd"));
                    let mut p = self.pane_out(&b, &id, &window, &cwd, "");
                    if let Some(a) = self.agent_of_block(&reg, &b) { p["agent"] = a.at("agent").clone(); p["agent_status"] = a.at("agent_status").clone(); }
                    out.push(p);
                }
            }
        }
        Ok(out)
    }
    pub fn block_get(&self, id: &str) -> Option<Value> {
        let mut p = self.blocks(None).ok()?.into_iter().find(|p| p.s("pane_id") == id)?;
        if let Some(f) = self.processes(id) { if f.b("foreground_cwd") { p["foreground_cwd"] = f.at("foreground_cwd").clone(); } }
        Some(p)
    }
    /// A new block beside this one (herdr's directions: right, down).
    pub fn block_split(&self, id: &str, cwd: &str, direction: &str, env: &[(String, String)]) -> HRes<Value> {
        let (_, window) = self.where_is(id)?;
        let dir = match direction { "down" => "below", "up" => "above", "left" => "left", _ => "right" };
        let mut a = args(&["split", "-b", id, "--split", dir, "--focus=false", "--json"]);
        if !cwd.is_empty() { a.extend(args(&["-c", cwd])); }
        a.extend(super::layout::shell_cmd(env, &self.socket()));
        let made = run::rex_json(self.srv(), &a, 30000)?;
        let block = [made.t("block_id"), made.arr("block_ids").first().and_then(|b| b.as_str()).unwrap_or_default().to_string()].into_iter().find(|b| !b.is_empty()).unwrap_or_default();
        let reg = registry::read(self.srv());
        let space = registry::space_of_window(&reg, &window).map(|s| s.0).unwrap_or_default();
        let cwd = if cwd.is_empty() { reg.at("blocks").at(id).t("cwd") } else { cwd.to_string() };
        registry::change(self.srv(), |r| r["blocks"][&block] = json!({ "window": window, "cwd": cwd }))?;
        Ok(self.pane_out(&block, &space, &window, &cwd, ""))
    }
    pub fn block_close(&self, id: &str) -> Option<Value> {
        run::rex(self.srv(), &args(&["block", "close", id]), "", 15000).ok()?;
        let _ = registry::change(self.srv(), |r| for k in ["blocks", "agents"] { if let Some(o) = r[k].as_object_mut() { o.remove(id); } });
        Some(json!({ "type": "ok" }))
    }
    /// herdr's `process_info` from Rex's `process`: the shell, and what runs in the foreground.
    pub fn processes(&self, id: &str) -> Option<Value> {
        let p = run::block(self.srv(), id, "process", Value::Null).ok()?;
        let fg = p.at("foreground");
        let shell = p.arr("ancestors").iter().rev().find(|a| SHELLS.contains(&a.s("name")) && a.s("name") != "login").map(|a| a.at("pid").clone())
            .unwrap_or_else(|| if SHELLS.contains(&fg.s("name")) { fg.at("pid").clone() } else { Value::Null });
        Some(json!({ "shell_pid": shell, "foreground_process_group_id": fg.at("pid"), "foreground_cwd": fg.at("cwd"),
            "foreground_processes": if fg.is_object() { json!([{ "pid": fg.at("pid"), "name": fg.at("name") }]) } else { json!([]) } }))
    }
    /// Is the block's shell at its prompt (nothing of its own in the foreground)?
    pub fn at_shell(&self, id: &str) -> bool { self.processes(id).map(|p| p.arr("foreground_processes").iter().all(|f| SHELLS.contains(&f.s("name")))).unwrap_or(false) }
    pub fn type_text(&self, id: &str, text: &str) -> HRes<Value> { run::rex(self.srv(), &args(&["send", "-b", id]), text, 15000).map(|_| json!({ "type": "ok" })) }
    pub fn press(&self, id: &str, keys: &[&str]) -> HRes<Value> {
        let mut a = args(&["send-key", "-b", id]); a.extend(args(keys));
        run::rex(self.srv(), &a, "", 15000).map(|_| json!({ "type": "ok" }))
    }
    /// A command at the block's shell, refused as herdr refuses it when a program runs there.
    pub fn run_line(&self, id: &str, command: &str) -> HRes<Value> {
        if self.processes(id).is_none() { return Err(run::err("pane_not_found", "Rex has no such block")); }
        if !self.at_shell(id) { return Err(run::err("pane_busy", "a program runs in this block")); }
        self.type_text(id, command)?;
        self.press(id, &["enter"])
    }
    /// The block's screen as text: `visible` its last screenful, `recent` its last `lines` lines.
    pub fn read(&self, id: &str, source: &str, lines: Option<i64>) -> HRes<String> {
        let all = run::rex(self.srv(), &args(&["capture", "-b", id, "--trim", "--unwrap"]), "", 30000)?;
        let rows = run::block(self.srv(), id, "size", Value::Null).ok().map(|s| s.i("rows")).filter(|r| *r > 0).unwrap_or(50);
        let n = if source == "visible" { rows } else { lines.unwrap_or(rows) }.max(1) as usize;
        let ls: Vec<&str> = all.lines().collect();
        Ok(ls[ls.len().saturating_sub(n)..].join("\n"))
    }
}

//! Agents on Rex. Rex has no agents of its own: the board types the program's command line at the block's shell,
//! names the block after the agent (`<role> <task>`), and keeps its name and kind in the registry. Its status is the
//! state the program reports (turns.rs). A prompt is pasted (bracketed paste) and entered as a key of its own.
use super::{registry, run, turns, Rex};
use crate::clock::{now, sleep_ms};
use crate::herdr::cli::{args, HRes};
use crate::json::*;

pub fn shq(s: &str) -> String { if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_./=:@%+,".contains(c)) { s.into() } else { format!("'{}'", s.replace('\'', "'\\''")) } }
/// herdr's key names in Rex's (`esc` is `escape`, `C-c` is `ctrl+c`).
pub fn key(k: &str) -> String {
    match k { "esc" => "escape".into(), "return" => "enter".into(), _ => k.strip_prefix("C-").map(|c| format!("ctrl+{c}")).unwrap_or_else(|| k.to_string()) }
}

impl Rex {
    /// The block an agent runs in: the target is its block, or its name.
    pub fn block_of(&self, target: &str) -> HRes<String> {
        if target.starts_with("block:") { return Ok(target.into()); }
        let reg = registry::read(self.srv());
        reg.at("agents").as_object().and_then(|o| o.iter().find(|(_, a)| a.s("name") == target).map(|(b, _)| b.clone()))
            .ok_or_else(|| run::err("agent_not_found", format!("no agent named {target}")))
    }
    /// The agent a block runs, as herdr shows one, or None when its program is no longer there.
    pub fn agent_of_block(&self, reg: &Value, block: &str) -> Option<Value> {
        let a = reg.at("agents").at(block);
        if !a.is_object() { return None; }
        let p = self.processes(block)?;
        let fg = p.arr("foreground_processes").first().cloned().unwrap_or(Value::Null);
        if super::screen::SHELLS.contains(&fg.s("name")) && now() - a.i("started") > 15000 { return None; } // back at its shell: gone
        let ps = run::block(self.srv(), block, "program_status", Value::Null).unwrap_or(Value::Null);
        let rec = turns::newest(&ps, fg.i("pid")); // only what this program reported: a record an earlier one left is not its
        let (status, seq, comp) = turns::observe(self.srv(), block, rec.as_ref());
        let window = reg.at("blocks").at(block).t("window");
        let space = registry::space_of_window(reg, &window).map(|s| s.0).unwrap_or_default();
        let cwd = reg.at("blocks").at(block).t("cwd");
        let mut o = json!({ "agent": a.at("kind"), "agent_status": status, "name": a.at("name"), "pane_id": block, "workspace_id": space, "tab_id": window,
            "terminal_id": block, "cwd": cwd, "foreground_cwd": p.at("foreground_cwd"), "state_change_seq": seq });
        if !comp.is_null() { o["completion_seq"] = comp; }
        Some(o)
    }
    pub fn agent_one(&self, target: &str) -> HRes<Option<Value>> {
        let Ok(b) = self.block_of(target) else { return Ok(None) };
        Ok(self.agent_of_block(&registry::read(self.srv()), &b))
    }
    pub fn agent_list(&self) -> HRes<Vec<Value>> {
        let reg = registry::read(self.srv());
        Ok(reg.at("agents").as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default().iter().filter_map(|b| self.agent_of_block(&reg, b)).collect())
    }
    /// The program typed at the block's shell; ready once it runs there instead of the shell.
    pub fn start(&self, name: &str, kind: &str, pane: &str, extra: &[String], timeout_ms: u64) -> HRes<Value> {
        if !self.at_shell(pane) { return Err(run::err("pane_busy", "a program already runs in this block")); }
        let program = crate::agents::kinds::program(kind).to_string();
        let line = std::iter::once(program.clone()).chain(extra.iter().cloned()).map(|s| shq(&s)).collect::<Vec<_>>().join(" ");
        registry::change(self.srv(), |r| r["agents"][pane] = json!({ "name": name, "kind": kind, "program": program, "started": now() }))?;
        let _ = run::rex(self.srv(), &args(&["block", "rename", pane, name]), "", 15000);
        self.run_line(pane, &line)?;
        let until = now() + timeout_ms as i64;
        loop {
            if !self.at_shell(pane) { break; }
            if now() >= until { return Err(run::err("timeout", format!("{kind} did not start in this block"))); }
            sleep_ms(250);
        }
        self.agent_one(pane)?.ok_or_else(|| run::err("agent_not_found", "the agent left its block"))
    }
    /// A prompt pasted and entered. Refused while the agent waits for an answer, as herdr refuses it.
    pub fn prompt(&self, target: &str, text: &str) -> HRes<Value> {
        let b = self.block_of(target)?;
        let a = self.agent_one(&b)?.ok_or_else(|| run::err("agent_not_running", "no agent runs in this block"))?;
        if a.s("agent_status") == "blocked" { return Err(run::err("agent_blocked", "the agent waits for an answer")); }
        self.type_text(&b, &format!("\u{1b}[200~{text}\u{1b}[201~"))?;
        sleep_ms(300);
        self.press(&b, &["enter"])?;
        Ok(json!({ "type": "ok", "agent": a }))
    }
    pub fn keys(&self, target: &str, keys: &[&str]) -> HRes<Value> {
        let b = self.block_of(target)?;
        let ks: Vec<String> = keys.iter().map(|k| key(k)).collect();
        self.press(&b, &ks.iter().map(String::as_str).collect::<Vec<_>>())
    }
}

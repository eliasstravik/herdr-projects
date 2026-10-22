//! What is at work in a pane, each thing in plain words: beside the agent (`Herdr::pane_childless`'s rule) or beside the
//! shell (`Herdr::pane_idle`'s). Empty: nothing. A look for the rule stops at the first thing; `all` names every one, for
//! the words of a clean-up kept because its space is in use (`details <id>` shows them all).
use super::cli::kids;
use super::procs::{command, wiring, Wiring};
use super::Herdr;
use crate::json::*;

const UNSAID: &str = "herdr does not say what runs in it";

/// A command line made short: its program, and the first word after it that is no flag (a path by its last part).
pub fn short(argv: &[String]) -> String {
    let base = |s: &str| s.trim_end_matches('/').rsplit('/').next().unwrap_or(s).chars().take(40).collect::<String>();
    let Some(first) = argv.first() else { return String::new() };
    let mut w = base(first);
    if let Some(next) = argv.iter().skip(1).find(|a| !a.starts_with('-') && !a.is_empty()) { w.push(' '); if next.contains('/') { w.push_str("…/"); } w.push_str(&base(next)); }
    w
}
/// One process herdr lists, in words: `node …/server.js (pid 812)`.
fn listed(p: &Value) -> String {
    let argv = strings(p.at("argv"));
    let w = if argv.is_empty() { p.t("name") } else { short(&argv) };
    format!("{} (pid {})", if w.is_empty() { "a process" } else { &w }, p.i("pid"))
}
/// One process herdr does not list (a child, a job), in words.
pub fn found(pid: i64) -> String {
    let argv: Vec<String> = command(pid).map(|c| c.split_whitespace().map(String::from).collect()).unwrap_or_default();
    format!("{} (pid {pid})", if argv.is_empty() { "a process".into() } else { short(&argv) })
}

/// What runs in the pane beside the program `agent`: a process of its group that is not its helper, one in a group of
/// its own under it, a job of its shell.
pub fn beside_agent(api: &Herdr, id: &str, agent: &str, all: bool) -> Vec<String> {
    let Some(i) = api.process_info(id) else { return vec![UNSAID.into()] };
    let procs: Vec<&Value> = i.arr("foreground_processes").iter().filter(|p| crate::gate::is_int(p.at("pid"))).collect();
    let front: Vec<i64> = procs.iter().map(|p| p.i("pid")).collect();
    if front.is_empty() { return vec![UNSAID.into()]; }
    let lead = i.i("foreground_process_group_id");
    let mut out = vec![];
    for p in &procs {
        if p.i("pid") == lead || (!agent.is_empty() && p.s("name") == agent) { continue; }
        let w = wiring(p.i("pid"), &front);
        if w != Wiring::Agent { out.push(format!("{} {}", listed(p), w.words())); if !all { return out; } }
    }
    for p in &procs {
        match kids(p.i("pid")) {
            Ok(k) => for c in k.iter().filter(|c| !front.contains(c)) { out.push(format!("{} runs in a group of its own", found(*c))); if !all { return out; } },
            Err(_) => { out.push(format!("what {} runs cannot be listed", listed(p))); if !all { return out; } }
        }
    }
    if crate::gate::is_int(i.at("shell_pid")) { out.extend(shell_jobs(i.i("shell_pid"), &front, all)); }
    out
}
/// What runs in the pane beside its shell.
pub fn beside_shell(api: &Herdr, id: &str, all: bool) -> Vec<String> {
    let Some(i) = api.process_info(id) else { return vec![UNSAID.into()] };
    if !crate::gate::is_int(i.at("shell_pid")) { return vec![UNSAID.into()]; }
    let sp = i.i("shell_pid");
    let mut out: Vec<String> = i.arr("foreground_processes").iter().filter(|p| p.i("pid") != sp).map(|p| format!("{} runs in it", listed(p))).collect();
    if out.is_empty() && i.i("foreground_process_group_id") != sp { out.push("a program runs in it".into()); }
    if !all && !out.is_empty() { out.truncate(1); return out; }
    out.extend(shell_jobs(sp, &[], all));
    out
}
/// The processes its shell runs in the background (not those in `front`).
fn shell_jobs(sp: i64, front: &[i64], all: bool) -> Vec<String> {
    match kids(sp) {
        Ok(k) => k.iter().filter(|c| !front.contains(c)).take(if all { usize::MAX } else { 1 }).map(|c| format!("{} runs in the background of its shell", found(*c))).collect(),
        Err(_) => vec!["what its shell runs cannot be listed".into()],
    }
}
